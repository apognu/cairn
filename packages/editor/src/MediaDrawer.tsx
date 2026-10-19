import { useEffect, useId, useRef, useState } from "react";
import { ImageEditor } from "./ImageEditor";
import { type Asset, assetLabel, thumbnail, useAsset, useDeleteAsset, useRenameAsset } from "./media";
import { useSidePanel } from "./sidePanel";
import * as ui from "./ui";

const when = (iso: string) =>
  new Date(iso).toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" });

function size(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** One image of the library: its title, what it is, where it is used, and Delete. */
export function MediaDrawer({
  id,
  onClose,
  onOpen,
  onOpenPage,
}: {
  id: string;
  onClose: () => void;
  /** Shows another image, the one this was made from. */
  onOpen: (id: string) => void;
  onOpenPage: (id: string) => void;
}) {
  const titleId = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const detail = useAsset(id);
  const remove = useDeleteAsset();
  const [confirming, setConfirming] = useState(false);
  const [editing, setEditing] = useState(false);

  useSidePanel(dialog);

  const asset = detail.data?.asset;
  const used = detail.data
    ? detail.data.pages.length +
      detail.data.entries.length +
      (detail.data.in_settings ? 1 : 0) +
      detail.data.copies.length
    : 0;

  return (
    <dialog ref={dialog} className={ui.drawer} aria-labelledby={`${titleId}-title`} onClose={onClose}>
      <header className={ui.drawerHead}>
        <h2 id={`${titleId}-title`} className={`${ui.drawerTitle} truncate`}>
          {asset ? assetLabel(asset) : "Image"}
        </h2>
        <button
          type="button"
          className={ui.icon}
          aria-label="Close image details"
          onClick={() => dialog.current?.close()}
        >
          ×
        </button>
      </header>

      {detail.isError && (
        <p role="alert" className="text-danger">
          {detail.error.message}
        </p>
      )}

      {asset && detail.data && (
        <>
          <a
            className="flex items-center justify-center mb-4.5 aspect-[4/3] overflow-hidden border border-border rounded-lg bg-hover"
            href={`/_a/${asset.id}`}
            target="_blank"
            rel="noreferrer"
            aria-label="Open the original"
          >
            <img className="w-full h-full object-contain" src={thumbnail(asset)} alt="" />
          </a>
          {asset.variants?.length !== 0 && (
            <div className={`${ui.inline} -mt-2 mb-4.5`}>
              <button type="button" className={ui.secondary} onClick={() => setEditing(true)}>
                Crop & annotate…
              </button>
            </div>
          )}
          {editing && (
            <ImageEditor asset={asset} onSaved={(copy) => onOpen(copy.id)} onClose={() => setEditing(false)} />
          )}

          <TitleField asset={asset} />

          <section className={ui.drawerSection}>
            <h3 className={ui.drawerHeading}>Details</h3>
            <dl className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 m-0 text-help">
              {asset.name && (
                <>
                  <dt className="text-muted">File</dt>
                  <dd className="m-0 wrap-anywhere">{asset.name}</dd>
                </>
              )}
              {asset.width && asset.height && (
                <>
                  <dt className="text-muted">Size</dt>
                  <dd className="m-0">
                    {asset.width} × {asset.height} px, {size(asset.bytes)}
                  </dd>
                </>
              )}
              <dt className="text-muted">Uploaded</dt>
              <dd className="m-0">
                {when(asset.created_at)}
                {detail.data.uploaded_by_name && ` by ${detail.data.uploaded_by_name}`}
              </dd>
              {detail.data.source && (
                <>
                  <dt className="text-muted">Made from</dt>
                  <dd className="m-0 min-w-0">
                    <button
                      type="button"
                      className={`${ui.linkButton} p-0 text-left wrap-anywhere`}
                      onClick={() => detail.data.source && onOpen(detail.data.source.id)}
                    >
                      {assetLabel(detail.data.source)}
                    </button>
                  </dd>
                </>
              )}
            </dl>
          </section>

          <section className={ui.drawerSection}>
            <h3 className={ui.drawerHeading}>Used on</h3>
            {used === 0 ? (
              <p className={`${ui.drawerNote} m-0`}>
                {asset.kept ? "No page uses it yet." : "No page uses it: it will be removed at the next cleanup."}
              </p>
            ) : (
              <ul className="m-0 p-0 list-none text-help" aria-label="Used on">
                {detail.data.pages.map((p) => (
                  <li key={p.id} className="mb-1">
                    <button type="button" className={`${ui.linkButton} p-0 text-left`} onClick={() => onOpenPage(p.id)}>
                      {p.title}
                    </button>
                  </li>
                ))}
                {detail.data.entries.map((key) => (
                  <li key={key} className="mb-1">
                    The snippet <code>{key}</code>
                  </li>
                ))}
                {detail.data.in_settings && <li className="mb-1">The site settings</li>}
                {detail.data.copies.length > 0 && (
                  <li className="mb-1">
                    {detail.data.copies.length === 1
                      ? "A copy made from it"
                      : `${detail.data.copies.length} copies made from it`}
                  </li>
                )}
              </ul>
            )}
          </section>

          {asset.kept && used > 0 && (
            <p className={`${ui.drawerNote} m-0`}>To delete it, take it out of the places above first.</p>
          )}
          {asset.kept &&
            used === 0 &&
            (confirming ? (
              <fieldset className="flex flex-wrap items-center gap-2 m-0 p-0 border-0" aria-label="Confirm delete">
                <span className="mr-auto">Delete this image?</span>
                <button type="button" className={ui.secondary} onClick={() => setConfirming(false)}>
                  Cancel
                </button>
                <button
                  type="button"
                  className={ui.primary}
                  disabled={remove.isPending}
                  onClick={() => remove.mutate(asset.id, { onSuccess: () => dialog.current?.close() })}
                >
                  Delete
                </button>
              </fieldset>
            ) : (
              <button type="button" className={ui.secondary} onClick={() => setConfirming(true)}>
                Delete
              </button>
            ))}
          {remove.isError && (
            <p role="alert" className="text-danger text-help">
              {remove.error.message}
            </p>
          )}
        </>
      )}
    </dialog>
  );
}

/** The title, saved when the field is left or Enter is pressed. */
function TitleField({ asset }: { asset: Asset }) {
  const id = useId();
  const rename = useRenameAsset();
  const [title, setTitle] = useState(asset.title ?? "");

  useEffect(() => setTitle(asset.title ?? ""), [asset.title]);

  const save = () => {
    if (title.trim() !== (asset.title ?? "")) rename.mutate({ id: asset.id, title });
  };

  return (
    <div className={ui.drawerSection}>
      <label htmlFor={id} className={ui.drawerHeading}>
        Title
      </label>
      <input
        id={id}
        className={ui.input}
        value={title}
        maxLength={200}
        placeholder={asset.name ?? "What the image shows"}
        onChange={(e) => setTitle(e.target.value)}
        onBlur={save}
        onKeyDown={(e) => {
          if (e.key === "Enter") save();
        }}
      />
      <p className={`${ui.drawerNote} m-0`}>
        {rename.isError ? (
          <span className="text-danger">{rename.error.message}</span>
        ) : rename.isSuccess && !rename.isPending ? (
          "Saved."
        ) : (
          "Used to find the image in the library; readers never see it."
        )}
      </p>
    </div>
  );
}
