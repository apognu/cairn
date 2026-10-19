import { useEffect, useId, useMemo, useRef, useState } from "react";
import { Icon } from "./icons";
import { useApiOperations, useApiOperationsOf, useCreatePage, usePageHeadings, usePages, usePending } from "./queries";
import type { Page } from "./tree";
import { hiddenBy, pageUrls } from "./tree";
import * as ui from "./ui";

const searchRow = "flex gap-2 p-2.5";
const searchInput = "flex-1 h-9.5 px-3 border border-accent rounded-[7px] bg-bg text-[15px]";
const list = "pt-0 px-1.5 pb-1.5";
/** A row of the list; the one the arrow keys are on is lit. */
const option =
  "flex items-center gap-2.5 w-full py-2 px-2.5 border-0 rounded-lg bg-transparent text-left cursor-pointer group-aria-selected/option:bg-accent-soft";

type Tab = "page" | "url";

interface Props {
  /** Pages linked by ID; a new page created from the picker arrives here too. With `anchors`, a place inside the page may come along. */
  onPage: (page: Page, inside?: { anchor: string; label: string }) => void;
  /** Lets a link point inside a page: one of an API page's operations, or a section of another page. */
  anchors?: boolean;
  /** Present when a plain web address is allowed too. */
  onUrl?: (url: string) => void;
  onClose: () => void;
  /** Where new pages go by default: under this page's parent. */
  nearPage?: string;
  initialUrl?: string;
  style?: React.CSSProperties;
}

/** Finds a page to link to: drafts included, with "Create page" when nothing matches. One picker serves text links and component fields. */
export function PagePicker({ onPage, anchors = false, onUrl, onClose, nearPage, initialUrl, style }: Props) {
  const id = useId();
  const pages = usePages();
  const pending = usePending();
  const create = useCreatePage();
  const [tab, setTab] = useState<Tab>("page");
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [chosenParent, setParent] = useState<string | null>(null);
  // An API page chosen, whose operations are being listed, or a page whose sections are.
  const [apiPage, setApiPage] = useState<Page | null>(null);
  const apiOperations = useApiOperations(apiPage?.id ?? null);
  const [sectionsOf, setSectionsOf] = useState<Page | null>(null);
  const headings = usePageHeadings(sectionsOf?.id ?? null);
  const input = useRef<HTMLInputElement>(null);
  const root = useRef<HTMLDivElement>(null);

  const all = (pages.data ?? []).filter((p) => p.kind !== "link");
  const urls = useMemo(() => pageUrls(pages.data ?? []), [pages.data]);
  const drafts = new Set(pending.data?.changes.filter((c) => c.kinds.includes("new")).map((c) => c.page_id));
  // A link to a hidden page is kept, but shows as plain text on the site.
  const hidden = useMemo(() => hiddenBy(pages.data ?? []), [pages.data]);

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
  const known = useApiOperationsOf(anchors ? all.filter((p) => p.kind === "api") : []);
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
  const pick = (page: Page) => (anchors && page.kind === "api" ? setApiPage(page) : onPage(page));
  const hasSections = (row: (typeof rows)[number] | undefined): row is Page =>
    anchors && row !== undefined && row !== "create" && !("operation" in row) && row.kind === "page";
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
      className="fixed z-30 w-110 max-w-[calc(100vw-32px)] border border-input rounded-xl bg-bg shadow-[0_16px_40px_rgb(16_20_22/18%)] text-ui leading-[1.4] overflow-hidden"
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
        <div className="flex gap-1 pt-2 px-2 pb-0 border-b border-border" role="tablist" aria-label="Link type">
          {(["page", "url"] as const).map((t) => (
            <button
              key={t}
              type="button"
              role="tab"
              className="h-8 px-2.5 border-0 bg-transparent text-muted cursor-pointer aria-selected:text-text aria-selected:font-semibold aria-selected:shadow-[inset_0_-2px_0_var(--accent)]"
              aria-selected={tab === t}
              onClick={() => setTab(t)}
            >
              {t === "page" ? "Page" : "URL"}
            </button>
          ))}
        </div>
      )}

      {apiPage ? (
        <AnchorList
          page={apiPage}
          what="operations"
          rows={apiOperations.data?.map((o) => ({
            anchor: o.anchor,
            title: `${o.method} ${o.path}`,
            detail: o.summary,
            mono: true,
          }))}
          loading={apiOperations.isPending}
          empty="No operations known yet: use “Check sources now” in the page's settings, or publish."
          onBack={() => setApiPage(null)}
          onChoose={(operation) => onPage(apiPage, operation)}
        />
      ) : sectionsOf ? (
        <AnchorList
          page={sectionsOf}
          what="sections"
          // The page's title is its only first-level heading; sections start at the second.
          rows={headings.data
            ?.filter((h) => h.level >= 2)
            .map((h) => ({ anchor: h.anchor, title: h.text || "Untitled section", depth: h.level - 2 }))}
          loading={headings.isPending}
          empty="This page has no headings yet."
          onBack={() => setSectionsOf(null)}
          onChoose={(section) => onPage(sectionsOf, section)}
        />
      ) : tab === "url" && onUrl ? (
        <form
          className={searchRow}
          onSubmit={(e) => {
            e.preventDefault();
            const url = String(new FormData(e.currentTarget).get("url") ?? "").trim();
            if (url) onUrl(url);
          }}
        >
          <input
            name="url"
            className={searchInput}
            aria-label="Link address"
            placeholder="https://…"
            defaultValue={initialUrl}
          />
          <button type="submit" className={ui.primary}>
            Apply
          </button>
        </form>
      ) : (
        <>
          <div className={searchRow}>
            <label htmlFor={`${id}-q`} className="sr-only">
              Search pages
            </label>
            <input
              ref={input}
              className={searchInput}
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
                // At the end of what is typed, the right arrow goes into the page's sections, as the row's Sections button does.
                else if (
                  e.key === "ArrowRight" &&
                  e.currentTarget.selectionStart === query.length &&
                  hasSections(rows[choice])
                )
                  setSectionsOf(rows[choice]);
                else return;
                e.preventDefault();
              }}
            />
          </div>
          <div className={list} id={`${id}-list`} role="listbox" aria-label="Pages">
            {rows.map((row, i) =>
              row === "create" ? (
                <div
                  key="create"
                  id={`${id}-${i}`}
                  role="option"
                  className="group/option"
                  aria-selected={i === choice}
                  tabIndex={-1}
                >
                  <button
                    type="button"
                    className={option}
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={createPage}
                    disabled={create.isPending}
                  >
                    <Icon name="plus" size={16} />
                    <span>
                      <strong>Create page “{query.trim()}”</strong>
                      <small className="block text-muted">Draft page, linked right away. You can write it later.</small>
                    </span>
                  </button>
                  <label className="flex items-center gap-2 pt-0.5 pr-2.5 pb-2 pl-9 text-help text-muted">
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
                  className="group/option"
                  aria-selected={i === choice}
                  tabIndex={-1}
                >
                  <button
                    type="button"
                    className={option}
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={() => chooseOperation(row)}
                  >
                    <span>
                      <strong className={ui.mono}>
                        {row.operation.method} {row.operation.path}
                      </strong>
                      <small className="block text-muted">
                        {row.page.title}
                        {row.operation.summary ? ` · ${row.operation.summary}` : ""}
                      </small>
                    </span>
                  </button>
                </div>
              ) : (
                <div
                  key={row.id}
                  id={`${id}-${i}`}
                  role="option"
                  className="group/option flex items-center gap-1 pr-1 rounded-lg aria-selected:bg-accent-soft"
                  aria-selected={i === choice}
                  tabIndex={-1}
                >
                  <button
                    type="button"
                    className={`${option} min-w-0`}
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={() => pick(row)}
                  >
                    <span className="min-w-0">
                      <strong>{row.title}</strong>
                      {drafts.has(row.id) && <span className={ui.badge(undefined, "mr-1 ml-2")}>Draft</span>}
                      {hidden.has(row.id) && (
                        <span
                          className={ui.badge(undefined, "mr-1 ml-2")}
                          title="Not published: links to it show as plain text"
                        >
                          Hidden
                        </span>
                      )}
                      {anchors && row.kind === "api" && <span className={ui.badge(undefined, "mr-1 ml-2")}>API</span>}
                      <small className="block text-muted font-mono text-label overflow-hidden text-ellipsis">
                        {urls[row.id]}
                      </small>
                    </span>
                  </button>
                  {hasSections(row) && (
                    <button
                      type="button"
                      className="flex-none h-7 px-2 border-0 rounded-[5px] bg-transparent text-help text-muted cursor-pointer hover:bg-hover hover:text-text"
                      aria-label={`Sections of ${row.title}`}
                      onMouseDown={(e) => e.preventDefault()}
                      onClick={() => setSectionsOf(row)}
                    >
                      Sections ›
                    </button>
                  )}
                </div>
              ),
            )}
            {rows.length === 0 && <div className="py-2 px-2.5 text-muted">No pages yet.</div>}
          </div>
          {create.isError && (
            <p role="alert" className="text-danger">
              {create.error.message}
            </p>
          )}
          <div className="flex justify-between gap-3.5 py-2 px-4 border-t border-border bg-side text-label text-muted">
            <span className="whitespace-nowrap">
              ↵ to {canCreate && choice === 0 ? "create and link" : "link"}
              {hasSections(rows[choice]) && ", → for a section"}
            </span>
            <span>Links are stored by page, so moves never break them</span>
          </div>
        </>
      )}
    </div>
  );
}

type OperationRow = { page: Page; operation: OperationLink };

type OperationLink = { method: string; path: string; summary?: string | null; tags: string[]; anchor: string };

/** A place inside a page: an operation of an API page, or a section of another page. */
type AnchorRow = { anchor: string; title: string; detail?: string | null; mono?: boolean; depth?: number };

/** The second step for a page: the whole page, or one of its operations or sections. */
function AnchorList({
  page,
  what,
  rows: all,
  loading,
  empty,
  onBack,
  onChoose,
}: {
  page: Page;
  what: "operations" | "sections";
  rows: AnchorRow[] | undefined;
  loading: boolean;
  empty: string;
  onBack: () => void;
  onChoose: (inside?: { anchor: string; label: string }) => void;
}) {
  const id = useId();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => input.current?.focus(), []);

  const needle = query.trim().toLowerCase();
  const shown = (all ?? [])
    .filter((r) => !needle || `${r.title} ${r.detail ?? ""}`.toLowerCase().includes(needle))
    .slice(0, 50);
  // The whole page first; once something is typed, the first match is what Enter takes.
  const rows: (AnchorRow | null)[] = [null, ...shown];
  const choice = Math.min(active, rows.length - 1);
  const choose = (row: AnchorRow | null | undefined) =>
    onChoose(row ? { anchor: row.anchor, label: row.title } : undefined);

  return (
    <>
      <div className={searchRow}>
        <button type="button" className={ui.secondary} onClick={onBack} aria-label="Back to pages">
          ←
        </button>
        <input
          ref={input}
          className={searchInput}
          role="combobox"
          aria-expanded="true"
          aria-controls={`${id}-list`}
          aria-activedescendant={`${id}-${choice}`}
          aria-label={`Search the ${what} of ${page.title}`}
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
      <div
        className={`${list} max-h-90 overflow-y-auto`}
        id={`${id}-list`}
        role="listbox"
        aria-label={what === "operations" ? "Operations" : "Sections"}
      >
        {rows.map((row, i) => (
          <div
            key={row?.anchor ?? "page"}
            id={`${id}-${i}`}
            role="option"
            className="group/option"
            aria-selected={i === choice}
            tabIndex={-1}
          >
            <button
              type="button"
              className={option}
              style={row?.depth ? { paddingLeft: `${10 + row.depth * 16}px` } : undefined}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => choose(row)}
            >
              {row ? (
                <span className="min-w-0">
                  <strong className={row.mono ? ui.mono : undefined}>{row.title}</strong>
                  {row.detail && <small className="block text-muted">{row.detail}</small>}
                </span>
              ) : (
                <span>
                  <strong>The whole page</strong>
                  <small className="block text-muted">{page.title}</small>
                </span>
              )}
            </button>
          </div>
        ))}
        {!loading && (all ?? []).length === 0 && <div className="py-2 px-2.5 text-muted">{empty}</div>}
      </div>
    </>
  );
}
