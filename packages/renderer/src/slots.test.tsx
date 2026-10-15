import { expect, test } from "bun:test";
import { defineSlot } from "@cairn/sdk";
import { renderToString } from "react-dom/server";
import { SiteLayout } from "./layout";
import { defaultContext, type RenderContext } from "./types";

const context: RenderContext = {
  ...defaultContext,
  site: { name: "Docs", footer: { text: "Default footer text", columns: [] } },
  page: { id: "p", title: "Hello" },
};

const render = (slots: Parameters<typeof SiteLayout>[0]["slots"], ctx = context) =>
  renderToString(
    <SiteLayout context={ctx} headings={[]} slots={slots}>
      <p>body</p>
    </SiteLayout>,
  );

test("a Footer slot replaces the default footer, and can wrap it", () => {
  expect(render({})).toContain("Default footer text");

  const replaced = render({
    Footer: defineSlot("Footer", ({ context }) => <footer>{`Acme · ${context.site.name}`}</footer>),
  });
  expect(replaced).toContain("<footer>Acme · Docs</footer>");
  expect(replaced).not.toContain("Default footer text");

  const wrapped = render({ Footer: defineSlot("Footer", ({ children }) => <div className="wrap">{children}</div>) });
  expect(wrapped).toContain('<div class="wrap"><footer class="site-footer ');
});

test("the other slots land in their place, and NotFound only on the not-found page", () => {
  const slots = {
    Header: defineSlot("Header", () => <header>custom header</header>),
    SidebarFooter: defineSlot("SidebarFooter", () => <p>side</p>),
    PageFooter: defineSlot("PageFooter", () => <p>page end</p>),
    NotFound: defineSlot("NotFound", () => <p>lost</p>),
  };
  const html = render(slots);

  expect(html).toContain("<header>custom header</header>");
  expect(html).not.toContain('class="site-header"');
  expect(html).toContain("<p>side</p></nav>");
  expect(html).toContain("<p>page end</p></main>");
  expect(html).toContain("<p>body</p>");
  expect(html).not.toContain("lost");

  const missing = render(slots, { ...context, notFound: true });
  expect(missing).toContain("<p>lost</p>");
  expect(missing).not.toContain("<p>body</p>");
});
