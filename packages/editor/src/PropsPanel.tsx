import { componentsByName } from "@cairn/components";
import { propsSchema } from "@cairn/sdk";
import type { Editor } from "@tiptap/core";
import { NodeSelection } from "@tiptap/pm/state";
import { useEditorState } from "@tiptap/react";
import { useEffect, useId } from "react";
import { createPortal } from "react-dom";
import { CodeBlockFields } from "./CodeBlockFields";
import { Icon } from "./icons";
import { PropsHead } from "./PropsHead";
import { SchemaForm } from "./SchemaForm";
import { useSideDrawer } from "./sidePanel";

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
  const drawer = useSideDrawer();

  if (!active) return null;

  const body =
    active.kind === "component" ? (
      <ComponentFields editor={editor} active={active} />
    ) : active.kind === "image" ? (
      <ImageFields editor={editor} attrs={active.attrs} />
    ) : (
      <CodeBlockFields editor={editor} attrs={active.attrs} />
    );

  const label =
    active.kind === "component"
      ? `${componentsByName.get(active.name)?.title ?? "Block"} settings`
      : active.kind === "image"
        ? "Image settings"
        : "Code block settings";

  // The fields always go in the side column; when it is a drawer (narrow window), a button over the page opens it.
  return (
    <>
      {createPortal(body, host)}
      {drawer && <DrawerOpener label={label} editor={editor} open={drawer.open} />}
    </>
  );
}

function ComponentFields({ editor, active }: { editor: Editor; active: Extract<Active, { kind: "component" }> }) {
  const component = componentsByName.get(active.name);
  if (!component) return null;

  return (
    <>
      <PropsHead
        title={component.title}
        description={component.description}
        // Custom components are versioned by whoever wrote them, who may need to know which one a page uses.
        detail={component.name.includes("/") ? `${component.name} · v${component.version}` : undefined}
      />
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
  const id = useId();
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
      <PropsHead title="Image" description="A picture in the page, resized for every screen when published." />
      {typeof attrs.assetId === "string" && (
        <div className="props-preview">
          <img src={`/_a/${attrs.assetId}`} alt="" />
        </div>
      )}
      <div className="form-field">
        <label htmlFor={`${id}-alt`}>Alt text</label>
        <textarea
          id={`${id}-alt`}
          rows={3}
          value={alt}
          disabled={decorative}
          placeholder={decorative ? "Not needed for a decorative image" : "What the image shows"}
          onChange={(e) => set("alt", e.target.value)}
        />
        <small className="muted">Read aloud by screen readers, and shown when the image cannot load.</small>
      </div>
      <div className="form-field">
        <label className="form-check">
          <input type="checkbox" checked={decorative} onChange={(e) => set("decorative", e.target.checked)} />
          Decorative
        </label>
        <small className="muted">It adds nothing to the text, so screen readers skip it.</small>
      </div>
      {!alt.trim() && !decorative && (
        <p role="alert" className="props-warning">
          <Icon name="info" size={16} />
          Describe this image for people who cannot see it, or mark it decorative. Publishing waits until then.
        </p>
      )}
      <div className="form-field">
        <label htmlFor={`${id}-caption`}>Caption</label>
        <textarea id={`${id}-caption`} rows={2} value={caption} onChange={(e) => set("caption", e.target.value)} />
        <small className="muted">Shown under the image. Optional.</small>
      </div>
    </>
  );
}

/**
 * Opens the side column's drawer on a narrow window, for the selected block: a floating button named after it, Alt+Enter from the text,
 * or a double click on the block. Nothing opens by itself: that would take the focus from the text being typed.
 */
function DrawerOpener({ label, editor, open }: { label: string; editor: Editor; open: () => void }) {
  useEffect(() => {
    const dom = editor.view.dom;
    const onKey = (event: KeyboardEvent) => {
      if (event.altKey && event.key === "Enter" && !event.ctrlKey && !event.metaKey && !event.shiftKey) {
        event.preventDefault();
        open();
      }
    };
    // Double-clicking a component or an image opens its settings; inside a component's own text, a double click still selects a word.
    const onDoubleClick = (event: MouseEvent) => {
      const target = event.target instanceof Element ? event.target : null;
      const onBlock = target?.closest("[data-node-view-wrapper]") && !target.closest("[data-node-view-content]");
      if (onBlock || target?.tagName === "IMG") open();
    };
    dom.addEventListener("keydown", onKey);
    dom.addEventListener("dblclick", onDoubleClick);

    return () => {
      dom.removeEventListener("keydown", onKey);
      dom.removeEventListener("dblclick", onDoubleClick);
    };
  }, [editor, open]);

  return (
    <button type="button" className="props-open" aria-keyshortcuts="Alt+Enter" onClick={open}>
      <Icon name="sliders" size={16} />
      {label}
      <span className="key" aria-hidden="true">
        Alt ↵
      </span>
    </button>
  );
}
