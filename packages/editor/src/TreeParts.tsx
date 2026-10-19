import { type SyntheticEvent, useState } from "react";
import * as ui from "./ui";

/** Opens a row menu upwards when the scrolling sidebar would cut it off below the row and there is more room above. */
export function placeMenu(event: SyntheticEvent<HTMLDetailsElement>): void {
  const menu = event.currentTarget;
  const pop = menu.querySelector<HTMLElement>("[data-menu-pop]");
  if (!menu.open || !pop) return;

  const row = menu.getBoundingClientRect();
  const bounds = menu.closest(".sidebar")?.getBoundingClientRect() ?? { top: 0, bottom: window.innerHeight };
  const below = bounds.bottom - row.bottom;
  const above = row.top - bounds.top;
  menu.toggleAttribute("data-up", below < pop.offsetHeight + 8 && above > below);
}

/** A row of the page tree, or of the settings sections, which reuse it. */
export function rowClass({
  selected,
  depth,
  dragging = false,
  drop,
}: {
  selected: boolean;
  depth: number;
  dragging?: boolean;
  drop?: "before" | "inside" | "after";
}): string {
  // One background and one text color: the states are exclusive, the drag ones first, as the old rules ranked them.
  const state = dragging
    ? "border border-dashed border-input text-muted bg-transparent"
    : drop === "inside"
      ? `bg-accent-soft outline outline-accent ${selected ? "text-accent" : ""}`
      : selected
        ? "bg-accent-soft text-accent"
        : "hover:bg-hover focus-within:bg-hover";
  // Before or after: a line with a dot where the page will land.
  const line =
    drop === "before" || drop === "after"
      ? `before:content-[''] before:absolute before:left-4.5 before:right-2.5 before:h-0.5 before:rounded-[1px] before:bg-accent before:shadow-[-8px_0_0_-2px_var(--bg-side),-8px_0_0_0_var(--accent)] ${drop === "before" ? "before:-top-px" : "before:-bottom-px"}`
      : "";

  return `group/row relative flex items-center min-h-7.5 pr-1 rounded-md ${depth === 0 ? "mt-3.5" : ""} ${state} ${line}`;
}

/** The title button of a row. Top-level pages read as the sidebar's section headings. */
/** `hidden`: the page is not published, being hidden itself or under a hidden page. */
export function titleClass({
  selected,
  depth,
  hidden = false,
}: {
  selected: boolean;
  depth: number;
  hidden?: boolean;
}): string {
  const look =
    depth === 0
      ? `text-[11.5px] font-semibold tracking-[0.06em] uppercase ${selected ? "text-accent" : "text-muted"}`
      : selected
        ? "font-semibold"
        : "";

  return `flex-1 min-w-0 py-1 px-0 border-0 bg-transparent text-left cursor-pointer overflow-hidden text-ellipsis whitespace-nowrap ${look} ${hidden ? "opacity-60" : ""}`;
}

export function IconButton({ label, glyph, onClick }: { label: string; glyph: string; onClick: () => void }) {
  return (
    <button type="button" className={ui.icon} aria-label={label} title={label} onClick={onClick}>
      <span aria-hidden="true">{glyph}</span>
    </button>
  );
}

export function MenuItem({
  label,
  text,
  onClick,
  danger = false,
}: {
  label: string;
  text: string;
  onClick: () => void;
  danger?: boolean;
}) {
  return (
    <button
      type="button"
      className={`${ui.menuItem} ${danger ? "text-danger" : ""}`}
      aria-label={label}
      onClick={(e) => {
        e.currentTarget.closest("details")?.removeAttribute("open");
        onClick();
      }}
    >
      {text}
    </button>
  );
}

const field = "w-full h-6.5 px-2 border border-input rounded-md bg-bg text-help";

/** Inline form for a page title, or for a link's title and URL when `url` is given. */
export function TitleForm({
  initial,
  initialUrl,
  label,
  onSubmit,
  onCancel,
  askRedirect = false,
}: {
  initial: string;
  /** Present for links: the form then asks for a URL as well. */
  initialUrl?: string;
  label: string;
  onSubmit: (title: string, url?: string, keepOldUrl?: boolean) => void;
  onCancel: () => void;
  /** Offer to keep the old URL as a redirect (renaming a page). */
  askRedirect?: boolean;
}) {
  const [value, setValue] = useState(initial);
  const [url, setUrl] = useState(initialUrl ?? "");
  const [keep, setKeep] = useState(true);
  const isLink = initialUrl !== undefined;

  return (
    <form
      className="flex-1 flex flex-col gap-1 my-2 mr-2 py-0.5 px-0"
      onSubmit={(e) => {
        e.preventDefault();
        if (value.trim() && (!isLink || url.trim()))
          onSubmit(value.trim(), isLink ? url.trim() : undefined, askRedirect ? keep : undefined);
      }}
      onKeyDown={(e) => e.key === "Escape" && onCancel()}
    >
      <input
        className={field}
        aria-label={label}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        // Opening the form is the whole point of the click: type right away, with an existing name selected to replace.
        // biome-ignore lint/a11y/noAutofocus: the form appears in response to the user's action
        autoFocus
        onFocus={(e) => e.currentTarget.select()}
      />
      {isLink && (
        <input
          className={field}
          aria-label="Link URL"
          type="url"
          placeholder="https://"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
        />
      )}
      {askRedirect && (
        <label className="flex items-center gap-1.5 mt-1.5 text-label text-muted">
          <input type="checkbox" className="w-auto h-auto" checked={keep} onChange={(e) => setKeep(e.target.checked)} />
          Keep old URL as redirect
        </label>
      )}
      {/* A form with two text fields only submits on Enter when it has a submit button. */}
      <button type="submit" hidden>
        Save
      </button>
    </form>
  );
}
