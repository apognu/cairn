import type { components } from "./api/schema";

type SiteLink = components["schemas"]["SiteLink"];

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
    <div className="link-list">
      {links.map((link, index) => (
        // The rows have no identity besides their place, and are only added or removed at the end or by position.
        // biome-ignore lint/suspicious/noArrayIndexKey: see above
        <div className="row-inline" key={index}>
          <input
            aria-label={`${label}: label ${index + 1}`}
            placeholder="Label"
            value={link.label}
            onChange={(e) => set(index, { label: e.target.value })}
          />
          <input
            aria-label={`${label}: address ${index + 1}`}
            placeholder="https://…"
            value={link.url}
            onChange={(e) => set(index, { url: e.target.value })}
          />
          <button
            type="button"
            className="secondary"
            aria-label={`Remove ${label} link ${index + 1}`}
            onClick={() => onChange(links.filter((_, i) => i !== index))}
          >
            ×
          </button>
        </div>
      ))}
      <button type="button" className="secondary" onClick={() => onChange([...links, { label: "", url: "https://" }])}>
        Add a link
      </button>
    </div>
  );
}
