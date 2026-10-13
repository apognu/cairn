import { useEffect, useId, useState } from "react";
import type { components } from "./api/schema";
import { LinkList } from "./LinkLists";
import { LogoField } from "./LogoField";
import { useSaveSettings, useSettings, useThemeCheck } from "./queries";
import { ThemeColors } from "./ThemeColors";
import { ThemePreview } from "./ThemePreview";

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
    <div className="theme-settings">
      <form onSubmit={submit} className="settings-form">
        <h1>Site and theme</h1>
        <div className="form-field">
          <label htmlFor={`${id}-name`}>Site name</label>
          <input id={`${id}-name`} value={draft.site_name} onChange={(e) => change({ site_name: e.target.value })} />
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
        <div className="form-field">
          <label htmlFor={`${id}-accent`}>Accent color</label>
          <div className="row-inline color-field">
            <input
              type="color"
              aria-label="Accent color picker"
              value={draft.accent}
              onChange={(e) => change({ accent: e.target.value })}
            />
            <input
              id={`${id}-accent`}
              aria-label="Accent color (hex)"
              value={draft.accent}
              onChange={(e) => change({ accent: e.target.value })}
            />
          </div>
          <small className="muted">The dark-mode shade is derived from it.</small>
        </div>

        <div className="section-head">
          <h2>Colors</h2>
          <div role="tablist" aria-label="Mode" className="segmented">
            {(["light", "dark"] as const).map((m) => (
              <button key={m} type="button" role="tab" aria-selected={mode === m} onClick={() => setMode(m)}>
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
          <p role="alert" className="error-text">
            {invalid}
          </p>
        ) : problems.length > 0 ? (
          <ul role="alert" className="contrast-problems">
            {problems.map((problem) => (
              <li key={problem}>{problem}</li>
            ))}
          </ul>
        ) : (
          <p className="contrast-ok">Text contrast passes WCAG AA in light and dark mode</p>
        )}

        <h2>Header links</h2>
        <LinkList label="Header" links={draft.header_links} onChange={(header_links) => change({ header_links })} />

        <h2>Footer</h2>
        <div className="form-field">
          <label htmlFor={`${id}-footer`}>Footer text</label>
          <input
            id={`${id}-footer`}
            value={draft.footer.text}
            onChange={(e) => change({ footer: { ...draft.footer, text: e.target.value } })}
          />
        </div>
        {draft.footer.columns.map((column, index) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: columns have no identity besides their place
          <fieldset className="footer-column" key={index}>
            <legend>Column {index + 1}</legend>
            <div className="row-inline">
              <input
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
                className="secondary"
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
          className="secondary"
          onClick={() =>
            change({ footer: { ...draft.footer, columns: [...draft.footer.columns, { title: "", links: [] }] } })
          }
        >
          Add a column
        </button>

        <h2>Custom CSS</h2>
        <div className="form-field">
          <label htmlFor={`${id}-css`}>Stylesheet, loaded after the theme</label>
          <textarea
            id={`${id}-css`}
            className="code-input"
            rows={8}
            spellCheck={false}
            value={draft.custom_css}
            onChange={(e) => change({ custom_css: e.target.value })}
          />
          <small className="muted">Documented class names are in docs/CUSTOM_CSS.md.</small>
        </div>

        {error && (
          <p role="alert" className="error-text">
            {error}
          </p>
        )}
        <div className="dialog-actions sticky-actions">
          {saved && <output className="muted">Saved. It goes live with the next publish.</output>}
          <button
            type="button"
            className="secondary"
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
            className="primary"
            disabled={save.isPending || !dirty || problems.length > 0 || invalid !== null}
          >
            Save theme
          </button>
        </div>
      </form>

      <aside className="theme-previews" aria-label="Live preview">
        <h2>Live preview</h2>
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
        <p className="muted">The saved theme goes live with the next publish, which renders every page again.</p>
      </aside>
    </div>
  );
}
