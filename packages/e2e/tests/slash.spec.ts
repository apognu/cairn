import { expect, test } from "@playwright/test";
import { nameNewPage, publishFixture, quiet } from "./helpers";

test("a callout and a code block can be inserted from the slash menu", async ({ page }) => {
  const title = `Slash ${Math.random().toString(36).slice(2, 8)}`;

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, title);
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

  await page.getByLabel("Language").selectOption("json");
  await page.getByLabel("File name").fill("config.json");
  await expect(editor.locator("pre[data-file-name='config.json'] code.language-json")).toHaveCount(1);
});

test("published code is highlighted", async ({ page, request }) => {
  await quiet(page);
  await page.goto(await publishFixture(request, "article"));

  await expect(page.locator("pre code.language-json span[class*='hl-']").first()).toBeVisible();
});

test("the bubble menu formats a selection", async ({ page }) => {
  const title = `Bubble ${Math.random().toString(36).slice(2, 8)}`;

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, title);
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
  await nameNewPage(page, title);
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

test("feature tabs are edited through their settings form", async ({ page }) => {
  const title = `Features ${Math.random().toString(36).slice(2, 8)}`;

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, title);
  await page.getByRole("button", { name: title, exact: true }).click();

  const editor = page.locator(".ProseMirror");
  await editor.click();
  await page.keyboard.type("/feature tabs");
  await page.keyboard.press("Enter");

  // A selected atom shows its form.
  await editor.locator(".feature-tabs").click();
  const panel = page.getByRole("complementary", { name: "Component settings" });
  await expect(panel.getByRole("heading", { name: "Feature tabs" })).toBeVisible();

  await panel.getByLabel("Title").first().fill("Rolling deploys");
  await expect(editor.locator(".feature-card")).toContainText("Rolling deploys");

  const future = "2099-12-31";
  await panel.getByLabel("New until").first().fill(future);
  await expect(editor.locator(".feature-new")).toBeVisible();
  await panel.getByLabel("New until").first().fill("2000-01-01");
  await expect(editor.locator(".feature-new")).toHaveCount(0);
});

test("a card title is written on the canvas and lands in the page settings", async ({ page }) => {
  const title = `Editable ${Math.random().toString(36).slice(2, 8)}`;

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, title);
  await page.getByRole("button", { name: title, exact: true }).click();

  const editor = page.locator(".ProseMirror");
  await editor.click();
  await page.keyboard.type("/cards");
  await page.keyboard.press("Enter");

  const cardTitle = editor.locator(".feature-title .editable");
  await expect(cardTitle).toHaveText("Card");
  await cardTitle.click();
  await page.keyboard.press("Control+A");
  await page.keyboard.type("Quick start");
  await page.keyboard.press("Enter");
  await expect(cardTitle).toHaveText("Quick start");

  // The same props the settings form edits.
  await editor.locator(".feature-grid").click();
  await expect(page.getByLabel("Title").first()).toHaveValue("Quick start");
});

test("on a narrower window, block settings open from their button, Alt+Enter or a double click", async ({
  browser,
}) => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const title = `Drawer ${Math.random().toString(36).slice(2, 8)}`;

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, title);
  await page.getByRole("button", { name: title, exact: true }).click();

  const editor = page.locator(".ProseMirror");
  await editor.click();
  await page.keyboard.type("/feature tabs");
  await page.keyboard.press("Enter");
  await editor.locator(".feature-tabs").click();

  // The button names the block, and the drawer gives the cursor back to the text when it closes.
  const open = page.getByRole("button", { name: "Feature tabs settings" });
  await expect(open).toHaveAttribute("aria-keyshortcuts", "Alt+Enter");
  await page.screenshot({ path: "test-results/block-settings-light.png" });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.screenshot({ path: "test-results/block-settings-dark.png" });
  await page.emulateMedia({ colorScheme: "light" });
  // The column is out of the way until asked for, then it opens itself: the page actions on top, the block's settings under them.
  await expect(page.getByRole("dialog", { name: "Page and block settings" })).toBeHidden();
  await open.click();
  const drawer = page.getByRole("dialog", { name: "Page and block settings" });
  await expect(
    drawer.getByRole("navigation", { name: "Page actions" }).getByRole("button", { name: "History" }),
  ).toBeVisible();
  await expect(drawer.getByRole("heading", { name: "Feature tabs" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(drawer).toBeHidden();
  await expect(page.getByRole("dialog", { name: "Page and block settings" })).toBeHidden();

  await editor.locator(".feature-tabs").click();
  await page.keyboard.press("Alt+Enter");
  await expect(drawer).toBeVisible();
  await page.keyboard.press("Escape");

  await editor.locator(".feature-tabs").dblclick();
  await expect(drawer).toBeVisible();
});
