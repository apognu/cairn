import { defineSlot } from "@cairn/sdk";

export default defineSlot("Footer", ({ context }) => (
  <footer className="acme-footer">Acme · {context.site.name}</footer>
));
