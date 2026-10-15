import { useRestoreFromTrash, useTrash } from "./queries";
import * as ui from "./ui";

/** Deleted pages, which wait here and can come back at their old place. */
export function TrashSettings() {
  const trash = useTrash();
  const restore = useRestoreFromTrash();
  const restored = restore.data;

  return (
    <div className="max-w-205">
      <h1 className={ui.settingsTitle}>Trash</h1>
      <p className="text-muted">
        Deleted pages wait here for 90 days. A page comes back at its old place with its old URL, if that is still free.
      </p>

      <table className={ui.table} aria-label="Deleted pages">
        <thead>
          <tr>
            <th scope="col" className={ui.th}>
              Page
            </th>
            <th scope="col" className={ui.th}>
              Was at
            </th>
            <th scope="col" className={ui.th}>
              Deleted
            </th>
            <th scope="col" className={ui.th}>
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {trash.data?.length === 0 && (
            <tr>
              <td colSpan={4} className={`${ui.td} text-muted text-help`}>
                The trash is empty.
              </td>
            </tr>
          )}
          {trash.data?.map((t) => (
            <tr key={t.id}>
              <td className={ui.td}>
                <strong>{t.title}</strong>
                {t.subpages > 0 && (
                  <span className="text-muted"> and {t.subpages === 1 ? "1 subpage" : `${t.subpages} subpages`}</span>
                )}
              </td>
              <td className={ui.td}>
                <code>{t.old_url}</code>
              </td>
              <td className={ui.td}>
                {new Date(t.deleted_at).toLocaleDateString("en-GB", {
                  day: "numeric",
                  month: "short",
                  year: "numeric",
                })}
              </td>
              <td className={ui.td}>
                <button
                  type="button"
                  disabled={restore.isPending}
                  onClick={() => restore.mutate(t.id)}
                  aria-label={`Restore ${t.title}`}
                >
                  Restore
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {restored && (
        <output className="text-muted">
          “{restored.page.title}” is back
          {restored.old_place_kept
            ? " at its old place."
            : ", but not at its old place or URL: that was taken, so check where it landed."}
        </output>
      )}
      {restore.isError && (
        <p role="alert" className="text-danger">
          {restore.error.message}
        </p>
      )}
    </div>
  );
}
