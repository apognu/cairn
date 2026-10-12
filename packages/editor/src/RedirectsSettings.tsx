import { useId, useState } from "react";
import type { components } from "./api/schema";
import { useAddRedirect, usePages, useRedirects, useRemoveRedirect } from "./queries";

type Entry = components["schemas"]["RedirectEntry"];

const originLabel = { move: "moved", rename: "previous URL", manual: "manual" } as const;
const inactiveLabel = {
  page_at_path: "A page now lives at this path",
  target_gone: "Its page was deleted",
  loop: "Part of a redirect loop",
} as const;

/** Every redirect of the site: where it starts, where it ends up, and why it was made. */
export function RedirectsSettings() {
  const id = useId();
  const redirects = useRedirects();
  const pages = usePages();
  const add = useAddRedirect();
  const remove = useRemoveRedirect();
  const [from, setFrom] = useState("");
  const [toPage, setToPage] = useState("");
  const [toUrl, setToUrl] = useState("");

  const error = add.error ?? remove.error;

  return (
    <div className="settings-form wide">
      <h1>Redirects</h1>
      <p className="muted">
        Old URLs that lead somewhere else. Moves and renames add them automatically; chains are collapsed when the site
        is built, so a visitor is redirected once.
      </p>

      <table className="changes redirects" aria-label="Redirects">
        <thead>
          <tr>
            <th scope="col">From</th>
            <th scope="col">To</th>
            <th scope="col">Source</th>
            <th scope="col">
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {redirects.data?.length === 0 && (
            <tr>
              <td colSpan={4} className="muted">
                No redirects yet.
              </td>
            </tr>
          )}
          {redirects.data?.map((r) => (
            <tr key={r.from_path} data-inactive={r.inactive ?? undefined}>
              <td>
                <code>{r.from_path}</code>
              </td>
              <td>
                <Destination entry={r} />
              </td>
              <td>
                <span className="badge">{originLabel[r.origin]}</span>
              </td>
              <td>
                <button
                  type="button"
                  className="icon"
                  aria-label={`Delete redirect from ${r.from_path}`}
                  onClick={() => remove.mutate(r.from_path)}
                >
                  ×
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <h2>Add a redirect</h2>
      <form
        className="redirect-form"
        onSubmit={(e) => {
          e.preventDefault();
          add.mutate(
            { from_path: from, to_page_id: toPage || undefined, to_url: toPage ? undefined : toUrl },
            {
              onSuccess: () => {
                setFrom("");
                setToUrl("");
                setToPage("");
              },
            },
          );
        }}
      >
        <div className="form-field">
          <label htmlFor={`${id}-from`}>From</label>
          <input id={`${id}-from`} placeholder="/old-path" value={from} onChange={(e) => setFrom(e.target.value)} />
        </div>
        <div className="form-field">
          <label htmlFor={`${id}-page`}>To a page</label>
          <select id={`${id}-page`} value={toPage} onChange={(e) => setToPage(e.target.value)}>
            <option value="">Choose a page…</option>
            {pages.data
              ?.filter((p) => p.kind === "page")
              .map((p) => (
                <option key={p.id} value={p.id}>
                  {p.title}
                </option>
              ))}
          </select>
        </div>
        <div className="form-field">
          <label htmlFor={`${id}-url`}>Or to a URL</label>
          <input
            id={`${id}-url`}
            placeholder="https://… or /path"
            disabled={toPage !== ""}
            value={toUrl}
            onChange={(e) => setToUrl(e.target.value)}
          />
        </div>
        <button type="submit" className="primary" disabled={add.isPending || !from || (!toPage && !toUrl)}>
          Add redirect
        </button>
      </form>
      {error && (
        <p role="alert" className="error-text">
          {error.message}
        </p>
      )}
    </div>
  );
}

function Destination({ entry }: { entry: Entry }) {
  if (entry.inactive) return <span className="muted">{inactiveLabel[entry.inactive]}</span>;

  return (
    <>
      {entry.to_page_title && <strong>{entry.to_page_title} </strong>}
      <code>{entry.to}</code>
    </>
  );
}
