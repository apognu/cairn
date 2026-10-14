import { defineComponent, z } from "@cairn/sdk";

export default defineComponent({
  name: "acme/boom",
  version: 1,
  title: "Boom",
  props: z.object({ label: z.string().default("x") }),
  render: () => {
    throw new Error("kaboom");
  },
});
