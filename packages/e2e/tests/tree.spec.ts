import { expect, test } from "@playwright/test";

test("nested pages survive a reload", async ({ page }) => {
  const suffix = Math.random().toString(36).slice(2, 8);
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

test("a page can be dragged under a new parent and an external link added", async ({ page, request }) => {
  const suffix = Math.random().toString(36).slice(2, 8);
  const [section, other, mover] = [`Section ${suffix}`, `Other ${suffix}`, `Mover ${suffix}`];

  await page.goto("/");
  for (const title of [section, other, mover]) {
    await page.getByRole("button", { name: "New page" }).click();
    await page.getByLabel("New page title").fill(title);
    await page.keyboard.press("Enter");
    await expect(page.getByRole("button", { name: title, exact: true })).toBeVisible();
  }

  // Dropped on the middle of a row, the page becomes its child. The tree redraws as the pages load, which can interrupt a drag, so it is
  // repeated until the server has the move.
  const byTitle = async (title: string) =>
    (await (await request.get("/api/pages")).json()).find((p: { title: string }) => p.title === title);
  await expect(async () => {
    await page
      .getByRole("button", { name: mover, exact: true })
      .dragTo(page.getByRole("button", { name: section, exact: true }));
    expect((await byTitle(mover)).parent_id).toBe((await byTitle(section)).id);
  }).toPass();
  expect((await byTitle(other)).parent_id).toBeNull();

  // The link is added from the section's menu and lives under it.
  await page.getByRole("button", { name: section, exact: true }).hover();
  await page.getByLabel(`More actions for ${section}`).click();
  await page.getByRole("button", { name: `Add an external link under ${section}` }).click();
  await page.getByLabel("New link title").fill(`Status ${suffix}`);
  await page.getByLabel("Link URL").fill("https://status.example.com");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("button", { name: `Status ${suffix}`, exact: true })).toBeVisible();

  const links = (await (await request.get("/api/pages")).json()).filter((p: { kind: string }) => p.kind === "link");
  expect(links.at(-1).external_url).toBe("https://status.example.com");
});
