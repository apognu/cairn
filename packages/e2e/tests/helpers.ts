import { readFileSync } from "node:fs";
import { type APIRequestContext, expect, type Locator, type Page } from "@playwright/test";

/**
 * Opens the publish dialog and confirms it, then waits for the "Published N pages" note. Every test shares one site, and a page another
 * test is still writing (an image whose alt text is not typed yet) blocks publishing, so a blocked dialog is closed and tried again.
 */
export async function publishAll(page: Page) {
  const published = page.getByText(/Published \d+ pages/);

  await expect(async () => {
    await page
      .getByRole("button", { name: /^Publish/ })
      .first()
      .click();
    const dialog = page.getByRole("dialog");
    const confirm = dialog.getByRole("button", { name: /^Publish/ });

    try {
      await expect(confirm).toBeEnabled({ timeout: 3000 });
      await confirm.click();
      // The site can also turn blocked between the check and the click (another test saved a page without alt text): the dialog then
      // shows the refusal, and the whole attempt is made again.
      await expect(published).toBeVisible({ timeout: 8000 });
    } catch (error) {
      await page.keyboard.press("Escape");
      throw error;
    }
  }).toPass({ timeout: 40_000 });
}

/** A complete 1x1 PNG. Uploads are decoded to make their variants, so a hand-made header is not enough. */
export const PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
  "base64",
);

/** Stops a tab of the published site from reloading itself when another test publishes (the dev server's live reload). */
export async function quiet(page: Page) {
  await page.route("**/__reload", (route) => route.abort());
}

export const SITE = "http://127.0.0.1:3001";

/**
 * Saves a page's draft through the API the way the editor does: holding the page's lock, which is taken for the save and given back.
 * The same user's editor tab, if one has the page open, loses its lock.
 */
export async function saveDraft(request: APIRequestContext, pageId: string, doc: unknown, base: string | null) {
  const lock = await request.post(`/api/pages/${pageId}/lock/take-over`, { data: { force: true } });
  expect(lock.ok(), await lock.text()).toBe(true);
  const { token } = await lock.json();

  const saved = await request.put(`/api/pages/${pageId}/draft`, {
    data: { doc, base_revision_id: base, lock_token: token },
  });
  expect(saved.ok(), await saved.text()).toBe(true);
  await request.delete(`/api/pages/${pageId}/lock?token=${token}`);
}

/**
 * Publishes a fixture document (`fixtures/<name>.json`) as a real page and returns its address on the site. The page is made through
 * the API, so the document goes through the same save, check, render and publish as anything an author writes. A publish that is
 * refused because another test has an unfinished page is tried again.
 */
export async function publishFixture(request: APIRequestContext, name: string): Promise<string> {
  const title = `fx ${name} ${Math.random().toString(36).slice(2, 8)}`;
  const doc = JSON.parse(readFileSync(new URL(`../../../fixtures/${name}.json`, import.meta.url), "utf8"));

  const page = await (await request.post("/api/pages", { data: { parent_id: null, title } })).json();
  await saveDraft(request, page.id, doc, null);

  await expect(async () => {
    const response = await request.post("/api/publish");
    expect(response.ok(), await response.text()).toBe(true);
  }).toPass({ timeout: 30_000 });

  return `${SITE}/${title.toLowerCase().replace(/\W+/g, "-")}/`;
}

/**
 * Types the title of a page being created (the "New page" or "Add a subpage" form is open) and waits for the editor to open it. The
 * editor always has a page open, so until the new one is there another page's editor is on screen, and typing would go into it.
 */
export async function nameNewPage(page: Page, title: string) {
  await page.getByLabel("New page title").fill(title);
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("crumbs")).toContainText(title);
}

/**
 * Runs a slash command at the cursor: types "/" and the query, waits until the menu highlights the item titled `title`, and picks it.
 * The menu follows the typing a moment later, and on a busy machine an Enter that comes first only starts a new line.
 */
export async function slashCommand(page: Page, query: string, title: string) {
  await page.keyboard.type(`/${query}`);
  await expect(page.getByRole("option", { selected: true }).locator("strong")).toHaveText(title);
  await page.keyboard.press("Enter");
}

/**
 * The open page's editor, once it can be typed in. Until the page's lock is taken the page shows read-only, and once it is the editor
 * replaces that view and puts the cursor at the start (`data-ready` then): a click, a key or a slash command before then lands in the
 * read-only view or is moved back to the start.
 */
export async function editablePage(page: Page): Promise<Locator> {
  const editor = page.locator('[data-ready] > .ProseMirror[contenteditable="true"]');
  await expect(editor).toBeVisible();
  return editor;
}

/**
 * Drags the tree row `title` onto the middle of `parent`'s row, which makes it a subpage, and waits until the server has the move. The
 * tree redraws as pages load, which can interrupt a drag, and the move is saved by a request that ends after the drop: the drag is
 * repeated until the move is there. With `alt`, the drop moves without keeping a redirect.
 */
export async function dragUnder(
  page: Page,
  request: APIRequestContext,
  title: string,
  parent: string,
  { alt = false }: { alt?: boolean } = {},
) {
  const byTitle = async (wanted: string) =>
    (await (await request.get("/api/pages")).json()).find((p: { title: string }) => p.title === wanted);
  const row = (name: string) => page.getByRole("button", { name, exact: true });

  await expect(async () => {
    if (alt) await page.keyboard.down("Alt");
    await row(title).dragTo(row(parent));
    if (alt) await page.keyboard.up("Alt");
    const parentId = (await byTitle(parent)).id;
    await expect.poll(async () => (await byTitle(title)).parent_id, { timeout: 3_000 }).toBe(parentId);
  }).toPass();
}
