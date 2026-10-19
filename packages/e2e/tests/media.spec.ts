import { expect, test } from "@playwright/test";
import { PNG, saveDraft } from "./helpers";

/** A file unique to this run, so uploading it really creates an asset. */
function uniquePng(): Buffer {
  return Buffer.concat([PNG, Buffer.from(Math.random().toString())]);
}

test("an image added in Media is named, found by its title, and cannot be deleted while a page shows it", async ({
  page,
  request,
}) => {
  const run = Math.random().toString(36).slice(2, 8);

  await page.goto("/#settings=media");
  await expect(page.getByRole("searchbox", { name: "Search images" })).toBeFocused();
  await page
    .getByLabel("Upload", { exact: true })
    .setInputFiles({ name: `shot-${run}.png`, mimeType: "image/png", buffer: uniquePng() });

  // A single upload opens its details, where it is named; the drawer's name changes with the title, so it is found as the one open.
  const drawer = page.getByRole("dialog");
  await expect(drawer.getByRole("heading", { name: `shot-${run}.png` })).toBeVisible();
  await drawer.getByLabel("Title").fill(`Login screen ${run}`);
  await drawer.getByLabel("Title").press("Enter");
  await expect(drawer.getByText("Saved.")).toBeVisible();
  await expect(drawer.getByText("No page uses it yet.")).toBeVisible();
  await drawer.getByRole("button", { name: "Close image details" }).click();

  await page.getByRole("searchbox", { name: "Search images" }).fill(`login SCREEN ${run}`);
  const images = page.getByRole("list", { name: "Images" }).getByRole("button");
  await expect(images).toHaveCount(1);
  await expect(images).toHaveText(`Login screen ${run}`);

  const [asset] = await (await request.get(`/api/assets?q=${run}`)).json();
  expect(asset.kept).toBe(true);
  const created = await (
    await request.post("/api/pages", { data: { parent_id: null, title: `Uses ${run}`, before: null } })
  ).json();
  await saveDraft(
    request,
    created.id,
    { type: "doc", content: [{ type: "image", attrs: { assetId: asset.id, alt: "Login" } }] },
    null,
  );

  await images.click();
  const named = page.getByRole("dialog", { name: `Login screen ${run}` });
  await expect(named.getByRole("list", { name: "Used on" })).toHaveText(`Uses ${run}`);
  await expect(named.getByRole("button", { name: "Delete" })).toHaveCount(0);
  expect((await request.delete(`/api/assets/${asset.id}`)).status()).toBe(409);
});

test("an unused image is taken out of the library", async ({ page, request }) => {
  const run = Math.random().toString(36).slice(2, 8);
  const upload = await request.post(`/api/assets?name=spare-${run}.png&keep=true`, {
    data: uniquePng(),
    headers: { "content-type": "application/octet-stream" },
  });
  const asset = await upload.json();

  await page.goto("/#settings=media");
  await page.getByRole("searchbox", { name: "Search images" }).fill(`spare-${run}`);
  await page
    .getByRole("list", { name: "Images" })
    .getByRole("button", { name: `spare-${run}.png` })
    .click();
  const drawer = page.getByRole("dialog", { name: `spare-${run}.png` });
  await drawer.getByRole("button", { name: "Delete" }).click();
  await drawer.getByRole("group", { name: "Confirm delete" }).getByRole("button", { name: "Delete" }).click();

  await expect(drawer).toBeHidden();
  await expect.poll(async () => (await (await request.get(`/api/assets/${asset.id}`)).json()).asset.kept).toBe(false);
});
