import { Extension } from "@tiptap/core";
import { TextSelection } from "@tiptap/pm/state";

/**
 * Enter on an empty last line of a tab leaves the tabs, onto a new paragraph after them, as in a list or steps. A tab panel is
 * isolating, so the editor's own Enter would only add lines to it, and a page with another block right below the tabs gave no way out.
 */
export const TabsEditing = Extension.create({
  name: "tabsEditing",
  // Before the editor's own Enter, which would add an empty paragraph to the tab.
  priority: 1000,

  addKeyboardShortcuts() {
    return {
      Enter: ({ editor }) => {
        const { $from, empty } = editor.state.selection;
        if (!empty || $from.depth < 3 || $from.parent.type.name !== "paragraph" || $from.parent.content.size > 0)
          return false;
        const panel = $from.node(-1);
        if (panel.type.name !== "tabPanel" || $from.index(-1) !== panel.childCount - 1) return false;

        const tabs = $from.node(-2);
        const after = $from.before(-2) + tabs.nodeSize;
        const tr = editor.state.tr;
        // The empty line goes, unless it is all the tab holds: a tab cannot be empty.
        if (panel.childCount > 1) tr.delete($from.before(), $from.after());

        const paragraph = tr.mapping.map(after);
        tr.insert(paragraph, editor.schema.nodes.paragraph.create());
        editor.view.dispatch(tr.setSelection(TextSelection.create(tr.doc, paragraph + 1)).scrollIntoView());
        return true;
      },
    };
  },
});
