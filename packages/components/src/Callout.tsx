import type { ReactNode } from "react";

export type CalloutProps = { tone?: string; children: ReactNode };

export function Callout({ tone = "note", children }: CalloutProps) {
  return (
    <aside className="callout" data-tone={tone}>
      {children}
    </aside>
  );
}
