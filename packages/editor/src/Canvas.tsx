import { EditorContent, useEditor } from "@tiptap/react";
import { RenderEnvContext } from "@yourdocs/components";
import { collectHeadings, type DocNode, defaultContext, type RenderContext, SiteLayout } from "@yourdocs/renderer";
import { useEffect, useRef, useState } from "react";
import { BubbleMenus } from "./BubbleMenus";
import { buildExtensions } from "./extensions";
import { PagePicker } from "./PagePicker";
import { PropsPanel } from "./PropsPanel";
import { PICKER_EVENT, type PickerRequest, pageUrlRegistry } from "./pageLink";
import { insertImages } from "./upload";

interface Props {
  doc: DocNode;
  context?: Partial<RenderContext>;
  editable?: boolean;
  /** Leave out the site header and navigation (the editor already has its own). */
  embedded?: boolean;
  onChange?: (doc: DocNode) => void;
  onBlur?: () => void;
  /** Where the settings panel of the component under the cursor is drawn. */
  panelHost?: HTMLElement | null;
}

/** The page body as ProseMirror, mounted where the published page puts its content. */
export function Canvas({ doc, context, editable = true, embedded = false, onChange, onBlur, panelHost }: Props) {
  const editor = useEditor({
    extensions: buildExtensions(editable),
    content: doc,
    editable,
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
  const editorRef = useRef(editor);
  editorRef.current = editor;
  const [picker, setPicker] = useState<{ request: PickerRequest; x: number; y: number } | null>(null);
  const [headings, setHeadings] = useState(() => collectHeadings(doc).list);

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
      setPicker({ request, x: at.left, y: at.bottom + 6 });
    };
    dom.addEventListener(PICKER_EVENT, onRequest);

    return () => dom.removeEventListener(PICKER_EVENT, onRequest);
  }, [editor, editable]);

  // The editor shows "new until" dates against today's date, as the publish render does with its own.
  const full = { ...defaultContext, today: new Date().toISOString().slice(0, 10), ...context };
  // Links by page ID show the page's current URL, whichever page moved since they were made.
  pageUrlRegistry.urls = full.urls;

  const linkTo = (link: { href: string; pageId: string | null; text: string }) => {
    if (!editor || !picker) return;
    const { request } = picker;
    const attrs = { href: link.href, pageId: link.pageId };
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
    <RenderEnvContext.Provider value={{ urls: full.urls, pages: full.pages, editing: editable, today: full.today }}>
      <SiteLayout context={full} headings={headings} embedded={embedded}>
        <EditorContent editor={editor} />
        {editor && editable && <BubbleMenus editor={editor} />}
        {editor && editable && panelHost && <PropsPanel editor={editor} host={panelHost} />}
        {picker && (
          <PagePicker
            nearPage={full.page.id}
            style={{ left: picker.x, top: picker.y }}
            initialUrl={String(editor?.getAttributes("link").href ?? "")}
            onClose={() => {
              setPicker(null);
              editor?.commands.focus();
            }}
            onPage={(page) => linkTo({ href: full.urls[page.id] ?? "", pageId: page.id, text: page.title })}
            onUrl={(url) => linkTo({ href: url, pageId: null, text: url })}
          />
        )}
      </SiteLayout>
    </RenderEnvContext.Provider>
  );
}
