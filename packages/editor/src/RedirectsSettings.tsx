import { useId, useState } from "react";
import type { components } from "./api/schema";
import { useAddRedirect, useCan, usePages, useRedirects, useRemoveRedirect } from "./queries";
import * as ui from "./ui";

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
  const canEdit = useCan("editor");
  const [from, setFrom] = useState("");
  const [toPage, setToPage] = useState("");
  const [toUrl, setToUrl] = useState("");

  const error = add.error ?? remove.error;

  return (
    <div className="max-w-205">
      <h1 className={ui.settingsTitle}>Redirects</h1>
      <p className="text-muted">
        Old URLs that lead somewhere else. Moves and renames add them automatically; chains are collapsed when the site
        is built, so a visitor is redirected once.
      </p>

      <table className={ui.table} aria-label="Redirects">
        <thead>
          <tr>
            <th scope="col" className={ui.th}>
              From
            </th>
            <th scope="col" className={ui.th}>
              To
            </th>
            <th scope="col" className={ui.th}>
              Source
            </th>
            <th scope="col" className={ui.th}>
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {redirects.data?.length === 0 && (
            <tr>
              <td colSpan={4} className={`${ui.td} text-muted text-help`}>
                No redirects yet.
              </td>
            </tr>
          )}
          {redirects.data?.map((r) => (
            <tr key={r.from_path} data-inactive={r.inactive ?? undefined}>
              <td className={cell(r)}>
                <code className={code}>{r.from_path}</code>
              </td>
              <td className={cell(r)}>
                <Destination entry={r} />
              </td>
              <td className={cell(r)}>
                <span className={ui.badge()}>{originLabel[r.origin]}</span>
              </td>
              <td className={cell(r)}>
                {canEdit && (
                  <button
                    type="button"
                    className={ui.icon}
                    aria-label={`Delete redirect from ${r.from_path}`}
                    onClick={() => remove.mutate(r.from_path)}
                  >
                    ×
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {canEdit && (
        <>
          <h2 className={ui.settingsHeading}>Add a redirect</h2>
          <form
            className="grid gap-1 max-w-130"
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
            <div className={ui.field}>
              <label htmlFor={`${id}-from`} className={ui.fieldLabel}>
                From
              </label>
              <input
                id={`${id}-from`}
                className={ui.input}
                placeholder="/old-path"
                value={from}
                onChange={(e) => setFrom(e.target.value)}
              />
            </div>
            <div className={ui.field}>
              <label htmlFor={`${id}-page`} className={ui.fieldLabel}>
                To a page
              </label>
              <select
                id={`${id}-page`}
                className={ui.select}
                value={toPage}
                onChange={(e) => setToPage(e.target.value)}
              >
                <option value="">Choose a page…</option>
                {pages.data
                  ?.filter((p) => p.kind !== "link")
                  .map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.title}
                    </option>
                  ))}
              </select>
            </div>
            <div className={ui.field}>
              <label htmlFor={`${id}-url`} className={ui.fieldLabel}>
                Or to a URL
              </label>
              <input
                id={`${id}-url`}
                className={ui.input}
                placeholder="https://… or /path"
                disabled={toPage !== ""}
                value={toUrl}
                onChange={(e) => setToUrl(e.target.value)}
              />
            </div>
            <button
              type="submit"
              className={`${ui.primary} justify-self-start`}
              disabled={add.isPending || !from || (!toPage && !toUrl)}
            >
              Add redirect
            </button>
          </form>
        </>
      )}
      {error && (
        <p role="alert" className="text-danger">
          {error.message}
        </p>
      )}
    </div>
  );
}

const code = "font-mono text-[12.5px]";
const cell = (entry: Entry) => `${ui.td} ${entry.inactive ? "opacity-65" : ""}`;

function Destination({ entry }: { entry: Entry }) {
  if (entry.inactive) return <span className="text-muted">{inactiveLabel[entry.inactive]}</span>;

  return (
    <>
      {entry.to_page_title && <strong>{entry.to_page_title} </strong>}
      <code className={code}>{entry.to}</code>
    </>
  );
}
