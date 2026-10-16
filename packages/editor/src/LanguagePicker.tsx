import { useEffect, useId, useMemo, useRef, useState } from "react";
import { Icon } from "./icons";
import list from "./languages.json";
import * as ui from "./ui";

interface Language {
  /** As written in the document; the publish highlighter's test checks that it knows every one. */
  value: string;
  label: string;
  /** Other names people search by: file extensions, tools, nicknames. */
  aliases?: string[];
}

const plain: Language = { value: "", label: "Plain text", aliases: ["text", "none"] };
const languages: Language[] = [plain, ...(list as Language[]).sort((a, b) => a.label.localeCompare(b.label))];

/** The language field of a code block: the chosen language, which opens a searchable list under it. */
export function LanguagePicker({
  id,
  value,
  onChange,
}: {
  id: string;
  value: string;
  onChange: (next: string) => void;
}) {
  const listId = useId();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const root = useRef<HTMLDivElement>(null);
  const toggle = useRef<HTMLButtonElement>(null);
  const listbox = useRef<HTMLDivElement>(null);

  // A language written some other way (in an imported page, say) stays shown and selectable instead of silently turning to plain text.
  const current = languages.find((l) => l.value === value) ?? { value, label: value };
  const all = languages.includes(current) ? languages : [current, ...languages];

  const matches = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return all;

    // A name typed in full comes first ("c" is C, not every language with a c), then names starting with it, then names containing it.
    const rank = ({ value, label, aliases = [] }: Language) => {
      const names = [value, label.toLowerCase(), ...aliases];
      if (names.includes(q)) return 0;
      if (names.some((name) => name.startsWith(q))) return 1;
      if (names.some((name) => name.includes(q))) return 2;
      return 3;
    };
    return all
      .map((language) => [rank(language), language] as const)
      .filter(([r]) => r < 3)
      .sort(([a], [b]) => a - b)
      .map(([, language]) => language);
  }, [all, query]);

  const choice = Math.min(active, matches.length - 1);

  useEffect(() => {
    if (!open) return;

    const onPointer = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", onPointer);

    return () => document.removeEventListener("pointerdown", onPointer);
  }, [open]);

  // The list opens with the chosen language in its middle, and the arrow keys keep the lit row in view.
  const opening = useRef(false);
  useEffect(() => {
    if (!open) return;
    listbox.current
      ?.querySelector(`[data-index="${choice}"]`)
      ?.scrollIntoView({ block: opening.current ? "center" : "nearest" });
    opening.current = false;
  }, [open, choice]);

  const show = () => {
    setQuery("");
    setActive(Math.max(all.indexOf(current), 0));
    opening.current = !open;
    setOpen(!open);
  };

  const close = () => {
    setOpen(false);
    toggle.current?.focus();
  };

  const choose = (language: Language | undefined) => {
    if (!language) return;
    onChange(language.value);
    close();
  };

  return (
    <div ref={root}>
      <button
        ref={toggle}
        id={id}
        type="button"
        className="flex items-center gap-2 w-full h-8.5 px-2.5 border border-input rounded-md bg-bg text-text text-ui text-left cursor-pointer"
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={show}
      >
        <span className="flex-1 min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">{current.label}</span>
        <Icon name="chevron" size={14} className="text-muted" />
      </button>
      {open && (
        <fieldset
          className="flex min-w-0 flex-col gap-2 mt-1.5 p-2 border border-input rounded-lg bg-bg"
          onKeyDown={(event) => {
            if (event.key !== "Escape") return;
            // Only the list closes, not the panel around it.
            event.preventDefault();
            event.stopPropagation();
            close();
          }}
        >
          <legend className="sr-only">Languages</legend>
          <input
            type="search"
            className={ui.input}
            // biome-ignore lint/a11y/noAutofocus: opening the list is asking to search it
            autoFocus
            placeholder={`Search ${languages.length - 1} languages`}
            aria-label="Search languages"
            role="combobox"
            aria-expanded="true"
            aria-controls={listId}
            aria-activedescendant={matches.length > 0 ? `${listId}-${choice}` : undefined}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setActive(0);
            }}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") setActive((choice + 1) % Math.max(matches.length, 1));
              else if (e.key === "ArrowUp") setActive((choice + matches.length - 1) % Math.max(matches.length, 1));
              else if (e.key === "Enter") choose(matches[choice]);
              else return;
              e.preventDefault();
            }}
          />
          <div ref={listbox} id={listId} role="listbox" aria-label="Languages" className="max-h-58 overflow-y-auto">
            {matches.map((language, i) => (
              <div
                key={language.value}
                id={`${listId}-${i}`}
                data-index={i}
                role="option"
                className="group/option"
                aria-selected={i === choice}
                tabIndex={-1}
              >
                <button
                  type="button"
                  tabIndex={-1}
                  className="flex items-center gap-2 w-full py-1.5 px-2 border-0 rounded-md bg-transparent text-text text-ui text-left cursor-pointer group-aria-selected/option:bg-accent-soft"
                  onPointerMove={() => i !== choice && setActive(i)}
                  // The focus stays in the search field, which drives the list.
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => choose(language)}
                >
                  <span className="flex-1 min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">
                    {language.label}
                  </span>
                  {language.value === value && <Icon name="check" size={14} className="text-accent" />}
                </button>
              </div>
            ))}
            {matches.length === 0 && <small className="block py-1.5 px-2 text-muted">No language matches.</small>}
          </div>
        </fieldset>
      )}
    </div>
  );
}
