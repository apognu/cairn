import type { Editor } from "@tiptap/core";
import { NodeSelection } from "@tiptap/pm/state";
import { useEditorState } from "@tiptap/react";
import { componentsByName } from "@yourdocs/components";
import { propsSchema } from "@yourdocs/sdk";
import { createPortal } from "react-dom";
import { SchemaForm } from "./SchemaForm";

/** The settings of the component the cursor is in, as a form generated from its props schema. */
export function PropsPanel({ editor, host }: { editor: Editor; host: HTMLElement }) {
  const active = useEditorState({
    editor,
    selector: ({ editor: e }) => {
      const { selection } = e.state;
      // A selected atom (feature tabs, what's new) is the component itself.
      if (selection instanceof NodeSelection && componentsByName.has(selection.node.type.name)) {
        return { name: selection.node.type.name, attrs: selection.node.attrs as Record<string, unknown> };
      }

      const { $from } = selection;

      for (let depth = $from.depth; depth > 0; depth--) {
        const node = $from.node(depth);
        if (componentsByName.has(node.type.name))
          return { name: node.type.name, attrs: node.attrs as Record<string, unknown> };
      }
      return null;
    },
  });

  const component = active && componentsByName.get(active.name);
  if (!active || !component) return null;

  return createPortal(
    <>
      <div className="props-head">
        <h2>{component.title}</h2>
        <span className="mono muted">
          {component.name} · v{component.version}
        </span>
      </div>
      <p className="muted">
        {component.name.includes("/") ? "Custom" : "Built-in"} component · form generated from its schema
      </p>
      <SchemaForm
        schema={propsSchema(component)}
        value={active.attrs}
        // Not `.focus()`: the form owns the focus while it is being edited.
        onChange={(name, next) =>
          editor
            .chain()
            .updateAttributes(active.name, { [name]: next })
            .run()
        }
      />
    </>,
    host,
  );
}
