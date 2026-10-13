import { defineComponent, z } from "@yourdocs/sdk";

export default defineComponent({
  name: "acme/red",
  version: 1,
  title: "Red",
  props: z.object({ label: z.string().default("x") }),
  render: (props) => <p style={{ color: "#ff0000" }}>{props.label}</p>,
});
