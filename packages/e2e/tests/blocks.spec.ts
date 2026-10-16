import { type APIRequestContext, expect, type Locator, type Page, test } from "@playwright/test";
import { saveDraft } from "./helpers";

const text = (value: string) => [{ type: "text", text: value }];
const paragraph = (value: string, id?: string) => ({ type: "paragraph", attrs: { id }, content: text(value) });

async function openPage(page: Page, request: APIRequestContext) {
  const title = `Blocks ${Math.random().toString(36).slice(2, 8)}`;
  const created = await (await request.post("/api/pages", { data: { parent_id: null, title } })).json();
  const doc = {
    type: "doc",
    content: [
      { type: "heading", attrs: { level: 2, id: "alpha-id" }, content: text("Alpha") },
      paragraph("First paragraph", "one-id"),
      {
        type: "cards",
        attrs: { columns: "2", cards: [{ icon: "rocket", title: "Quickstart", description: "Start here" }] },
      },
      {
        type: "tabs",
        content: [
          { type: "tabPanel", attrs: { label: "Linux" }, content: [paragraph("Use the package manager.")] },
          { type: "tabPanel", attrs: { label: "macOS" }, content: [paragraph("Use Homebrew.")] },
        ],
      },
      paragraph("Last paragraph", "last-id"),
    ],
  };
  await saveDraft(request, created.id, doc, null);
  await page.goto(`/#page=${created.id}`);
  // Read-only until the page's lock is taken; then the editor puts the cursor at the start, which would undo a click made before.
  const editor = page.locator('.ProseMirror[contenteditable="true"]');
  await expect(editor).toContainText("Last paragraph");
  await expect(editor).toBeFocused();
  return created.id as string;
}

/** The top-level blocks as the editor holds them: a textblock by its text, anything else by its type. Empty lines are left out. */
function blocks(page: Page) {
  return page.evaluate(() => {
    const { editor } = document.querySelector(".ProseMirror") as unknown as {
      editor: { getJSON: () => { content: { type: string; content?: { text?: string }[] }[] } };
    };
    return editor
      .getJSON()
      .content.filter((node) => node.type !== "paragraph" || node.content?.length)
      .map((node) =>
        node.type === "paragraph" || node.type === "heading" ? (node.content?.[0]?.text ?? "") : node.type,
      );
  });
}

/** Drags `block` by its handle and lets go at `y`, in steps as a hand would: `dragTo` jumps there and the browser loses the drop. */
async function drag(page: Page, block: Locator, y: number) {
  await block.hover();
  const handle = page.locator(".block-handle");
  await expect(handle).toBeVisible();
  const from = await handle.boundingBox();
  const editor = await page.locator(".ProseMirror").boundingBox();
  if (!from || !editor) throw new Error("not laid out");

  await page.mouse.move(from.x + 10, from.y + 12);
  await page.mouse.down();
  await page.mouse.move(editor.x + editor.width / 2, y, { steps: 10 });
  await expect(page.locator(".block-drop-line")).toBeVisible();
  await page.mouse.up();
  await expect(page.locator(".block-drop-line")).toBeHidden();
}

test("a component dragged by its handle moves, and is not copied", async ({ page, request }) => {
  await openPage(page, request);

  const last = await page.getByText("Last paragraph").boundingBox();
  if (!last) throw new Error("not laid out");
  // On the lower half of the last block: after it.
  await drag(page, page.locator(".ProseMirror > .react-renderer").first(), last.y + last.height - 2);

  await expect.poll(() => blocks(page)).toEqual(["Alpha", "First paragraph", "tabs", "Last paragraph", "cards"]);
});

test("a block with content inside moves whole, to the top", async ({ page, request }) => {
  await openPage(page, request);

  const alpha = await page.locator(".ProseMirror > h2").boundingBox();
  if (!alpha) throw new Error("not laid out");
  await drag(page, page.getByText("Use the package manager."), alpha.y + 2);

  await expect.poll(() => blocks(page)).toEqual(["tabs", "Alpha", "First paragraph", "cards", "Last paragraph"]);
});

test("a heading keeps its ID when moved", async ({ page, request }) => {
  const id = await openPage(page, request);

  const first = await page.getByText("First paragraph").boundingBox();
  if (!first) throw new Error("not laid out");
  await drag(page, page.locator(".ProseMirror > h2"), first.y + first.height - 2);

  await expect.poll(() => blocks(page)).toEqual(["First paragraph", "Alpha", "cards", "tabs", "Last paragraph"]);
  await expect(page.getByRole("status")).toHaveText("Saved");
  const draft = await (await request.get(`/api/pages/${id}/draft`)).json();
  const heading = draft.doc.content.find((node: { type: string }) => node.type === "heading");
  expect(heading.attrs.id).toBe("alpha-id");
});

test("Alt+Shift+Up and Down move the block at the cursor", async ({ page, request }) => {
  await openPage(page, request);

  await page.getByText("Last paragraph").click();
  await page.keyboard.press("End");
  // ProseMirror reads the caret the browser placed a moment later; a shortcut pressed before would move the block it was in before.
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            document.querySelector(".ProseMirror") as unknown as {
              editor: { state: { selection: { $from: { parent: { textContent: string } } } } };
            }
          ).editor.state.selection.$from.parent.textContent,
      ),
    )
    .toBe("Last paragraph");
  await page.keyboard.press("Alt+Shift+ArrowUp");
  await page.keyboard.press("Alt+Shift+ArrowUp");
  await expect.poll(() => blocks(page)).toEqual(["Alpha", "First paragraph", "Last paragraph", "cards", "tabs"]);

  // The cursor went with the block: typing lands in it.
  await page.keyboard.type("!");
  await page.keyboard.press("Alt+Shift+ArrowDown");
  await expect.poll(() => blocks(page)).toEqual(["Alpha", "First paragraph", "cards", "Last paragraph!", "tabs"]);
});
