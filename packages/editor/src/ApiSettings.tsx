import { useQuery } from "@tanstack/react-query";
import { ApiReference } from "@yourdocs/renderer";
import { useEffect, useId, useState } from "react";
import type { components } from "./api/schema";
import { useApiConfig, useCheckApiConfig, useSaveApiConfig } from "./queries";

type ApiConfig = components["schemas"]["ApiConfig"];
type ApiSource = components["schemas"]["ApiSource"];
type GitRef = components["schemas"]["GitRef"];
type Page = components["schemas"]["Page"];

const refKinds = { latest_release: "Latest release", tag: "Tag", branch: "Branch" } as const;

/** Where an API reference page gets its specs, how they merge, and what readers see. Specs are fetched at every publish. */
export function ApiSettings({ page }: { page: Page }) {
  const id = useId();
  const config = useApiConfig(page.id);
  const save = useSaveApiConfig(page.id);
  const check = useCheckApiConfig(page.id);
  const [draft, setDraft] = useState<ApiConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  // Bumped by each successful check, so the preview loads the spec it just merged.
  const [version, setVersion] = useState(0);
  // Before any check, what the live site was published with; nothing yet when it was never published.
  const live = useQuery({
    queryKey: ["api-spec", page.id],
    retry: false,
    queryFn: async () => (await fetch(`/api/pages/${page.id}/api-spec`)).ok,
  });

  useEffect(() => {
    if (config.data) setDraft(config.data);
  }, [config.data]);

  if (!draft) return <p className="muted pad">{config.error ? config.error.message : "Loading…"}</p>;

  const change = (patch: Partial<ApiConfig>) => {
    setDraft({ ...draft, ...patch });
    setSaved(false);
  };
  const setSource = (index: number, source: ApiSource) =>
    change({ sources: draft.sources.map((s, i) => (i === index ? source : s)) });
  const move = (index: number, by: -1 | 1) => {
    const sources = [...draft.sources];
    const [moved] = sources.splice(index, 1);
    sources.splice(index + by, 0, moved);
    change({ sources });
  };
  const dirty = JSON.stringify(draft) !== JSON.stringify(config.data);

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSaved(false);
    save.mutate(draft, { onSuccess: () => setSaved(true), onError: (err) => setError(err.message) });
  };

  const checked = check.data && check.data.errors.length === 0;
  const preview = checked || live.data === true;

  return (
    <div className="api-layout">
      <form className="settings-form wide api-settings" onSubmit={submit}>
        <h1>{page.title}</h1>
        <p className="muted">API reference page. Specs are fetched from their sources and merged at every publish.</p>

        <div className="section-head">
          <h2>Sources</h2>
          <span className="muted">merged in this order</span>
        </div>
        {draft.sources.map((source, index) => (
          // Sources have no identity besides their place in the merge order.
          // biome-ignore lint/suspicious/noArrayIndexKey: see above
          <fieldset className="api-source" key={index}>
            <legend>
              <span className="badge">{source.type === "github" ? "GitHub" : "URL"}</span> Source {index + 1}
            </legend>
            {source.type === "url" ? (
              <div className="form-field">
                <label htmlFor={`${id}-url-${index}`}>Spec URL</label>
                <input
                  id={`${id}-url-${index}`}
                  value={source.url}
                  placeholder="https://example.com/openapi.yaml"
                  onChange={(e) => setSource(index, { ...source, url: e.target.value })}
                />
              </div>
            ) : (
              <GithubFields id={`${id}-gh-${index}`} source={source} onChange={(next) => setSource(index, next)} />
            )}
            <div className="row-inline source-foot">
              <div className="form-field">
                <label htmlFor={`${id}-prefix-${index}`}>Tag prefix</label>
                <input
                  id={`${id}-prefix-${index}`}
                  value={source.prefix ?? ""}
                  onChange={(e) => setSource(index, { ...source, prefix: e.target.value })}
                />
              </div>
              <button
                type="button"
                className="secondary"
                aria-label={`Move source ${index + 1} up`}
                disabled={index === 0}
                onClick={() => move(index, -1)}
              >
                ↑
              </button>
              <button
                type="button"
                className="secondary"
                aria-label={`Move source ${index + 1} down`}
                disabled={index === draft.sources.length - 1}
                onClick={() => move(index, 1)}
              >
                ↓
              </button>
              <button
                type="button"
                className="secondary"
                aria-label={`Remove source ${index + 1}`}
                onClick={() => change({ sources: draft.sources.filter((_, i) => i !== index) })}
              >
                ×
              </button>
            </div>
          </fieldset>
        ))}
        <div className="row-inline">
          <button
            type="button"
            className="secondary"
            onClick={() =>
              change({
                sources: [
                  ...draft.sources,
                  { type: "github", repo: "", path: "", git_ref: { type: "latest_release" } },
                ],
              })
            }
          >
            + Add GitHub source
          </button>
          <button
            type="button"
            className="secondary"
            onClick={() => change({ sources: [...draft.sources, { type: "url", url: "https://" }] })}
          >
            + Add URL source
          </button>
        </div>

        <h2>Information</h2>
        <p className="muted">By default the title, version and description come from the first source.</p>
        {(["title", "version", "description"] as const).map((key) => (
          <div className="form-field" key={key}>
            <label htmlFor={`${id}-${key}`}>{key[0].toUpperCase() + key.slice(1)}</label>
            <input
              id={`${id}-${key}`}
              value={draft.merge[key] ?? ""}
              placeholder="From the first source"
              onChange={(e) => change({ merge: { ...draft.merge, [key]: e.target.value || null } })}
            />
          </div>
        ))}

        <h2>Display</h2>
        <div className="form-field">
          <label htmlFor={`${id}-servers`}>Servers shown to readers</label>
          <input
            id={`${id}-servers`}
            value={draft.display.servers.join(", ")}
            placeholder="From the specs"
            onChange={(e) =>
              change({
                display: {
                  ...draft.display,
                  servers: e.target.value
                    .split(",")
                    .map((s) => s.trim())
                    .filter(Boolean),
                },
              })
            }
          />
          <small className="muted">Separate several with commas.</small>
        </div>
        <label className="form-check">
          <input
            type="checkbox"
            checked={draft.display.beta}
            onChange={(e) => change({ display: { ...draft.display, beta: e.target.checked } })}
          />
          Mark as beta: a pill next to the name in the site's sidebar
        </label>
        <label className="form-check">
          <input
            type="checkbox"
            checked={draft.display.request_client}
            onChange={(e) => change({ display: { ...draft.display, request_client: e.target.checked } })}
          />
          Let readers send test requests
        </label>
        <label className="form-check">
          <input
            type="checkbox"
            checked={!draft.display.hide_models}
            onChange={(e) => change({ display: { ...draft.display, hide_models: !e.target.checked } })}
          />
          Show schema models
        </label>

        <CheckResult result={check.data} error={check.error?.message} pending={check.isPending}>
          <button
            type="button"
            className="secondary"
            disabled={check.isPending}
            onClick={() =>
              check.mutate(draft, {
                onSuccess: (result) => {
                  if (result.errors.length === 0) setVersion((n) => n + 1);
                },
              })
            }
          >
            {check.isPending ? "Checking…" : "Check sources now"}
          </button>
        </CheckResult>

        {error && (
          <p role="alert" className="error-text">
            {error}
          </p>
        )}
        <div className="row-inline">
          <button type="submit" className="primary" disabled={!dirty || save.isPending}>
            {save.isPending ? "Saving…" : "Save"}
          </button>
          {saved && <output className="muted">Saved</output>}
        </div>
      </form>
      <aside className="api-preview" aria-label="Preview">
        <div className="section-head">
          <h2>Preview</h2>
          <span className="muted">Scalar, with your theme</span>
        </div>
        {preview ? (
          <ApiReference
            key={version}
            api={{
              specUrl: `/api/pages/${page.id}/api-spec?v=${version}`,
              scalarUrl: "/api/scalar.js",
              operations: [],
              display: {
                servers: draft.display.servers,
                hideModels: draft.display.hide_models,
                requestClient: draft.display.request_client,
              },
            }}
          />
        ) : (
          <p className="muted">Check the sources to preview the reference here.</p>
        )}
      </aside>
    </div>
  );
}

function GithubFields({
  id,
  source,
  onChange,
}: {
  id: string;
  source: Extract<ApiSource, { type: "github" }>;
  onChange: (source: ApiSource) => void;
}) {
  const ref: GitRef = source.git_ref ?? { type: "latest_release" };
  const isAsset = source.asset !== null && source.asset !== undefined;

  return (
    <>
      <div className="form-field">
        <label htmlFor={`${id}-repo`}>Repository</label>
        <input
          id={`${id}-repo`}
          value={source.repo}
          placeholder="owner/name"
          onChange={(e) => onChange({ ...source, repo: e.target.value })}
        />
      </div>
      <div className="row-inline">
        <div className="form-field">
          <label htmlFor={`${id}-ref`}>Version</label>
          <select
            id={`${id}-ref`}
            value={ref.type}
            onChange={(e) =>
              onChange({
                ...source,
                git_ref:
                  e.target.value === "latest_release"
                    ? { type: "latest_release" }
                    : { type: e.target.value as "tag", name: "" },
              })
            }
          >
            {Object.entries(refKinds).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </div>
        {ref.type !== "latest_release" && (
          <div className="form-field">
            <label htmlFor={`${id}-name`}>{ref.type === "tag" ? "Tag" : "Branch"}</label>
            <input
              id={`${id}-name`}
              value={ref.name}
              onChange={(e) => onChange({ ...source, git_ref: { ...ref, name: e.target.value } })}
            />
          </div>
        )}
      </div>
      <label className="form-check">
        <input
          type="checkbox"
          checked={isAsset}
          onChange={(e) => onChange({ ...source, asset: e.target.checked ? "" : null })}
        />
        A release asset instead of a file in the repository
      </label>
      {isAsset ? (
        <div className="form-field">
          <label htmlFor={`${id}-asset`}>Asset file name</label>
          <input
            id={`${id}-asset`}
            value={source.asset ?? ""}
            placeholder="openapi.json"
            onChange={(e) => onChange({ ...source, asset: e.target.value })}
          />
        </div>
      ) : (
        <div className="form-field">
          <label htmlFor={`${id}-path`}>Path in the repository</label>
          <input
            id={`${id}-path`}
            value={source.path ?? ""}
            placeholder="openapi/spec.yaml"
            onChange={(e) => onChange({ ...source, path: e.target.value })}
          />
        </div>
      )}
    </>
  );
}

type Check = components["schemas"]["ApiCheck"];

/** What the last "Check sources now" found, or a reminder that the specs are only read at publish time. */
function CheckResult({
  result,
  error,
  pending,
  children,
}: {
  result: Check | undefined;
  error: string | undefined;
  pending: boolean;
  children: React.ReactNode;
}) {
  const errors = result?.errors ?? (error ? [error] : []);
  const merged = result && result.errors.length === 0;

  return (
    <output className="api-check" data-state={errors.length > 0 ? "error" : merged ? "ok" : undefined}>
      <div className="api-check-head">
        {pending ? (
          <span className="muted">Fetching the sources…</span>
        ) : merged ? (
          <span>
            Merged: <strong>{count(result.operations_count, "operation")}</strong> in {count(result.tags.length, "tag")}
            , no conflicts.
          </span>
        ) : errors.length > 0 ? (
          <span>The sources could not be merged.</span>
        ) : (
          <span className="muted">The specs are fetched and merged when you publish.</span>
        )}
        {children}
      </div>
      {errors.length > 0 && (
        <ul className="api-errors">
          {errors.map((message) => (
            <li key={message}>{message}</li>
          ))}
        </ul>
      )}
      {merged && result.sources.length > 0 && (
        <ul className="api-resolved">
          {result.sources.map((source) => (
            <li key={source.url}>
              <span>{source.label}</span>
              <span className="mono">{source.tag ?? source.etag ?? "fetched"}</span>
            </li>
          ))}
        </ul>
      )}
    </output>
  );
}

const count = (n: number, noun: string) => `${n} ${noun}${n === 1 ? "" : "s"}`;
