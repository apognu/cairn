import { expect, test } from "@playwright/test";
import { dragUnder, nameNewPage, publishAll } from "./helpers";

const SITE = "http://127.0.0.1:3001";

test("a move adds a redirect, an Alt-drop does not, and the publish dialog flags the lost URL", async ({
  page,
  request,
}) => {
  const suffix = Math.random().toString(36).slice(2, 8);
  const [section, kept, dropped] = [`Section ${suffix}`, `Kept ${suffix}`, `Dropped ${suffix}`];
  const slug = (title: string) => title.toLowerCase().replace(/\W+/g, "-");

  await page.goto("/");
  for (const title of [section, kept, dropped]) {
    await page.getByRole("button", { name: "New page" }).click();
    await nameNewPage(page, title);
    await expect(page.getByRole("button", { name: title, exact: true })).toBeVisible();
  }
  await publishAll(page);

  await dragUnder(page, request, kept, section);
  await expect(page.getByRole("img", { name: "Unpublished changes" }).first()).toBeVisible();
  await dragUnder(page, request, dropped, section, { alt: true });

  // The dialog says which old URL will 404, and which one is only redirected.
  await page
    .getByRole("button", { name: /^Publish/ })
    .first()
    .click();
  const dialog = page.getByRole("dialog");
  await expect(dialog.getByText("will be a 404 after publish")).toBeVisible();
  await expect(dialog.getByText(`/${slug(dropped)}/`).first()).toBeVisible();
  await expect(dialog.getByText(`adds redirect from`)).toBeVisible();
  await dialog.getByRole("button", { name: "Cancel" }).click();

  await publishAll(page);

  const redirected = await request.get(`${SITE}/${slug(kept)}/`, { maxRedirects: 0 });
  expect(redirected.status()).toBe(301);
  expect(redirected.headers().location).toBe(`/${slug(section)}/${slug(kept)}/`);
  expect((await request.get(`${SITE}/${slug(dropped)}/`, { maxRedirects: 0 })).status()).toBe(404);

  // The Redirects page lists the one the move made, and accepts and removes manual ones.
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Redirects", exact: true }).click();
  const table = page.getByRole("table", { name: "Redirects" });
  await expect(table.getByRole("row", { name: new RegExp(`/${slug(kept)}/.*moved`) })).toBeVisible();

  await page.getByLabel("From", { exact: true }).fill(`/manual-${suffix}`);
  await page.getByLabel("Or to a URL").fill("https://example.com/elsewhere");
  await page.getByRole("button", { name: "Add redirect" }).click();
  await expect(table.getByRole("row", { name: new RegExp(`/manual-${suffix}/.*manual`) })).toBeVisible();

  // A path a page already has is refused, naming the page.
  await page.getByLabel("From", { exact: true }).fill(`/${slug(section)}`);
  await page.getByLabel("Or to a URL").fill("https://example.com");
  await page.getByRole("button", { name: "Add redirect" }).click();
  await expect(page.getByRole("alert")).toContainText(`“${section}”`);

  await page.getByRole("button", { name: `Delete redirect from /manual-${suffix}/` }).click();
  await expect(table.getByText(`/manual-${suffix}/`)).toHaveCount(0);
});
