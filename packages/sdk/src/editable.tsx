import { createContext, useContext, useEffect, useRef } from "react";

/** The props of the component being drawn, and, in the editor only, how to change one of them. */
export interface EditableScope {
  props: unknown;
  onChange?: (path: string, value: string) => void;
}

export const EditableScopeContext = createContext<EditableScope | null>(null);

/** The value at a dotted path such as `tabs.0.label`. */
export function getPath(value: unknown, path: string): unknown {
  return path
    .split(".")
    .reduce<unknown>((current, key) => (current as Record<string, unknown> | undefined)?.[key], value);
}

/** A copy of `value` with the dotted path set, copying only what lies on the way. */
export function setPath<T>(value: T, path: string, next: unknown): T {
  const [key, ...rest] = path.split(".");
  const copy = (Array.isArray(value) ? [...value] : { ...(value as object) }) as Record<string, unknown>;
  copy[key] = rest.length === 0 ? next : setPath(copy[key], rest.join("."), next);

  return copy as T;
}

/** Text of the component's props that authors write directly on the canvas. On the published site it is the plain text. */
export function Editable({ path }: { path: string }) {
  const scope = useContext(EditableScopeContext);
  const value = scope ? getPath(scope.props, path) : undefined;
  const text = typeof value === "string" ? value : "";

  if (!scope?.onChange) return <>{text}</>;

  return <EditableText text={text} onCommit={(next) => scope.onChange?.(path, next)} />;
}

function EditableText({ text, onCommit }: { text: string; onCommit: (next: string) => void }) {
  const element = useRef<HTMLSpanElement>(null);

  // A change made elsewhere (the settings form) replaces what was typed here.
  useEffect(() => {
    if (element.current && element.current.textContent !== text) element.current.textContent = text;
  }, [text]);

  return (
    // biome-ignore lint/a11y/useSemanticElements: an <input> would change the box of the card, heading or link the text sits in
    <span
      ref={element}
      className="editable"
      contentEditable
      suppressContentEditableWarning
      role="textbox"
      tabIndex={0}
      onBlur={(e) => {
        const next = e.currentTarget.textContent ?? "";
        if (next !== text) onCommit(next);
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          e.currentTarget.blur();
        } else if (e.key === "Escape") {
          e.currentTarget.textContent = text;
          e.currentTarget.blur();
        }
      }}
    >
      {text}
    </span>
  );
}
