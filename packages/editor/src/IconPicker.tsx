import { registeredIcons, Icon as SiteIcon } from "@cairn/components";
import tags from "lucide-static/tags.json";
import { useEffect, useMemo, useRef, useState } from "react";
import { Icon } from "./icons";
import * as ui from "./ui";

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
    <div ref={root}>
      <button
        ref={toggle}
        id={id}
        type="button"
        className="flex items-center gap-2 w-full h-8.5 px-2.5 border border-input rounded-md bg-bg text-text text-ui text-left cursor-pointer"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        <SiteIcon name={value} size={18} />
        <span className="flex-1 min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">{value}</span>
        <Icon name="chevron" size={14} className="text-muted" />
      </button>
      {open && (
        <fieldset
          className="flex min-w-0 flex-col gap-2 mt-1.5 p-2 border border-input rounded-lg bg-bg"
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
            className={ui.input}
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
          <div className="grid grid-cols-[repeat(auto-fill,minmax(36px,1fr))] gap-0.5 max-h-58 overflow-y-auto">
            {matches.slice(0, SHOWN).map((name) => (
              <button
                key={name}
                type="button"
                className="flex items-center justify-center h-9 p-0 border border-transparent rounded-md bg-transparent text-text cursor-pointer hover:bg-hover aria-pressed:border-accent aria-pressed:bg-accent-soft aria-pressed:text-accent"
                title={name}
                aria-label={name}
                aria-pressed={name === value}
                onClick={() => choose(name)}
              >
                <SiteIcon name={name} size={20} />
              </button>
            ))}
          </div>
          <small className="text-muted">
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
