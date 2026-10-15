import { defineComponent, z } from "@cairn/sdk";

export default defineComponent({
  name: "acme/blue",
  version: 1,
  title: "Blue",
  props: z.object({ label: z.string().default("x") }),
  render: (props) => <p className="text-[#0000ff]">{props.label}</p>,
});
