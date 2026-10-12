import { useEffect, useId, useRef } from "react";
import { useDeleteImpact } from "./queries";
import type { Page } from "./tree";

/** Confirms a delete, naming the pages that go with it and every page whose links would break. */
export function DeleteDialog({ page, onConfirm, onClose }: { page: Page; onConfirm: () => void; onClose: () => void }) {
  const id = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const impact = useDeleteImpact(page.id);

  useEffect(() => dialog.current?.showModal(), []);

  const linked = impact.data?.linked_from ?? [];

  return (
    <dialog ref={dialog} className="settings" aria-labelledby={`${id}-title`} onClose={onClose}>
      <h2 id={`${id}-title`}>Delete “{page.title}”?</h2>
      {impact.data && impact.data.subpages > 0 && (
        <p>Its {impact.data.subpages === 1 ? "subpage goes" : `${impact.data.subpages} subpages go`} with it.</p>
      )}
      {impact.isPending && <p className="muted">Checking links…</p>}
      {linked.length > 0 && (
        <div className="publish-warning" role="alert">
          <div>
            <strong>{linked.length === 1 ? "1 link" : `${linked.length} links`} will stop working</strong>
            <ul aria-label="Linked from">
              {linked.map((l) => (
                <li key={`${l.source_id}-${l.target_title}-${l.kind}`}>
                  {l.source_title} <span className="muted">links to {l.target_title}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}
      <p className="muted">Deleted pages can be restored from the trash for 90 days.</p>
      <div className="dialog-actions">
        <button type="button" onClick={() => dialog.current?.close()}>
          Cancel
        </button>
        <button type="button" className="primary" disabled={impact.isPending} onClick={onConfirm}>
          Delete
        </button>
      </div>
    </dialog>
  );
}
