import { type SyntheticEvent, useState } from "react";

/** Opens a row menu upwards when the scrolling sidebar would cut it off below the row and there is more room above. */
export function placeMenu(event: SyntheticEvent<HTMLDetailsElement>): void {
  const menu = event.currentTarget;
  const pop = menu.querySelector<HTMLElement>(".menu-pop");
  if (!menu.open || !pop) return;

  const row = menu.getBoundingClientRect();
  const bounds = menu.closest(".sidebar")?.getBoundingClientRect() ?? { top: 0, bottom: window.innerHeight };
  const below = bounds.bottom - row.bottom;
  const above = row.top - bounds.top;
  menu.toggleAttribute("data-up", below < pop.offsetHeight + 8 && above > below);
}

export function IconButton({ label, glyph, onClick }: { label: string; glyph: string; onClick: () => void }) {
  return (
    <button type="button" className="icon" aria-label={label} title={label} onClick={onClick}>
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
      className={danger ? "danger" : undefined}
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
      className="title-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (value.trim() && (!isLink || url.trim()))
          onSubmit(value.trim(), isLink ? url.trim() : undefined, askRedirect ? keep : undefined);
      }}
      onKeyDown={(e) => e.key === "Escape" && onCancel()}
    >
      <input
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
          aria-label="Link URL"
          type="url"
          placeholder="https://"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
        />
      )}
      {askRedirect && (
        <label className="keep-url">
          <input type="checkbox" checked={keep} onChange={(e) => setKeep(e.target.checked)} />
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
