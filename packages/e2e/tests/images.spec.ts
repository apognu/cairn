import { expect, test } from "@playwright/test";
import { editablePage, nameNewPage, PNG, slashCommand } from "./helpers";

test("uploading the same file twice stores it once", async ({ request }) => {
  // A file unique to this run, so the first upload really creates an asset.
  const unique = Buffer.concat([PNG, Buffer.from(Math.random().toString())]);
  const send = () =>
    request.post("/api/assets", { data: unique, headers: { "content-type": "application/octet-stream" } });

  const first = await send();
  const second = await send();

  expect(first.status()).toBe(201);
  expect(second.status()).toBe(200);
  expect((await first.json()).id).toBe((await second.json()).id);
});

test("only images are accepted", async ({ request }) => {
  const response = await request.post("/api/assets", {
    data: "<svg onload=alert(1)>",
    headers: { "content-type": "image/svg+xml" },
  });

  expect(response.status()).toBe(422);
});

test("an image can be inserted from the slash menu and given alt text", async ({ page, request }) => {
  const title = `Images ${Math.random().toString(36).slice(2, 8)}`;

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, title);
  await page.getByRole("button", { name: title, exact: true }).click();
  const id = new URL(page.url()).hash.replace("#page=", "");

  const editor = await editablePage(page);
  await editor.click();
  const chooser = page.waitForEvent("filechooser");
  await slashCommand(page, "image", "Image");
  await (await chooser).setFiles({ name: "dot.png", mimeType: "image/png", buffer: PNG });

  const image = editor.locator("figure img");
  await expect(image).toHaveAttribute("src", /^\/_a\/[0-9a-f-]{36}$/);
  await expect(image).toHaveJSProperty("naturalWidth", 1);

  await image.click();
  await page.getByLabel("Alt text").fill("A single pixel");
  await expect(image).toHaveAttribute("alt", "A single pixel");

  await expect(page.getByRole("status")).toHaveText("Saved");
  const draft = await (await request.get(`/api/pages/${id}/draft`)).json();
  expect(JSON.stringify(draft.doc)).toContain("A single pixel");
});

test("a picture that claims to be enormous is refused, and normal ones get WebP copies", async ({ request }) => {
  // The header of a 30000 x 30000 image: a few dozen bytes that would need gigabytes to decode.
  const bomb = Buffer.from(PNG);
  bomb.writeUInt32BE(30_000, 16);
  bomb.writeUInt32BE(30_000, 20);
  const refused = await request.post("/api/assets", {
    data: bomb,
    headers: { "content-type": "application/octet-stream" },
  });

  expect(refused.status()).toBe(422);
  expect((await refused.json()).message).toContain("pixels");

  const unique = Buffer.concat([PNG, Buffer.from(Math.random().toString())]);
  const accepted = await request.post("/api/assets", {
    data: unique,
    headers: { "content-type": "application/octet-stream" },
  });
  const asset = await accepted.json();

  expect(asset.variants.length).toBeGreaterThan(0);
  expect(asset.placeholder).toMatch(/^#[0-9a-f]{6}$/);
});
