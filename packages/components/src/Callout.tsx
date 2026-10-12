import { defineComponent, z } from "@yourdocs/sdk";
import type { ReactNode } from "react";

export const callout = defineComponent({
  name: "callout",
  version: 1,
  title: "Callout",
  props: z.object({
    tone: z.enum(["note", "tip", "warning"]).default("note").meta({ title: "Type" }),
  }),
  render: ({ tone }, children) => <Callout tone={tone}>{children}</Callout>,
});

export type CalloutProps = { tone?: string; children: ReactNode };

export function Callout({ tone = "note", children }: CalloutProps) {
  return (
    <aside className="callout" data-tone={tone}>
      {children}
    </aside>
  );
}
