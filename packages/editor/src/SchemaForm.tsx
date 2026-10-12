import type { JsonSchema } from "@yourdocs/sdk";
import { useId } from "react";
import { usePages } from "./queries";

type Values = Record<string, unknown>;

const humanize = (name: string) => name.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/^./, (c) => c.toUpperCase());

/** A fresh value for a field, used when an item is added to a list. */
function defaultFor(schema: JsonSchema): unknown {
  if (schema.default !== undefined) return schema.default;
  if (schema.enum) return schema.enum[0];

  switch (schema.type) {
    case "number":
    case "integer":
      return 0;
    case "boolean":
      return false;
    case "array":
      return [];
    case "object":
      return Object.fromEntries(
        Object.entries(schema.properties ?? {}).map(([key, child]) => [key, defaultFor(child)]),
      );
    default:
      return "";
  }
}

/** A form for the props described by a JSON Schema: edits call `onChange` with the whole new value of the changed field. */
export function SchemaForm({
  schema,
  value,
  onChange,
}: {
  schema: JsonSchema;
  value: Values;
  onChange: (name: string, next: unknown) => void;
}) {
  return (
    <>
      {Object.entries(schema.properties ?? {}).map(([name, child]) => (
        <Field
          key={name}
          label={child.title ?? humanize(name)}
          schema={child}
          value={value[name] ?? defaultFor(child)}
          onChange={(next) => onChange(name, next)}
        />
      ))}
    </>
  );
}

function Field({
  label,
  schema,
  value,
  onChange,
}: {
  label: string;
  schema: JsonSchema;
  value: unknown;
  onChange: (next: unknown) => void;
}) {
  const id = useId();

  if (schema.type === "array")
    return <ListField label={label} schema={schema} value={Array.isArray(value) ? value : []} onChange={onChange} />;

  if (schema.type === "object") {
    const object = (value ?? {}) as Values;
    return (
      <fieldset className="form-group">
        <legend>{label}</legend>
        <SchemaForm schema={schema} value={object} onChange={(name, next) => onChange({ ...object, [name]: next })} />
      </fieldset>
    );
  }

  if (schema.type === "boolean") {
    return (
      <label className="form-check">
        <input type="checkbox" checked={value === true} onChange={(e) => onChange(e.target.checked)} />
        {label}
      </label>
    );
  }

  return (
    <div className="form-field">
      <label htmlFor={id}>{label}</label>
      {schema["x-field"] === "page" ? (
        <PageSelect id={id} value={String(value ?? "")} onChange={(next) => onChange(next || undefined)} />
      ) : schema["x-field"] === "date" ? (
        <input
          id={id}
          type="date"
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value || undefined)}
        />
      ) : schema.enum ? (
        <select id={id} value={String(value)} onChange={(e) => onChange(e.target.value)}>
          {schema.enum.map((option) => (
            <option key={String(option)}>{String(option)}</option>
          ))}
        </select>
      ) : schema.type === "number" || schema.type === "integer" ? (
        <input id={id} type="number" value={Number(value)} onChange={(e) => onChange(Number(e.target.value))} />
      ) : (
        <input id={id} type="text" value={String(value ?? "")} onChange={(e) => onChange(e.target.value)} />
      )}
      {schema.description && <small className="muted">{schema.description}</small>}
    </div>
  );
}

function ListField({
  label,
  schema,
  value,
  onChange,
}: {
  label: string;
  schema: JsonSchema;
  value: unknown[];
  onChange: (next: unknown[]) => void;
}) {
  const item = schema.items ?? {};
  const replace = (index: number, next: unknown) => onChange(value.map((v, i) => (i === index ? next : v)));
  const move = (from: number, to: number) => {
    const next = [...value];
    next.splice(to, 0, ...next.splice(from, 1));
    onChange(next);
  };

  return (
    <fieldset className="form-group">
      <legend>{label}</legend>
      {value.map((entry, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: list items have no identity of their own
        <div className="form-item" key={i}>
          <Field label={`${label} ${i + 1}`} schema={item} value={entry} onChange={(next) => replace(i, next)} />
          <div className="form-item-actions">
            <button
              type="button"
              aria-label={`Move ${label} ${i + 1} up`}
              disabled={i === 0}
              onClick={() => move(i, i - 1)}
            >
              ↑
            </button>
            <button
              type="button"
              aria-label={`Move ${label} ${i + 1} down`}
              disabled={i === value.length - 1}
              onClick={() => move(i, i + 1)}
            >
              ↓
            </button>
            <button
              type="button"
              aria-label={`Remove ${label} ${i + 1}`}
              onClick={() => onChange(value.filter((_, j) => j !== i))}
            >
              ×
            </button>
          </div>
        </div>
      ))}
      <button type="button" className="form-add" onClick={() => onChange([...value, defaultFor(item)])}>
        Add item
      </button>
    </fieldset>
  );
}

/** Until the page picker of M2, a plain list of pages: titles for people, IDs in the document. */
function PageSelect({ id, value, onChange }: { id: string; value: string; onChange: (next: string) => void }) {
  const pages = usePages();

  return (
    <select id={id} value={value} onChange={(e) => onChange(e.target.value)}>
      <option value="">No link</option>
      {pages.data?.map((page) => (
        <option key={page.id} value={page.id}>
          {page.title}
        </option>
      ))}
    </select>
  );
}
