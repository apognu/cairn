import { ApiReference } from "@cairn/renderer";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useId, useState } from "react";
import { serverUrl } from "./api/base";
import type { components } from "./api/schema";
import { useApiConfig, useCan, useCheckApiConfig, useSaveApiConfig } from "./queries";
import * as ui from "./ui";

type ApiConfig = components["schemas"]["ApiConfig"];
type ApiSource = components["schemas"]["ApiSource"];
type GitRef = components["schemas"]["GitRef"];
type Page = components["schemas"]["Page"];

const refKinds = { latest_release: "Latest release", tag: "Tag", branch: "Branch" } as const;

/** Where an API reference page gets its specs, how they merge, and what readers see. Specs are fetched at every publish. */
export function ApiSettings({ page }: { page: Page }) {
  const id = useId();
  const canEdit = useCan("editor");
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
    queryFn: async () => (await fetch(serverUrl(`/api/pages/${page.id}/api-spec`))).ok,
  });

  useEffect(() => {
    if (config.data) setDraft(config.data);
  }, [config.data]);

  if (!draft) return <p className="text-muted px-2.5">{config.error ? config.error.message : "Loading…"}</p>;

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
    // The preview goes under the form when the canvas, not the window, is too narrow for both: the tree takes room on a tablet.
    <div className="@container">
      <div className="grid grid-cols-1 @3xl:grid-cols-[minmax(0,640px)_minmax(320px,1fr)] gap-8 items-start">
        <form className="max-w-205 min-w-0" onSubmit={submit}>
          <fieldset disabled={!canEdit} className="contents">
            <h1 className={ui.settingsTitle}>{page.title}</h1>
            <p className="text-muted">
              API reference page. Specs are fetched from their sources and merged at every publish.
            </p>

            <div className={ui.sectionHead}>
              <h2 className="m-0 text-[17px]">Sources</h2>
              <span className="text-muted">merged in this order</span>
            </div>
            {draft.sources.map((source, index) => (
              // Sources have no identity besides their place in the merge order.
              // biome-ignore lint/suspicious/noArrayIndexKey: see above
              <fieldset className="mx-0 mt-0 mb-2.5 py-3 px-3.5 border border-input rounded-[10px]" key={index}>
                <legend className="px-1.5 text-help font-semibold">
                  <span className={ui.badge()}>{source.type === "github" ? "GitHub" : "URL"}</span> Source {index + 1}
                </legend>
                {source.type === "url" ? (
                  <div className={ui.field}>
                    <label htmlFor={`${id}-url-${index}`} className={ui.fieldLabel}>
                      Spec URL
                    </label>
                    <input
                      id={`${id}-url-${index}`}
                      className={`${ui.input} flex-none`}
                      value={source.url}
                      placeholder="https://example.com/openapi.yaml"
                      onChange={(e) => setSource(index, { ...source, url: e.target.value })}
                    />
                  </div>
                ) : (
                  <GithubFields id={`${id}-gh-${index}`} source={source} onChange={(next) => setSource(index, next)} />
                )}
                <div className={`${ui.inline} items-end`}>
                  <div className={`${ui.fieldBase} flex-1`}>
                    <label htmlFor={`${id}-prefix-${index}`} className={ui.fieldLabel}>
                      Tag prefix
                    </label>
                    <input
                      id={`${id}-prefix-${index}`}
                      className={`${ui.input} flex-none`}
                      value={source.prefix ?? ""}
                      onChange={(e) => setSource(index, { ...source, prefix: e.target.value })}
                    />
                  </div>
                  <button
                    type="button"
                    className={ui.secondary}
                    aria-label={`Move source ${index + 1} up`}
                    disabled={index === 0}
                    onClick={() => move(index, -1)}
                  >
                    ↑
                  </button>
                  <button
                    type="button"
                    className={ui.secondary}
                    aria-label={`Move source ${index + 1} down`}
                    disabled={index === draft.sources.length - 1}
                    onClick={() => move(index, 1)}
                  >
                    ↓
                  </button>
                  <button
                    type="button"
                    className={ui.secondary}
                    aria-label={`Remove source ${index + 1}`}
                    onClick={() => change({ sources: draft.sources.filter((_, i) => i !== index) })}
                  >
                    ×
                  </button>
                </div>
              </fieldset>
            ))}
            <div className={ui.inline}>
              <button
                type="button"
                className={ui.secondary}
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
                className={ui.secondary}
                onClick={() => change({ sources: [...draft.sources, { type: "url", url: "https://" }] })}
              >
                + Add URL source
              </button>
            </div>

            <h2 className={ui.settingsHeading}>Information</h2>
            <p className="text-muted">By default the title, version and description come from the first source.</p>
            {(["title", "version", "description"] as const).map((key) => (
              <div className={ui.field} key={key}>
                <label htmlFor={`${id}-${key}`} className={ui.fieldLabel}>
                  {key[0].toUpperCase() + key.slice(1)}
                </label>
                <input
                  id={`${id}-${key}`}
                  className={ui.input}
                  value={draft.merge[key] ?? ""}
                  placeholder="From the first source"
                  onChange={(e) => change({ merge: { ...draft.merge, [key]: e.target.value || null } })}
                />
              </div>
            ))}

            <h2 className={ui.settingsHeading}>Display</h2>
            <div className={ui.field}>
              <label htmlFor={`${id}-servers`} className={ui.fieldLabel}>
                Servers shown to readers
              </label>
              <input
                id={`${id}-servers`}
                className={ui.input}
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
              <small className="text-muted">Separate several with commas.</small>
            </div>
            <label className={ui.check}>
              <input
                type="checkbox"
                checked={draft.display.beta}
                onChange={(e) => change({ display: { ...draft.display, beta: e.target.checked } })}
              />
              Mark as beta: a pill next to the name in the site's sidebar
            </label>
            <label className={ui.check}>
              <input
                type="checkbox"
                checked={draft.display.request_client}
                onChange={(e) => change({ display: { ...draft.display, request_client: e.target.checked } })}
              />
              Let readers send test requests
            </label>
            <label className={ui.check}>
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
                className={`${ui.secondary} ml-auto`}
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
              <p role="alert" className="text-danger">
                {error}
              </p>
            )}
            <div className={`${ui.inline} ${ui.stickyActions}`}>
              <button type="submit" className={ui.primary} disabled={!dirty || save.isPending}>
                {save.isPending ? "Saving…" : "Save"}
              </button>
              {saved && <output className="text-muted">Saved</output>}
            </div>
          </fieldset>
        </form>
        <aside
          className="@3xl:sticky @3xl:top-0 max-h-[calc(100vh-100px)] overflow-auto p-4 border border-border rounded-[10px] bg-side"
          aria-label="Preview"
        >
          <div className="flex items-center justify-between mt-0 mb-2.5">
            <h2 className="m-0 text-label tracking-[0.06em] uppercase">Preview</h2>
            <span className="text-muted">Scalar, with your theme</span>
          </div>
          {preview ? (
            <ApiReference
              key={version}
              compact
              api={{
                specUrl: serverUrl(`/api/pages/${page.id}/api-spec?v=${version}`),
                scalarUrl: serverUrl("/api/scalar.js"),
                operations: [],
                display: {
                  servers: draft.display.servers,
                  hideModels: draft.display.hide_models,
                  requestClient: draft.display.request_client,
                },
              }}
            />
          ) : (
            <p className="text-muted">Check the sources to preview the reference here.</p>
          )}
        </aside>
      </div>
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
      <div className={ui.field}>
        <label htmlFor={`${id}-repo`} className={ui.fieldLabel}>
          Repository
        </label>
        <input
          id={`${id}-repo`}
          className={`${ui.input} flex-none`}
          value={source.repo}
          placeholder="owner/name"
          onChange={(e) => onChange({ ...source, repo: e.target.value })}
        />
      </div>
      <div className={`${ui.inline} items-end`}>
        <div className={`${ui.field} flex-1`}>
          <label htmlFor={`${id}-ref`} className={ui.fieldLabel}>
            Version
          </label>
          <select
            id={`${id}-ref`}
            className={ui.select}
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
          <div className={`${ui.field} flex-1`}>
            <label htmlFor={`${id}-name`} className={ui.fieldLabel}>
              {ref.type === "tag" ? "Tag" : "Branch"}
            </label>
            <input
              id={`${id}-name`}
              className={`${ui.input} flex-none`}
              value={ref.name}
              onChange={(e) => onChange({ ...source, git_ref: { ...ref, name: e.target.value } })}
            />
          </div>
        )}
      </div>
      <label className={ui.check}>
        <input
          type="checkbox"
          checked={isAsset}
          onChange={(e) => onChange({ ...source, asset: e.target.checked ? "" : null })}
        />
        A release asset instead of a file in the repository
      </label>
      {isAsset ? (
        <div className={ui.field}>
          <label htmlFor={`${id}-asset`} className={ui.fieldLabel}>
            Asset file name
          </label>
          <input
            id={`${id}-asset`}
            className={`${ui.input} flex-none`}
            value={source.asset ?? ""}
            placeholder="openapi.json"
            onChange={(e) => onChange({ ...source, asset: e.target.value })}
          />
        </div>
      ) : (
        <div className={ui.field}>
          <label htmlFor={`${id}-path`} className={ui.fieldLabel}>
            Path in the repository
          </label>
          <input
            id={`${id}-path`}
            className={`${ui.input} flex-none`}
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
    <output
      className={`box-border w-full block my-5 py-3 px-3.5 border rounded-lg bg-side ${errors.length > 0 ? "border-danger" : merged ? "border-accent" : "border-border"}`}
    >
      <div className="flex flex-wrap items-center gap-3">
        {pending ? (
          <span className="text-muted">Fetching the sources…</span>
        ) : merged ? (
          <span>
            Merged: <strong>{count(result.operations_count, "operation")}</strong> in {count(result.tags.length, "tag")}
            , no conflicts.
          </span>
        ) : errors.length > 0 ? (
          <span>The sources could not be merged.</span>
        ) : (
          <span className="text-muted">The specs are fetched and merged when you publish.</span>
        )}
        {children}
      </div>
      {errors.length > 0 && (
        <ul className="mt-2.5 mb-0 p-0 list-none text-help text-danger">
          {errors.map((message) => (
            <li key={message}>{message}</li>
          ))}
        </ul>
      )}
      {merged && result.sources.length > 0 && (
        <ul className="mt-2.5 mb-0 p-0 list-none text-help">
          {result.sources.map((source) => (
            <li key={source.url} className="flex justify-between gap-3 py-1 border-t border-border">
              <span>{source.label}</span>
              <span className={ui.mono}>{source.tag ?? source.etag ?? "fetched"}</span>
            </li>
          ))}
        </ul>
      )}
    </output>
  );
}

const count = (n: number, noun: string) => `${n} ${noun}${n === 1 ? "" : "s"}`;
