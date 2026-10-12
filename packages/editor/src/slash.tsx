import { type Editor, Extension, type Range } from "@tiptap/core";
import { ReactRenderer } from "@tiptap/react";
import Suggestion, { type SuggestionKeyDownProps, type SuggestionProps } from "@tiptap/suggestion";
import { forwardRef, useEffect, useImperativeHandle, useState } from "react";

export interface SlashItem {
  title: string;
  hint: string;
  keywords: string;
  run: (editor: Editor, range: Range) => void;
}

const item = (title: string, hint: string, keywords: string, run: SlashItem["run"]): SlashItem => ({
  title,
  hint,
  keywords,
  run,
});

// Every command first removes the typed "/query".
export const slashItems: SlashItem[] = [
  item("Heading 2", "Section title", "h2 heading title", (e, r) =>
    e.chain().focus().deleteRange(r).setNode("heading", { level: 2 }).run(),
  ),
  item("Heading 3", "Subsection title", "h3 heading subtitle", (e, r) =>
    e.chain().focus().deleteRange(r).setNode("heading", { level: 3 }).run(),
  ),
  item("Bulleted list", "A list of points", "ul bullet list", (e, r) =>
    e.chain().focus().deleteRange(r).toggleBulletList().run(),
  ),
  item("Numbered list", "Steps in order", "ol numbered list", (e, r) =>
    e.chain().focus().deleteRange(r).toggleOrderedList().run(),
  ),
  item("Quote", "Set a passage apart", "blockquote quote", (e, r) =>
    e.chain().focus().deleteRange(r).toggleBlockquote().run(),
  ),
  item("Code block", "Code with a language and file name", "code snippet pre", (e, r) =>
    e.chain().focus().deleteRange(r).setCodeBlock().run(),
  ),
  item("Callout", "A highlighted note, tip or warning", "callout note tip warning aside admonition", (e, r) =>
    e
      .chain()
      .focus()
      .deleteRange(r)
      .insertContent({ type: "callout", attrs: { tone: "note" }, content: [{ type: "paragraph" }] })
      .run(),
  ),
  item("Table", "Rows and columns", "table grid", (e, r) =>
    e.chain().focus().deleteRange(r).insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(),
  ),
  item("Divider", "A horizontal rule", "hr divider rule line", (e, r) =>
    e.chain().focus().deleteRange(r).setHorizontalRule().run(),
  ),
];

export function filterSlashItems(query: string): SlashItem[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return slashItems.filter((i) => words.every((w) => `${i.title} ${i.keywords}`.toLowerCase().includes(w)));
}

type ListHandle = { onKeyDown: (event: KeyboardEvent) => boolean };

const SlashList = forwardRef<ListHandle, SuggestionProps<SlashItem, SlashItem>>(function SlashList(
  { items, command },
  ref,
) {
  const [active, setActive] = useState(0);

  // biome-ignore lint/correctness/useExhaustiveDependencies: the highlighted row restarts at the top whenever the list changes
  useEffect(() => setActive(0), [items]);

  useImperativeHandle(ref, () => ({
    onKeyDown(event) {
      if (items.length === 0) return false;
      if (event.key === "ArrowDown") setActive((a) => (a + 1) % items.length);
      else if (event.key === "ArrowUp") setActive((a) => (a + items.length - 1) % items.length);
      else if (event.key === "Enter" || event.key === "Tab") command(items[active]);
      else return false;
      return true;
    },
  }));

  if (items.length === 0) return <div className="slash-menu muted">No matching block</div>;

  return (
    <div
      className="slash-menu"
      role="listbox"
      aria-label="Insert a block"
      aria-activedescendant={`slash-${active}`}
      tabIndex={-1}
    >
      {items.map((it, i) => (
        <button
          type="button"
          role="option"
          id={`slash-${i}`}
          key={it.title}
          aria-selected={i === active}
          // Mouse down, not click: a click would first move focus out of the editor and close the menu.
          onMouseDown={(e) => {
            e.preventDefault();
            command(it);
          }}
        >
          <strong>{it.title}</strong>
          <span className="muted">{it.hint}</span>
        </button>
      ))}
    </div>
  );
});

function place(element: HTMLElement, props: SuggestionProps<SlashItem, SlashItem>) {
  const rect = props.clientRect?.();
  if (!rect) return;
  element.style.left = `${rect.left}px`;
  element.style.top = `${rect.bottom + 4}px`;
}

/** Typing "/" at the start of a block or after a space opens a menu of blocks to insert. */
export const SlashCommand = Extension.create({
  name: "slashCommand",
  addProseMirrorPlugins() {
    return [
      Suggestion<SlashItem, SlashItem>({
        editor: this.editor,
        char: "/",
        allowSpaces: true,
        // Code blocks hold literal text: a slash there is just a slash.
        allow: ({ state, range }) => !state.doc.resolve(range.from).parent.type.spec.code,
        items: ({ query }) => filterSlashItems(query),
        command: ({ editor, range, props }) => props.run(editor, range),
        render: () => {
          let renderer: ReactRenderer<ListHandle, SuggestionProps<SlashItem, SlashItem>> | null = null;

          return {
            onStart(props) {
              renderer = new ReactRenderer(SlashList, { props, editor: props.editor });
              renderer.element.classList.add("slash-popup");
              document.body.append(renderer.element);
              place(renderer.element, props);
            },
            onUpdate(props) {
              renderer?.updateProps(props);
              if (renderer) place(renderer.element, props);
            },
            onKeyDown({ event }: SuggestionKeyDownProps) {
              if (event.key === "Escape") {
                renderer?.destroy();
                renderer = null;
                return true;
              }
              return renderer?.ref?.onKeyDown(event) ?? false;
            },
            onExit() {
              renderer?.destroy();
              renderer = null;
            },
          };
        },
      }),
    ];
  },
});
