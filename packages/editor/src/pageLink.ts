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
      // A section of the target page. The server and the renderer read it; without it here, the next save would drop it.
      anchor: {
        default: null,
        parseHTML: (el) => el.getAttribute("data-anchor"),
        renderHTML: (attrs) => (attrs.anchor ? { "data-anchor": attrs.anchor } : {}),
      },
    };
  },
  renderHTML({ HTMLAttributes, mark }) {
    const anchor = mark.attrs.anchor ? `#${String(mark.attrs.anchor)}` : "";
    let attributes: Record<string, unknown> = HTMLAttributes;

    if (mark.attrs.pageId) {
      const byId = pageUrlRegistry.urls[String(mark.attrs.pageId)];
      // A page that no longer exists: the published page shows plain words, the editor shows them as a link to fix, never the stale
      // address saved next to the ID.
      attributes = byId
        ? { ...HTMLAttributes, href: `${byId}${anchor}` }
        : { ...HTMLAttributes, href: null, "data-broken": "", title: "This page no longer exists" };
    } else if (anchor && HTMLAttributes.href) {
      attributes = { ...HTMLAttributes, href: `${String(HTMLAttributes.href)}${anchor}` };
    }

    return this.parent?.({ mark, HTMLAttributes: attributes }) ?? ["a", attributes, 0];
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
