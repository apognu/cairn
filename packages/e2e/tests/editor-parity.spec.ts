import { expect, test } from "@playwright/test";
import pixelmatch from "pixelmatch";
import { PNG } from "pngjs";

// Anti-aliasing at fractional positions (rounded corners) can differ by a few pixels between the two layouts, so allow a handful.
function differingPixels(a: Buffer, b: Buffer): number {
  const left = PNG.sync.read(a);
  const right = PNG.sync.read(b);
  if (left.width !== right.width || left.height !== right.height) return Number.POSITIVE_INFINITY;

  return pixelmatch(left.data, right.data, undefined, left.width, left.height, { threshold: 0.1 });
}

// The editor (read-only Tiptap) and the published page (V8 render, hydrated) must look the same for every fixture.
for (const scheme of ["light", "dark"] as const) {
  for (const fixture of ["demo", "article", "features"]) {
    test(`${fixture} looks the same in the editor and when published (${scheme})`, async ({ browser }) => {
      const options = { viewport: { width: 1280, height: 900 }, colorScheme: scheme } as const;
      const published = await browser.newPage(options);
      const editor = await browser.newPage(options);
      const problems: string[] = [];
      for (const page of [published, editor]) {
        page.on("pageerror", (e) => problems.push(e.message));
        page.on("console", (m) => m.type() === "error" && !m.text().includes("404") && problems.push(m.text()));
      }

      await published.goto(`/docs/${fixture}`);
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
      expect(
        differingPixels(await editor.locator(".site").screenshot(), await published.locator(".site").screenshot()),
      ).toBeLessThan(50);
      expect(problems).toEqual([]);
    });
  }
}
