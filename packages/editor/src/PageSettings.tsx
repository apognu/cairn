import { useEffect, useId, useRef, useState } from "react";
import {
  type PageReview,
  useAddAlias,
  useBacklinks,
  usePageReview,
  usePageUrl,
  useRemoveAlias,
  useSetCustomUrl,
  useSetDescription,
  useSetHeader,
  useSetPageReview,
  useUsers,
} from "./queries";
import { useSidePanel } from "./sidePanel";
import type { Page } from "./tree";
import * as ui from "./ui";

/** The lists of aliases and of pages linking here. */
const list = "mt-0 mb-2 p-0 list-none border border-input rounded-lg";
const item = "flex items-center gap-2 py-2 px-2.5 border-b border-border last:border-b-0";
const empty = "mt-0 mb-2 text-help text-muted";

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
      className={ui.drawer}
      aria-labelledby={`${id}-title`}
      onClose={() => {
        // Escape closes the drawer without moving the focus out of the description first.
        saveDescription();
        onClose();
      }}
    >
      <header className={ui.drawerHead}>
        <h2 id={`${id}-title`} className={ui.drawerTitle}>
          Page settings
        </h2>
        <button
          type="button"
          className={ui.icon}
          aria-label="Close page settings"
          onClick={() => dialog.current?.close()}
        >
          ×
        </button>
      </header>

      {!data ? (
        <p className={ui.drawerNote}>Loading…</p>
      ) : (
        <>
          <fieldset className={`${ui.drawerSection} border-0 p-0`}>
            <legend className={ui.drawerHeading}>URL</legend>
            <label className={ui.checkInField}>
              <input
                type="radio"
                name={`${id}-mode`}
                checked={!useCustom}
                onChange={() => {
                  setUseCustom(false);
                  if (data.custom_url) saveUrl(null);
                }}
              />
              <span className="flex-none">From its place in the tree</span>
              {/* A long path gives way to the label beside it. */}
              <code
                className="min-w-0 ml-auto overflow-hidden text-ellipsis whitespace-nowrap font-mono text-label text-muted"
                title={data.derived_url}
              >
                {data.derived_url}
              </code>
            </label>
            <label className={ui.checkInField}>
              <input type="radio" name={`${id}-mode`} checked={useCustom} onChange={() => setUseCustom(true)} />
              Custom URL
            </label>
            {useCustom && (
              <form
                className={ui.inline}
                onSubmit={(e) => {
                  e.preventDefault();
                  saveUrl(custom);
                }}
              >
                <input
                  className={ui.rowInput}
                  aria-label="Custom URL"
                  placeholder="/webhooks"
                  value={custom}
                  onChange={(e) => setCustomPath(e.target.value)}
                />
                <button type="submit" className={ui.rowSubmit} disabled={setCustom.isPending}>
                  Save
                </button>
              </form>
            )}
            {useCustom && setCustom.error && (
              <small role="alert" className="text-danger">
                {setCustom.error.message}
              </small>
            )}
            {useCustom && <small className="text-muted">Stays the same when the page moves in the tree.</small>}
          </fieldset>

          <section className={ui.drawerSection}>
            <h3 className={ui.drawerHeading}>
              Aliases <small className="font-normal text-muted">old URLs that redirect here</small>
            </h3>
            {data.aliases.length === 0 && <p className={empty}>None yet.</p>}
            <ul className={list} aria-label="Aliases" hidden={data.aliases.length === 0}>
              {data.aliases.map((a) => (
                <li key={a.path} className={item}>
                  <code className={ui.mono}>{a.path}</code>
                  <span className={ui.badge(undefined, "mr-auto")}>{originLabel[a.origin]}</span>
                  <button
                    type="button"
                    className={ui.linkButton}
                    onClick={() => setCustom.mutate({ custom_url: a.path })}
                  >
                    Make canonical
                  </button>
                  <button
                    type="button"
                    className={ui.icon}
                    aria-label={`Remove alias ${a.path}`}
                    onClick={() => removeAlias.mutate(a.path)}
                  >
                    ×
                  </button>
                </li>
              ))}
            </ul>
            <form
              className={ui.inline}
              onSubmit={(e) => {
                e.preventDefault();
                addAlias.mutate(alias, { onSuccess: () => setAlias("") });
              }}
            >
              <input
                className={ui.rowInput}
                aria-label="New alias"
                placeholder="/old-path"
                value={alias}
                onChange={(e) => setAlias(e.target.value)}
              />
              <button type="submit" className={ui.rowSubmit} disabled={addAlias.isPending}>
                Add
              </button>
            </form>
            {addAlias.error && (
              <small role="alert" className="text-danger">
                {addAlias.error.message}
              </small>
            )}
          </section>

          <fieldset className={`${ui.drawerSection} border-0 p-0`}>
            <legend className={ui.drawerHeading}>Navigation</legend>
            <label className={ui.checkInField}>
              <input
                type="checkbox"
                checked={page.in_header}
                disabled={setHeader.isPending}
                onChange={(e) => setHeader.mutate({ id: page.id, in_header: e.target.checked })}
              />
              Show in header
            </label>
            <small className="text-muted">A page with subpages becomes a section with a sidebar of its own.</small>
          </fieldset>

          <section className={ui.drawerSection}>
            <h3 className={ui.drawerHeading}>Search and sharing</h3>
            <label htmlFor={`${id}-description`} className={ui.fieldLabel}>
              Description
            </label>
            <textarea
              id={`${id}-description`}
              className={ui.textarea}
              rows={3}
              maxLength={300}
              value={description}
              placeholder="A sentence for link cards and search results"
              onChange={(e) => setDescriptionText(e.target.value)}
              onBlur={saveDescription}
            />
            <ReviewFields pageId={page.id} />
          </section>

          <section className={ui.drawerSection}>
            <h3 className={ui.drawerHeading}>Linked from</h3>
            {backlinks.data?.linked_from.length === 0 && backlinks.data.terms.length === 0 && (
              <p className={empty}>No page links here yet.</p>
            )}
            <ul
              className={list}
              aria-label="Linked from"
              hidden={!backlinks.data?.linked_from.length && !backlinks.data?.terms.length}
            >
              {backlinks.data?.linked_from.map((l) => (
                <li key={`${l.source_id}-${l.kind}`} className={item}>
                  <span>{l.source_title}</span>
                  <span className={ui.badge(undefined, "mr-auto")}>{l.kind}</span>
                </li>
              ))}
              {backlinks.data?.terms.map((t) => (
                <li key={`term-${t.term}`} className={item}>
                  <span>{t.term}</span>
                  <span className={ui.badge(undefined, "mr-auto")}>glossary term</span>
                </li>
              ))}
            </ul>
          </section>

          {error && (
            <p role="alert" className="text-danger">
              {error.message}
            </p>
          )}
          <p className={ui.drawerNote}>URL changes are pending until you publish.</p>
        </>
      )}
    </dialog>
  );
}

/** Who looks after the page and when to check it again: saved as soon as they change, and never published. */
function ReviewFields({ pageId }: { pageId: string }) {
  const id = useId();
  const review = usePageReview(pageId);
  const users = useUsers();
  const save = useSetPageReview(pageId);
  const current = review.data ?? { owner_id: null, review_by: null };
  // Both fields are sent on every save: start from what was last sent, so a save made before the previous one answered keeps it.
  const sent = useRef<PageReview | null>(null);
  const update = (change: Partial<PageReview>) => {
    sent.current = { ...(sent.current ?? current), ...change };
    save.mutate(sent.current);
  };
  // Typing a year changes the value at every digit: the date is saved when the field is left, not while it is typed.
  const [date, setDate] = useState<string | null>(null);
  const saveDate = () => {
    if (date === null || date === (current.review_by ?? "")) return;
    update({ review_by: date || null });
  };

  return (
    <>
      <div className="grid grid-cols-2 gap-3 mt-3">
        <div className={ui.fieldBase}>
          <label htmlFor={`${id}-owner`} className={ui.fieldLabel}>
            Owner
          </label>
          <select
            id={`${id}-owner`}
            className={ui.select}
            value={current.owner_id ?? ""}
            disabled={!review.data}
            onChange={(e) => update({ owner_id: e.target.value || null })}
          >
            <option value="">Nobody</option>
            {users.data?.map((u) => (
              <option key={u.id} value={u.id}>
                {u.name}
              </option>
            ))}
          </select>
        </div>
        <div className={ui.fieldBase}>
          <label htmlFor={`${id}-review`} className={ui.fieldLabel}>
            Review by
          </label>
          <input
            id={`${id}-review`}
            type="date"
            className={ui.input}
            value={date ?? current.review_by ?? ""}
            disabled={!review.data}
            onChange={(e) => setDate(e.target.value)}
            onBlur={saveDate}
            onKeyDown={(e) => e.key === "Enter" && saveDate()}
          />
        </div>
      </div>
      {save.isError && (
        <p role="alert" className="text-danger">
          {save.error.message}
        </p>
      )}
    </>
  );
}
