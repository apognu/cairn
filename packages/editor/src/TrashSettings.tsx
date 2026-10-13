import { useRestoreFromTrash, useTrash } from "./queries";

/** Deleted pages, which wait here and can come back at their old place. */
export function TrashSettings() {
  const trash = useTrash();
  const restore = useRestoreFromTrash();
  const restored = restore.data;

  return (
    <div className="settings-form wide">
      <h1>Trash</h1>
      <p className="muted">
        Deleted pages wait here for 90 days. A page comes back at its old place with its old URL, if that is still free.
      </p>

      <table className="changes" aria-label="Deleted pages">
        <thead>
          <tr>
            <th scope="col">Page</th>
            <th scope="col">Was at</th>
            <th scope="col">Deleted</th>
            <th scope="col">
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {trash.data?.length === 0 && (
            <tr>
              <td colSpan={4} className="muted">
                The trash is empty.
              </td>
            </tr>
          )}
          {trash.data?.map((t) => (
            <tr key={t.id}>
              <td>
                <strong>{t.title}</strong>
                {t.subpages > 0 && (
                  <span className="muted"> and {t.subpages === 1 ? "1 subpage" : `${t.subpages} subpages`}</span>
                )}
              </td>
              <td>
                <code>{t.old_url}</code>
              </td>
              <td>
                {new Date(t.deleted_at).toLocaleDateString("en-GB", {
                  day: "numeric",
                  month: "short",
                  year: "numeric",
                })}
              </td>
              <td>
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
        <output className="muted">
          “{restored.page.title}” is back
          {restored.old_place_kept
            ? " at its old place."
            : ", but not at its old place or URL: that was taken, so check where it landed."}
        </output>
      )}
      {restore.isError && (
        <p role="alert" className="error-text">
          {restore.error.message}
        </p>
      )}
    </div>
  );
}
