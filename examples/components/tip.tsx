import { defineComponent, z } from "@cairn/sdk";

// Styled with Tailwind utilities alone. The theme's colors (`bg-accent-soft`, `text-accent`, `border-border`…) follow dark mode and the
// site's accent; Tailwind's own palette is not there, and a fixed color fails the build.
export default defineComponent({
  name: "acme/tip",
  version: 1,
  title: "Acme tip",
  description: "A short tip in the accent color",
  props: z.object({ text: z.string().default("Try it") }),
  render: (props) => (
    <p className="acme-tip my-4 py-3 px-4 rounded-xl border border-dashed border-accent bg-accent-soft text-accent font-semibold">
      {props.text}
    </p>
  ),
});
