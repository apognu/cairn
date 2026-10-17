import { defineComponent, z } from "@cairn/sdk";

/** A titled section readers open and close; closed at first unless told otherwise. A real `<details>`, so it needs no script. */
export const collapsible = defineComponent({
  name: "collapsible",
  version: 1,
  title: "Collapsible section",
  description: "A titled section readers open when they need it: details, an FAQ answer, a long example.",
  props: z.object({
    title: z.string().default("Details").meta({ title: "Title" }),
    open: z.boolean().default(false).meta({ title: "Open at first" }),
  }),
  render: ({ title, open }, children) => (
    <details className="collapsible" open={open}>
      <summary className="collapsible-summary">{title}</summary>
      <div className="collapsible-body">{children}</div>
    </details>
  ),
});
