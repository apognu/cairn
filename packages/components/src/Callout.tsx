import type { ReactNode } from "react";

export type CalloutProps = { tone?: string; title?: string; children: ReactNode };

export function Callout({ tone = "note", title, children }: CalloutProps) {
  return (
    <aside className="callout" data-tone={tone}>
      {title ? <p className="callout-title">{title}</p> : null}
      {children}
    </aside>
  );
}
