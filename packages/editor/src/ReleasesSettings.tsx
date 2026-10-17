import { useId, useRef, useState } from "react";
import type { components } from "./api/schema";
import { useCan, useMe, useReleases, useRollback, useSiteInfo } from "./queries";
import { useModal } from "./sidePanel";
import * as ui from "./ui";

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
const quiet = `${ui.td} text-muted text-help whitespace-nowrap`;

export function ReleasesSettings() {
  const releases = useReleases();
  const site = useSiteInfo();
  const me = useMe();
  const canPublish = useCan("publisher");
  const [asking, setAsking] = useState<Row | null>(null);

  return (
    <>
      <div className="max-w-280">
        <h1 className={ui.settingsTitle}>Releases</h1>
        <p className="text-muted">
          Every publish creates a release. Rolling back makes an earlier one the live site again; drafts are never
          touched.
        </p>
        {site.data && (
          <p className="text-muted">
            Target: <strong>{site.data.target}</strong> · {site.data.url.replace(/^https?:\/\//, "")}
          </p>
        )}

        <table className={ui.table} aria-label="Releases">
          <thead>
            <tr>
              <th scope="col" className={ui.th}>
                Release
              </th>
              <th scope="col" className={ui.th}>
                Published
              </th>
              <th scope="col" className={ui.th}>
                Changes
              </th>
              <th scope="col" className={ui.th}>
                By
              </th>
              <th scope="col" className={ui.th}>
                Status
              </th>
              <th scope="col" className={ui.th}>
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {releases.data?.length === 0 && (
              <tr>
                <td colSpan={6} className={quiet}>
                  Nothing has been published yet.
                </td>
              </tr>
            )}
            {releases.data?.map((row) => (
              <tr key={row.release.id}>
                <td className={ui.td}>
                  <strong>#{row.number}</strong>
                </td>
                <td className={quiet}>{when(row.release.created_at)}</td>
                <td className={ui.td}>{changesText(row)}</td>
                <td className={quiet}>{row.release.created_by === me.data?.id ? "You" : row.published_by}</td>
                <td className={ui.td}>
                  {row.release.status === "live" && (
                    <span className={ui.badge("new")} data-kind="new">
                      Live
                    </span>
                  )}
                </td>
                <td className={ui.td}>
                  {row.can_roll_back && canPublish && (
                    <button
                      type="button"
                      className={ui.tinyButton}
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
          <p role="alert" className="text-danger">
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
    <dialog ref={dialog} className={ui.dialog} aria-labelledby={`${id}-title`} onClose={onClose}>
      <h2 id={`${id}-title`} className={ui.dialogTitle}>
        Roll back to release #{row.number}?
      </h2>
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
        <p role="alert" className="text-danger">
          {rollback.error.message}
        </p>
      )}
      <div className={ui.actions}>
        <button type="button" className={ui.secondary} onClick={() => dialog.current?.close()}>
          Cancel
        </button>
        <button
          type="button"
          className={ui.primary}
          disabled={rollback.isPending}
          onClick={() => rollback.mutate(row.release.id, { onSuccess: () => dialog.current?.close() })}
        >
          {rollback.isPending ? "Rolling back…" : "Roll back"}
        </button>
      </div>
    </dialog>
  );
}
