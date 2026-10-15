import { createContext, createElement, useContext } from "react";

/** An icon of the Lucide collection: its SVG elements, each a tag and its attributes, drawn in a 24x24 box with a 2px stroke. */
export type IconNode = [tag: string, attrs: Record<string, string>][];

// The drawings `Icon` can use. `ssr.js` and the editor register the whole collection; a published page registers only the icons it
// shows, which the server render lists in the page (`UsedIcons`). Components stay free of the collection, which would weigh on every page.
const registry = new Map<string, IconNode>();

export function registerIcons(icons: Record<string, IconNode>) {
  for (const [name, node] of Object.entries(icons)) registry.set(name, node);
}

/** Every registered icon name, for the editor's picker. */
export const registeredIcons = (): string[] => [...registry.keys()];

const FALLBACK = "star";

/** Collects the names of the icons a render draws, so `UsedIcons` can list them. Absent in the editor. */
export const IconUsageContext = createContext<Set<string> | null>(null);

export function Icon({ name, size = 20 }: { name: string; size?: number }) {
  const usage = useContext(IconUsageContext);
  const drawn = registry.has(name) ? name : FALLBACK;
  usage?.add(drawn);

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {/* The elements of an icon never move: their position is their key. */}
      {(registry.get(drawn) ?? []).map(([tag, attrs], i) => createElement(tag, { key: i, ...attrs }))}
    </svg>
  );
}

/**
 * The drawings of the icons rendered so far, as JSON for the browser to register before hydrating. It must come after every `Icon` of the
 * page in render order; in the browser it renders the same list from what it registered, so hydration finds the same markup.
 */
export function UsedIcons() {
  const usage = useContext(IconUsageContext);
  const used = Object.fromEntries([...(usage ?? [])].sort().map((name) => [name, registry.get(name)]));
  // `<` in a script would let a value close it; Lucide's data has none, but JSON in HTML is escaped as a rule.
  const json = JSON.stringify(used).replace(/</g, "\\u003c");

  return (
    // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON data, escaped above
    <script type="application/json" id="icons" dangerouslySetInnerHTML={{ __html: json }} />
  );
}

/** Names the first built-in icons had, before Lucide, and the Lucide icon that draws the same thing. */
export const legacyIconNames: Record<string, string> = { bolt: "zap", git: "git-branch", chart: "chart-column" };

/** Props with every `icon` field written with a legacy name renamed; the migration of the components that show icons. */
export function upgradeIconNames(value: Record<string, unknown>): Record<string, unknown> {
  const walk = (v: unknown): unknown => {
    if (Array.isArray(v)) return v.map(walk);
    if (v === null || typeof v !== "object") return v;

    return Object.fromEntries(
      Object.entries(v).map(([key, inner]) => [
        key,
        key === "icon" && typeof inner === "string" ? (legacyIconNames[inner] ?? inner) : walk(inner),
      ]),
    );
  };

  return walk(value) as Record<string, unknown>;
}
