import { RenderEnvContext } from "@cairn/components";
import {
  collectHeadings,
  type DocNode,
  defaultContext,
  migrateDoc,
  type RenderContext,
  renderNodes,
  SiteLayout,
} from "@cairn/renderer";
import { EditorContent, useEditor } from "@tiptap/react";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { BubbleMenus } from "./BubbleMenus";
import { buildExtensions } from "./extensions";
import { PagePicker } from "./PagePicker";
import { PropsPanel } from "./PropsPanel";
import { PICKER_EVENT, type PickerRequest, pageUrlRegistry } from "./pageLink";
import { inHost, popoverHost } from "./popover";
import { useLibrary } from "./queries";
import { setGlossary } from "./termSuggestion";
import { insertImages, useUploads } from "./upload";

interface Props {
  doc: DocNode;
  context?: Partial<RenderContext>;
  editable?: boolean;
  /** Leave out the site header and navigation (the editor already has its own). */
  embedded?: boolean;
  onChange?: (doc: DocNode) => void;
  /** The document as the editor holds it once loaded, before any edit. */
  onReady?: (doc: DocNode) => void;
  onBlur?: () => void;
  /** Where the settings panel of the component under the cursor is drawn. */
  panelHost?: HTMLElement | null;
  /** In place of the page's title: the field to rename it. */
  title?: ReactNode;
  /** Drawn under the title in place of the description: the field to write it. */
  lead?: ReactNode;
}

/** The page body as ProseMirror, mounted where the published page puts its content. */
export function Canvas({
  doc,
  context,
  editable = true,
  embedded = false,
  onChange,
  onReady,
  onBlur,
  panelHost,
  title,
  lead,
}: Props) {
  const editor = useEditor({
    extensions: buildExtensions(editable),
    // A page saved by an older version of a component is upgraded as it opens.
    content: migrateDoc(doc),
    editable,
    onCreate: ({ editor: e }) => onReady?.(e.getJSON() as DocNode),
    onUpdate: ({ editor: e }) => onChange?.(e.getJSON() as DocNode),
    onBlur: () => onBlur?.(),
    editorProps: {
      // Pasted or dropped pictures are uploaded and inserted where they landed.
      handlePaste: (_view, event) => {
        const files = Array.from(event.clipboardData?.files ?? []).filter((f) => f.type.startsWith("image/"));
        if (files.length === 0 || !editorRef.current) return false;
        void insertImages(editorRef.current, files);
        return true;
      },
      handleDrop: (view, event) => {
        const files = Array.from(event.dataTransfer?.files ?? []).filter((f) => f.type.startsWith("image/"));
        if (files.length === 0 || !editorRef.current) return false;
        const at = view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos;
        void insertImages(editorRef.current, files, at);
        return true;
      },
    },
  });
  // Opening a page puts the cursor at the start of it, ready to type, unless the user has already moved on to a field of their own
  // (the editor loads a moment after the click that opened the page). `data-ready` says it is done, for the end-to-end tests: a click
  // made before would be moved back to the start.
  const [ready, setReady] = useState(false);
  useEffect(() => {
    if (!editor || !editable) return;
    const active = document.activeElement;
    const typing =
      active instanceof HTMLInputElement ||
      active instanceof HTMLTextAreaElement ||
      active instanceof HTMLSelectElement;
    // The caret starts at the top too: from the end, the first arrow key would scroll the whole page down to it.
    if (!typing) editor.commands.focus("start", { scrollIntoView: false });
    editor.view.dom.closest(".canvas")?.scrollTo({ top: 0 });
    setReady(true);
  }, [editor, editable]);

  const editorRef = useRef(editor);
  editorRef.current = editor;
  // `host` is the pane that scrolls the canvas: the picker is placed in it, so it moves with the words it links.
  const [picker, setPicker] = useState<{
    request: PickerRequest;
    x: number;
    y: number;
    host: HTMLElement | null;
  } | null>(null);
  const library = useLibrary();
  const uploads = useUploads();
  const [headings, setHeadings] = useState(() => collectHeadings(doc).list);

  useEffect(() => {
    setGlossary((library.data ?? []).filter((e) => e.kind === "term").map((e) => ({ key: e.key, aliases: e.aliases })));
  }, [library.data]);

  useEffect(() => {
    if (!editor) return;

    const update = () => setHeadings(collectHeadings(editor.getJSON() as DocNode).list);
    editor.on("update", update);

    return () => {
      editor.off("update", update);
    };
  }, [editor]);

  useEffect(() => {
    if (!editor || !editable) return;

    const dom = editor.view.dom;
    const onRequest = (event: Event) => {
      const request = (event as CustomEvent<PickerRequest>).detail;
      const at = editor.view.coordsAtPos(request.from);
      const host = popoverHost(dom);
      const { left, top } = host ? inHost(host, at.left, at.bottom + 6) : { left: at.left, top: at.bottom + 6 };
      setPicker({ request, x: left, y: top, host });
    };
    dom.addEventListener(PICKER_EVENT, onRequest);

    return () => {
      dom.removeEventListener(PICKER_EVENT, onRequest);
    };
  }, [editor, editable]);

  useEffect(() => {
    if (!editor) return;

    const dom = editor.view.dom;
    // In the canvas a link card, a feature card or a linked word is something to select and edit, not to follow: a plain click only
    // selects it (and opens its settings). Cmd/Ctrl-click or the middle button opens the target in a new tab: a page of the site in
    // the editor, since its address only exists on the published site, and any other address as it is. This holds read-only too: the
    // editor's own address serves the site only with the self-hosted target.
    const onLink = (event: MouseEvent) => {
      const link = event.target instanceof Element ? event.target.closest("a[href]") : null;
      if (!link || event.button === 2) return;
      event.preventDefault();
      if (event.button === 0 && !event.metaKey && !event.ctrlKey && !event.shiftKey) return;

      const href = link.getAttribute("href") ?? "";
      if (/^[a-z][a-z0-9+.-]*:/i.test(href)) {
        window.open(href, "_blank", "noopener");
        return;
      }
      const path = href.split("#")[0].replace(/\/$/, "");
      const page = Object.entries(pageUrlRegistry.urls).find(([, url]) => url.replace(/\/$/, "") === path)?.[0];
      if (page) window.open(`${window.location.pathname}#page=${page}`, "_blank", "noopener");
    };
    dom.addEventListener("click", onLink, true);
    dom.addEventListener("auxclick", onLink, true);

    return () => {
      dom.removeEventListener("click", onLink, true);
      dom.removeEventListener("auxclick", onLink, true);
    };
  }, [editor]);

  // The editor shows "new until" dates against today's date, as the publish render does with its own.
  const full = { ...defaultContext, today: new Date().toISOString().slice(0, 10), ...context };
  // Links by page ID show the page's current URL, whichever page moved since they were made.
  pageUrlRegistry.urls = full.urls;

  // What the library holds, as the editor draws it: a variable's value, a snippet's blocks.
  const entries = library.data ?? [];
  const variables = Object.fromEntries(
    entries.filter((e) => e.kind === "variable").map((e) => [e.key, e.text_value ?? ""]),
  );
  const snippets = Object.fromEntries(
    entries
      .filter((e) => e.kind === "snippet" && e.doc)
      .map((e) => [
        e.key,
        renderNodes((e.doc as DocNode).content, { urls: full.urls, assets: {}, anchors: new WeakMap() }),
      ]),
  );

  const linkTo = (link: { href: string; pageId: string | null; text: string; anchor?: string }) => {
    if (!editor || !picker) return;
    const { request } = picker;
    const attrs = { href: link.href, pageId: link.pageId, anchor: link.anchor ?? null };
    const chain = editor.chain().focus();

    if (request.mode === "selection") {
      chain.setTextSelection({ from: request.from, to: request.to }).setMark("link", attrs).run();
    } else {
      // Typing `[[` leaves the brackets just before the cursor; they were only the way in.
      const typed = editor.state.doc.textBetween(Math.max(0, request.from - 2), request.from) === "[[";
      chain
        .insertContentAt(
          { from: typed ? request.from - 2 : request.from, to: request.to },
          { type: "text", text: link.text, marks: [{ type: "link", attrs }] },
        )
        .run();
    }
    setPicker(null);
  };

  return (
    <RenderEnvContext.Provider
      value={{ urls: full.urls, pages: full.pages, variables, snippets, editing: editable, today: full.today }}
    >
      <SiteLayout context={full} headings={headings} embedded={embedded} title={title} lead={lead}>
        {/* Positioned: the block handle is placed against it. */}
        <EditorContent editor={editor} className="relative" data-ready={ready || undefined} />
        {editor && editable && <BubbleMenus editor={editor} />}
        {editable && uploads > 0 && (
          <output className="fixed z-40 bottom-5 left-1/2 -translate-x-1/2 flex items-center gap-2 py-2 px-3.5 border border-input rounded-lg bg-bg shadow-popover text-ui leading-[1.4]">
            <span
              className="w-3.5 h-3.5 border-2 border-input border-t-accent rounded-full animate-[spin_0.8s_linear_infinite] motion-reduce:[animation-duration:3s]"
              aria-hidden="true"
            />
            {uploads === 1 ? "Uploading image…" : `Uploading ${uploads} images…`}
          </output>
        )}
        {editor && editable && panelHost && <PropsPanel editor={editor} host={panelHost} />}
        {picker &&
          createPortal(
            <PagePicker
              nearPage={full.page.id}
              style={{ position: picker.host ? "absolute" : undefined, left: picker.x, top: picker.y }}
              initialUrl={String(editor?.getAttributes("link").href ?? "")}
              onClose={() => {
                setPicker(null);
                editor?.commands.focus();
              }}
              operations
              onPage={(page, operation) =>
                linkTo({
                  href: full.urls[page.id] ?? "",
                  pageId: page.id,
                  text: operation?.label ?? page.title,
                  anchor: operation?.anchor,
                })
              }
              onUrl={(url) => linkTo({ href: url, pageId: null, text: url })}
            />,
            picker.host ?? document.body,
          )}
      </SiteLayout>
    </RenderEnvContext.Provider>
  );
}
