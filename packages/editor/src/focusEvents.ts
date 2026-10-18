import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";

/**
 * Tiptap's own focus events, with the focus sent a moment later. On iOS and Android, Tiptap's `focus` command focuses the page while its
 * transaction is still open, so the focus transaction went first. When that one changed the document (a page ending with a list gets its
 * empty last paragraph then), the command's transaction no longer applied ("Applying a mismatched transaction") and the editor went
 * blank. Replaces the core `focusEvents`, turned off where this is used.
 */
export const DeferredFocusEvents = Extension.create({
  name: "deferredFocusEvents",

  addProseMirrorPlugins() {
    const { editor } = this;

    return [
      new Plugin({
        key: new PluginKey("deferredFocusEvents"),
        props: {
          handleDOMEvents: {
            focus: (view, event) => {
              editor.isFocused = true;
              queueMicrotask(() => {
                if (editor.isDestroyed) return;
                view.dispatch(editor.state.tr.setMeta("focus", { event }).setMeta("addToHistory", false));
              });

              return false;
            },
            blur: (view, event) => {
              editor.isFocused = false;
              view.dispatch(editor.state.tr.setMeta("blur", { event }).setMeta("addToHistory", false));

              return false;
            },
          },
        },
      }),
    ];
  },
});
