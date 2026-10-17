import { type JsonSchema, operationLabel, pageTarget } from "@cairn/sdk";
import { useId, useState } from "react";
import { IconPicker } from "./IconPicker";
import { PagePicker } from "./PagePicker";
import { useLibrary, usePageHeadings, usePages } from "./queries";
import * as ui from "./ui";

/** A group of fields (an object), or a list of entries. */
const group = "mt-0 mx-0 mb-3.5 min-w-0";

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
  labelHidden = false,
  schema,
  value,
  onChange,
}: {
  label: string;
  /** Read by screen readers only: a list item's header already says what the field holds. */
  labelHidden?: boolean;
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
      <fieldset className={`${group} p-3 border border-input rounded-[10px] bg-side`}>
        <legend className={ui.fieldLabel}>{label}</legend>
        <SchemaForm schema={schema} value={object} onChange={(name, next) => onChange({ ...object, [name]: next })} />
      </fieldset>
    );
  }

  if (schema.type === "boolean") {
    return (
      <label className={ui.check}>
        <input type="checkbox" checked={value === true} onChange={(e) => onChange(e.target.checked)} />
        {label}
      </label>
    );
  }

  return (
    <div className={ui.field}>
      <label htmlFor={id} className={labelHidden ? "sr-only" : ui.fieldLabel}>
        {label}
      </label>
      {schema["x-field"] === "page" ? (
        <PageField id={id} value={String(value ?? "")} onChange={(next) => onChange(next || undefined)} />
      ) : schema["x-field"] === "icon" ? (
        <IconPicker id={id} value={String(value ?? "")} onChange={onChange} />
      ) : schema["x-field"] === "date" ? (
        <input
          id={id}
          className={ui.input}
          type="date"
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value || undefined)}
        />
      ) : schema["x-field"] === "variable" || schema["x-field"] === "snippet" ? (
        <LibraryKey id={id} kind={schema["x-field"]} value={String(value ?? "")} onChange={(next) => onChange(next)} />
      ) : schema["x-field"] === "text" ? (
        <textarea
          id={id}
          className={ui.textarea}
          rows={6}
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value)}
        />
      ) : schema.enum ? (
        <select id={id} className={ui.select} value={String(value)} onChange={(e) => onChange(e.target.value)}>
          {schema.enum.map((option) => (
            <option key={String(option)} value={String(option)}>
              {readable(String(option))}
            </option>
          ))}
        </select>
      ) : schema.type === "number" || schema.type === "integer" ? (
        <input
          id={id}
          className={ui.input}
          type="number"
          value={Number(value)}
          onChange={(e) => onChange(Number(e.target.value))}
        />
      ) : (
        <input
          id={id}
          className={ui.input}
          type="text"
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value)}
        />
      )}
      {schema.description && <small className="text-muted">{schema.description}</small>}
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
  // "Add card", not "Add cards": the item's own title, or the list's in the singular.
  const one = (item.title ?? label.replace(/ies$/, "y").replace(/s$/, "")).toLowerCase();
  const replace = (index: number, next: unknown) => onChange(value.map((v, i) => (i === index ? next : v)));
  const move = (from: number, to: number) => {
    const next = [...value];
    next.splice(to, 0, ...next.splice(from, 1));
    onChange(next);
  };

  return (
    <fieldset className={`${group} p-0 border-0`}>
      <legend className={`${ui.fieldLabel} p-0 mb-1.5`}>{label}</legend>
      {value.map((entry, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: list items have no identity of their own
        <details className="group/item mb-2 border border-input rounded-lg bg-bg" key={i} open={i === 0}>
          <summary className="flex items-center gap-2 min-w-0 py-2 px-2.5 cursor-pointer list-none [&::-webkit-details-marker]:hidden group-open/item:font-semibold">
            <span className="text-muted" aria-hidden="true">
              ⠿
            </span>
            <span className="flex-1 min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">
              {item["x-field"] === "page" ? (
                <PageLabel value={String(entry ?? "")} empty={`No ${one} chosen`} />
              ) : (
                (summaryOf(entry) ?? `${label} ${i + 1}`)
              )}
            </span>
            <span className="flex gap-0.5">
              <button
                type="button"
                className={ui.itemButton}
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
                className={ui.itemButton}
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
                className={ui.itemButton}
                aria-label={`Remove ${label} ${i + 1}`}
                // Fewer than the schema allows and the component could no longer render.
                disabled={value.length <= (schema.minItems ?? 0)}
                onClick={(e) => {
                  e.preventDefault();
                  onChange(value.filter((_, j) => j !== i));
                }}
              >
                ×
              </button>
            </span>
          </summary>
          <div className="pt-1 px-3 pb-0">
            {item.type === "object" ? (
              <SchemaForm
                schema={item}
                value={(entry ?? {}) as Values}
                onChange={(name, next) => replace(i, { ...(entry as Values), [name]: next })}
              />
            ) : (
              <Field
                label={`${label} ${i + 1}`}
                labelHidden={item["x-field"] === "page"}
                schema={item}
                value={entry}
                onChange={(next) => replace(i, next)}
              />
            )}
          </div>
        </details>
      ))}
      <button
        type="button"
        className="h-8 px-2.5 border border-dashed border-input rounded-md bg-bg cursor-pointer"
        onClick={() => onChange([...value, defaultFor(item)])}
      >
        + Add {one}
      </button>
    </fieldset>
  );
}

/** A schema value as authors read it: `warning` is Warning, `code-group` is Code group. */
function readable(value: string): string {
  const words = value.replace(/[-_]/g, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** What a collapsed list item is called: its label, title or name when it has one. */
function summaryOf(entry: unknown): string | undefined {
  if (typeof entry === "string") return entry || undefined;
  if (typeof entry !== "object" || entry === null) return undefined;

  const record = entry as Values;
  const named = record.label ?? record.title ?? record.name;
  return typeof named === "string" && named ? named : undefined;
}

/**
 * What a page field points at, as authors know it: the page's title, or for an API operation the operation then the page, or for a section
 * the page then the section's heading.
 */
function PageLabel({ value, empty }: { value: string; empty: string }) {
  const pages = usePages();
  const target = pageTarget(value);
  const chosen = pages.data?.find((p) => p.id === target.id);
  const operation = operationLabel(target.anchor);
  const headings = usePageHeadings(chosen && target.anchor && !operation ? chosen.id : null);
  const section = headings.data?.find((h) => h.anchor === target.anchor)?.text;

  // Still pointing at a page that was deleted: it counts as a broken link until it is changed or removed.
  if (value && pages.data !== undefined && !chosen) return <span className="text-danger">Deleted page</span>;
  if (!chosen) return <span className="text-muted">{empty}</span>;
  if (operation)
    return (
      <>
        {operation} <span className="text-muted">· {chosen.title}</span>
      </>
    );
  if (target.anchor)
    return (
      <>
        {chosen.title} <span className="text-muted">› {section ?? `#${target.anchor}`}</span>
      </>
    );

  return <>{chosen.title}</>;
}

/** A component's page field: what it points at, and a button that opens the page picker. */
export function PageField({ id, value, onChange }: { id: string; value: string; onChange: (next: string) => void }) {
  const [open, setOpen] = useState(false);

  return (
    <div className="relative flex gap-1.5">
      <button
        type="button"
        id={id}
        className="flex flex-1 min-w-0 items-center justify-between gap-2 h-8.5 px-2.5 border border-input rounded-md bg-bg cursor-pointer"
        aria-haspopup="dialog"
        onClick={() => setOpen(true)}
      >
        <span className="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">
          <PageLabel value={value} empty="No link" />
        </span>
        <span className={`flex-none ${ui.linkButton}`}>{value ? "Change" : "Choose"}</span>
      </button>
      {value && (
        <button type="button" className={ui.icon} aria-label="Remove link" onClick={() => onChange("")}>
          ×
        </button>
      )}
      {open && (
        <PagePicker
          style={{ position: "absolute", left: 0, right: 0, top: "100%", width: "auto" }}
          onClose={() => setOpen(false)}
          anchors
          onPage={(page, chosenOperation) => {
            onChange(chosenOperation ? `${page.id}#${chosenOperation.anchor}` : page.id);
            setOpen(false);
          }}
        />
      )}
    </div>
  );
}

/** A component's variable or snippet field: a choice among the library's entries of that kind. */
function LibraryKey({
  id,
  kind,
  value,
  onChange,
}: {
  id: string;
  kind: "variable" | "snippet";
  value: string;
  onChange: (next: string) => void;
}) {
  const library = useLibrary();
  const keys = library.data?.filter((e) => e.kind === kind).map((e) => e.key) ?? [];

  return (
    <select id={id} className={ui.select} value={value} onChange={(e) => onChange(e.target.value)}>
      <option value="">Choose a {kind}…</option>
      {/* An entry that was deleted since still shows, so the choice is visible and can be changed. */}
      {value && !keys.includes(value) && <option value={value}>{value} (missing)</option>}
      {keys.map((key) => (
        <option key={key} value={key}>
          {key}
        </option>
      ))}
    </select>
  );
}
