import { expect, type Page } from "@playwright/test";

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
