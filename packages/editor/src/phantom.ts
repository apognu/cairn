import { Extension } from "@tiptap/core";
import { Plugin, PluginKey, TextSelection } from "@tiptap/pm/state";
import { Decoration, DecorationSet, type EditorView } from "@tiptap/pm/view";

const TEXT = "Write content here…";

/**
 * A dimmed "Write content here…" line at the end of the page, so authors see where to click. Only decorations: the document is left as
 * it is, since an empty paragraph would publish as a blank line. A page that already ends in an empty paragraph (the one kept after a
 * final table or tabs) shows the text in it; any other page gets a line after its last block that adds that paragraph when clicked.
 */
export const PhantomLine = Extension.create({
  name: "phantomLine",
  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("phantomLine"),
        props: {
          // The gap between the last block and the line is the editor's own box, and next to a non-editable last child the browser
          // puts no caret there: the click would leave the cursor where it was. It goes to the end of the page instead, as it did
          // before the line was drawn.
          handleDOMEvents: {
            mousedown: (view, event) => {
              const blocks = [...view.dom.children].filter((el) => el.getAttribute("contenteditable") !== "false");
              const lastBlock = blocks.at(-1);
              if (event.target !== view.dom || !lastBlock || event.clientY < lastBlock.getBoundingClientRect().bottom)
                return false;

              event.preventDefault();
              view.dispatch(view.state.tr.setSelection(TextSelection.atEnd(view.state.doc)).scrollIntoView());
              view.focus();
              return true;
            },
          },
          decorations: (state) => {
            const { doc } = state;
            const last = doc.lastChild;
            if (!last) return null;

            if (last.type.name === "paragraph" && last.content.size === 0) {
              const from = doc.content.size - last.nodeSize;
              // With the cursor in it, the text would push the cursor to its right: the line is plain and empty while written in.
              if (state.selection.from > from) return null;
              return DecorationSet.create(doc, [
                Decoration.node(from, doc.content.size, { class: "phantom-line", "data-placeholder": TEXT }),
              ]);
            }

            const widget = (view: EditorView) => {
              const line = document.createElement("p");
              line.className = "phantom-line";
              line.dataset.placeholder = TEXT;
              line.contentEditable = "false";
              // Mouse down, not click: by the click the editor would already have put the cursor somewhere else.
              line.addEventListener("mousedown", (event) => {
                event.preventDefault();
                const end = view.state.doc.content.size;
                const tr = view.state.tr.insert(end, view.state.schema.nodes.paragraph.create());
                view.dispatch(tr.setSelection(TextSelection.create(tr.doc, end + 1)).scrollIntoView());
                view.focus();
              });
              return line;
            };

            return DecorationSet.create(doc, [
              Decoration.widget(doc.content.size, widget, { side: 1, key: "phantom-line", ignoreSelection: true }),
            ]);
          },
        },
      }),
    ];
  },
});
