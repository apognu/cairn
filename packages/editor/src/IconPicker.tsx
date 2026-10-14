import { registeredIcons, Icon as SiteIcon } from "@cairn/components";
import tags from "lucide-static/tags.json";
import { useEffect, useMemo, useRef, useState } from "react";
import { Icon } from "./icons";

const tagsByName = tags as Record<string, string[]>;

// Drawing every icon at once makes the grid slow to open; a search finds the rest.
const SHOWN = 240;

/** An icon field: the chosen icon, which opens a searchable grid of the whole collection under it. */
export function IconPicker({ id, value, onChange }: { id: string; value: string; onChange: (next: string) => void }) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const root = useRef<HTMLDivElement>(null);
  const toggle = useRef<HTMLButtonElement>(null);
  const names = useMemo(() => registeredIcons().sort(), []);

  const matches = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return names;

    // The exact name, then names starting with the search, then names containing it, then icons whose tags match: "trash" picks the
    // bin, not a bin found through "delete".
    const rank = (name: string) =>
      name === q
        ? 0
        : name.startsWith(q)
          ? 1
          : name.includes(q)
            ? 2
            : tagsByName[name]?.some((tag) => tag.includes(q))
              ? 3
              : 4;
    return names
      .map((name) => [rank(name), name] as const)
      .filter(([r]) => r < 4)
      .sort(([a, x], [b, y]) => a - b || x.localeCompare(y))
      .map(([, name]) => name);
  }, [names, query]);

  useEffect(() => {
    if (!open) return;

    const onPointer = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", onPointer);

    return () => document.removeEventListener("pointerdown", onPointer);
  }, [open]);

  const choose = (name: string) => {
    onChange(name);
    setOpen(false);
    setQuery("");
    toggle.current?.focus();
  };

  return (
    <div className="icon-picker" ref={root}>
      <button
        ref={toggle}
        id={id}
        type="button"
        className="icon-picker-current"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        <SiteIcon name={value} size={18} />
        <span>{value}</span>
        <Icon name="chevron" size={14} />
      </button>
      {open && (
        <fieldset
          className="icon-picker-panel"
          onKeyDown={(event) => {
            if (event.key !== "Escape") return;
            // Only the grid closes, not the drawer around it.
            event.preventDefault();
            event.stopPropagation();
            setOpen(false);
            toggle.current?.focus();
          }}
        >
          <legend className="sr-only">Icons</legend>
          <input
            type="search"
            // biome-ignore lint/a11y/noAutofocus: opening the grid is asking to search it
            autoFocus
            placeholder={`Search ${names.length.toLocaleString("en")} icons`}
            aria-label="Search icons"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && matches[0]) {
                e.preventDefault();
                choose(matches[0]);
              }
            }}
          />
          <div className="icon-grid">
            {matches.slice(0, SHOWN).map((name) => (
              <button
                key={name}
                type="button"
                title={name}
                aria-label={name}
                aria-pressed={name === value}
                onClick={() => choose(name)}
              >
                <SiteIcon name={name} size={20} />
              </button>
            ))}
          </div>
          <small className="muted">
            {matches.length === 0
              ? "No icon matches."
              : matches.length > SHOWN
                ? `${SHOWN} of ${matches.length.toLocaleString("en")} shown: search to find the others.`
                : `${matches.length} ${matches.length === 1 ? "icon" : "icons"}.`}
          </small>
        </fieldset>
      )}
    </div>
  );
}
