import type { JsonSchema } from "@yourdocs/sdk";
import { useId, useState } from "react";
import { PagePicker } from "./PagePicker";
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
        <PageField id={id} value={String(value ?? "")} onChange={(next) => onChange(next || undefined)} />
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
    <fieldset className="form-group list-group">
      <legend>{label}</legend>
      {value.map((entry, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: list items have no identity of their own
        <details className="form-item" key={i} open={i === 0}>
          <summary>
            <span className="grip" aria-hidden="true">
              ⠿
            </span>
            <span className="form-item-title">{summaryOf(entry) ?? `${label} ${i + 1}`}</span>
            <span className="form-item-actions">
              <button
                type="button"
                aria-label={`Move ${label} ${i + 1} up`}
                disabled={i === 0}
                onClick={(e) => {
                  e.preventDefault();
                  move(i, i - 1);
                }}
              >
                ↑
              </button>
              <button
                type="button"
                aria-label={`Move ${label} ${i + 1} down`}
                disabled={i === value.length - 1}
                onClick={(e) => {
                  e.preventDefault();
                  move(i, i + 1);
                }}
              >
                ↓
              </button>
              <button
                type="button"
                aria-label={`Remove ${label} ${i + 1}`}
                onClick={(e) => {
                  e.preventDefault();
                  onChange(value.filter((_, j) => j !== i));
                }}
              >
                ×
              </button>
            </span>
          </summary>
          <div className="form-item-body">
            {item.type === "object" ? (
              <SchemaForm
                schema={item}
                value={(entry ?? {}) as Values}
                onChange={(name, next) => replace(i, { ...(entry as Values), [name]: next })}
              />
            ) : (
              <Field label={`${label} ${i + 1}`} schema={item} value={entry} onChange={(next) => replace(i, next)} />
            )}
          </div>
        </details>
      ))}
      <button type="button" className="form-add" onClick={() => onChange([...value, defaultFor(item)])}>
        + Add item
      </button>
    </fieldset>
  );
}

/** What a collapsed list item is called: its label, title or name when it has one. */
function summaryOf(entry: unknown): string | undefined {
  if (typeof entry === "string") return entry || undefined;
  if (typeof entry !== "object" || entry === null) return undefined;

  const record = entry as Values;
  const named = record.label ?? record.title ?? record.name;
  return typeof named === "string" && named ? named : undefined;
}

/** A component's page field: the chosen page's title and a button that opens the page picker. */
function PageField({ id, value, onChange }: { id: string; value: string; onChange: (next: string) => void }) {
  const pages = usePages();
  const [open, setOpen] = useState(false);
  const chosen = pages.data?.find((p) => p.id === value);

  return (
    <div className="page-field">
      <button type="button" id={id} className="page-field-button" aria-haspopup="dialog" onClick={() => setOpen(true)}>
        {chosen ? chosen.title : <span className="muted">No link</span>}
        <span className="link-button">{chosen ? "Change" : "Choose"}</span>
      </button>
      {chosen && (
        <button type="button" className="icon" aria-label="Remove link" onClick={() => onChange("")}>
          ×
        </button>
      )}
      {open && (
        <PagePicker
          style={{ position: "absolute", left: 0, right: 0, top: "100%", width: "auto" }}
          onClose={() => setOpen(false)}
          onPage={(page) => {
            onChange(page.id);
            setOpen(false);
          }}
        />
      )}
    </div>
  );
}
