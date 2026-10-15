import type { components } from "./api/schema";
import * as ui from "./ui";

type SiteLink = components["schemas"]["SiteLink"];

const linkInput = "flex-1 min-w-0 h-8.5 px-2.5 border border-input rounded-md bg-bg text-text text-ui";

/** An ordered list of label and address pairs, as in the header and the footer columns. */
export function LinkList({
  label,
  links,
  onChange,
}: {
  label: string;
  links: SiteLink[];
  onChange: (links: SiteLink[]) => void;
}) {
  const set = (index: number, patch: Partial<SiteLink>) =>
    onChange(links.map((link, i) => (i === index ? { ...link, ...patch } : link)));

  return (
    <div className="flex flex-col items-start gap-2">
      {links.map((link, index) => (
        // The rows have no identity besides their place, and are only added or removed at the end or by position.
        // biome-ignore lint/suspicious/noArrayIndexKey: see above
        <div className={`${ui.inline} w-full`} key={index}>
          <input
            className={linkInput}
            aria-label={`${label}: label ${index + 1}`}
            placeholder="Label"
            value={link.label}
            onChange={(e) => set(index, { label: e.target.value })}
          />
          <input
            className={linkInput}
            aria-label={`${label}: address ${index + 1}`}
            placeholder="https://…"
            value={link.url}
            onChange={(e) => set(index, { url: e.target.value })}
          />
          <button
            type="button"
            className={ui.secondary}
            aria-label={`Remove ${label} link ${index + 1}`}
            onClick={() => onChange(links.filter((_, i) => i !== index))}
          >
            ×
          </button>
        </div>
      ))}
      <button
        type="button"
        className={ui.secondary}
        onClick={() => onChange([...links, { label: "", url: "https://" }])}
      >
        Add a link
      </button>
    </div>
  );
}
