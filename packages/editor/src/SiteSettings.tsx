import { useEffect, useId, useState } from "react";
import { useSaveSettings, useSettings } from "./queries";
import { uploadImage } from "./upload";

/** Site name, logo and accent color. They reach the published site with the next publish. */
export function SiteSettings() {
  const id = useId();
  const settings = useSettings();
  const save = useSaveSettings();
  const [name, setName] = useState("");
  const [accent, setAccent] = useState("#0b6e4f");
  const [logo, setLogo] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  // "Saved" describes the form as it was saved, not after a later change.
  // biome-ignore lint/correctness/useExhaustiveDependencies: runs on every change of the fields
  useEffect(() => setSaved(false), [name, accent, logo]);

  useEffect(() => {
    if (!settings.data) return;
    setName(settings.data.site_name);
    setAccent(settings.data.accent);
    setLogo(settings.data.logo_asset_id ?? null);
  }, [settings.data]);

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    setSaved(false);
    setError(null);
    save.mutate(
      { site_name: name, accent, logo_asset_id: logo },
      { onSuccess: () => setSaved(true), onError: (err) => setError(err.message) },
    );
  };

  return (
    <form onSubmit={submit} className="settings-form">
      <h1>Site and theme</h1>
      <div className="form-field">
        <label htmlFor={`${id}-name`}>Site name</label>
        <input id={`${id}-name`} value={name} onChange={(e) => setName(e.target.value)} />
      </div>
      <div className="form-field">
        <label htmlFor={`${id}-accent`}>Accent color</label>
        <div className="row-inline color-field">
          <input id={`${id}-accent`} type="color" value={accent} onChange={(e) => setAccent(e.target.value)} />
          <input aria-label="Accent color (hex)" value={accent} onChange={(e) => setAccent(e.target.value)} />
          <small className="muted">The dark-mode shade is derived from it.</small>
        </div>
      </div>
      <div className="form-field">
        <label htmlFor={`${id}-logo`}>Logo</label>
        <div className="logo-field">
          {logo ? (
            <img className="logo-preview" src={`/_a/${logo}`} alt="Current logo" />
          ) : (
            <span className="muted">The site name alone</span>
          )}
          {/* The browser's own file input cannot be styled; it stays reachable by keyboard, drawn as the button after it. */}
          <input
            id={`${id}-logo`}
            className="sr-only"
            type="file"
            accept="image/png,image/jpeg,image/gif,image/webp"
            onChange={(e) => {
              const file = e.target.files?.[0];
              e.target.value = "";
              if (file)
                uploadImage(file).then(
                  (asset) => setLogo(asset.id),
                  (err: Error) => setError(err.message),
                );
            }}
          />
          <label htmlFor={`${id}-logo`} className="button">
            {logo ? "Replace" : "Upload"}
          </label>
          {logo && (
            <button type="button" onClick={() => setLogo(null)}>
              Remove
            </button>
          )}
        </div>
      </div>
      {error && (
        <p role="alert" className="error-text">
          {error}
        </p>
      )}
      <div className="dialog-actions">
        {saved && <output className="muted">Saved. It goes live with the next publish.</output>}
        <button type="submit" className="primary" disabled={save.isPending}>
          Save
        </button>
      </div>
    </form>
  );
}
