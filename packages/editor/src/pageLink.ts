import { Extension, InputRule } from "@tiptap/core";
import Link from "@tiptap/extension-link";

/** What the editor shows as the address of a link by page ID. The canvas keeps it current. */
export const pageUrlRegistry: { urls: Record<string, string> } = { urls: {} };

/** A link mark that can point at a page by ID: moves and renames never break it. */
export const PageLink = Link.extend({
  addAttributes() {
    return {
      ...this.parent?.(),
      pageId: {
        default: null,
        parseHTML: (el) => el.getAttribute("data-page-id"),
        renderHTML: (attrs) => (attrs.pageId ? { "data-page-id": attrs.pageId } : {}),
      },
    };
  },
  renderHTML({ HTMLAttributes, mark }) {
    const byId = mark.attrs.pageId ? pageUrlRegistry.urls[String(mark.attrs.pageId)] : undefined;
    return (
      this.parent?.({ mark, HTMLAttributes: byId ? { ...HTMLAttributes, href: byId } : HTMLAttributes }) ?? [
        "a",
        HTMLAttributes,
        0,
      ]
    );
  },
}).configure({ openOnClick: false });

export type PickerRequest =
  /** Link the selected words. */
  | { mode: "selection"; from: number; to: number }
  /** Insert a link with the page's title at the cursor, replacing the `[[` just before it if there is one. */
  | { mode: "insert"; from: number; to: number };

export const PICKER_EVENT = "yourdocs:page-picker";

function open(view: { dom: HTMLElement }, request: PickerRequest) {
  view.dom.dispatchEvent(new CustomEvent<PickerRequest>(PICKER_EVENT, { detail: request }));
}

/** Opens the page picker: ⌘K on a selection (or at the cursor), or by typing `[[`. */
export const PagePickerTriggers = Extension.create({
  name: "pagePickerTriggers",
  addKeyboardShortcuts() {
    return {
      "Mod-k": ({ editor }) => {
        const { from, to, empty } = editor.state.selection;
        open(editor.view, { mode: empty ? "insert" : "selection", from, to });
        return true;
      },
    };
  },
  addInputRules() {
    return [
      new InputRule({
        find: /\[\[$/,
        handler: () => {
          // Returning nothing lets the second bracket be typed. The picker removes both once something is chosen,
          // so Escape leaves them as typed. It opens after that typing has happened.
          setTimeout(() => {
            const { from, to } = this.editor.state.selection;
            open(this.editor.view, { mode: "insert", from, to });
          });
        },
      }),
    ];
  },
});
