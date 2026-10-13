import { useEffect, useId, useRef, useState } from "react";
import {
  useAddAlias,
  useBacklinks,
  usePageUrl,
  useRemoveAlias,
  useSetCustomUrl,
  useSetDescription,
  useSetHeader,
} from "./queries";
import { useSidePanel } from "./sidePanel";
import type { Page } from "./tree";

const originLabel = { move: "moved", rename: "previous URL", manual: "manual" } as const;

/** The URL decisions of one page: derived or custom path, and the old URLs that redirect to it. */
export function PageSettings({ page, onClose }: { page: Page; onClose: () => void }) {
  const id = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const url = usePageUrl(page.id);
  const setCustom = useSetCustomUrl(page.id);
  const addAlias = useAddAlias(page.id);
  const removeAlias = useRemoveAlias(page.id);
  const setHeader = useSetHeader();
  const backlinks = useBacklinks(page.id);
  const setDescription = useSetDescription();
  const [description, setDescriptionText] = useState(page.description ?? "");
  const [custom, setCustomPath] = useState("");
  const [useCustom, setUseCustom] = useState(false);
  const [alias, setAlias] = useState("");

  useSidePanel(dialog);
  useEffect(() => {
    if (!url.data) return;
    setUseCustom(url.data.custom_url !== null && url.data.custom_url !== undefined);
    setCustomPath(url.data.custom_url ?? "");
  }, [url.data]);

  const data = url.data;
  // Errors show next to what caused them; these have no field of their own.
  const error = removeAlias.error ?? setHeader.error ?? setDescription.error;
  const saveUrl = (path: string | null) => setCustom.mutate({ custom_url: path });
  const savedDescription = useRef(page.description ?? "");
  const saveDescription = () => {
    if (description === savedDescription.current) return;
    savedDescription.current = description;
    setDescription.mutate({ id: page.id, description });
  };

  return (
    <dialog
      ref={dialog}
      className="drawer"
      aria-labelledby={`${id}-title`}
      onClose={() => {
        // Escape closes the drawer without moving the focus out of the description first.
        saveDescription();
        onClose();
      }}
    >
      <header className="drawer-head">
        <h2 id={`${id}-title`}>Page settings</h2>
        <button type="button" className="icon" aria-label="Close page settings" onClick={() => dialog.current?.close()}>
          ×
        </button>
      </header>

      {!data ? (
        <p className="muted">Loading…</p>
      ) : (
        <>
          <fieldset className="form-field">
            <legend>URL</legend>
            <label className="form-check">
              <input
                type="radio"
                name={`${id}-mode`}
                checked={!useCustom}
                onChange={() => {
                  setUseCustom(false);
                  if (data.custom_url) saveUrl(null);
                }}
              />
              <span>From its place in the tree</span>
              <code className="mono muted" title={data.derived_url}>
                {data.derived_url}
              </code>
            </label>
            <label className="form-check">
              <input type="radio" name={`${id}-mode`} checked={useCustom} onChange={() => setUseCustom(true)} />
              Custom URL
            </label>
            {useCustom && (
              <form
                className="row-inline"
                onSubmit={(e) => {
                  e.preventDefault();
                  saveUrl(custom);
                }}
              >
                <input
                  aria-label="Custom URL"
                  placeholder="/webhooks"
                  value={custom}
                  onChange={(e) => setCustomPath(e.target.value)}
                />
                <button type="submit" disabled={setCustom.isPending}>
                  Save
                </button>
              </form>
            )}
            {useCustom && setCustom.error && (
              <small role="alert" className="error-text">
                {setCustom.error.message}
              </small>
            )}
            {useCustom && <small className="muted">Stays the same when the page moves in the tree.</small>}
          </fieldset>

          <section className="form-field">
            <h3>
              Aliases <small className="muted">old URLs that redirect here</small>
            </h3>
            {data.aliases.length === 0 && <p className="muted empty">None yet.</p>}
            <ul className="aliases" aria-label="Aliases" hidden={data.aliases.length === 0}>
              {data.aliases.map((a) => (
                <li key={a.path}>
                  <code className="mono">{a.path}</code>
                  <span className="badge">{originLabel[a.origin]}</span>
                  <button
                    type="button"
                    className="link-button"
                    onClick={() => setCustom.mutate({ custom_url: a.path })}
                  >
                    Make canonical
                  </button>
                  <button
                    type="button"
                    className="icon"
                    aria-label={`Remove alias ${a.path}`}
                    onClick={() => removeAlias.mutate(a.path)}
                  >
                    ×
                  </button>
                </li>
              ))}
            </ul>
            <form
              className="row-inline"
              onSubmit={(e) => {
                e.preventDefault();
                addAlias.mutate(alias, { onSuccess: () => setAlias("") });
              }}
            >
              <input
                aria-label="New alias"
                placeholder="/old-path"
                value={alias}
                onChange={(e) => setAlias(e.target.value)}
              />
              <button type="submit" disabled={addAlias.isPending}>
                Add
              </button>
            </form>
            {addAlias.error && (
              <small role="alert" className="error-text">
                {addAlias.error.message}
              </small>
            )}
          </section>

          <fieldset className="form-field">
            <legend>Navigation</legend>
            <label className="form-check">
              <input
                type="checkbox"
                checked={page.in_header}
                disabled={setHeader.isPending}
                onChange={(e) => setHeader.mutate({ id: page.id, in_header: e.target.checked })}
              />
              Show in header
            </label>
            <small className="muted">A page with subpages becomes a section with a sidebar of its own.</small>
          </fieldset>

          <section className="form-field">
            <h3>Search and sharing</h3>
            <label htmlFor={`${id}-description`}>Description</label>
            <textarea
              id={`${id}-description`}
              rows={3}
              maxLength={300}
              value={description}
              placeholder="A sentence for link cards and search results"
              onChange={(e) => setDescriptionText(e.target.value)}
              onBlur={saveDescription}
            />
          </section>

          <section className="form-field">
            <h3>Linked from</h3>
            {backlinks.data?.linked_from.length === 0 && <p className="muted empty">No page links here yet.</p>}
            <ul className="aliases" aria-label="Linked from" hidden={!backlinks.data?.linked_from.length}>
              {backlinks.data?.linked_from.map((l) => (
                <li key={`${l.source_id}-${l.kind}`}>
                  <span>{l.source_title}</span>
                  <span className="badge">{l.kind}</span>
                </li>
              ))}
            </ul>
          </section>

          {error && (
            <p role="alert" className="error-text">
              {error.message}
            </p>
          )}
          <p className="muted">URL changes are pending until you publish.</p>
        </>
      )}
    </dialog>
  );
}
