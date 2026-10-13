import type { CSSProperties } from "react";

/** The site in one mode, drawn with the custom properties the server compiled from the unsaved settings. */
export function ThemePreview({
  mode,
  vars,
  name,
  logo,
  links,
}: {
  mode: "light" | "dark";
  vars: Record<string, string>;
  name: string;
  logo: string | null;
  links: string[];
}) {
  return (
    <section className="theme-preview" data-theme={mode} style={vars as CSSProperties} aria-label={`${mode} preview`}>
      <div className="theme-preview-header">
        {logo && <img src={`/_a/${logo}`} alt="" height={16} />}
        <strong>{name}</strong>
        <span className="theme-preview-section">Guides</span>
        {links.map((link) => (
          <span key={link} className="muted">
            {link}
          </span>
        ))}
      </div>
      <div className="theme-preview-body">
        <aside>Surface</aside>
        <div>
          <h3>Webhooks</h3>
          <p>
            Get an HTTPS request every time a deploy starts or finishes. See{" "}
            <span className="theme-preview-link">retries</span>.
          </p>
          <pre>
            <code>POST /hooks</code>
          </pre>
        </div>
      </div>
    </section>
  );
}
