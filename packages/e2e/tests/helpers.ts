import { expect, type Page } from "@playwright/test";

/**
 * Opens the publish dialog and confirms it, then waits for the "Published N pages" note. Every test shares one site, and a page another
 * test is still writing (an image whose alt text is not typed yet) blocks publishing, so a blocked dialog is closed and tried again.
 */
export async function publishAll(page: Page) {
  await expect(async () => {
    await page
      .getByRole("button", { name: /^Publish/ })
      .first()
      .click();
    const dialog = page.getByRole("dialog");
    const confirm = dialog.getByRole("button", { name: /^Publish/ });

    try {
      await expect(confirm).toBeEnabled({ timeout: 3000 });
    } catch (error) {
      await dialog.getByRole("button", { name: "Cancel" }).click();
      throw error;
    }
    await confirm.click();
  }).toPass({ timeout: 40_000 });

  await expect(page.getByText(/Published \d+ pages/)).toBeVisible();
}

/** A complete 1x1 PNG. Uploads are decoded to make their variants, so a hand-made header is not enough. */
export const PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
  "base64",
);
