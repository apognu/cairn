import type { Editor } from "@tiptap/core";
import { closeHistory } from "@tiptap/pm/history";
import type { Node as PmNode } from "@tiptap/pm/model";
import { TextSelection } from "@tiptap/pm/state";
import { useEditorState } from "@tiptap/react";
import { useEffect, useId, useRef, useState } from "react";
import { PropsHead } from "./PropsHead";
import * as ui from "./ui";

/** The tabs of the tab group (or code group) at `at`: their labels, order and number. */
export function TabsFields({ editor, at }: { editor: Editor; at: number }) {
  const id = useId();
  const tabs = useEditorState({
    editor,
    selector: ({ editor: e }) => {
      const node = e.state.doc.nodeAt(at);
      if (node?.type.name !== "tabs") return null;
      return {
        code: node.attrs.variant === "code",
        labels: childrenOf(node).map((panel) => String(panel.attrs.label ?? "")),
      };
    },
  });
  const inputs = useRef<(HTMLInputElement | null)[]>([]);
  // After adding a tab, its label field takes the focus, ready to name it.
  const [focus, setFocus] = useState<number | null>(null);

  useEffect(() => {
    if (focus === null) return;
    inputs.current[focus]?.select();
    setFocus(null);
  }, [focus]);

  if (!tabs) return null;

  const panels = () => {
    const node = editor.state.doc.nodeAt(at);
    return node ? childrenOf(node) : [];
  };
  // Every change rebuilds the group from its panels, and shows the tab `show` with the cursor in it.
  // Not `.focus()`: the form keeps the focus.
  const rebuild = (list: PmNode[], show: number) => {
    const node = editor.state.doc.nodeAt(at);
    if (!node) return;

    // Its own undo step, even right after another change.
    const tr = closeHistory(editor.state.tr).replaceWith(at, at + node.nodeSize, node.type.create(node.attrs, list));
    const start = at + 1 + list.slice(0, show).reduce((size, panel) => size + panel.nodeSize, 0);
    editor.view.dispatch(tr.setSelection(TextSelection.near(tr.doc.resolve(start + 1))));
  };
  const rename = (index: number, label: string) => {
    const list = panels();
    const start = at + 1 + list.slice(0, index).reduce((size, panel) => size + panel.nodeSize, 0);
    editor.view.dispatch(editor.state.tr.setNodeMarkup(start, undefined, { ...list[index].attrs, label }));
  };
  const move = (from: number, to: number) => {
    const list = panels();
    list.splice(to, 0, ...list.splice(from, 1));
    rebuild(list, to);
  };
  const remove = (index: number) => {
    const list = panels().filter((_, i) => i !== index);
    rebuild(list, Math.min(index, list.length - 1));
  };
  const add = () => {
    const list = panels();
    const { schema } = editor.state;
    // A code group's new tab holds code in the language of the last one, as the tabs beside it do.
    const last = list[list.length - 1]?.firstChild;
    const content = tabs.code
      ? schema.nodes.codeBlock.create(last?.type.name === "codeBlock" ? { language: last.attrs.language } : null)
      : schema.nodes.paragraph.create();
    const label = tabs.code ? "Language" : `Tab ${list.length + 1}`;

    rebuild([...list, schema.nodes.tabPanel.create({ label }, content)], list.length);
    setFocus(list.length);
  };

  return (
    <>
      <PropsHead
        title={tabs.code ? "Code group" : "Tabs"}
        description={
          tabs.code
            ? "The same code in several languages, one tab each."
            : "Content in panels that readers switch between. The first tab shows when the page opens."
        }
      />
      <fieldset className="mt-0 mx-0 mb-3.5 min-w-0 p-0 border-0">
        <legend className={`${ui.fieldLabel} p-0 mb-1.5`}>Tabs</legend>
        {tabs.labels.map((label, i) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: tabs are positional, and two can share a label
          <div key={i} className="flex items-center gap-1.5 mb-2">
            <label htmlFor={`${id}-${i}`} className="sr-only">
              Label of tab {i + 1}
            </label>
            <input
              id={`${id}-${i}`}
              ref={(input) => {
                inputs.current[i] = input;
              }}
              className={ui.rowInput}
              value={label}
              placeholder="Label"
              onChange={(e) => rename(i, e.target.value)}
            />
            <button
              type="button"
              className={ui.itemButton}
              aria-label={`Move tab ${label || i + 1} up`}
              disabled={i === 0}
              onClick={() => move(i, i - 1)}
            >
              ↑
            </button>
            <button
              type="button"
              className={ui.itemButton}
              aria-label={`Move tab ${label || i + 1} down`}
              disabled={i === tabs.labels.length - 1}
              onClick={() => move(i, i + 1)}
            >
              ↓
            </button>
            <button
              type="button"
              className={ui.itemButton}
              aria-label={`Remove tab ${label || i + 1}`}
              // A group needs one tab at least; deleting the whole group is the block menu's Delete.
              disabled={tabs.labels.length === 1}
              onClick={() => remove(i)}
            >
              ×
            </button>
          </div>
        ))}
        <button
          type="button"
          className="h-8 px-2.5 border border-dashed border-input rounded-md bg-bg cursor-pointer"
          onClick={add}
        >
          + Add tab
        </button>
      </fieldset>
    </>
  );
}

const childrenOf = (node: PmNode) => Array.from({ length: node.childCount }, (_, i) => node.child(i));
