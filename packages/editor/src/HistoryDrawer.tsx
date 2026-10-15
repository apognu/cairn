import type { DocNode } from "@cairn/renderer";
import { useEffect, useId, useMemo, useRef, useState } from "react";
import { api } from "./api/client";
import { unwrap } from "./api/unwrap";
import { blockLines, diffLines } from "./diff";
import { useHistory, useRestoreVersion, useRevision } from "./queries";
import { useSidePanel } from "./sidePanel";
import type { Page } from "./tree";
import * as ui from "./ui";
import { saveSettled } from "./useDraftSync";

const version = "border-b border-border last:border-b-0";
/** A version's old title or description can be long: it wraps beside its checkbox instead of running out of the drawer. */
const option = "flex items-start gap-2 mt-0 mb-1.5 text-ui font-normal";
const optionText = "flex-1 min-w-0 wrap-anywhere";

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
  // Title and description come back only when ticked; another version starts unticked.
  const [withTitle, setWithTitle] = useState(false);
  const [withDescription, setWithDescription] = useState(false);
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
    <dialog ref={dialog} className={ui.drawer} aria-labelledby={`${id}-title`} onClose={onClose}>
      <header className={ui.drawerHead}>
        <h2 id={`${id}-title`} className={ui.drawerTitle}>
          History
        </h2>
        <button type="button" className={ui.icon} aria-label="Close history" onClick={() => dialog.current?.close()}>
          ×
        </button>
      </header>
      <p className={ui.drawerNote}>
        Every save is kept. Restoring a version makes it the page's draft; the draft it replaces stays here.
      </p>

      <ul className="mt-0 mb-4.5 p-0 list-none border border-input rounded-lg" aria-label="Versions">
        {history.isPending && <li className={`${version} text-muted`}>Loading…</li>}
        {sessions.map((s) => (
          <li key={s.newest.id} className={version}>
            <button
              type="button"
              className="flex items-center gap-2.5 w-full py-2 px-3 border-0 bg-transparent text-left cursor-pointer aria-pressed:bg-accent-soft"
              aria-pressed={chosen === s.newest.id}
              onClick={() => {
                setChosen(s.newest.id);
                setWithTitle(false);
                setWithDescription(false);
              }}
            >
              <strong>{when(s.newest.created_at)}</strong>
              {s.published && (
                <span className={ui.badge("new")} data-kind="new">
                  Published
                </span>
              )}
              <span className="ml-auto text-muted">{s.count > 1 ? `${s.count} saves` : "1 save"}</span>
            </button>
          </li>
        ))}
      </ul>

      {chosen && revision.data && (
        <section className={ui.drawerSection}>
          <h3 className={ui.drawerHeading}>
            {changed ? "Restoring this version would" : "This version is the same as the page now"}
          </h3>
          <ul
            className="mt-0 mb-3 p-0 list-none border border-input rounded-lg overflow-hidden font-mono text-[12.5px]"
            aria-label="Changes"
          >
            {diff
              .filter((l) => l.kind !== "same")
              .map((l, i) => (
                // biome-ignore lint/suspicious/noArrayIndexKey: diff lines can repeat
                <li key={i} className={l.kind === "added" ? ui.diffAdded : ui.diffRemoved} data-kind={l.kind}>
                  {l.kind === "added" ? "+ " : "− "}
                  {l.text || "(empty line)"}
                </li>
              ))}
          </ul>
          <fieldset className={`${ui.drawerSection} border-0 p-0`}>
            <legend className={ui.drawerHeading}>Also restore</legend>
            <label className={option}>
              <input
                type="checkbox"
                className="mt-0.5"
                checked={withTitle}
                disabled={revision.data.title === page.title}
                onChange={(e) => setWithTitle(e.target.checked)}
              />
              <span className={optionText}>
                Title <span className="text-muted">{versionValue(revision.data.title, page.title)}</span>
              </span>
            </label>
            <label className={option}>
              <input
                type="checkbox"
                className="mt-0.5"
                checked={withDescription}
                disabled={
                  !revision.data.description_recorded || (revision.data.description ?? "") === (page.description ?? "")
                }
                onChange={(e) => setWithDescription(e.target.checked)}
              />
              <span className={optionText}>
                Description{" "}
                <span className="text-muted">
                  {revision.data.description_recorded
                    ? versionValue(revision.data.description ?? "", page.description ?? "", "no description")
                    : "not recorded for this version"}
                </span>
              </span>
            </label>
          </fieldset>
          {restore.isError && (
            <p role="alert" className="text-danger">
              {restore.error.message}
            </p>
          )}
          <div className={ui.actions}>
            <button
              type="button"
              className={ui.primary}
              disabled={restore.isPending}
              onClick={() =>
                restore.mutate(
                  { revision_id: chosen, title: withTitle, description: withDescription },
                  {
                    onSuccess: () => {
                      onRestored();
                      dialog.current?.close();
                    },
                  },
                )
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

/** What a version's title or description was, or that it is the same as now. The page keeps its URL either way. */
function versionValue(then: string, now: string, empty = "none") {
  if (then === now) return "unchanged";
  return then ? `“${then}”` : empty;
}
