import { componentsByName } from "@cairn/components";
import { propsSchema } from "@cairn/sdk";
import type { Editor } from "@tiptap/core";
import { NodeSelection } from "@tiptap/pm/state";
import { useEditorState } from "@tiptap/react";
import { useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { CodeBlockFields } from "./CodeBlockFields";
import { ImageEditor } from "./ImageEditor";
import { Icon } from "./icons";
import { openMediaPicker, placeImage } from "./MediaPicker";
import { type Asset, useAsset } from "./media";
import { PropsHead } from "./PropsHead";
import { SchemaForm } from "./SchemaForm";
import { useSideDrawer } from "./sidePanel";
import { TabsFields } from "./TabsFields";
import * as ui from "./ui";

type Active =
  | { kind: "component"; name: string; attrs: Record<string, unknown>; at: number }
  | { kind: "image" | "codeBlock"; attrs: Record<string, unknown>; at: number };

/**
 * The settings of the component, image or code block the cursor is in: a form generated from the props schema, or fixed fields. In a tab
 * group, its tabs follow: in a code group the cursor is always in a code block, which would leave them out of reach.
 */
export function PropsPanel({ editor, host }: { editor: Editor; host: HTMLElement }) {
  const found = useEditorState({
    editor,
    selector: ({ editor: e }): { active: Active | null; tabsAt: number | null } | null => {
      const { selection } = e.state;
      if (selection instanceof NodeSelection) {
        const { node } = selection;
        // A selected atom (feature tabs, what's new) is the component itself.
        if (componentsByName.has(node.type.name))
          return {
            active: { kind: "component", name: node.type.name, attrs: node.attrs, at: selection.from },
            tabsAt: null,
          };
        if (node.type.name === "image")
          return { active: { kind: "image", attrs: node.attrs, at: selection.from }, tabsAt: null };
      }

      const { $from } = selection;
      let active: Active | null = null;
      let tabsAt: number | null = null;

      for (let depth = $from.depth; depth > 0; depth--) {
        const node = $from.node(depth);
        if (!active && componentsByName.has(node.type.name))
          active = { kind: "component", name: node.type.name, attrs: node.attrs, at: $from.before(depth) };
        else if (!active && node.type.name === "codeBlock")
          active = { kind: "codeBlock", attrs: node.attrs, at: $from.before(depth) };
        else if (tabsAt === null && node.type.name === "tabs") tabsAt = $from.before(depth);
      }
      return active || tabsAt !== null ? { active, tabsAt } : null;
    },
  });
  const drawer = useSideDrawer();

  if (!found) return null;
  const { active, tabsAt } = found;

  const fields = !active ? null : active.kind === "component" ? (
    <ComponentFields editor={editor} active={active} />
  ) : active.kind === "image" ? (
    <ImageFields editor={editor} attrs={active.attrs} at={active.at} />
  ) : (
    <CodeBlockFields editor={editor} attrs={active.attrs} />
  );
  const body = (
    <>
      {fields}
      {tabsAt !== null && (
        <div className={fields ? "mt-7" : undefined}>
          <TabsFields editor={editor} at={tabsAt} />
        </div>
      )}
    </>
  );

  const label = !active
    ? "Tabs settings"
    : active.kind === "component"
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

  const schema = propsSchema(component);
  const parsed = component.props.safeParse(active.attrs);
  // Settings the schema refuses would stop the publish: say so here, next to the fields, naming the field.
  const problems = parsed.success
    ? []
    : parsed.error.issues.map((issue) => {
        const [name] = issue.path;
        const label = typeof name === "string" ? (schema.properties?.[name]?.title ?? name) : undefined;
        return label ? `${label}: ${issue.message}` : issue.message;
      });

  return (
    <>
      <PropsHead
        title={component.title}
        description={component.description}
        // Custom components are versioned by whoever wrote them, who may need to know which one a page uses.
        detail={component.name.includes("/") ? `${component.name} · v${component.version}` : undefined}
      />
      <SchemaForm
        schema={schema}
        value={active.attrs}
        // Not `.focus()`: the form owns the focus while it is being edited.
        onChange={(name, next) =>
          editor
            .chain()
            .updateAttributes(active.name, { [name]: next })
            .run()
        }
      />
      {problems.length > 0 && (
        <p
          role="alert"
          className="flex gap-2 mt-0 mb-3.5 py-2.5 px-3 border border-danger rounded-lg text-danger text-help leading-[1.45]"
        >
          <Icon name="info" size={16} className="flex-none mt-px" />
          <span>
            {problems.map((problem) => (
              <span key={problem} className="block">
                {problem}. Publishing waits until it is fixed.
              </span>
            ))}
          </span>
        </p>
      )}
    </>
  );
}

function ImageFields({ editor, attrs, at }: { editor: Editor; attrs: Record<string, unknown>; at: number }) {
  const id = useId();
  // The image being edited, kept as it was when the editor opened: saving puts the copy in the page, which changes `attrs`.
  const [editing, setEditing] = useState<Asset | null>(null);
  const saved = useRef(false);
  const asset = useAsset(typeof attrs.assetId === "string" ? attrs.assetId : null).data?.asset;
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
        <div className="flex justify-center mt-0 mb-4.5 p-2.5 border border-border rounded-lg bg-hover">
          <img className="max-w-full max-h-45 object-contain rounded-sm" src={`/_a/${attrs.assetId}`} alt="" />
        </div>
      )}
      <div className={`${ui.inline} mb-4.5`}>
        <button type="button" className={ui.secondary} onClick={() => openMediaPicker(editor, { mode: "replace", at })}>
          Replace…
        </button>
        <button
          type="button"
          className={ui.secondary}
          disabled={!asset || asset.variants?.length === 0}
          onClick={() => setEditing(asset ?? null)}
        >
          Crop & annotate…
        </button>
      </div>
      {editing && (
        <ImageEditor
          asset={editing}
          saveLabel="Save and replace"
          onSaved={(copy) => {
            placeImage(editor, { mode: "replace", at }, copy);
            saved.current = true;
          }}
          onClose={() => {
            setEditing(null);
            // After a save the page has changed: the focus goes back to it, so Undo there takes the copy out again.
            if (saved.current) editor.commands.focus();
            saved.current = false;
          }}
        />
      )}
      <div className={ui.field}>
        <label htmlFor={`${id}-alt`} className={ui.fieldLabel}>
          Alt text
        </label>
        <textarea
          id={`${id}-alt`}
          className={ui.textarea}
          rows={3}
          value={alt}
          disabled={decorative}
          placeholder={decorative ? "Not needed for a decorative image" : "What the image shows"}
          onChange={(e) => set("alt", e.target.value)}
        />
        <small className="text-muted">Read aloud by screen readers, and shown when the image cannot load.</small>
      </div>
      <div className={ui.field}>
        <label className={ui.checkInField}>
          <input type="checkbox" checked={decorative} onChange={(e) => set("decorative", e.target.checked)} />
          Decorative
        </label>
        <small className="text-muted">It adds nothing to the text, so screen readers skip it.</small>
      </div>
      {!alt.trim() && !decorative && (
        <p
          role="alert"
          className="flex gap-2 mt-0 mb-3.5 py-2.5 px-3 border border-warn-border rounded-lg bg-warn-bg text-warn-text text-help leading-[1.45]"
        >
          <Icon name="info" size={16} className="flex-none mt-px text-warn-icon" />
          Describe this image for people who cannot see it, or mark it decorative. Publishing waits until then.
        </p>
      )}
      <div className={ui.field}>
        <label htmlFor={`${id}-caption`} className={ui.fieldLabel}>
          Caption
        </label>
        <textarea
          id={`${id}-caption`}
          className={ui.textarea}
          rows={2}
          value={caption}
          onChange={(e) => set("caption", e.target.value)}
        />
        <small className="text-muted">Shown under the image. Optional.</small>
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
    // It must stand out from the page without hiding much of it.
    <button
      type="button"
      className="fixed right-5 bottom-5 z-20 inline-flex items-center gap-2 h-10 pr-2 pl-3.5 border-0 rounded-[20px] bg-accent text-bg text-ui font-semibold shadow-[0_6px_18px_rgb(16_20_22/22%)] cursor-pointer hover:brightness-108 focus-visible:outline-offset-3"
      aria-keyshortcuts="Alt+Enter"
      onClick={open}
    >
      <Icon name="sliders" size={16} />
      {label}
      <span
        className="py-0.5 px-1.75 rounded-[10px] bg-[color-mix(in_srgb,var(--bg)_22%,transparent)] font-[inherit] text-label font-medium"
        aria-hidden="true"
      >
        Alt ↵
      </span>
    </button>
  );
}
