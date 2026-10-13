import { defineSlot } from "@yourdocs/sdk";

export default defineSlot("Footer", ({ context }) => (
  <footer className="acme-footer">Acme · {context.site.name}</footer>
));
