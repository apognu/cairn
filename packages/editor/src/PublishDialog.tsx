import { useEffect, useId, useRef, useState } from "react";
import { serverUrl } from "./api/base";
import type { components } from "./api/schema";
import { DependencyNotes, ModeSwitch, PickBar, useHotfix } from "./Hotfix";
import { Icon } from "./icons";
import { ago, clock } from "./LockBanner";
import { badge, ChangeRow, ChecksStrip, code, LockWarning, urlNote } from "./PublishParts";
import { pendingCount, useChecks, usePending, usePreview, usePromote, usePublish, useSiteInfo } from "./queries";
import { useModal } from "./sidePanel";
import * as ui from "./ui";
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
  const hotfix = useHotfix(changes);
  // A hotfix is judged only by the pages it takes; the rest stays as published.
  const counts = (pageId: string) => !hotfix.on || hotfix.going.has(pageId);
  const stopping = problems.filter((p) => counts(p.page_id));
  // Someone else's open editor on a page that goes out: what they saved goes out, what they did not stays with them. The warning
  // comes first.
  const others = (useLocks().data ?? []).filter((l) => !l.mine && l.page_title !== "");
  const lockedBy = new Map(others.map((l) => [l.page_id, l.user_name]));
  const live = site.data?.live;
  const going = hotfix.on ? hotfix.going.size : count;
  const blocked =
    pending.isPending || checks.isPending || stopping.length > 0 || (hotfix.on && hotfix.picked.size === 0);

  return (
    <dialog
      ref={dialog}
      className={`${ui.dialogBase} w-[860px] max-w-[calc(100vw-32px)] max-h-[calc(100vh-64px)]`}
      aria-labelledby={`${id}-title`}
      onClose={onClose}
    >
      <h2 id={`${id}-title`} className="mt-0 mb-1 text-[20px] tracking-[-0.01em]">
        Publish changes
      </h2>
      <p className="text-muted">
        {site.data && (
          <>
            To <strong>{site.data.target}</strong> · {site.data.url.replace(/^https?:\/\//, "")}
            {live ? ` · last published ${ago(live.release.created_at)} by ${live.published_by}` : " · never published"}.{" "}
          </>
        )}
        {count === 0 ? "Nothing has changed since the live site." : "Compared with the live site."}
      </p>

      <LockWarning locks={others.filter((l) => counts(l.page_id) && changes.some((c) => c.page_id === l.page_id))} />

      {count > 0 && <ModeSwitch hotfix={hotfix} count={count} base={live ? clock(live.release.created_at) : null} />}
      {hotfix.on && <PickBar hotfix={hotfix} />}

      {stopping.length > 0 && (
        <div
          className="flex gap-3 my-3.5 py-3 px-3.5 border border-badge-del rounded-lg bg-badge-del-bg text-badge-del"
          role="alert"
        >
          <Icon name="info" size={18} className="flex-none mt-0.5" />
          <div>
            <strong>
              {stopping.length === 1 ? "1 problem stops" : `${stopping.length} problems stop`} this publish
            </strong>
            <ul className={ui.warningList} aria-label="Problems">
              {stopping.map((p) => (
                <li key={`${p.page_id}-${p.kind}-${p.message}`}>
                  <strong>{p.page_title}</strong> <span>{p.message}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}

      {!hotfix.on && lost.length > 0 && (
        <div className={ui.warning} role="alert">
          <Icon name="info" size={18} className={ui.warningIcon} />
          <div>
            <strong>{lost.length === 1 ? "1 URL" : `${lost.length} URLs`} will be a 404 after publish</strong>
            <ul className={ui.warningList}>
              {lost.map((l) => (
                <li key={l.path}>
                  <code className={code}>{l.path}</code> <span className="text-muted">{l.title}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}

      {!hotfix.on && broken.length > 0 && (
        <div className={ui.warning} role="alert">
          <Icon name="info" size={18} className={ui.warningIcon} />
          <div>
            <strong>{broken.length === 1 ? "1 link points" : `${broken.length} links point`} to a deleted page</strong>
            <ul className={ui.warningList}>
              {broken.map((b) => (
                <li key={`${b.source_id}-${b.target_title}`}>
                  {b.source_title} <span className="text-muted">links to {b.target_title}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}

      {count > 0 && (
        <table className={ui.table}>
          <thead>
            <tr>
              {hotfix.on && (
                <th scope="col" className={`${ui.th} w-7`}>
                  <span className="sr-only">Include</span>
                </th>
              )}
              <th scope="col" className={ui.th}>
                Change
              </th>
              <th scope="col" className={ui.th}>
                Page
              </th>
              <th scope="col" className={ui.th}>
                Last change
              </th>
              <th scope="col" className={ui.th}>
                Size
              </th>
              <th scope="col" className={ui.th}>
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {settingsChanged && !hotfix.on && (
              <tr>
                <td className={ui.td}>
                  <span className={ui.badge("edited")} data-kind="edited">
                    {badge.edited}
                  </span>
                </td>
                <td colSpan={4} className={ui.td}>
                  <strong>Site and theme</strong>
                  <div className={`${urlNote} text-muted`}>every page is rendered again</div>
                </td>
              </tr>
            )}
            {changes.map((c) => (
              <ChangeRow
                key={c.page_id}
                change={c}
                lockedBy={lockedBy.get(c.page_id)}
                select={
                  hotfix.on && (
                    <input
                      type="checkbox"
                      className="mt-0.5"
                      aria-label={`Include ${c.title}`}
                      checked={hotfix.going.has(c.page_id)}
                      disabled={!hotfix.picked.has(c.page_id) && hotfix.going.has(c.page_id)}
                      onChange={() => hotfix.toggle(c.page_id)}
                    />
                  )
                }
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

      {hotfix.on && <DependencyNotes hotfix={hotfix} />}

      <ChecksStrip
        stayPending={
          hotfix.on ? count - [...hotfix.going].filter((id) => changes.some((c) => c.page_id === id)).length : 0
        }
        problems={stopping}
        files={live?.release.file_count}
        bytes={live?.release.bytes}
        maxFiles={site.data?.capabilities.max_files}
      />

      {(publish.isError || preview.isError || promote.isError) && (
        <p role="alert" className="text-danger">
          {(publish.error ?? preview.error ?? promote.error)?.message}
        </p>
      )}
      {building && <UploadProgress progress={progress} />}
      {previewed && !building && (
        <output className="flex items-center gap-2 my-4 text-muted text-help">
          <Icon name="check" size={14} className="flex-none text-accent" />
          <span>
            Preview ready.{" "}
            <a href={previewed.preview_url ?? `/_preview/${previewed.id}/`} target="_blank" rel="noopener">
              Open it
            </a>
            {canPromote ? "" : " · Publishing builds the site again."}
          </span>
        </output>
      )}
      <div className={`${ui.actions} flex-wrap items-center`}>
        <button type="button" className={ui.dialogButton} onClick={() => dialog.current?.close()}>
          {previewed ? "Close" : "Cancel"}
        </button>
        {site.data?.capabilities.preview && !hotfix.on && (
          <button
            type="button"
            className={ui.dialogButton}
            disabled={busy || blocked}
            onClick={() => run(preview, () => preview.mutate())}
          >
            {preview.isPending ? "Building preview…" : previewed ? "Deploy new preview" : "Deploy preview"}
          </button>
        )}
        {previewed && canPromote && (
          <button
            type="button"
            className={ui.dialogButton}
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
          className={ui.primary}
          disabled={busy || blocked}
          onClick={() =>
            run(publish, () =>
              publish.mutate(hotfix.on ? [...hotfix.picked] : undefined, {
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
            : going === 0 && !hotfix.on
              ? "Publish site"
              : `Publish ${going} ${going === 1 ? "change" : "changes"}`}
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
      <div className={`${urlNote} text-warn-icon font-semibold`}>
        <code className={code}>{old_url}</code> · 404 after publish
      </div>
    );
  }
  if (change.redirected) {
    return (
      <div className={`${urlNote} text-muted`}>
        adds redirect from <code className={code}>{old_url}</code>
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
    <div className="flex flex-col gap-1.5 my-4 text-muted text-help">
      <progress
        className="progress-bar"
        aria-label="Upload"
        value={progress ? (progress.total === 0 ? 1 : progress.sent) : undefined}
        max={progress ? Math.max(progress.total, 1) : undefined}
      />
      <output>{label}</output>
    </div>
  );
}
