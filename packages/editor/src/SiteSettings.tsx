import { useEffect, useId, useState } from "react";
import type { components } from "./api/schema";
import { LinkList } from "./LinkLists";
import { LogoField } from "./LogoField";
import { useSaveSettings, useSettings, useThemeCheck } from "./queries";
import { ThemeColors } from "./ThemeColors";
import { ThemePreview } from "./ThemePreview";
import * as ui from "./ui";

type Settings = components["schemas"]["Settings"];

/** Name, logos, colors, header links and footer of the site. They reach the published site with the next publish. */
export function SiteSettings() {
  const id = useId();
  const settings = useSettings();
  const save = useSaveSettings();
  const [draft, setDraft] = useState<Settings | null>(null);
  const [mode, setMode] = useState<"light" | "dark">("light");
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const check = useThemeCheck(draft);

  useEffect(() => {
    if (settings.data) setDraft(settings.data);
  }, [settings.data]);

  if (!draft) return null;

  const change = (patch: Partial<Settings>) => {
    setDraft({ ...draft, ...patch });
    setSaved(false);
  };
  const problems = check.data?.problems ?? [];
  // The server refuses what it cannot check (a bad hex color, a link without a scheme) and says why.
  const invalid = check.error?.message ?? null;
  const dirty = JSON.stringify(draft) !== JSON.stringify(settings.data);

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    setSaved(false);
    setError(null);
    save.mutate(draft, { onSuccess: () => setSaved(true), onError: (err) => setError(err.message) });
  };

  return (
    <div className="flex items-start gap-8">
      <form onSubmit={submit} className="flex-none w-130">
        <h1 className={ui.settingsTitle}>Site and theme</h1>
        <div className={ui.field}>
          <label htmlFor={`${id}-name`} className={ui.fieldLabel}>
            Site name
          </label>
          <input
            id={`${id}-name`}
            className={ui.input}
            value={draft.site_name}
            onChange={(e) => change({ site_name: e.target.value })}
          />
        </div>
        <LogoField
          label="Logo"
          value={draft.logo_asset_id ?? null}
          onChange={(logo_asset_id) => change({ logo_asset_id })}
          onError={setError}
        />
        <LogoField
          label="Dark mode logo"
          dark
          value={draft.logo_dark_asset_id ?? null}
          onChange={(logo_dark_asset_id) => change({ logo_dark_asset_id })}
          onError={setError}
        />
        <div className={ui.field}>
          <label htmlFor={`${id}-accent`} className={ui.fieldLabel}>
            Accent color
          </label>
          <div className={`${ui.inline} items-center`}>
            <input
              type="color"
              className="flex-none w-8.5 h-8.5 p-0.5 border border-input rounded-md bg-bg cursor-pointer"
              aria-label="Accent color picker"
              value={draft.accent}
              onChange={(e) => change({ accent: e.target.value })}
            />
            <input
              id={`${id}-accent`}
              className="flex-none w-27.5 min-w-0 h-8.5 px-2.5 border border-input rounded-md bg-bg font-mono"
              aria-label="Accent color (hex)"
              value={draft.accent}
              onChange={(e) => change({ accent: e.target.value })}
            />
          </div>
          <small className="text-muted">The dark-mode shade is derived from it.</small>
        </div>

        <div className={ui.sectionHead}>
          <h2 className="m-0 text-[17px]">Colors</h2>
          <div role="tablist" aria-label="Mode" className={ui.segmented}>
            {(["light", "dark"] as const).map((m) => (
              <button
                key={m}
                type="button"
                role="tab"
                className={ui.segment}
                aria-selected={mode === m}
                onClick={() => setMode(m)}
              >
                {m === "light" ? "Light" : "Dark"}
              </button>
            ))}
          </div>
        </div>
        <ThemeColors
          mode={mode}
          colors={draft[mode]}
          accent={draft.accent}
          onChange={(colors) => change({ [mode]: colors })}
        />
        {invalid ? (
          <p role="alert" className="text-danger">
            {invalid}
          </p>
        ) : problems.length > 0 ? (
          <ul role="alert" className="mt-2.5 mb-0 pl-4.5 text-danger text-help">
            {problems.map((problem) => (
              <li key={problem}>{problem}</li>
            ))}
          </ul>
        ) : (
          <p className="mt-2.5 mb-0 text-accent text-help">Text contrast passes WCAG AA in light and dark mode</p>
        )}

        <h2 className={ui.settingsHeading}>Header links</h2>
        <LinkList label="Header" links={draft.header_links} onChange={(header_links) => change({ header_links })} />

        <h2 className={ui.settingsHeading}>Footer</h2>
        <div className={ui.field}>
          <label htmlFor={`${id}-footer`} className={ui.fieldLabel}>
            Footer text
          </label>
          <input
            id={`${id}-footer`}
            className={ui.input}
            value={draft.footer.text}
            onChange={(e) => change({ footer: { ...draft.footer, text: e.target.value } })}
          />
        </div>
        {draft.footer.columns.map((column, index) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: columns have no identity besides their place
          <fieldset className="mx-0 my-3 pt-2.5 px-3 pb-3 border border-border rounded-lg" key={index}>
            <legend className="px-1 text-muted text-label">Column {index + 1}</legend>
            <div className={`${ui.inline} mb-2`}>
              <input
                className="flex-1 min-w-0 h-8.5 px-2.5 border border-input rounded-md bg-bg text-text text-ui"
                aria-label={`Column ${index + 1} title`}
                placeholder="Title"
                value={column.title}
                onChange={(e) =>
                  change({
                    footer: {
                      ...draft.footer,
                      columns: draft.footer.columns.map((c, i) => (i === index ? { ...c, title: e.target.value } : c)),
                    },
                  })
                }
              />
              <button
                type="button"
                className={ui.secondary}
                onClick={() =>
                  change({ footer: { ...draft.footer, columns: draft.footer.columns.filter((_, i) => i !== index) } })
                }
              >
                Remove column
              </button>
            </div>
            <LinkList
              label={`Column ${index + 1}`}
              links={column.links}
              onChange={(links) =>
                change({
                  footer: {
                    ...draft.footer,
                    columns: draft.footer.columns.map((c, i) => (i === index ? { ...c, links } : c)),
                  },
                })
              }
            />
          </fieldset>
        ))}
        <button
          type="button"
          className={ui.secondary}
          onClick={() =>
            change({ footer: { ...draft.footer, columns: [...draft.footer.columns, { title: "", links: [] }] } })
          }
        >
          Add a column
        </button>

        <h2 className={ui.settingsHeading}>Custom CSS</h2>
        <div className={ui.field}>
          <label htmlFor={`${id}-css`} className={ui.fieldLabel}>
            Stylesheet, loaded after the theme
          </label>
          <textarea
            id={`${id}-css`}
            className={ui.codeInput}
            rows={8}
            spellCheck={false}
            value={draft.custom_css}
            onChange={(e) => change({ custom_css: e.target.value })}
          />
          <small className="text-muted">Documented class names are in docs/CUSTOM_CSS.md.</small>
        </div>

        {error && (
          <p role="alert" className="text-danger">
            {error}
          </p>
        )}
        <div className={`${ui.actions} ${ui.stickyActions}`}>
          {saved && <output className="text-muted">Saved. It goes live with the next publish.</output>}
          <button
            type="button"
            className={ui.secondary}
            disabled={!dirty}
            onClick={() => {
              setDraft(settings.data ?? null);
              setSaved(false);
            }}
          >
            Discard
          </button>
          <button
            type="submit"
            className={ui.primary}
            disabled={save.isPending || !dirty || problems.length > 0 || invalid !== null}
          >
            Save theme
          </button>
        </div>
      </form>

      <aside className="sticky top-0 flex-1 min-w-80 flex flex-col gap-4" aria-label="Live preview">
        <h2 className="m-0 text-muted text-label tracking-[0.06em] uppercase">Live preview</h2>
        {(["light", "dark"] as const).map((m) => (
          <ThemePreview
            key={m}
            mode={m}
            vars={check.data?.[m] ?? {}}
            name={draft.site_name}
            logo={
              m === "dark" ? (draft.logo_dark_asset_id ?? draft.logo_asset_id ?? null) : (draft.logo_asset_id ?? null)
            }
            links={draft.header_links.map((l) => l.label).filter(Boolean)}
          />
        ))}
        <p className="m-0 text-muted text-help">
          The saved theme goes live with the next publish, which renders every page again.
        </p>
      </aside>
    </div>
  );
}
