import { type APIRequestContext, expect, test } from "@playwright/test";
import { editablePage, nameNewPage, PNG, saveDraft, slashCommand } from "./helpers";

/** A file unique to this run, so uploading it really creates an asset. */
function uniquePng(): Buffer {
  return Buffer.concat([PNG, Buffer.from(Math.random().toString())]);
}

async function addToLibrary(request: APIRequestContext, name: string): Promise<{ id: string }> {
  const upload = await request.post(`/api/assets?name=${name}&keep=true`, {
    data: uniquePng(),
    headers: { "content-type": "application/octet-stream" },
  });
  return upload.json();
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
  const asset = await addToLibrary(request, `spare-${run}.png`);

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

test("an image already uploaded is inserted from the library, and replacing it is one undo step", async ({
  page,
  request,
}) => {
  const run = Math.random().toString(36).slice(2, 8);
  const first = await addToLibrary(request, `first-${run}.png`);
  const second = await addToLibrary(request, `second-${run}.png`);

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, `Picked ${run}`);
  await page.getByRole("button", { name: `Picked ${run}`, exact: true }).click();
  const editor = await editablePage(page);
  await editor.click();
  await slashCommand(page, "from library", "Image from library");

  const picker = page.getByRole("dialog", { name: "Choose an image" });
  await picker.getByRole("searchbox", { name: "Search images" }).fill(`first-${run}`);
  await picker.getByRole("button", { name: `first-${run}.png` }).click();
  await picker.getByRole("button", { name: "Insert", exact: true }).click();
  const image = editor.locator("figure img");
  await expect(image).toHaveAttribute("src", `/_a/${first.id}`);

  await image.click();
  await page.getByRole("button", { name: "Replace…" }).click();
  const replacing = page.getByRole("dialog", { name: "Replace the image" });
  await replacing.getByRole("searchbox", { name: "Search images" }).fill(`second-${run}`);
  await replacing.getByRole("button", { name: `second-${run}.png` }).dblclick();
  await expect(replacing).toBeHidden();
  await expect(image).toHaveAttribute("src", `/_a/${second.id}`);

  await page.keyboard.press("ControlOrMeta+z");
  await expect(image).toHaveAttribute("src", `/_a/${first.id}`);
  // Every test publishes everything pending: an image without alt text would hold up the others.
  await image.click();
  await page.getByLabel("Alt text").fill("The first image");
  await expect(page.getByRole("status")).toHaveText("Saved");
  // Nothing was uploaded again.
  expect(await (await request.get(`/api/assets?q=${run}`)).json()).toHaveLength(2);
});
