import { Extension } from "@tiptap/core";
import type { Node as PmNode } from "@tiptap/pm/model";
import { Plugin, PluginKey, TextSelection } from "@tiptap/pm/state";
import { Decoration, DecorationSet, type EditorView } from "@tiptap/pm/view";
import * as ui from "./ui";

/** The step around the cursor, when the cursor is in one of its paragraphs (not deeper, in a list or a callout of its body). */
function stepAt(view: { state: EditorView["state"] }) {
  const { $from, empty } = view.state.selection;
  if (!empty || $from.depth < 2 || $from.parent.type.name !== "paragraph") return null;
  const step = $from.node(-1);
  if (step.type.name !== "step") return null;

  return { $from, step, index: $from.index(-1), stepPos: $from.before(-1), stepIndex: $from.index(-2) };
}

const newStep = (view: EditorView) =>
  view.state.schema.nodes.step.create(null, view.state.schema.nodes.paragraph.create());

/**
 * Adding and removing steps. Enter at the end of a step's title starts the next step, and on an empty last line (or an empty step)
 * leaves the block, as in a list, so a body can still hold several paragraphs; Backspace at the start of a step's title merges it into
 * the step before. Editing only: the button is not part of the page.
 */
export const StepsEditing = Extension.create({
  name: "stepsEditing",
  // Before the editor's own Enter and Backspace, which would only add or join paragraphs inside the step.
  priority: 1000,

  addKeyboardShortcuts() {
    return {
      Enter: ({ editor }) => {
        const at = stepAt(editor.view);
        if (!at) return false;
        const { view } = editor;
        const { $from, step } = at;
        const stepEnd = at.stepPos + step.nodeSize;
        const empty = $from.parent.content.size === 0;

        // At the end of a title: the next step, after this one's body.
        if (at.index === 0 && !empty && $from.parentOffset === $from.parent.content.size) {
          const tr = view.state.tr.insert(stepEnd, newStep(view));
          view.dispatch(tr.setSelection(TextSelection.create(tr.doc, stepEnd + 2)).scrollIntoView());
          return true;
        }
        if (!empty || at.index !== step.childCount - 1) return false;

        // An empty last line, or an empty step: out of the block, onto a new paragraph after it. The empty line goes; an empty step goes
        // with it, and the whole block when it was the only step.
        const steps = $from.node(-2);
        const stepsPos = $from.before(-2);
        const after = stepsPos + steps.nodeSize;
        const tr = view.state.tr;
        if (step.childCount > 1) tr.delete($from.before(), $from.after());
        else if (steps.childCount === 1) tr.delete(stepsPos, after);
        else tr.delete(at.stepPos, stepEnd);

        const paragraph = tr.mapping.map(after);
        tr.insert(paragraph, view.state.schema.nodes.paragraph.create());
        view.dispatch(tr.setSelection(TextSelection.create(tr.doc, paragraph + 1)).scrollIntoView());
        return true;
      },

      Backspace: ({ editor }) => {
        const at = stepAt(editor.view);
        if (!at || at.index !== 0 || at.$from.parentOffset !== 0 || at.stepIndex === 0) return false;
        const { view } = editor;

        // An empty step is removed, and the cursor goes to the end of the step before; one with text joins that step, its title
        // becoming a paragraph of the step's body.
        if (at.step.childCount === 1 && at.step.firstChild?.content.size === 0) {
          const tr = view.state.tr.delete(at.stepPos, at.stepPos + at.step.nodeSize);
          view.dispatch(tr.setSelection(TextSelection.near(tr.doc.resolve(at.stepPos - 1), -1)).scrollIntoView());
          return true;
        }
        view.dispatch(view.state.tr.join(at.stepPos).scrollIntoView());
        return true;
      },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("stepsEditing"),
        props: {
          // An "Add step" button under the last step of every steps block.
          decorations: (state) => {
            const widgets: Decoration[] = [];
            state.doc.descendants((node: PmNode, pos: number) => {
              if (node.type.name !== "steps") return true;
              const end = pos + node.nodeSize - 1;
              widgets.push(
                Decoration.widget(end, addButton, { side: 1, key: `add-step-${pos}`, ignoreSelection: true }),
              );
              return false;
            });

            return DecorationSet.create(state.doc, widgets);
          },
        },
      }),
    ];
  },
});

function addButton(view: EditorView, getPos: () => number | undefined) {
  const item = document.createElement("li");
  item.className = "steps-add";
  item.contentEditable = "false";

  const button = document.createElement("button");
  button.type = "button";
  button.className = `${ui.smallButton} text-ui text-muted hover:text-text`;
  button.textContent = "+ Add step";
  // Mouse down, not click: the click would first move the cursor out of the step the new one follows.
  button.addEventListener("mousedown", (event) => event.preventDefault());
  button.addEventListener("click", () => {
    const end = getPos();
    if (end === undefined) return;
    const tr = view.state.tr.insert(end, newStep(view));
    view.dispatch(tr.setSelection(TextSelection.create(tr.doc, end + 2)).scrollIntoView());
    view.focus();
  });
  item.append(button);

  return item;
}
