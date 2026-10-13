import { useEffect, useId, useMemo, useRef, useState } from "react";
import { Icon } from "./icons";
import { useCreatePage, usePages, usePending } from "./queries";
import type { Page } from "./tree";
import { pageUrls } from "./tree";

type Tab = "page" | "url";

interface Props {
  /** Pages linked by ID; a new page created from the picker arrives here too. */
  onPage: (page: Page) => void;
  /** Present when a plain web address is allowed too. */
  onUrl?: (url: string) => void;
  onClose: () => void;
  /** Where new pages go by default: under this page's parent. */
  nearPage?: string;
  initialUrl?: string;
  style?: React.CSSProperties;
}

/** Finds a page to link to: drafts included, with "Create page" when nothing matches. One picker serves text links and component fields. */
export function PagePicker({ onPage, onUrl, onClose, nearPage, initialUrl, style }: Props) {
  const id = useId();
  const pages = usePages();
  const pending = usePending();
  const create = useCreatePage();
  const [tab, setTab] = useState<Tab>("page");
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [chosenParent, setParent] = useState<string | null>(null);
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
  const rows = [...(canCreate ? ["create" as const] : []), ...matches];
  const choice = Math.min(active, rows.length - 1);

  const createPage = () => {
    // Enter pressed twice would create the page twice.
    if (create.isPending) return;
    create.mutate({ parent_id: parent || null, title: query.trim() }, { onSuccess: (page) => onPage(page) });
  };
  const choose = (row: (typeof rows)[number] | undefined) => {
    if (row === "create") createPage();
    else if (row) onPage(row);
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

      {tab === "url" && onUrl ? (
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
              ) : (
                <div key={row.id} id={`${id}-${i}`} role="option" aria-selected={i === choice} tabIndex={-1}>
                  <button type="button" onMouseDown={(e) => e.preventDefault()} onClick={() => onPage(row)}>
                    <span>
                      <strong>{row.title}</strong>
                      {drafts.has(row.id) && <span className="badge">Draft</span>}
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
