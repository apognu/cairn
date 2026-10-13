import { expect, type Page, test } from "@playwright/test";
import { nameNewPage, publishAll, quiet } from "./helpers";

const SITE = "http://127.0.0.1:3001";

async function newPage(page: Page, title: string, text: string) {
  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, title);
  await page.getByRole("button", { name: title, exact: true }).click();
  await page.locator(".ProseMirror").click();
  await page.keyboard.type(text);
  await expect(page.getByRole("status")).toHaveText("Saved");
}

async function publish(page: Page) {
  await publishAll(page);
}

test("the published site serves pages, redirects, the 404 page and search", async ({ page, browser }) => {
  const slug = `site-${Math.random().toString(36).slice(2, 8)}`;
  await newPage(page, slug, "Zebrafish migration notes");
  await publish(page);

  const reader = await browser.newPage();
  await quiet(reader);

  // A page, at its clean URL, with and without the trailing slash.
  await reader.goto(`${SITE}/${slug}/`);
  await expect(reader.locator(".doc-content")).toContainText("Zebrafish migration notes");
  await reader.goto(`${SITE}/${slug}`);
  await expect(reader).toHaveURL(`${SITE}/${slug}/`);

  // The page shows when it was last updated and offers its Markdown version, which is served next to it.
  await expect(reader.getByText(/Last updated \d+ \w+ \d{4}/)).toBeVisible();
  await expect(reader.getByRole("link", { name: "View as Markdown" })).toHaveAttribute("href", `/${slug}.md`);
  const markdown = await reader.request.get(`${SITE}/${slug}.md`);
  expect(markdown.status()).toBe(200);
  expect(await markdown.text()).toContain(`# ${slug}\n\nZebrafish migration notes`);

  // The site root follows the redirect rule to a page.
  await reader.goto(`${SITE}/`);
  await expect(reader.locator(".doc-title")).toBeVisible();
  expect(new URL(reader.url()).pathname).not.toBe("/");

  // An unknown address gets the 404 page, with a 404 status.
  const missing = await reader.goto(`${SITE}/nothing/here/`);
  expect(missing?.status()).toBe(404);
  await expect(reader.getByText("There is no page at this address")).toBeVisible();

  // Search finds the page by its text. Other tests publish at the same time and the index files are replaced with each release,
  // so a search that started on the previous release is tried again from a fresh page load.
  await expect(async () => {
    await reader.goto(`${SITE}/${slug}/`);
    await reader.getByPlaceholder("Search").fill("zebrafish");
    await expect(reader.locator(".pagefind-ui__result-link", { hasText: slug })).toBeVisible({ timeout: 2000 });
  }).toPass();
});

test("an open tab of the site reloads after a publish", async ({ page, browser }) => {
  const slug = `live-${Math.random().toString(36).slice(2, 8)}`;
  await newPage(page, slug, "Version one");
  await publish(page);

  const reader = await browser.newPage();
  const connected = reader.waitForRequest("**/__reload");
  await reader.goto(`${SITE}/${slug}/`);
  await expect(reader.locator(".doc-content")).toContainText("Version one");
  // The page listens for releases once its event stream is open.
  await connected;
  await reader.waitForTimeout(300);

  await page.locator(".ProseMirror").click();
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.type("Version two");
  await expect(page.getByRole("status")).toHaveText("Saved");
  await publish(page);

  await expect(reader.locator(".doc-content")).toContainText("Version two");
});
