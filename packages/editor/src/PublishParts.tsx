import type { DocNode } from "@cairn/renderer";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "./api/client";
import type { components } from "./api/schema";
import { unwrap } from "./api/unwrap";
import { blockLines, diffLines } from "./diff";
import { Icon } from "./icons";
import { ago } from "./LockBanner";
import * as ui from "./ui";
import type { LockView } from "./useLock";

type Change = components["schemas"]["Change"];
type Problem = components["schemas"]["Problem"];

export const badge: Record<Change["kinds"][number], string> = {
  new: "New",
  edited: "Edited",
  renamed: "Renamed",
  moved: "Moved",
  header: "Header",
  deleted: "Deleted",
};

/** A saved version never changes, so it is read once. */
/** What a change does to a URL, under its page's title. */
export const urlNote = "mt-0.5 text-help";
export const code = "font-mono text-[12.5px]";

function useRevision(id: string | null | undefined) {
  return useQuery({
    queryKey: ["revision", id],
    enabled: Boolean(id),
    staleTime: Number.POSITIVE_INFINITY,
    queryFn: async () => unwrap(await api.GET("/api/revisions/{id}", { params: { path: { id: id ?? "" } } })),
  });
}

/** Who else is in the middle of editing: their last saved version is what goes out. */
export function LockWarning({ locks }: { locks: LockView[] }) {
  if (locks.length === 0) return null;

  return (
    <div className={ui.warning} role="alert">
      <Icon name="lock" size={18} className={ui.warningIcon} />
      <div>
        <strong>
          {locks.length === 1 ? "1 page is being edited right now" : `${locks.length} pages are being edited right now`}
        </strong>
        <ul className="my-1.5 p-0 list-none" aria-label="Pages being edited">
          {locks.map((l) => (
            <li key={l.page_id}>
              <span className="font-semibold">{l.page_title}</span>{" "}
              <span className="text-muted">
                {l.user_name} · {l.last_saved_at ? `last saved ${ago(l.last_saved_at)}` : "nothing saved yet"}
              </span>
            </li>
          ))}
        </ul>
        <p className="text-muted">
          Publishing now uses their last saved versions. Anything they haven't saved stays in their editor.
        </p>
      </div>
    </div>
  );
}

/** One pending change: what it is, who made it, how big it is, and its diff on request. */
export function ChangeRow({
  change,
  note,
  lockedBy,
  select,
}: {
  change: Change;
  note: React.ReactNode;
  lockedBy: string | undefined;
  /** In a hotfix, the checkbox that picks the page. */
  select?: React.ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const current = useRevision(change.revision_id);
  const live = useRevision(change.live_revision_id);
  const content = change.kinds.includes("edited") || change.kinds.includes("new");
  const now = current.data ? blockLines(current.data.doc as DocNode) : [];
  const before = live.data ? blockLines(live.data.doc as DocNode) : [];
  const lines = content && current.data && (live.data || !change.live_revision_id) ? diffLines(before, now) : null;
  const added = lines?.filter((l) => l.kind === "added").length ?? 0;
  const removed = lines?.filter((l) => l.kind === "removed").length ?? 0;
  const words = now.join(" ").split(/\s+/).filter(Boolean).length;

  return (
    <>
      <tr>
        {select && <td className={`${ui.tdBase} w-7`}>{select}</td>}
        <td className={ui.td}>
          {change.kinds.map((k) => (
            <span key={k} className={ui.badge(k)} data-kind={k}>
              {badge[k]}
            </span>
          ))}
        </td>
        <td className={ui.td}>
          <strong>{change.title}</strong>
          {change.old_title && <span className="text-muted"> · was “{change.old_title}”</span>}
          {lockedBy && (
            <span
              className="inline-flex items-center gap-1 ml-2 text-warn-icon text-label"
              title={`${lockedBy} is editing`}
            >
              <Icon name="lock" size={12} /> being edited
            </span>
          )}
          {note}
          {change.kinds.length === 1 &&
            change.kinds[0] === "edited" &&
            change.revision_id &&
            change.revision_id === change.live_revision_id && (
              <div className={`${urlNote} text-muted`}>a page it links to or a snippet it uses changed</div>
            )}
        </td>
        <td className={`${ui.td} text-muted text-help`}>
          {current.data ? `${current.data.author_name} · ${ago(current.data.created_at)}` : "—"}
        </td>
        <td className={`${ui.td} font-mono text-[12.5px] whitespace-nowrap`}>
          {!lines ? (
            "—"
          ) : change.live_revision_id ? (
            <>
              <span className="text-badge-new">+{added}</span> <span className="text-badge-del">−{removed}</span>
            </>
          ) : (
            `${words.toLocaleString("en")} w`
          )}
        </td>
        <td className={ui.td}>
          {lines && (
            <button type="button" className={ui.tinyButton} aria-expanded={open} onClick={() => setOpen(!open)}>
              {open ? "Hide diff" : "View diff"}
            </button>
          )}
        </td>
      </tr>
      {open && lines && (
        <tr>
          <td colSpan={select ? 6 : 5} className="pt-0 pb-2.5 pr-3 pl-0 border-b border-border align-top">
            <ul
              className="mt-0 mb-3 p-0 list-none max-h-70 overflow-auto border border-input rounded-lg font-mono text-[12.5px]"
              aria-label={`Changes to ${change.title}`}
            >
              {lines
                .filter((l) => l.kind !== "same")
                .map((l, i) => (
                  // biome-ignore lint/suspicious/noArrayIndexKey: diff lines can repeat
                  <li key={i} className={l.kind === "added" ? ui.diffAdded : ui.diffRemoved} data-kind={l.kind}>
                    {l.kind === "added" ? "+ " : "− "}
                    {l.text || "(empty line)"}
                  </li>
                ))}
            </ul>
          </td>
        </tr>
      )}
    </>
  );
}

const mb = (bytes: number) => `${(bytes / 1_000_000).toLocaleString("en", { maximumFractionDigits: 1 })} MB`;

/** The checks that passed, and the size of the site against the target's limit. */
export function ChecksStrip({
  problems,
  stayPending,
  files,
  bytes,
  maxFiles,
}: {
  problems: Problem[];
  /** In a hotfix, how many changes it leaves pending. */
  stayPending: number;
  files: number | null | undefined;
  bytes: number | null | undefined;
  maxFiles: number | null | undefined;
}) {
  const passed = [
    { kinds: ["broken_link", "broken_operation_link", "broken_section_link"], text: "No broken links" },
    { kinds: ["invalid_props"], text: "All component props valid" },
    { kinds: ["missing_alt"], text: "Every image has alt text" },
  ].filter((check) => !problems.some((p) => check.kinds.includes(p.kind)));

  return (
    <div className="flex flex-wrap items-center gap-x-4.5 gap-y-2 mt-0 mb-4 text-help">
      {passed.map((check) => (
        <span key={check.text} className="inline-flex items-center gap-1.5 text-badge-new">
          <Icon name="check" size={14} />
          {check.text}
        </span>
      ))}
      {stayPending > 0 && (
        <span className="text-muted">
          {stayPending === 1 ? "1 change stays pending" : `${stayPending} changes stay pending`}
        </span>
      )}
      {files != null && (
        <span className="ml-auto text-muted">
          Last build {files.toLocaleString("en")} files{bytes != null && ` · ${mb(bytes)}`}
          {maxFiles != null && ` · limit ${maxFiles.toLocaleString("en")} files`}
        </span>
      )}
    </div>
  );
}
