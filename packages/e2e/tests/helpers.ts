import { expect, type Page } from "@playwright/test";

/** Opens the publish dialog and confirms it, then waits for the "Published N pages" note. */
export async function publishAll(page: Page) {
  await page
    .getByRole("button", { name: /^Publish/ })
    .first()
    .click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: /^Publish/ })
    .click();
  await expect(page.getByText(/Published \d+ pages/)).toBeVisible();
}
