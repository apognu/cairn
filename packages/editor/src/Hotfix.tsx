import { useQueries, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "./api/client";
import type { components } from "./api/schema";
import { unwrap } from "./api/unwrap";
import { Icon } from "./icons";
import { useMe } from "./queries";

type Change = components["schemas"]["Change"];

/**
 * The hotfix side of the publish dialog: which pages are picked, which the server says go along with them (a new parent, a page left
 * under a deleted one), and the links that will show as plain text.
 */
export function useHotfix(changes: Change[]) {
  const [on, setOn] = useState(false);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const me = useMe();
  const sorted = [...picked].sort();

  const plan = useQuery({
    queryKey: ["hotfix-plan", sorted],
    enabled: on && sorted.length > 0,
    queryFn: async () => unwrap(await api.POST("/api/publish/plan", { body: { pages: sorted } })),
  });
  // The same versions the rows show, read from the same cache: who saved each page's draft.
  const authors = useQueries({
    queries: changes.map((c) => ({
      queryKey: ["revision", c.revision_id],
      enabled: on && Boolean(c.revision_id),
      staleTime: Number.POSITIVE_INFINITY,
      queryFn: async () =>
        unwrap(await api.GET("/api/revisions/{id}", { params: { path: { id: c.revision_id ?? "" } } })),
    })),
  });
  const mine = changes
    .filter((c, i) => me.data && c.revision_id !== c.live_revision_id && authors[i].data?.author_id === me.data.id)
    .map((c) => c.page_id);

  const pulled = on && sorted.length > 0 ? (plan.data?.pulled ?? []) : [];
  const going = new Set([...picked, ...pulled.map((p) => p.page_id)]);
  const toggle = (id: string) =>
    setPicked((now) => {
      const next = new Set(now);
      if (!next.delete(id)) next.add(id);
      return next;
    });

  return {
    on,
    setOn,
    picked,
    toggle,
    include: (id: string) => setPicked((now) => new Set([...now, id])),
    pickMine: () => setPicked(new Set(mine)),
    pickNone: () => setPicked(new Set()),
    mine,
    pulled,
    plainLinks: on && sorted.length > 0 ? (plan.data?.plain_links ?? []) : [],
    /** The pages this publish takes: picked, and pulled in. */
    going,
  };
}

export type Hotfix = ReturnType<typeof useHotfix>;

/** "Everything pending" or "Selected pages (hotfix)". */
export function ModeSwitch({ hotfix, count, base }: { hotfix: Hotfix; count: number; base: string | null }) {
  return (
    <div className="publish-mode">
      <fieldset className="segmented">
        <legend className="sr-only">What to publish</legend>
        <button type="button" aria-pressed={!hotfix.on} onClick={() => hotfix.setOn(false)}>
          Everything pending ({count})
        </button>
        <button type="button" aria-pressed={hotfix.on} onClick={() => hotfix.setOn(true)}>
          Selected pages (hotfix)
        </button>
      </fieldset>
      <span className="muted">
        {hotfix.on
          ? base
            ? `Builds on the live release from ${base}`
            : "Builds on an empty site"
          : "Diffs against the live site"}
      </span>
    </div>
  );
}

/** How to pick, and the shortcuts. */
export function PickBar({ hotfix }: { hotfix: Hotfix }) {
  return (
    <div className="pick-bar">
      <span className="muted">Only ticked pages go live, each as its latest saved draft. The rest stays pending.</span>
      <span className="muted">Select:</span>
      <button type="button" className="small" onClick={hotfix.pickMine} disabled={hotfix.mine.length === 0}>
        My changes ({hotfix.mine.length})
      </button>
      <button type="button" className="small" onClick={hotfix.pickNone} disabled={hotfix.picked.size === 0}>
        None
      </button>
    </div>
  );
}

/** What else goes out, and which links will show as plain text, each with a way to include its page. */
export function DependencyNotes({ hotfix }: { hotfix: Hotfix }) {
  if (hotfix.pulled.length === 0 && hotfix.plainLinks.length === 0) return null;

  return (
    <div className="publish-warning dependency-notes">
      <Icon name="info" size={18} />
      <ul aria-label="Dependencies">
        {hotfix.pulled.map((p) => (
          <li key={p.page_id}>
            <strong>{p.title}</strong> goes out too:{" "}
            {p.reason === "parent"
              ? "a selected page now lives under it."
              : "it sits under a page this publish removes."}
          </li>
        ))}
        {hotfix.plainLinks.map((l) => (
          <li key={`${l.source_title}-${l.target_id}`}>
            <strong>
              {l.source_title} links to {l.target_title}
            </strong>
            , which isn't selected and isn't live yet. That link shows as plain text until {l.target_title} is
            published.{" "}
            <button type="button" className="link-button" onClick={() => hotfix.include(l.target_id)}>
              Include it
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
