import { collectHeadings, type DocNode } from "@cairn/renderer";
import type { Editor } from "@tiptap/core";
import { TextSelection } from "@tiptap/pm/state";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { BLOCK_MENU_EVENT, type BlockMenuRequest, startOf } from "./blockHandle";
import { inHost, popoverHost } from "./popover";
import * as ui from "./ui";

/** What a text block can become; lists and quotes take the whole block along. */
const kinds = [
  { label: "Paragraph", is: (n: string, level: number) => n === "paragraph" && level === 0 },
  { label: "Heading 2", is: (n: string, level: number) => n === "heading" && level === 2 },
  { label: "Heading 3", is: (n: string, level: number) => n === "heading" && level === 3 },
  { label: "Bulleted list", is: (n: string) => n === "bulletList" },
  { label: "Numbered list", is: (n: string) => n === "orderedList" },
  { label: "Quote", is: (n: string) => n === "blockquote" },
] as const;
type Kind = (typeof kinds)[number]["label"];

const convertible = new Set(["paragraph", "heading", "bulletList", "orderedList", "blockquote"]);

/** The menu of a top-level block, opened from its handle: duplicate, delete, turn into another kind, copy a heading's link. */
export function BlockMenu({ editor, liveUrl }: { editor: Editor; liveUrl: string | null | undefined }) {
  const [open, setOpen] = useState<{ index: number; x: number; y: number; host: HTMLElement | null } | null>(null);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const dom = editor.view.dom;
    const onRequest = (event: Event) => {
      const { index, left, bottom } = (event as CustomEvent<BlockMenuRequest>).detail;
      const host = popoverHost(dom);
      const at = host ? inHost(host, left, bottom + 4) : { left, top: bottom + 4 };
      setOpen({ index, x: at.left, y: at.top, host });
    };
    dom.addEventListener(BLOCK_MENU_EVENT, onRequest);

    return () => dom.removeEventListener(BLOCK_MENU_EVENT, onRequest);
  }, [editor]);

  // Too close to the bottom of the window, it opens above the handle instead.
  useLayoutEffect(() => {
    const box = root.current?.getBoundingClientRect();
    if (!open || !box || box.bottom <= window.innerHeight - 8) return;
    setOpen({ ...open, y: Math.max(0, open.y - box.height - 32) });
  }, [open]);

  // Like every popover: a click elsewhere or Escape closes it; the first item takes the focus, the arrows move between items.
  useEffect(() => {
    if (!open) return;
    root.current?.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
    const outside = (event: PointerEvent) => {
      if (event.target instanceof Node && !root.current?.contains(event.target)) setOpen(null);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(null);
        editor.commands.focus();
      }
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", onKey);

    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", onKey);
    };
  }, [open, editor]);

  if (!open) return null;
  const { doc } = editor.state;
  if (open.index >= doc.childCount) return null;

  const node = doc.child(open.index);
  const pos = startOf(doc, open.index);
  const level = node.type.name === "heading" ? Number(node.attrs.level) : 0;
  const json = editor.getJSON() as DocNode;
  const heading = level > 0 ? json.content?.[open.index] : undefined;
  const anchor = heading && collectHeadings(json).byNode.get(heading);

  const done = () => {
    setOpen(null);
    editor.commands.focus();
  };
  const duplicate = () => {
    // Without IDs, UniqueID gives the copy and everything in it new ones; it only renumbers some kinds of duplicates on its own.
    editor
      .chain()
      .focus()
      .insertContentAt(pos + node.nodeSize, withoutIds(node.toJSON()))
      .run();
    done();
  };
  const remove = () => {
    editor
      .chain()
      .focus()
      .deleteRange({ from: pos, to: pos + node.nodeSize })
      .run();
    done();
  };
  const turnInto = (kind: Kind) => {
    // The whole block is selected, so a list or quote is converted as a whole, not only its first line.
    const chain = editor
      .chain()
      .focus()
      .command(({ tr }) => {
        tr.setSelection(TextSelection.between(tr.doc.resolve(pos + 1), tr.doc.resolve(pos + node.nodeSize - 1)));
        return true;
      })
      .clearNodes();
    if (kind === "Paragraph") chain.setParagraph();
    else if (kind === "Heading 2") chain.setHeading({ level: 2 });
    else if (kind === "Heading 3") chain.setHeading({ level: 3 });
    else if (kind === "Bulleted list") chain.toggleBulletList();
    else if (kind === "Numbered list") chain.toggleOrderedList();
    else chain.toggleBlockquote();
    chain.run();
    done();
  };
  const copyLink = () => {
    if (liveUrl && anchor) void navigator.clipboard.writeText(`${liveUrl}#${anchor}`);
    done();
  };

  const onArrows = (event: React.KeyboardEvent) => {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    const items = [...(root.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
    const at = items.indexOf(document.activeElement as HTMLButtonElement);
    items[(at + (event.key === "ArrowDown" ? 1 : items.length - 1)) % items.length]?.focus();
  };

  const item = `${ui.menuItem} flex items-center justify-between gap-4 disabled:opacity-55 disabled:cursor-default`;
  const menu = (
    <div
      ref={root}
      role="menu"
      aria-label="Block actions"
      className="absolute z-20 flex flex-col min-w-48 p-1 border border-input rounded-lg bg-bg shadow-popover text-ui leading-[1.4] whitespace-nowrap"
      style={{ left: open.x, top: open.y, position: open.host ? "absolute" : "fixed" }}
      onKeyDown={onArrows}
    >
      <button type="button" role="menuitem" className={item} onClick={duplicate}>
        Duplicate
      </button>
      {level > 0 && (
        <button
          type="button"
          role="menuitem"
          className={item}
          disabled={!liveUrl}
          title={liveUrl ? undefined : "The page is not published yet"}
          onClick={copyLink}
        >
          Copy link
        </button>
      )}
      <button type="button" role="menuitem" className={`${item} text-danger`} onClick={remove}>
        Delete
      </button>
      {convertible.has(node.type.name) && (
        <>
          <hr className={ui.menuRule} />
          <span className="px-2.5 pt-1 pb-0.5 text-label text-muted">Turn into</span>
          {kinds.map((kind) => {
            const current = kind.is(node.type.name, level);
            return (
              <button
                key={kind.label}
                type="button"
                role="menuitemradio"
                aria-checked={current}
                className={item}
                onClick={() => (current ? done() : turnInto(kind.label))}
              >
                {kind.label}
                {current && <span className="text-accent">✓</span>}
              </button>
            );
          })}
        </>
      )}
    </div>
  );

  return createPortal(menu, open.host ?? document.body);
}

function withoutIds(json: Record<string, unknown>): Record<string, unknown> {
  const attrs = json.attrs as Record<string, unknown> | undefined;
  const content = json.content as Record<string, unknown>[] | undefined;

  return {
    ...json,
    ...(attrs && "id" in attrs ? { attrs: { ...attrs, id: null } } : {}),
    ...(content ? { content: content.map(withoutIds) } : {}),
  };
}
