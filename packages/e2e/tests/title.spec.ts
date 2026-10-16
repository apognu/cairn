import { expect, test } from "@playwright/test";

test("the title above the page renames it on Enter, with the redirect choice", async ({ page, request }) => {
  const title = `Title ${Math.random().toString(36).slice(2, 8)}`;
  const created = await (await request.post("/api/pages", { data: { parent_id: null, title } })).json();
  await page.goto(`/#page=${created.id}`);
  const field = page.getByLabel("Page title");
  await expect(field).toHaveValue(title);
  const keep = page.getByLabel("Keep old URL as redirect");
  await expect(keep).toBeHidden();

  // Escape puts the title back.
  await field.fill(`${title} draft`);
  await expect(keep).toBeChecked();
  await field.press("Escape");
  await expect(field).toHaveValue(title);
  await expect(keep).toBeHidden();

  // Leaving the field to untick the box keeps the change; Enter in the field then renames.
  await field.fill(`${title} renamed`);
  await keep.uncheck();
  await expect(field).toHaveValue(`${title} renamed`);
  const sent = page.waitForRequest((r) => r.method() === "POST" && r.url().endsWith(`/api/pages/${created.id}`));
  await field.press("Enter");
  expect((await sent).postDataJSON()).toMatchObject({ title: `${title} renamed`, redirect: false });

  await expect(page.getByRole("button", { name: `${title} renamed`, exact: true })).toBeVisible();
  await expect(keep).toBeHidden();
  await expect(page.getByLabel("Description")).toBeFocused();
});
