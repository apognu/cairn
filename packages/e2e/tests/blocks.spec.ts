import { expect, type Page, test } from "@playwright/test";
import { saveDraft } from "./helpers";

const text = (value: string) => [{ type: "text", text: value }];

async function openPage(page: Page, request: import("@playwright/test").APIRequestContext) {
  const created = await (
    await request.post("/api/pages", {
      data: { parent_id: null, title: `Blocks ${Math.random().toString(36).slice(2, 8)}` },
    })
  ).json();
  const doc = {
    type: "doc",
    content: [
      { type: "heading", attrs: { level: 2, id: "alpha-id" }, content: text("Alpha") },
      { type: "paragraph", attrs: { id: "one-id" }, content: text("First paragraph") },
      { type: "paragraph", attrs: { id: "two-id" }, content: text("Second paragraph") },
    ],
  };
  await saveDraft(request, created.id, doc, null);
  await page.goto(`/#page=${created.id}`);
  // Read-only until the page's lock is taken; then the editor puts the cursor at the start, which would undo a click made before.
  const editor = page.locator('.ProseMirror[contenteditable="true"]');
  await expect(editor).toContainText("Second paragraph");
  await expect(editor).toBeFocused();
  return created.id as string;
}

const order = (page: Page) => page.locator(".ProseMirror > :is(h2, p):not(.phantom-line)").allInnerTexts();

test("a block dragged by its handle moves, keeping its ID", async ({ page, request }) => {
  const id = await openPage(page, request);

  await page.locator(".ProseMirror > h2").hover();
  const handle = page.locator(".block-handle");
  await expect(handle).toBeVisible();
  const from = await handle.boundingBox();
  const last = await page.getByText("Second paragraph").boundingBox();
  if (!from || !last) throw new Error("blocks not laid out");

  // Dropped on the right half of the last paragraph: ProseMirror puts the block after it. In steps, as a hand would: `dragTo` jumps
  // there and the drop is lost.
  await page.mouse.move(from.x + 10, from.y + 12);
  await page.mouse.down();
  await page.mouse.move(last.x + last.width - 10, last.y + last.height / 2, { steps: 10 });
  await page.mouse.up();

  await expect.poll(() => order(page)).toEqual(["First paragraph", "Second paragraph", "Alpha"]);
  await expect(page.getByRole("status")).toHaveText("Saved");
  const draft = await (await request.get(`/api/pages/${id}/draft`)).json();
  const blocks = draft.doc.content.filter((node: { content?: unknown }) => node.content);
  expect(blocks.map((node: { attrs: { id: string } }) => node.attrs.id)).toEqual(["one-id", "two-id", "alpha-id"]);
});

test("Alt+Shift+Up and Down move the block at the cursor", async ({ page, request }) => {
  await openPage(page, request);

  await page.getByText("Second paragraph").click();
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
    .toBe("Second paragraph");
  await page.keyboard.press("Alt+Shift+ArrowUp");
  await page.keyboard.press("Alt+Shift+ArrowUp");
  await expect.poll(() => order(page)).toEqual(["Second paragraph", "Alpha", "First paragraph"]);

  // The cursor went with the block: typing lands in it.
  await page.keyboard.type("!");
  await page.keyboard.press("Alt+Shift+ArrowDown");
  await expect.poll(() => order(page)).toEqual(["Alpha", "Second paragraph!", "First paragraph"]);
});
