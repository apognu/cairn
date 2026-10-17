import { Icon } from "@cairn/components";
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
  /** The header links; one with an icon shows only the icon, as on a wide screen. */
  links: { label: string; icon?: string | null }[];
}) {
  return (
    <section
      className="overflow-hidden border border-border rounded-[10px] bg-bg text-text text-[13.5px]"
      data-theme={mode}
      style={vars as CSSProperties}
      aria-label={`${mode} preview`}
    >
      <div className="flex items-center gap-2.5 h-10 px-4 border-b border-border text-help">
        {logo && <img src={`/_a/${logo}`} alt="" height={16} />}
        <strong>{name}</strong>
        <span className="py-2.75 font-semibold shadow-[inset_0_-2px_0_var(--accent)]">Guides</span>
        {links.map((link) => (
          <span key={link.label} className="flex text-muted" title={link.icon ? link.label : undefined}>
            {link.icon ? <Icon name={link.icon} size={15} /> : link.label}
          </span>
        ))}
      </div>
      <div className="flex gap-4 p-4">
        <aside className="flex-none self-stretch w-22.5 p-2 rounded-md bg-side text-muted text-label">Surface</aside>
        <div>
          <h3 className="m-0 text-[20px]">Webhooks</h3>
          <p className="mt-1.5 mb-2.5 text-body text-help leading-[1.6]">
            Get an HTTPS request every time a deploy starts or finishes. See{" "}
            <span className="text-link underline underline-offset-2">retries</span>.
          </p>
          <pre className="m-0 py-2 px-2.5 rounded-md bg-code font-mono text-[12.5px] leading-[normal]">
            <code>POST /hooks</code>
          </pre>
        </div>
      </div>
    </section>
  );
}
