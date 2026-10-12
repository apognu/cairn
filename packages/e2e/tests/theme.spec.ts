import { readFileSync } from "node:fs";
import { expect, type Page, test } from "@playwright/test";

const SITE = "http://127.0.0.1:3001";
const PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg==",
  "base64",
);

async function newPage(page: Page, title: string) {
  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await page.getByLabel("New page title").fill(title);
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: title, exact: true }).click();
}

// Settings belong to the whole site and nothing else here depends on them, so they are left as this test sets them (putting them
// back would race with a second run of the same test).
test("changing the accent color, name and logo reaches every published page", async ({ page }) => {
  const tag = Math.random().toString(36).slice(2, 8);
  const [first, second] = [`accent-a-${tag}`, `accent-b-${tag}`];
  await newPage(page, first);
  await newPage(page, second);

  await page.getByRole("button", { name: "Settings" }).click();
  await page.getByLabel("Site name").fill(`Halyard ${tag}`);
  await page.getByLabel("Accent color (hex)").fill("#aa3300");
  await page.getByLabel("Logo").setInputFiles({ name: "logo.png", mimeType: "image/png", buffer: PNG });
  await expect(page.getByAltText("Current logo")).toBeVisible();
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByRole("dialog")).toBeHidden();

  await page.getByRole("button", { name: "Publish", exact: true }).click();
  await expect(page.getByText(/Published \d+ pages/)).toBeVisible();

  for (const slug of [first, second]) {
    const html = readFileSync(`.e2e-data/site/${slug}/index.html`, "utf8");
    expect(html, slug).toContain("--accent-base:#aa3300");
    expect(html, slug).toContain(`Halyard ${tag}`);
    expect(html, slug).toContain('class="site-logo"');
  }

  const reader = await page.context().newPage();
  await reader.goto(`${SITE}/${second}/`);
  expect(
    await reader.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--accent").trim()),
  ).toBe("#aa3300");
});

test("the published site follows the system theme and has a toggle that is remembered", async ({ page, browser }) => {
  const slug = `theme-${Math.random().toString(36).slice(2, 8)}`;
  await newPage(page, slug);
  await page.getByRole("button", { name: "Publish", exact: true }).click();
  await expect(page.getByText(/Published \d+ pages/)).toBeVisible();

  const background = (p: Page) => p.evaluate(() => getComputedStyle(document.body).backgroundColor);
  const dark = await (await browser.newContext({ colorScheme: "dark" })).newPage();

  await dark.goto(`${SITE}/${slug}/`);
  expect(await background(dark)).toBe("rgb(18, 20, 22)");

  // Auto → Light: the choice beats the system setting, and survives a reload without a flash of the other theme.
  await dark.getByRole("button", { name: /Change theme/ }).click();
  await expect(dark.locator("html")).toHaveAttribute("data-theme", "light");
  expect(await background(dark)).toBe("rgb(255, 255, 255)");
  await dark.reload({ waitUntil: "commit" });
  await expect(dark.locator("html")).toHaveAttribute("data-theme", "light");
});

test("pages are prefetched on hover and fade between each other", async ({ page, request }) => {
  const slug = `fast-${Math.random().toString(36).slice(2, 8)}`;
  await newPage(page, slug);
  await page.getByRole("button", { name: "Publish", exact: true }).click();
  await expect(page.getByText(/Published \d+ pages/)).toBeVisible();

  const reader = await page.context().newPage();
  await reader.goto(`${SITE}/${slug}/`);
  const other = reader.locator(`.site-nav a:not([href="/${slug}/"])`).first();
  const href = await other.getAttribute("href");
  await other.hover();
  await expect(reader.locator(`link[rel="prefetch"][href$="${href}"]`)).toHaveCount(1);

  const css = await (
    await request.get(`${SITE}${await reader.locator('link[href*="site-"]').getAttribute("href")}`)
  ).text();
  expect(css).toContain("@view-transition");
  expect(await reader.locator('script[type="speculationrules"]').count()).toBe(1);
});
