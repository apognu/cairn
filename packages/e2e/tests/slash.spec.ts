import { expect, test } from "@playwright/test";
import { editablePage, nameNewPage, publishFixture, quiet, saveDraft, slashCommand } from "./helpers";

test("a callout and a code block can be inserted from the slash menu", async ({ page }) => {
  const title = `Slash ${Math.random().toString(36).slice(2, 8)}`;

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, title);
  await page.getByRole("button", { name: title, exact: true }).click();

  const editor = await editablePage(page);
  await editor.click();

  await page.keyboard.type("/callout");
  await expect(page.getByRole("option", { name: /Callout/ })).toBeVisible();
  await page.keyboard.press("Enter");
  await page.keyboard.type("Careful here");
  await expect(editor.locator("aside.callout")).toContainText("Careful here");

  // Leave the callout through the paragraph the editor keeps after the last block.
  await page.keyboard.press("ArrowDown");
  await slashCommand(page, "code block", "Code block");
  await expect(editor.locator("pre")).toBeVisible();

  await page.getByLabel("Language").click();
  await page.getByLabel("Search languages").fill("json");
  await page.keyboard.press("Enter");
  await expect(page.getByLabel("Language")).toHaveText("JSON");
  await page.getByLabel("File name").fill("config.json");
  await expect(editor.locator("pre[data-file-name='config.json'] code.language-json")).toHaveCount(1);

  // The canvas colors code like the published page, once the server has highlighted it.
  await editor.locator("pre").click();
  await page.keyboard.type('{ "ok": true }');
  await expect(editor.locator("pre .code-token", { hasText: '"ok"' })).toBeVisible();
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

  const editor = await editablePage(page);
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

  const editor = await editablePage(page);
  await editor.click();
  await slashCommand(page, "callout", "Callout");
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

  const editor = await editablePage(page);
  await editor.click();
  await slashCommand(page, "feature tabs", "Feature tabs");

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

  const editor = await editablePage(page);
  await editor.click();
  await slashCommand(page, "cards", "Cards");

  const cardTitle = editor.locator(".feature-title .editable");
  await expect(cardTitle).toHaveText("Card");
  await cardTitle.click();
  await page.keyboard.press("Control+A");
  await page.keyboard.type("Quick start");
  await page.keyboard.press("Enter");
  await expect(cardTitle).toHaveText("Quick start");

  // The same props the settings form edits.
  await editor.locator(".feature-grid").click();
  await expect(page.getByLabel("Title", { exact: true }).first()).toHaveValue("Quick start");
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

  const editor = await editablePage(page);
  await editor.click();
  await slashCommand(page, "feature tabs", "Feature tabs");
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

test("steps are added with Enter, and removed or merged with Backspace", async ({ page }) => {
  const title = `Steps ${Math.random().toString(36).slice(2, 8)}`;

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, title);
  await page.getByRole("button", { name: title, exact: true }).click();

  const editor = await editablePage(page);
  await editor.click();
  await slashCommand(page, "steps", "Steps");

  // Each step's lines, the first being its title.
  const steps = editor.locator("ol.steps > li.step");
  const lines = () => steps.evaluateAll((items) => items.map((li) => [...li.children].map((p) => p.textContent)));

  // The cursor starts on the second step's body. Enter on an empty last line leaves the block, as in a list.
  await page.keyboard.type("Body");
  await page.keyboard.press("Enter");
  await page.keyboard.press("Enter");
  await page.keyboard.type("After the steps");
  await expect.poll(lines).toEqual([
    ["First step", ""],
    ["Second step", "Body"],
  ]);
  await expect(editor.locator("ol.steps + p")).toHaveText("After the steps");

  // Enter at the end of a title starts the next step, after the body. As with Home below, ProseMirror reads the moved cursor a moment
  // later.
  await editor.getByText("Second step").click();
  await page.keyboard.press("End");
  await page.waitForTimeout(100);
  await page.keyboard.press("Enter");
  await page.keyboard.type("Third");
  await expect.poll(lines).toEqual([["First step", ""], ["Second step", "Body"], ["Third"]]);

  // Backspace at the start of a title makes the step a line of the one before. ProseMirror reads the cursor moved by Home a moment
  // later, as it does for a person's keys.
  await page.keyboard.press("Home");
  await page.waitForTimeout(100);
  await page.keyboard.press("Backspace");
  await expect.poll(lines).toEqual([
    ["First step", ""],
    ["Second step", "Body", "Third"],
  ]);

  // Backspace removes an empty step again.
  await editor.getByText("First step").click();
  await page.keyboard.press("End");
  await page.waitForTimeout(100);
  await page.keyboard.press("Enter");
  await expect(steps).toHaveCount(3);
  await page.keyboard.press("Backspace");
  await expect(steps).toHaveCount(2);

  // Enter on an empty step leaves the block, as in a list.
  await editor.getByText("Second step").click();
  await page.keyboard.press("End");
  await page.waitForTimeout(100);
  await page.keyboard.press("Enter");
  await page.keyboard.press("Enter");
  await page.keyboard.type("After an empty step");
  await expect(steps).toHaveCount(2);
  await expect(editor.locator("ol.steps + p")).toHaveText("After an empty step");
});

test("an empty last line in a tab leaves the tabs, even with a block right below them", async ({ page, request }) => {
  const title = `Tabs exit ${Math.random().toString(36).slice(2, 8)}`;
  const created = await (await request.post("/api/pages", { data: { parent_id: null, title } })).json();
  const text = (value: string) => [{ type: "text", text: value }];
  await saveDraft(
    request,
    created.id,
    {
      type: "doc",
      content: [
        {
          type: "tabs",
          content: [
            {
              type: "tabPanel",
              attrs: { label: "Linux" },
              content: [{ type: "paragraph", content: text("Use apt.") }],
            },
            {
              type: "tabPanel",
              attrs: { label: "macOS" },
              content: [{ type: "paragraph", content: text("Use Homebrew.") }],
            },
          ],
        },
        { type: "callout", attrs: { tone: "note" }, content: [{ type: "paragraph", content: text("Right below.") }] },
      ],
    },
    null,
  );

  await page.goto(`/#page=${created.id}`);
  const editor = await editablePage(page);
  await editor.getByText("Use apt.").click();
  // ProseMirror reads the cursor moved by End a moment later, as it does for a person's keys.
  await page.keyboard.press("End");
  await page.waitForTimeout(100);
  await page.keyboard.press("Enter");
  await page.keyboard.press("Enter");
  await page.keyboard.type("Between the blocks");

  await expect(editor.locator(".tabs-panel").first().locator("p")).toHaveText(["Use apt."]);
  await expect(editor.locator(".tabs + p")).toHaveText("Between the blocks");
  await expect(editor.locator(".tabs + p + aside.callout")).toContainText("Right below.");
});
