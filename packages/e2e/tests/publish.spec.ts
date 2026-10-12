import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";

test("publishing writes one page per page, with its text", async ({ page }) => {
  const slug = `pub-${Math.random().toString(36).slice(2, 8)}`;

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await page.getByLabel("New page title").fill(slug);
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: slug, exact: true }).click();
  await page.locator(".ProseMirror").click();
  await page.keyboard.type("Hello from the editor");
  await expect(page.getByRole("status")).toHaveText("Saved");

  await page.getByRole("button", { name: "Publish" }).click();
  await expect(page.getByText(/Published \d+ pages/)).toBeVisible();

  const html = readFileSync(`.e2e-data/site/${slug}/index.html`, "utf8");
  expect(html).toContain("Hello from the editor");
  expect(html).toContain('<h1 class="doc-title">');
});
