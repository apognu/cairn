import { expect, test } from "@playwright/test";
import { nameNewPage } from "./helpers";

const suffix = () => Math.random().toString(36).slice(2, 8);

async function newPage(page: import("@playwright/test").Page, title: string) {
  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, title);
  await page.getByRole("button", { name: title, exact: true }).click();
  await expect(page.locator(".ProseMirror")).toBeVisible();
}

test("unsaved text comes back after a reload", async ({ page }) => {
  // The server refuses every save, so only the browser copy holds the text.
  await page.route("**/api/pages/*/draft", (route) =>
    route.request().method() === "PUT" ? route.abort() : route.continue(),
  );

  await newPage(page, `Autosave ${suffix()}`);
  await page.locator(".ProseMirror").click();
  await page.keyboard.type("Typed but never saved");
  await expect(page.getByRole("status")).toContainText("Not saved");
  await page.waitForTimeout(500);

  await page.unroute("**/api/pages/*/draft");
  await page.reload();
  await expect(page.locator(".ProseMirror")).toContainText("Typed but never saved");

  // With the server back, the restored text is saved.
  await expect(page.getByRole("status")).toHaveText("Saved");
});

test("saved text survives a reload", async ({ page }) => {
  await newPage(page, `Saved ${suffix()}`);
  await page.locator(".ProseMirror").click();
  await page.keyboard.type("Kept on the server");
  await expect(page.getByRole("status")).toHaveText("Saved");

  await page.evaluate(() => indexedDB.deleteDatabase("cairn"));
  await page.reload();
  await expect(page.locator(".ProseMirror")).toContainText("Kept on the server");
});

test("a save based on an old revision is refused with a conflict", async ({ request }) => {
  const page = await (await request.post("/api/pages", { data: { title: `Conflict ${suffix()}` } })).json();
  const doc = { type: "doc", content: [{ type: "paragraph" }] };
  const { token } = await (await request.post(`/api/pages/${page.id}/lock`)).json();
  const save = (base: string | null) =>
    request.put(`/api/pages/${page.id}/draft`, { data: { doc, base_revision_id: base, lock_token: token } });

  const first = await (await save(null)).json();
  expect((await save(first.base_revision_id)).status()).toBe(200);
  expect((await save(first.base_revision_id)).status()).toBe(409);
});
