import { expect, test } from "@playwright/test";

test("a callout and a code block can be inserted from the slash menu", async ({ page }) => {
  const title = `Slash ${Math.random().toString(36).slice(2, 8)}`;

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await page.getByLabel("New page title").fill(title);
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: title, exact: true }).click();

  const editor = page.locator(".ProseMirror");
  await editor.click();

  await page.keyboard.type("/callout");
  await expect(page.getByRole("option", { name: /Callout/ })).toBeVisible();
  await page.keyboard.press("Enter");
  await page.keyboard.type("Careful here");
  await expect(editor.locator("aside.callout")).toContainText("Careful here");

  // Leave the callout through the paragraph the editor keeps after the last block.
  await page.keyboard.press("ArrowDown");
  await page.keyboard.type("/code block");
  await page.keyboard.press("Enter");
  await expect(editor.locator("pre")).toBeVisible();

  await page.getByLabel("Language").fill("json");
  await page.getByLabel("File name").fill("config.json");
  await expect(editor.locator("pre[data-file-name='config.json'] code.language-json")).toHaveCount(1);
});

test("published code is highlighted", async ({ page }) => {
  await page.goto("/docs/article");

  await expect(page.locator("pre code.language-json span[class*='hl-']").first()).toBeVisible();
});

test("the bubble menu formats a selection", async ({ page }) => {
  const title = `Bubble ${Math.random().toString(36).slice(2, 8)}`;

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await page.getByLabel("New page title").fill(title);
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: title, exact: true }).click();

  const editor = page.locator(".ProseMirror");
  await editor.click();
  await page.keyboard.type("Make this bold");
  await expect(editor).toContainText("Make this bold");

  // The menu follows selection changes with a small delay, so a keyboard selection made too soon after typing may be missed: retry it.
  const toolbar = page.getByRole("toolbar", { name: "Text formatting" });
  await expect(async () => {
    await page.keyboard.press("End");
    await page.keyboard.press("Shift+Home");
    await expect(toolbar).toBeVisible({ timeout: 1000 });
  }).toPass();
  await page.getByRole("button", { name: "Bold" }).click();
  await expect(editor.locator("strong")).toHaveText("Make this bold");
});

test("a component's settings form edits the document", async ({ page, request }) => {
  const title = `Props ${Math.random().toString(36).slice(2, 8)}`;

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await page.getByLabel("New page title").fill(title);
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: title, exact: true }).click();
  const id = new URL(page.url()).hash.replace("#page=", "");

  const editor = page.locator(".ProseMirror");
  await editor.click();
  await page.keyboard.type("/callout");
  await page.keyboard.press("Enter");
  await page.keyboard.type("Settings test");

  const panel = page.getByRole("complementary", { name: "Component settings" });
  await expect(panel.getByRole("heading", { name: "Callout" })).toBeVisible();
  await panel.getByLabel("Type").selectOption("warning");
  await expect(editor.locator("aside.callout[data-tone='warning']")).toBeVisible();

  // The edit reaches the saved document, not just the screen.
  await expect(page.getByRole("status")).toHaveText("Saved");
  const draft = await (await request.get(`/api/pages/${id}/draft`)).json();
  expect(JSON.stringify(draft.doc)).toContain('"tone":"warning"');
});
