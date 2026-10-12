import { expect, test } from "@playwright/test";

test("nested pages survive a reload", async ({ page }) => {
  const suffix = Date.now().toString(36);
  const names = [`Guide ${suffix}`, `Install ${suffix}`, `Linux ${suffix}`];

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await page.getByLabel("New page title").fill(names[0]);
  await page.keyboard.press("Enter");
  await expect(page.getByRole("button", { name: names[0], exact: true })).toBeVisible();

  for (const [parent, child] of [
    [names[0], names[1]],
    [names[1], names[2]],
  ]) {
    await page.getByRole("button", { name: parent, exact: true }).hover();
    await page.getByRole("button", { name: `Add a subpage to ${parent}` }).click();
    await page.getByLabel("New page title").fill(child);
    await page.keyboard.press("Enter");
    await expect(page.getByRole("button", { name: child, exact: true })).toBeVisible();
  }

  await page.reload();

  for (const name of names) {
    await expect(page.getByRole("button", { name, exact: true })).toBeVisible();
  }
});
