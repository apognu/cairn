import { expect, type Page, test } from "@playwright/test";
import { editablePage, saveDraft } from "./helpers";

const paragraph = (text?: string) => ({ type: "paragraph", content: text ? [{ type: "text", text }] : undefined });

async function openPage(page: Page, content: unknown[]) {
  const title = `Paste ${Math.random().toString(36).slice(2, 8)}`;
  const created = await (await page.request.post("/api/pages", { data: { parent_id: null, title } })).json();
  await saveDraft(page.request, created.id, { type: "doc", content }, null);
  await page.goto(`/#page=${created.id}`);
  return editablePage(page);
}

/** Pastes plain text where the cursor is, as the clipboard of a browser's address bar holds it. */
async function paste(page: Page, text: string) {
  await page.evaluate((value) => {
    const data = new DataTransfer();
    data.setData("text/plain", value);
    document.activeElement?.dispatchEvent(
      new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }),
    );
  }, text);
}

/** Selects from `from` to `to` (ProseMirror positions) in the page editor, as a mouse or the keyboard would. */
const select = (page: Page, from: number, to = from) =>
  page.evaluate(
    ([a, b]) =>
      (
        document.querySelector("[data-ready] > .ProseMirror") as unknown as {
          editor: { commands: { setTextSelection: (range: { from: number; to: number }) => void } };
        }
      ).editor.commands.setTextSelection({ from: a, to: b }),
    [from, to],
  );

const json = (page: Page) =>
  page.evaluate(() =>
    (
      document.querySelector("[data-ready] > .ProseMirror") as unknown as { editor: { getJSON: () => unknown } }
    ).editor.getJSON(),
  );

test("an address pasted over words links them, and undo takes the link off", async ({ page }) => {
  const editor = await openPage(page, [paragraph("Read the guide first.")]);
  // "the", after "Read " at the start of the first paragraph.
  await select(page, 6, 9);
  await paste(page, "https://example.com/guide");

  const link = editor.locator('a[href="https://example.com/guide"]');
  await expect(link).toHaveText("the");
  await expect(editor).toContainText("Read the guide first.");

  await page.keyboard.press("Control+z");
  await expect(link).toHaveCount(0);
  await expect(editor).toContainText("Read the guide first.");
});

test("a video address on an empty line becomes its player, and undo brings the line back", async ({ page }) => {
  const editor = await openPage(page, [paragraph("Intro"), paragraph()]);
  // The empty paragraph after "Intro".
  await select(page, 8);
  await paste(page, "https://youtu.be/dQw4w9WgXcQ");

  await expect(editor.locator("iframe[src='https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ']")).toBeVisible();
  expect(JSON.stringify(await json(page))).toContain('"url":"https://youtu.be/dQw4w9WgXcQ"');

  await page.keyboard.press("Control+z");
  await expect(editor.locator("iframe")).toHaveCount(0);
});

test("a video address pasted in a sentence stays text", async ({ page }) => {
  const editor = await openPage(page, [paragraph("Watch ")]);
  await select(page, 7);
  await paste(page, "https://youtu.be/dQw4w9WgXcQ");

  await expect(editor).toContainText("Watch https://youtu.be/dQw4w9WgXcQ");
  await expect(editor.locator("iframe")).toHaveCount(0);
});
