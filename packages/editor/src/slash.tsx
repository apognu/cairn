import { customComponents } from "@cairn/components/custom";
import { sampleProps } from "@cairn/sdk";
import { type Editor, Extension, type Range } from "@tiptap/core";
import { ReactRenderer } from "@tiptap/react";
import Suggestion, { type SuggestionKeyDownProps, type SuggestionProps } from "@tiptap/suggestion";
import { Fragment, forwardRef, useEffect, useImperativeHandle, useState } from "react";
import { Icon, type IconName } from "./icons";
import { inHost, popoverHost } from "./popover";
import * as ui from "./ui";
import { pickAndInsertImages } from "./upload";

export interface SlashItem {
  group: "Components" | "Reusable" | "Basic";
  icon: IconName;
  title: string;
  hint: string;
  keywords: string;
  run: (editor: Editor, range: Range) => void;
}

const item = (
  group: SlashItem["group"],
  icon: IconName,
  title: string,
  hint: string,
  keywords: string,
  run: SlashItem["run"],
): SlashItem => ({
  group,
  icon,
  title,
  hint,
  keywords,
  run,
});

// Every command first removes the typed "/query".
export const slashItems: SlashItem[] = [
  item("Basic", "heading", "Heading 2", "Section title", "h2 heading title", (e, r) =>
    e.chain().focus().deleteRange(r).setNode("heading", { level: 2 }).run(),
  ),
  item("Basic", "heading", "Heading 3", "Subsection title", "h3 heading subtitle", (e, r) =>
    e.chain().focus().deleteRange(r).setNode("heading", { level: 3 }).run(),
  ),
  item("Basic", "list", "Bulleted list", "A list of points", "ul bullet list", (e, r) =>
    e.chain().focus().deleteRange(r).toggleBulletList().run(),
  ),
  item("Basic", "numbered", "Numbered list", "Steps in order", "ol numbered list", (e, r) =>
    e.chain().focus().deleteRange(r).toggleOrderedList().run(),
  ),
  item("Basic", "quote", "Quote", "Set a passage apart", "blockquote quote", (e, r) =>
    e.chain().focus().deleteRange(r).toggleBlockquote().run(),
  ),
  item("Basic", "code", "Code block", "Code with a language and file name", "code snippet pre", (e, r) =>
    e.chain().focus().deleteRange(r).setCodeBlock().run(),
  ),
  item(
    "Components",
    "info",
    "Callout",
    "A highlighted note, tip or warning",
    "callout note tip warning aside admonition",
    (e, r) =>
      e
        .chain()
        .focus()
        .deleteRange(r)
        .insertContent({ type: "callout", attrs: { tone: "note" }, content: [{ type: "paragraph" }] })
        .run(),
  ),
  item("Components", "tabs", "Feature tabs", "Cards grouped into tabs", "features tabs cards grid", (e, r) =>
    e
      .chain()
      .focus()
      .deleteRange(r)
      .insertContent({
        type: "featureTabs",
        attrs: {
          families: [
            {
              label: "Family",
              features: [
                { icon: "rocket", title: "Feature", description: "What it does", page: undefined, newUntil: undefined },
              ],
            },
          ],
        },
      })
      .run(),
  ),
  item("Components", "sparkle", "What's new", "Recently added features", "new whats new features changelog", (e, r) =>
    e
      .chain()
      .focus()
      .deleteRange(r)
      .insertContent({
        type: "whatsNew",
        attrs: { title: "What's new", items: [{ icon: "star", title: "Feature", description: "What changed" }] },
      })
      .run(),
  ),
  item("Components", "tabs", "Tabs", "Content in switchable panels", "tabs panels switch", (e, r) =>
    e
      .chain()
      .focus()
      .deleteRange(r)
      .insertContent({
        type: "tabs",
        content: ["First", "Second"].map((label) => ({
          type: "tabPanel",
          attrs: { label },
          content: [{ type: "paragraph" }],
        })),
      })
      .run(),
  ),
  item("Components", "code", "Code group", "The same code in several languages", "code group tabs languages", (e, r) =>
    e
      .chain()
      .focus()
      .deleteRange(r)
      .insertContent({
        type: "tabs",
        attrs: { variant: "code" },
        content: [
          { label: "npm", language: "bash" },
          { label: "yarn", language: "bash" },
        ].map(({ label, language }) => ({
          type: "tabPanel",
          attrs: { label },
          content: [{ type: "codeBlock", attrs: { language } }],
        })),
      })
      .run(),
  ),
  item("Components", "numbered", "Steps", "A numbered procedure", "steps procedure numbered how to", (e, r) =>
    e
      .chain()
      .focus()
      .deleteRange(r)
      .insertContent({
        type: "steps",
        content: ["First step", "Second step"].map((title) => ({
          type: "step",
          content: [{ type: "paragraph", content: [{ type: "text", text: title }] }, { type: "paragraph" }],
        })),
      })
      .run(),
  ),
  item("Components", "grid", "Cards", "A grid of cards with icons and links", "cards grid icons links", (e, r) =>
    e.chain().focus().deleteRange(r).insertContent({ type: "cards", attrs: {} }).run(),
  ),
  item("Components", "table", "File tree", "Folders and files", "file tree folders files directory", (e, r) =>
    e.chain().focus().deleteRange(r).insertContent({ type: "fileTree", attrs: {} }).run(),
  ),
  item("Components", "link", "Link card", "A card for one page", "link card page", (e, r) =>
    e.chain().focus().deleteRange(r).insertContent({ type: "linkCard", attrs: {} }).run(),
  ),
  item("Components", "info", "See also", "Links to related pages", "see also related links callout", (e, r) =>
    e
      .chain()
      .focus()
      .deleteRange(r)
      .insertContent({ type: "seeAlso", attrs: { pages: [] } })
      .run(),
  ),
  item("Components", "grid", "Related pages", "Cards linking to other pages", "related pages cards", (e, r) =>
    e
      .chain()
      .focus()
      .deleteRange(r)
      .insertContent({ type: "relatedPages", attrs: { title: "Related", pages: [] } })
      .run(),
  ),
  item(
    "Reusable",
    "link",
    "Snippet",
    "Insert shared content, e.g. Prerequisites",
    "snippet shared reusable include",
    (e, r) =>
      e
        .chain()
        .focus()
        .deleteRange(r)
        .insertContent({ type: "snippet", attrs: { key: "" } })
        .run(),
  ),
  item("Reusable", "code", "Variable", "A value such as the latest version", "variable value reusable", (e, r) =>
    e
      .chain()
      .focus()
      .deleteRange(r)
      .insertContent({ type: "variable", attrs: { key: "" } })
      .run(),
  ),
  item("Basic", "image", "Image", "Upload a picture", "image picture photo upload figure", (e, r) => {
    e.chain().focus().deleteRange(r).run();
    pickAndInsertImages(e);
  }),
  item("Basic", "table", "Table", "Rows and columns", "table grid", (e, r) =>
    e.chain().focus().deleteRange(r).insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(),
  ),
  item("Basic", "divider", "Divider", "A horizontal rule", "hr divider rule line", (e, r) =>
    e.chain().focus().deleteRange(r).setHorizontalRule().run(),
  ),
];

// Components come first, as in the menu design; the sort is stable, so each group keeps its order.
const groupOrder: SlashItem["group"][] = ["Components", "Reusable", "Basic"];

// The deployer's components, from the folder `build-components` bundled. A new one starts with props that satisfy its schema.
slashItems.push(
  ...customComponents.map((component) =>
    item(
      "Components",
      "grid",
      component.title,
      component.description ?? "Custom component",
      `${component.name} ${component.keywords ?? ""} custom`,
      (e, r) =>
        e
          .chain()
          .focus()
          .deleteRange(r)
          .insertContent({ type: component.name, attrs: sampleProps(component) })
          .run(),
    ),
  ),
);

export function filterSlashItems(query: string): SlashItem[] {
  const text = query.toLowerCase().trim();
  const words = text.split(/\s+/).filter(Boolean);
  // Within a group, an item whose title contains what was typed comes before one that only matches its keywords.
  const rank = (i: SlashItem) => (i.title.toLowerCase().includes(text) ? 0 : 1);

  return slashItems
    .filter((i) => words.every((w) => `${i.title} ${i.keywords}`.toLowerCase().includes(w)))
    .sort((a, b) => groupOrder.indexOf(a.group) - groupOrder.indexOf(b.group) || rank(a) - rank(b));
}

type ListHandle = { onKeyDown: (event: KeyboardEvent) => boolean };

const SlashList = forwardRef<ListHandle, SuggestionProps<SlashItem, SlashItem>>(function SlashList(
  { items, command },
  ref,
) {
  const [active, setActive] = useState(0);

  // biome-ignore lint/correctness/useExhaustiveDependencies: the highlighted row restarts at the top whenever the list changes
  useEffect(() => setActive(0), [items]);
  // The list scrolls: the option the arrows reach stays in view.
  useEffect(() => document.getElementById(`slash-${active}`)?.scrollIntoView({ block: "nearest" }), [active]);

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

  if (items.length === 0)
    return <div className={`${ui.cursorMenu} w-[393px] py-2 px-3 text-muted`}>No matching block</div>;

  return (
    <div
      className={`${ui.cursorMenu} w-[393px] p-1.5`}
      role="listbox"
      aria-label="Insert a block"
      aria-activedescendant={`slash-${active}`}
      tabIndex={-1}
    >
      {items.map((it, i) => (
        <Fragment key={it.title}>
          {items[i - 1]?.group !== it.group && (
            <div
              className="pt-2 px-2.5 pb-1 text-[11.5px] font-semibold tracking-[0.06em] uppercase text-muted"
              role="presentation"
            >
              {it.group}
            </div>
          )}
          <button
            type="button"
            role="option"
            className={ui.cursorOption}
            id={`slash-${i}`}
            aria-selected={i === active}
            // Mouse down, not click: a click would first move focus out of the editor and close the menu.
            onMouseDown={(e) => {
              e.preventDefault();
              command(it);
            }}
          >
            <span className="flex flex-none items-center justify-center w-8.5 h-8.5 border border-input rounded-lg bg-bg text-body group-aria-selected/option:border-accent group-aria-selected/option:text-accent">
              <Icon name={it.icon} />
            </span>
            <span className="flex flex-col">
              <strong>{it.title}</strong>
              <span className="text-muted text-[12.5px]">{it.hint}</span>
            </span>
          </button>
        </Fragment>
      ))}
      <div
        className="sticky -bottom-1.5 flex justify-between gap-3 mt-1 pt-2 px-2.5 pb-0.5 border-t border-border bg-bg text-label text-muted"
        aria-hidden="true"
      >
        <span>↑↓ to move</span>
        <span>↵ to insert</span>
        <span className="ml-auto">Type to filter</span>
      </div>
    </div>
  );
});

const MENU_HEIGHT = 476;
const MENU_WIDTH = 393;
const MARGIN = 8;

/**
 * Below the cursor, or above it when there is more room there; never past the edges of the pane that scrolls the text (`host`), or of
 * the window without one. In a pane the menu is positioned in its scrolled content, so it moves with the text.
 */
export function place(
  element: HTMLElement,
  rect: DOMRect | null | undefined,
  height = MENU_HEIGHT,
  width = MENU_WIDTH,
  host: HTMLElement | null = null,
) {
  if (!rect) return;

  // The room around the text is what the pane shows, or the window outside one.
  const view = host?.getBoundingClientRect() ?? new DOMRect(0, 0, window.innerWidth, window.innerHeight);
  const below = view.bottom - rect.bottom - MARGIN;
  const above = rect.top - view.top - MARGIN;
  const up = below < Math.min(height, 280) && above > below;
  const menu = element.firstElementChild;
  const x = Math.max(view.left + MARGIN, Math.min(rect.left, view.right - width - MARGIN));
  const y = up ? rect.top - 4 : rect.bottom + 4;
  const at = host ? inHost(host, x, y) : { left: x, top: y };

  element.style.position = host ? "absolute" : "fixed";
  element.style.left = `${at.left}px`;
  element.style.top = `${at.top}px`;
  // Opening upwards, the menu ends where the text starts, whatever its height.
  element.style.transform = up ? "translateY(-100%)" : "";
  if (menu instanceof HTMLElement) menu.style.maxHeight = `${Math.min(height, up ? above : below)}px`;
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
          // Inside the pane that scrolls the canvas, so the menu moves with the typed text.
          let host: HTMLElement | null = null;

          return {
            onStart(props) {
              renderer = new ReactRenderer(SlashList, { props, editor: props.editor });
              renderer.element.classList.add("fixed", "z-20");
              host = popoverHost(props.editor.view.dom);
              (host ?? document.body).append(renderer.element);
              place(renderer.element, props.clientRect?.(), MENU_HEIGHT, MENU_WIDTH, host);
            },
            onUpdate(props) {
              renderer?.updateProps(props);
              if (renderer) place(renderer.element, props.clientRect?.(), MENU_HEIGHT, MENU_WIDTH, host);
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
