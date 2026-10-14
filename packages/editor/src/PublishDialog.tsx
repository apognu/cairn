import { useEffect, useId, useRef, useState } from "react";
import { serverUrl } from "./api/base";
import type { components } from "./api/schema";
import { Icon } from "./icons";
import { ago } from "./LockBanner";
import { badge, ChangeRow, ChecksStrip, LockWarning } from "./PublishParts";
import { pendingCount, useChecks, usePending, usePreview, usePromote, usePublish, useSiteInfo } from "./queries";
import { useModal } from "./sidePanel";
import { useLocks } from "./useLock";

type Change = components["schemas"]["Change"];
type Progress = components["schemas"]["PublishProgress"];

/** What publishing will change on the live site, with every URL that is about to stop working called out. */
export function PublishDialog({ onClose, onPublished }: { onClose: () => void; onPublished: (pages: number) => void }) {
  const id = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const pending = usePending();
  const publish = usePublish();
  const checks = useChecks();
  const problems = checks.data ?? [];
  const site = useSiteInfo();
  const preview = usePreview();
  const promote = usePromote();
  const previewed = preview.data;
  const canPromote = site.data?.capabilities.promote ?? false;
  // One build at a time; an action clears what the others last said, so an old error is not read as the new one's.
  const busy = publish.isPending || preview.isPending || promote.isPending;
  const building = publish.isPending || preview.isPending;
  const progress = useUploadProgress(building);
  const blocked = pending.isPending || checks.isPending || problems.length > 0;
  const run = (action: { reset: () => void }, go: () => void) => {
    for (const other of [publish, preview, promote]) if (other !== action) other.reset();
    go();
  };

  useModal(dialog);

  const changes = pending.data?.changes ?? [];
  const settingsChanged = pending.data?.settings ?? false;
  const count = pendingCount(pending.data);
  const lost = pending.data?.lost_urls ?? [];
  const broken = pending.data?.broken_links ?? [];
  const lostPaths = new Set(lost.map((l) => l.path));
  // Someone else's open editor on a page that goes out: what they saved goes out, what they did not stays with them. The warning
  // comes first.
  const others = (useLocks().data ?? []).filter((l) => !l.mine && l.page_title !== "");
  const lockedBy = new Map(others.map((l) => [l.page_id, l.user_name]));
  const live = site.data?.live;

  return (
    <dialog ref={dialog} className="settings publish" aria-labelledby={`${id}-title`} onClose={onClose}>
      <h2 id={`${id}-title`}>Publish changes</h2>
      <p className="muted">
        {site.data && (
          <>
            To <strong>{site.data.target}</strong> · {site.data.url.replace(/^https?:\/\//, "")}
            {live ? ` · last published ${ago(live.release.created_at)} by ${live.published_by}` : " · never published"}.{" "}
          </>
        )}
        {count === 0 ? "Nothing has changed since the live site." : "Compared with the live site."}
      </p>

      <LockWarning locks={others.filter((l) => changes.some((c) => c.page_id === l.page_id))} />

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
              <th scope="col">Last change</th>
              <th scope="col">Size</th>
              <th scope="col">
                <span className="sr-only">Actions</span>
              </th>
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
                <td colSpan={4}>
                  <strong>Site and theme</strong>
                  <div className="url-note">every page is rendered again</div>
                </td>
              </tr>
            )}
            {changes.map((c) => (
              <ChangeRow
                key={c.page_id}
                change={c}
                lockedBy={lockedBy.get(c.page_id)}
                note={
                  <UrlNote
                    change={c}
                    lost={c.old_url !== null && c.old_url !== undefined && lostPaths.has(c.old_url)}
                  />
                }
              />
            ))}
          </tbody>
        </table>
      )}

      <ChecksStrip
        problems={problems}
        files={live?.release.file_count}
        bytes={live?.release.bytes}
        maxFiles={site.data?.capabilities.max_files}
      />

      {(publish.isError || preview.isError || promote.isError) && (
        <p role="alert" className="error-text">
          {(publish.error ?? preview.error ?? promote.error)?.message}
        </p>
      )}
      {building && <UploadProgress progress={progress} />}
      {previewed && !building && (
        <output className="publish-preview">
          <Icon name="check" size={14} />
          <span>
            Preview ready.{" "}
            <a href={previewed.preview_url ?? `/_preview/${previewed.id}/`} target="_blank" rel="noopener">
              Open it
            </a>
            {canPromote ? "" : " · Publishing builds the site again."}
          </span>
        </output>
      )}
      <div className="dialog-actions">
        <button type="button" onClick={() => dialog.current?.close()}>
          {previewed ? "Close" : "Cancel"}
        </button>
        {site.data?.capabilities.preview && (
          <button type="button" disabled={busy || blocked} onClick={() => run(preview, () => preview.mutate())}>
            {preview.isPending ? "Building preview…" : previewed ? "Deploy new preview" : "Deploy preview"}
          </button>
        )}
        {previewed && canPromote && (
          <button
            type="button"
            disabled={busy}
            onClick={() =>
              run(promote, () =>
                promote.mutate(previewed.id, {
                  onSuccess: (release) => {
                    onPublished(release.page_count);
                    dialog.current?.close();
                  },
                }),
              )
            }
          >
            {promote.isPending ? "Promoting…" : "Promote preview"}
          </button>
        )}
        <button
          type="button"
          className="primary"
          disabled={busy || blocked}
          onClick={() =>
            run(publish, () =>
              publish.mutate(undefined, {
                onSuccess: (release) => {
                  onPublished(release.page_count);
                  dialog.current?.close();
                },
              }),
            )
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

/** What the server reports of the build being sent, from the moment `active` turns on; `null` until the first files go. */
function useUploadProgress(active: boolean): Progress | null {
  const [progress, setProgress] = useState<Progress | null>(null);

  useEffect(() => {
    if (!active) return;

    setProgress(null);
    const source = new EventSource(serverUrl("/api/publish/progress"));
    source.onmessage = (event) => setProgress(JSON.parse(event.data) as Progress);

    return () => source.close();
  }, [active]);

  return progress;
}

function UploadProgress({ progress }: { progress: Progress | null }) {
  // Before the first report the pages are still rendering: the bar has no value, which draws it as busy.
  const label = !progress
    ? "Building the site…"
    : progress.total === 0
      ? "Nothing new to upload"
      : `Uploaded ${progress.sent} of ${progress.total} ${progress.total === 1 ? "file" : "files"}`;

  return (
    <div className="publish-progress">
      <progress
        aria-label="Upload"
        value={progress ? (progress.total === 0 ? 1 : progress.sent) : undefined}
        max={progress ? Math.max(progress.total, 1) : undefined}
      />
      <output>{label}</output>
    </div>
  );
}
