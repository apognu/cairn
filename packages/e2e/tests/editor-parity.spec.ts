import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import pixelmatch from "pixelmatch";
import { PNG } from "pngjs";
import { publishFixture, quiet } from "./helpers";

// The share of pixels that differ. Text and rounded corners are anti-aliased differently when the two pages put the content at different
// fractional positions (the published one has a breadcrumb and a date above it), so a small share is allowed; a missing or misplaced
// element differs by far more.
function differingShare(a: Buffer, b: Buffer): number {
  const left = PNG.sync.read(a);
  const right = PNG.sync.read(b);
  if (left.width !== right.width || left.height !== right.height) return 1;

  return (
    pixelmatch(left.data, right.data, undefined, left.width, left.height, { threshold: 0.1 }) /
    (left.width * left.height)
  );
}

// The editor (read-only Tiptap) and the published page (V8 render, hydrated) must look the same for every fixture.
for (const scheme of ["light", "dark"] as const) {
  for (const fixture of ["tabs", "article", "features", "steps", "cards", "filetree", "codegroup"]) {
    test(`${fixture} looks the same in the editor and when published (${scheme})`, async ({ browser, request }) => {
      const options = { viewport: { width: 1440, height: 900 }, colorScheme: scheme } as const;
      const published = await browser.newPage(options);
      const editor = await browser.newPage(options);
      const problems: string[] = [];
      for (const page of [published, editor]) {
        page.on("pageerror", (e) => problems.push(e.message));
        page.on(
          "console",
          (m) =>
            m.type() === "error" &&
            !m.text().includes("404") &&
            !m.text().includes("ERR_FAILED") &&
            problems.push(m.text()),
        );
      }

      // The same document twice: published as a real page, and handed to the editor's read-only view.
      const doc = readFileSync(new URL(`../../../fixtures/${fixture}.json`, import.meta.url), "utf8");
      await editor.addInitScript(
        ([name, text]) => window.localStorage.setItem(`fixture:${name}`, text),
        [fixture, doc],
      );
      await quiet(published);
      await published.goto(await publishFixture(request, fixture));
      await editor.goto(`/#fixture=${fixture}`);
      await expect(editor.locator(".ProseMirror")).toBeVisible();
      await expect(published.locator(".doc-content")).toBeVisible();

      // Components that enhance themselves once scripts run (feature tabs) must be done before comparing.
      if (fixture === "features") {
        await expect(published.locator(".feature-tabs[data-ready]")).toBeVisible();
        await expect(editor.locator(".feature-tabs[data-ready]")).toBeVisible();
      }

      // Text drawn before the bundled fonts arrive would be measured with the fallback.
      await Promise.all([published, editor].map((p) => p.evaluate(() => document.fonts.ready)));

      // Syntax coloring is added at publish only: flatten it, leaving the same text, before comparing.
      await published.evaluate(() => {
        for (const code of document.querySelectorAll("pre code")) code.replaceChildren(code.textContent ?? "");
      });

      expect(await editor.locator(".doc-content").innerText()).toBe(
        await published.locator(".doc-content").innerText(),
      );
      // The content area: the site chrome around it differs on purpose (the published page has a sidebar, breadcrumbs and a date).
      expect(
        differingShare(
          await editor.locator(".doc-content").screenshot(),
          await published.locator(".doc-content").screenshot(),
        ),
      ).toBeLessThan(0.005);
      expect(problems).toEqual([]);
    });
  }
}
