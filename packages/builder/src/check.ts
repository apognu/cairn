import { runInNewContext } from "node:vm";
import { sampleProps } from "@cairn/sdk";
import { BuildError, type Collected } from "./collect";

type Ssr = { render: (json: string) => string };

// A color written into the markup stays the same in dark mode; components style themselves with the theme's custom properties, or with
// the Tailwind utilities made from them. An inline style or an arbitrary utility (`text-[#00f]`) is a fixed color.
const FIXED_COLOR = /(style="[^"]*|class="[^"]*\[)(#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i;

const SITE = {
  name: "Test site",
  footer: { text: "Footer", columns: [{ title: "Links", links: [{ label: "Home", url: "/" }] }] },
};

/**
 * Renders every component with sample props, and every slot in a whole page, through the built `ssr.js` in a context with no browser
 * globals, as V8 will. Throws a BuildError naming the first one that fails.
 */
export function testRender(script: string, found: Collected): void {
  const ssr = runInNewContext(`${script}\n;SSR`, {}) as Ssr;
  const render = (input: object) => ssr.render(JSON.stringify(input));

  for (const { file, definition } of found.components) {
    const label = `${file}: ${definition.name}`;
    let doc: object;
    try {
      doc = { type: "doc", content: [{ type: definition.name, attrs: sampleProps(definition) }] };
    } catch (error) {
      throw new BuildError(`${label}: cannot make sample props from its schema: ${(error as Error).message}`);
    }

    let html: string;
    try {
      const problems = JSON.parse(render({ doc, validate: true })) as string[];
      if (problems.length > 0) throw new Error(problems.join("; "));
      html = render({ doc, context: { site: SITE, page: { id: "sample", title: "Sample" } } });
    } catch (error) {
      throw new BuildError(`${label} fails to render with its sample props: ${(error as Error).message}`);
    }

    if (FIXED_COLOR.test(html))
      throw new BuildError(
        `${label} writes a fixed color into its markup, which does not follow the theme in dark mode; use the theme's custom properties (var(--text), var(--accent), …) in a stylesheet, or its utilities (text-text, bg-accent-soft, …)`,
      );
  }

  const doc = { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "Sample" }] }] };
  for (const [name, notFound] of [
    ["layout", false],
    ["not-found page", true],
  ] as const) {
    try {
      render({ doc, context: { site: SITE, page: { id: "sample", title: "Sample" }, notFound } });
    } catch (error) {
      const slots = found.slots.map((s) => `${s.definition.name} (${s.file})`).join(", ");
      throw new BuildError(
        `the ${name} fails to render with the slots ${slots || "(none)"}: ${(error as Error).message}`,
      );
    }
  }
}
