import { expect, test } from "@playwright/test";

// The editor (read-only Tiptap) and the published page (V8 render, hydrated) must look the same for every fixture.
for (const scheme of ["light", "dark"] as const) {
  for (const fixture of ["demo", "article"]) {
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

      expect(await editor.locator(".doc-content").innerText()).toBe(
        await published.locator(".doc-content").innerText(),
      );
      expect(await editor.locator(".site").screenshot()).toEqual(await published.locator(".site").screenshot());
      expect(problems).toEqual([]);
    });
  }
}
