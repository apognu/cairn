import { useEffect, useId, useRef } from "react";
import type { components } from "./api/schema";
import { Icon } from "./icons";
import { pendingCount, useChecks, usePending, usePublish } from "./queries";

type Change = components["schemas"]["Change"];

const badge: Record<Change["kinds"][number], string> = {
  new: "New",
  edited: "Edited",
  renamed: "Renamed",
  moved: "Moved",
  header: "Header",
  deleted: "Deleted",
};

/** What publishing will change on the live site, with every URL that is about to stop working called out. */
export function PublishDialog({ onClose, onPublished }: { onClose: () => void; onPublished: (pages: number) => void }) {
  const id = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const pending = usePending();
  const publish = usePublish();
  const checks = useChecks();
  const problems = checks.data ?? [];

  useEffect(() => dialog.current?.showModal(), []);

  const changes = pending.data?.changes ?? [];
  const settingsChanged = pending.data?.settings ?? false;
  const count = pendingCount(pending.data);
  const lost = pending.data?.lost_urls ?? [];
  const broken = pending.data?.broken_links ?? [];
  const lostPaths = new Set(lost.map((l) => l.path));

  return (
    <dialog ref={dialog} className="settings publish" aria-labelledby={`${id}-title`} onClose={onClose}>
      <h2 id={`${id}-title`}>Publish changes</h2>
      <p className="muted">
        {count === 0 ? "Nothing has changed since the live site." : "Compared with the live site."}
      </p>

      {problems.length > 0 && (
        <div className="publish-problems" role="alert">
          <Icon name="info" size={18} />
          <div>
            <strong>
              {problems.length === 1 ? "1 problem stops" : `${problems.length} problems stop`} this publish
            </strong>
            <ul aria-label="Problems">
              {problems.map((p) => (
                <li key={`${p.page_id}-${p.kind}-${p.message}`}>
                  <strong>{p.page_title}</strong> <span>{p.message}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}

      {lost.length > 0 && (
        <div className="publish-warning" role="alert">
          <Icon name="info" size={18} />
          <div>
            <strong>{lost.length === 1 ? "1 URL" : `${lost.length} URLs`} will be a 404 after publish</strong>
            <ul>
              {lost.map((l) => (
                <li key={l.path}>
                  <code>{l.path}</code> <span className="muted">{l.title}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}

      {broken.length > 0 && (
        <div className="publish-warning" role="alert">
          <Icon name="info" size={18} />
          <div>
            <strong>{broken.length === 1 ? "1 link points" : `${broken.length} links point`} to a deleted page</strong>
            <ul>
              {broken.map((b) => (
                <li key={`${b.source_id}-${b.target_title}`}>
                  {b.source_title} <span className="muted">links to {b.target_title}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}

      {count > 0 && (
        <table className="changes">
          <thead>
            <tr>
              <th scope="col">Change</th>
              <th scope="col">Page</th>
            </tr>
          </thead>
          <tbody>
            {settingsChanged && (
              <tr>
                <td>
                  <span className="badge" data-kind="edited">
                    {badge.edited}
                  </span>
                </td>
                <td>
                  <strong>Site and theme</strong>
                  <div className="url-note">every page is rendered again</div>
                </td>
              </tr>
            )}
            {changes.map((c) => (
              <tr key={c.page_id}>
                <td>
                  {c.kinds.map((k) => (
                    <span key={k} className="badge" data-kind={k}>
                      {badge[k]}
                    </span>
                  ))}
                </td>
                <td>
                  <strong>{c.title}</strong>
                  <UrlNote
                    change={c}
                    lost={c.old_url !== null && c.old_url !== undefined && lostPaths.has(c.old_url)}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {publish.isError && (
        <p role="alert" className="error-text">
          {publish.error.message}
        </p>
      )}
      <div className="dialog-actions">
        <button type="button" onClick={() => dialog.current?.close()}>
          Cancel
        </button>
        <button
          type="button"
          className="primary"
          disabled={publish.isPending || pending.isPending || checks.isPending || problems.length > 0}
          onClick={() =>
            publish.mutate(undefined, {
              onSuccess: (release) => {
                onPublished(release.page_count);
                dialog.current?.close();
              },
            })
          }
        >
          {publish.isPending
            ? "Publishing…"
            : count === 0
              ? "Publish site"
              : `Publish ${count} ${count === 1 ? "change" : "changes"}`}
        </button>
      </div>
    </dialog>
  );
}

function UrlNote({ change, lost }: { change: Change; lost: boolean }) {
  const { old_url, new_url } = change;
  if (!old_url || old_url === new_url) return null;

  if (lost) {
    return (
      <div className="url-note lost">
        <code>{old_url}</code> · 404 after publish
      </div>
    );
  }
  if (change.redirected) {
    return (
      <div className="url-note">
        adds redirect from <code>{old_url}</code>
      </div>
    );
  }
  return null;
}
