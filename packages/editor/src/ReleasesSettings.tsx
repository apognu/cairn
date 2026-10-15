import { useId, useRef, useState } from "react";
import type { components } from "./api/schema";
import { useMe, useReleases, useRollback, useSiteInfo } from "./queries";
import { useModal } from "./sidePanel";

type Row = components["schemas"]["ReleaseRow"];

const SHOWN_CHANGES = 3;

const DAY = 24 * 60 * 60 * 1000;

/** "Today 10:41", "Yesterday 17:12", "Oct 8, 15:03". */
function when(timestamp: string, now = new Date()): string {
  const date = new Date(timestamp);
  const time = date.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });
  const midnight = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((midnight(now) - midnight(date)) / DAY);

  if (days === 0) return `Today ${time}`;
  if (days === 1) return `Yesterday ${time}`;
  return `${date.toLocaleDateString("en-US", { month: "short", day: "numeric" })}, ${time}`;
}

function changesText(row: Row): string {
  if (row.changes.length === 0) return "No page changed";
  const names = row.changes.slice(0, SHOWN_CHANGES).join(", ");
  const more = row.changes.length - SHOWN_CHANGES;
  const count = row.changes.length === 1 ? "1 change" : `${row.changes.length} changes`;

  return `${count} · ${names}${more > 0 ? ` +${more}` : ""}`;
}

/** Every release that was the site, newest first, with a way back to an earlier one. */
export function ReleasesSettings() {
  const releases = useReleases();
  const site = useSiteInfo();
  const me = useMe();
  const [asking, setAsking] = useState<Row | null>(null);

  return (
    <>
      <div className="settings-form wide releases">
        <h1>Releases</h1>
        <p className="muted">
          Every publish creates a release. Rolling back makes an earlier one the live site again; drafts are never
          touched.
        </p>
        {site.data && (
          <p className="muted">
            Target: <strong>{site.data.target}</strong> · {site.data.url.replace(/^https?:\/\//, "")}
          </p>
        )}

        <table className="changes" aria-label="Releases">
          <thead>
            <tr>
              <th scope="col">Release</th>
              <th scope="col">Published</th>
              <th scope="col">Changes</th>
              <th scope="col">By</th>
              <th scope="col">Status</th>
              <th scope="col">
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {releases.data?.length === 0 && (
              <tr>
                <td colSpan={6} className="muted">
                  Nothing has been published yet.
                </td>
              </tr>
            )}
            {releases.data?.map((row) => (
              <tr key={row.release.id}>
                <td>
                  <strong>#{row.number}</strong>
                </td>
                <td className="muted">{when(row.release.created_at)}</td>
                <td>{changesText(row)}</td>
                <td className="muted">{row.release.created_by === me.data?.id ? "You" : row.published_by}</td>
                <td>
                  {row.release.status === "live" && (
                    <span className="badge" data-kind="new">
                      Live
                    </span>
                  )}
                </td>
                <td>
                  {row.can_roll_back && (
                    <button
                      type="button"
                      className="small"
                      onClick={() => setAsking(row)}
                      aria-label={`Roll back to #${row.number}`}
                    >
                      Roll back to this
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        {releases.isError && (
          <p role="alert" className="error-text">
            {releases.error.message}
          </p>
        )}
      </div>
      {/* Outside the form, whose heading styles would apply to the dialog's. */}
      {asking && <RollbackDialog row={asking} onClose={() => setAsking(null)} />}
    </>
  );
}

function RollbackDialog({ row, onClose }: { row: Row; onClose: () => void }) {
  const id = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const rollback = useRollback();

  useModal(dialog);

  return (
    <dialog ref={dialog} className="settings" aria-labelledby={`${id}-title`} onClose={onClose}>
      <h2 id={`${id}-title`}>Roll back to release #{row.number}?</h2>
      <ul>
        <li>The live site will match #{row.number} within a few seconds, its redirects and search index included.</li>
        <li>What was published after it shows as pending again; no draft is lost.</li>
        {!row.exact && (
          <li>
            This server no longer has its files, so it is rendered again from its saved pages. Snippets, variables and
            terms are the current ones.
          </li>
        )}
      </ul>
      {rollback.isError && (
        <p role="alert" className="error-text">
          {rollback.error.message}
        </p>
      )}
      <div className="dialog-actions">
        <button type="button" onClick={() => dialog.current?.close()}>
          Cancel
        </button>
        <button
          type="button"
          className="primary"
          disabled={rollback.isPending}
          onClick={() => rollback.mutate(row.release.id, { onSuccess: () => dialog.current?.close() })}
        >
          {rollback.isPending ? "Rolling back…" : "Roll back"}
        </button>
      </div>
    </dialog>
  );
}
