import type { DocNode } from "@cairn/renderer";
import { useEffect, useId, useMemo, useRef, useState } from "react";
import { api } from "./api/client";
import { unwrap } from "./api/unwrap";
import { blockLines, diffLines } from "./diff";
import { useHistory, useRestoreVersion, useRevision } from "./queries";
import { useSidePanel } from "./sidePanel";
import type { Page } from "./tree";
import { saveSettled } from "./useDraftSync";

/** Saves less than this far apart, by one author, read as one editing session. */
const SESSION_GAP_MS = 30 * 60 * 1000;

const when = (iso: string) =>
  new Date(iso).toLocaleString("en-GB", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });

/** The page's saved versions, grouped into editing sessions, with what restoring one would change. */
export function HistoryDrawer({
  page,
  onClose,
  onRestored,
}: {
  page: Page;
  onClose: () => void;
  onRestored: () => void;
}) {
  const id = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const history = useHistory(page.id);
  const restore = useRestoreVersion(page.id);
  const [chosen, setChosen] = useState<string | null>(null);
  const [current, setCurrent] = useState<DocNode | null>(null);
  const revision = useRevision(chosen);

  useSidePanel(dialog);
  // What the page is now, to compare the chosen version with: after the save that leaving the canvas for this button started.
  useEffect(() => {
    saveSettled(page.id)
      .then(() => api.GET("/api/pages/{id}/draft", { params: { path: { id: page.id } } }))
      .then((r) => setCurrent(unwrap(r).doc as DocNode))
      // Without the current draft there is simply no comparison to show.
      .catch(() => setCurrent(null));
  }, [page.id]);

  const sessions = useMemo(() => {
    const groups: { newest: NonNullable<typeof history.data>[number]; count: number; published: boolean }[] = [];
    for (const v of history.data ?? []) {
      const last = groups.at(-1);
      const gap = last
        ? new Date(last.newest.created_at).getTime() - new Date(v.created_at).getTime()
        : Number.POSITIVE_INFINITY;
      const sameSession =
        last &&
        gap < SESSION_GAP_MS &&
        last.newest.author_id === v.author_id &&
        !v.published_at &&
        !last.newest.published_at;

      if (sameSession) last.count++;
      else groups.push({ newest: v, count: 1, published: Boolean(v.published_at) });
    }
    return groups;
  }, [history.data]);

  const diff = useMemo(
    () => (revision.data && current ? diffLines(blockLines(current), blockLines(revision.data.doc as DocNode)) : []),
    [revision.data, current],
  );
  const changed = diff.some((l) => l.kind !== "same");

  return (
    <dialog ref={dialog} className="drawer wide-drawer" aria-labelledby={`${id}-title`} onClose={onClose}>
      <header className="drawer-head">
        <h2 id={`${id}-title`}>History</h2>
        <button type="button" className="icon" aria-label="Close history" onClick={() => dialog.current?.close()}>
          ×
        </button>
      </header>
      <p className="muted">
        Every save is kept. Restoring a version makes it the page's draft; the draft it replaces stays here.
      </p>

      <ul className="versions" aria-label="Versions">
        {history.isPending && <li className="muted">Loading…</li>}
        {sessions.map((s) => (
          <li key={s.newest.id}>
            <button type="button" aria-pressed={chosen === s.newest.id} onClick={() => setChosen(s.newest.id)}>
              <strong>{when(s.newest.created_at)}</strong>
              {s.published && (
                <span className="badge" data-kind="new">
                  Published
                </span>
              )}
              <span className="muted">{s.count > 1 ? `${s.count} saves` : "1 save"}</span>
            </button>
          </li>
        ))}
      </ul>

      {chosen && revision.data && (
        <section className="form-field">
          <h3>{changed ? "Restoring this version would" : "This version is the same as the page now"}</h3>
          <ul className="diff" aria-label="Changes">
            {diff
              .filter((l) => l.kind !== "same")
              .map((l, i) => (
                // biome-ignore lint/suspicious/noArrayIndexKey: diff lines can repeat
                <li key={i} data-kind={l.kind}>
                  {l.kind === "added" ? "+ " : "− "}
                  {l.text || "(empty line)"}
                </li>
              ))}
          </ul>
          {restore.isError && (
            <p role="alert" className="error-text">
              {restore.error.message}
            </p>
          )}
          <div className="dialog-actions">
            <button
              type="button"
              className="primary"
              disabled={restore.isPending}
              onClick={() =>
                restore.mutate(chosen, {
                  onSuccess: () => {
                    onRestored();
                    dialog.current?.close();
                  },
                })
              }
            >
              Restore as draft
            </button>
          </div>
        </section>
      )}
    </dialog>
  );
}
