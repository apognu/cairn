import { componentsByName } from "@cairn/components";
import { propsSchema } from "@cairn/sdk";
import type { Editor } from "@tiptap/core";
import { NodeSelection } from "@tiptap/pm/state";
import { useEditorState } from "@tiptap/react";
import { useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { CodeBlockFields } from "./CodeBlockFields";
import { SchemaForm } from "./SchemaForm";
import { useModal } from "./sidePanel";
import { useNarrow } from "./useNarrow";

type Active =
  | { kind: "component"; name: string; attrs: Record<string, unknown>; at: number }
  | { kind: "image" | "codeBlock"; attrs: Record<string, unknown>; at: number };

/** The settings of the component, image or code block the cursor is in: a form generated from the props schema, or fixed fields. */
export function PropsPanel({ editor, host }: { editor: Editor; host: HTMLElement }) {
  const active = useEditorState({
    editor,
    selector: ({ editor: e }): Active | null => {
      const { selection } = e.state;
      if (selection instanceof NodeSelection) {
        const { node } = selection;
        // A selected atom (feature tabs, what's new) is the component itself.
        if (componentsByName.has(node.type.name))
          return { kind: "component", name: node.type.name, attrs: node.attrs, at: selection.from };
        if (node.type.name === "image") return { kind: "image", attrs: node.attrs, at: selection.from };
      }

      const { $from } = selection;

      for (let depth = $from.depth; depth > 0; depth--) {
        const node = $from.node(depth);
        if (componentsByName.has(node.type.name))
          return { kind: "component", name: node.type.name, attrs: node.attrs, at: $from.before(depth) };
        if (node.type.name === "codeBlock") return { kind: "codeBlock", attrs: node.attrs, at: $from.before(depth) };
      }
      return null;
    },
  });
  const narrow = useNarrow();

  if (!active) return null;

  const body =
    active.kind === "component" ? (
      <ComponentFields editor={editor} active={active} />
    ) : active.kind === "image" ? (
      <ImageFields editor={editor} attrs={active.attrs} />
    ) : (
      <CodeBlockFields editor={editor} attrs={active.attrs} />
    );

  // Too narrow for a column: the same fields open in a modal drawer instead.
  return narrow ? <PanelDrawer key={active.at}>{body}</PanelDrawer> : createPortal(body, host);
}

function ComponentFields({ editor, active }: { editor: Editor; active: Extract<Active, { kind: "component" }> }) {
  const component = componentsByName.get(active.name);
  if (!component) return null;

  return (
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
    </>
  );
}

function ImageFields({ editor, attrs }: { editor: Editor; attrs: Record<string, unknown> }) {
  const alt = String(attrs.alt ?? "");
  const caption = String(attrs.caption ?? "");
  const decorative = attrs.decorative === true;
  const set = (name: "alt" | "caption" | "decorative", value: string | boolean) =>
    editor
      .chain()
      .updateAttributes("image", { [name]: value })
      .run();

  return (
    <>
      <div className="props-head">
        <h2>Image</h2>
      </div>
      <div className="form-field">
        <label>
          Alt text
          <input value={alt} disabled={decorative} onChange={(e) => set("alt", e.target.value)} />
        </label>
        <label className="form-check">
          <input type="checkbox" checked={decorative} onChange={(e) => set("decorative", e.target.checked)} />
          Decorative
        </label>
        {!alt.trim() && !decorative && (
          <span role="alert" className="alt-warning">
            Describe this image for people who cannot see it, or mark it decorative.
          </span>
        )}
      </div>
      <div className="form-field">
        <label>
          Caption
          <input value={caption} onChange={(e) => set("caption", e.target.value)} />
        </label>
      </div>
    </>
  );
}

/**
 * A modal drawer from the right, opened from a floating button: opening it by itself would take the focus from the text being typed.
 * Selecting something else (a new `key`) closes it.
 */
function PanelDrawer({ children }: { children: React.ReactNode }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const id = useId();
  const [open, setOpen] = useState(false);

  useModal(dialog, open);

  if (!open)
    return (
      <button type="button" className="props-open" onClick={() => setOpen(true)}>
        Block settings
      </button>
    );

  return createPortal(
    <dialog ref={dialog} className="drawer props-drawer" aria-labelledby={`${id}-title`} onClose={() => setOpen(false)}>
      <header className="drawer-head">
        <span id={`${id}-title`} className="sr-only">
          Settings
        </span>
        <button type="button" className="icon" aria-label="Close settings" onClick={() => dialog.current?.close()}>
          ×
        </button>
      </header>
      {children}
    </dialog>,
    document.body,
  );
}
