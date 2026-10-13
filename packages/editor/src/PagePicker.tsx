import { useEffect, useId, useMemo, useRef, useState } from "react";
import { Icon } from "./icons";
import { useApiOperations, useApiOperationsOf, useCreatePage, usePages, usePending } from "./queries";
import type { Page } from "./tree";
import { pageUrls } from "./tree";

type Tab = "page" | "url";

interface Props {
  /** Pages linked by ID; a new page created from the picker arrives here too. An API page may come with one of its operations. */
  onPage: (page: Page, operation?: { anchor: string; label: string }) => void;
  /** Lets an API page be narrowed to one of its operations. */
  operations?: boolean;
  /** Present when a plain web address is allowed too. */
  onUrl?: (url: string) => void;
  onClose: () => void;
  /** Where new pages go by default: under this page's parent. */
  nearPage?: string;
  initialUrl?: string;
  style?: React.CSSProperties;
}

/** Finds a page to link to: drafts included, with "Create page" when nothing matches. One picker serves text links and component fields. */
export function PagePicker({ onPage, operations = false, onUrl, onClose, nearPage, initialUrl, style }: Props) {
  const id = useId();
  const pages = usePages();
  const pending = usePending();
  const create = useCreatePage();
  const [tab, setTab] = useState<Tab>("page");
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [chosenParent, setParent] = useState<string | null>(null);
  // An API page chosen, whose operations are being listed.
  const [apiPage, setApiPage] = useState<Page | null>(null);
  const apiOperations = useApiOperations(apiPage?.id ?? null);
  const input = useRef<HTMLInputElement>(null);
  const root = useRef<HTMLDivElement>(null);

  const all = (pages.data ?? []).filter((p) => p.kind !== "link");
  const urls = useMemo(() => pageUrls(pages.data ?? []), [pages.data]);
  const drafts = new Set(pending.data?.changes.filter((c) => c.kinds.includes("new")).map((c) => c.page_id));

  // New pages start next to the page being edited, which is where a related page usually belongs.
  const parent = chosenParent ?? pages.data?.find((p) => p.id === nearPage)?.parent_id ?? "";

  useEffect(() => input.current?.focus(), []);

  // Like every popover: a click elsewhere or Escape closes it, wherever the focus went since it opened.
  useEffect(() => {
    const outside = (event: PointerEvent) => {
      if (event.target instanceof Node && !root.current?.contains(event.target)) onClose();
    };
    const onEscape = (event: KeyboardEvent) => event.key === "Escape" && onClose();
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", onEscape);

    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", onEscape);
    };
  }, [onClose]);

  const needle = query.trim().toLowerCase();
  const matches = all
    .filter((p) => !needle || p.title.toLowerCase().includes(needle) || (urls[p.id] ?? "").includes(needle))
    .slice(0, 8);
  const canCreate = needle !== "" && !all.some((p) => p.title.toLowerCase() === needle);
  // Operations of API pages are found by what they do: `POST /orders` or its summary, once something is typed.
  const known = useApiOperationsOf(operations ? all.filter((p) => p.kind === "api") : []);
  const operationRows: OperationRow[] = needle
    ? known
        .flatMap(({ page, operations: list }) => list.map((operation) => ({ page, operation })))
        .filter(({ page, operation }) =>
          `${page.title} ${operation.method} ${operation.path} ${operation.summary ?? ""}`
            .toLowerCase()
            .includes(needle),
        )
        .slice(0, 8)
    : [];
  const rows = [...(canCreate ? ["create" as const] : []), ...matches, ...operationRows];
  const choice = Math.min(active, rows.length - 1);

  const createPage = () => {
    // Enter pressed twice would create the page twice.
    if (create.isPending) return;
    create.mutate({ parent_id: parent || null, title: query.trim() }, { onSuccess: (page) => onPage(page) });
  };
  const pick = (page: Page) => (operations && page.kind === "api" ? setApiPage(page) : onPage(page));
  const chooseOperation = ({ page, operation }: OperationRow) =>
    onPage(page, { anchor: operation.anchor, label: `${operation.method} ${operation.path}` });
  const choose = (row: (typeof rows)[number] | undefined) => {
    if (row === "create") createPage();
    else if (row && "operation" in row) chooseOperation(row);
    else if (row) pick(row);
  };

  return (
    <div
      ref={root}
      className="picker"
      role="dialog"
      aria-label="Link to"
      style={style}
      onKeyDown={(e) => {
        // Stopped here so a drawer around the picker does not close with it (the document listener only sees Escape from elsewhere).
        if (e.key === "Escape") {
          e.stopPropagation();
          onClose();
        }
      }}
    >
      {onUrl && (
        <div className="picker-tabs" role="tablist" aria-label="Link type">
          {(["page", "url"] as const).map((t) => (
            <button key={t} type="button" role="tab" aria-selected={tab === t} onClick={() => setTab(t)}>
              {t === "page" ? "Page" : "URL"}
            </button>
          ))}
        </div>
      )}

      {apiPage ? (
        <OperationList
          page={apiPage}
          operations={apiOperations.data}
          loading={apiOperations.isPending}
          onBack={() => setApiPage(null)}
          onChoose={(operation) => onPage(apiPage, operation)}
        />
      ) : tab === "url" && onUrl ? (
        <form
          className="picker-url"
          onSubmit={(e) => {
            e.preventDefault();
            const url = String(new FormData(e.currentTarget).get("url") ?? "").trim();
            if (url) onUrl(url);
          }}
        >
          <input name="url" aria-label="Link address" placeholder="https://…" defaultValue={initialUrl} />
          <button type="submit" className="primary">
            Apply
          </button>
        </form>
      ) : (
        <>
          <div className="picker-search">
            <label htmlFor={`${id}-q`} className="sr-only">
              Search pages
            </label>
            <input
              ref={input}
              id={`${id}-q`}
              role="combobox"
              aria-expanded="true"
              aria-controls={`${id}-list`}
              aria-activedescendant={rows.length > 0 ? `${id}-${choice}` : undefined}
              placeholder="Search pages"
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                setActive(0);
              }}
              onKeyDown={(e) => {
                if (e.key === "ArrowDown") setActive((choice + 1) % Math.max(rows.length, 1));
                else if (e.key === "ArrowUp") setActive((choice + rows.length - 1) % Math.max(rows.length, 1));
                else if (e.key === "Enter") choose(rows[choice]);
                else return;
                e.preventDefault();
              }}
            />
          </div>
          <div className="picker-list" id={`${id}-list`} role="listbox" aria-label="Pages">
            {rows.map((row, i) =>
              row === "create" ? (
                <div key="create" id={`${id}-${i}`} role="option" aria-selected={i === choice} tabIndex={-1}>
                  <button
                    type="button"
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={createPage}
                    disabled={create.isPending}
                  >
                    <Icon name="plus" size={16} />
                    <span>
                      <strong>Create page “{query.trim()}”</strong>
                      <small className="muted">Draft page, linked right away. You can write it later.</small>
                    </span>
                  </button>
                  <label className="picker-parent">
                    Under
                    <select
                      value={parent}
                      onChange={(e) => setParent(e.target.value)}
                      aria-label="Parent of the new page"
                    >
                      <option value="">Top level</option>
                      {all.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.title}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>
              ) : "operation" in row ? (
                <div
                  key={`${row.page.id}${row.operation.anchor}`}
                  id={`${id}-${i}`}
                  role="option"
                  aria-selected={i === choice}
                  tabIndex={-1}
                >
                  <button type="button" onMouseDown={(e) => e.preventDefault()} onClick={() => chooseOperation(row)}>
                    <span>
                      <strong className="mono">
                        {row.operation.method} {row.operation.path}
                      </strong>
                      <small className="muted">
                        {row.page.title}
                        {row.operation.summary ? ` · ${row.operation.summary}` : ""}
                      </small>
                    </span>
                  </button>
                </div>
              ) : (
                <div key={row.id} id={`${id}-${i}`} role="option" aria-selected={i === choice} tabIndex={-1}>
                  <button type="button" onMouseDown={(e) => e.preventDefault()} onClick={() => pick(row)}>
                    <span>
                      <strong>{row.title}</strong>
                      {drafts.has(row.id) && <span className="badge">Draft</span>}
                      {operations && row.kind === "api" && <span className="badge">API</span>}
                      <small className="muted mono">{urls[row.id]}</small>
                    </span>
                  </button>
                </div>
              ),
            )}
            {rows.length === 0 && <div className="muted picker-empty">No pages yet.</div>}
          </div>
          {create.isError && (
            <p role="alert" className="error-text">
              {create.error.message}
            </p>
          )}
          <div className="picker-foot muted">
            <span>↵ to {canCreate && choice === 0 ? "create and link" : "link"}</span>
            <span>Links are stored by page, so moves never break them</span>
          </div>
        </>
      )}
    </div>
  );
}

type OperationRow = { page: Page; operation: OperationLink };

type OperationLink = { method: string; path: string; summary?: string | null; tags: string[]; anchor: string };

/** The second step for an API page: the whole page, or one of its operations. */
function OperationList({
  page,
  operations,
  loading,
  onBack,
  onChoose,
}: {
  page: Page;
  operations: OperationLink[] | undefined;
  loading: boolean;
  onBack: () => void;
  onChoose: (operation?: { anchor: string; label: string }) => void;
}) {
  const id = useId();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => input.current?.focus(), []);

  const needle = query.trim().toLowerCase();
  const shown = (operations ?? [])
    .filter((o) => !needle || `${o.method} ${o.path} ${o.summary ?? ""}`.toLowerCase().includes(needle))
    .slice(0, 50);
  // The whole page first; once something is typed, the first matching operation is what Enter takes.
  const rows: (OperationLink | null)[] = [null, ...shown];
  const choice = Math.min(active, rows.length - 1);
  const choose = (row: OperationLink | null | undefined) =>
    onChoose(row ? { anchor: row.anchor, label: `${row.method} ${row.path}` } : undefined);

  return (
    <>
      <div className="picker-search">
        <button type="button" className="secondary" onClick={onBack} aria-label="Back to pages">
          ←
        </button>
        <input
          ref={input}
          role="combobox"
          aria-expanded="true"
          aria-controls={`${id}-list`}
          aria-activedescendant={`${id}-${choice}`}
          aria-label={`Search the operations of ${page.title}`}
          placeholder={`Search ${page.title}`}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setActive(e.target.value.trim() ? 1 : 0);
          }}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") setActive((choice + 1) % rows.length);
            else if (e.key === "ArrowUp") setActive((choice + rows.length - 1) % rows.length);
            else if (e.key === "Enter") choose(rows[choice]);
            else if (e.key === "Backspace" && query === "") onBack();
            else return;
            e.preventDefault();
          }}
        />
      </div>
      <div className="picker-list" id={`${id}-list`} role="listbox" aria-label="Operations">
        {rows.map((row, i) => (
          <div key={row?.anchor ?? "page"} id={`${id}-${i}`} role="option" aria-selected={i === choice} tabIndex={-1}>
            <button type="button" onMouseDown={(e) => e.preventDefault()} onClick={() => choose(row)}>
              {row ? (
                <span>
                  <strong className="mono">
                    {row.method} {row.path}
                  </strong>
                  {row.summary && <small className="muted">{row.summary}</small>}
                </span>
              ) : (
                <span>
                  <strong>The whole page</strong>
                  <small className="muted">{page.title}</small>
                </span>
              )}
            </button>
          </div>
        ))}
        {!loading && (operations ?? []).length === 0 && (
          <div className="muted picker-empty">
            No operations known yet: use “Check sources now” in the page's settings, or publish.
          </div>
        )}
      </div>
    </>
  );
}
