import { readFileSync } from "node:fs";
import { type APIRequestContext, expect, type Page } from "@playwright/test";

/**
 * Opens the publish dialog and confirms it, then waits for the "Published N pages" note. Every test shares one site, and a page another
 * test is still writing (an image whose alt text is not typed yet) blocks publishing, so a blocked dialog is closed and tried again.
 */
export async function publishAll(page: Page) {
  const published = page.getByText(/Published \d+ pages/);

  await expect(async () => {
    await page
      .getByRole("button", { name: /^Publish/ })
      .first()
      .click();
    const dialog = page.getByRole("dialog");
    const confirm = dialog.getByRole("button", { name: /^Publish/ });

    try {
      await expect(confirm).toBeEnabled({ timeout: 3000 });
      await confirm.click();
      // The site can also turn blocked between the check and the click (another test saved a page without alt text): the dialog then
      // shows the refusal, and the whole attempt is made again.
      await expect(published).toBeVisible({ timeout: 8000 });
    } catch (error) {
      await page.keyboard.press("Escape");
      throw error;
    }
  }).toPass({ timeout: 40_000 });
}

/** A complete 1x1 PNG. Uploads are decoded to make their variants, so a hand-made header is not enough. */
export const PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
  "base64",
);

/** Stops a tab of the published site from reloading itself when another test publishes (the dev server's live reload). */
export async function quiet(page: Page) {
  await page.route("**/__reload", (route) => route.abort());
}

export const SITE = "http://127.0.0.1:3001";

/**
 * Publishes a fixture document (`fixtures/<name>.json`) as a real page and returns its address on the site. The page is made through
 * the API, so the document goes through the same save, check, render and publish as anything an author writes. A publish that is
 * refused because another test has an unfinished page is tried again.
 */
export async function publishFixture(request: APIRequestContext, name: string): Promise<string> {
  const title = `fx ${name} ${Math.random().toString(36).slice(2, 8)}`;
  const doc = JSON.parse(readFileSync(new URL(`../../../fixtures/${name}.json`, import.meta.url), "utf8"));

  const page = await (await request.post("/api/pages", { data: { parent_id: null, title } })).json();
  const saved = await request.put(`/api/pages/${page.id}/draft`, { data: { doc, base_revision_id: null } });
  expect(saved.ok(), await saved.text()).toBe(true);

  await expect(async () => {
    const response = await request.post("/api/publish");
    expect(response.ok(), await response.text()).toBe(true);
  }).toPass({ timeout: 30_000 });

  return `${SITE}/${title.toLowerCase().replace(/\W+/g, "-")}/`;
}
