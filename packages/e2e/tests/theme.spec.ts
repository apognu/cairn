import { readFileSync } from "node:fs";
import { expect, type Page, test } from "@playwright/test";
import { nameNewPage, PNG, publishAll, quiet } from "./helpers";

const SITE = "http://127.0.0.1:3001";

async function newPage(page: Page, title: string) {
  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, title);
  await page.getByRole("button", { name: title, exact: true }).click();
}

// Settings belong to the whole site and nothing else here depends on them, so they are left as this test sets them (putting them
// back would race with a second run of the same test).
test("changing the accent color, name and logo reaches every published page", async ({ page }) => {
  const tag = Math.random().toString(36).slice(2, 8);
  const [first, second] = [`accent-a-${tag}`, `accent-b-${tag}`];
  await newPage(page, first);
  await newPage(page, second);

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel("Site name").fill(`Halyard ${tag}`);
  await page.getByLabel("Accent color (hex)").fill("#aa3300");
  await page
    .getByLabel("Logo", { exact: true })
    .setInputFiles({ name: "logo.png", mimeType: "image/png", buffer: PNG });
  await expect(page.getByAltText("Current logo")).toBeVisible();
  await page.getByRole("button", { name: "Save theme" }).click();
  await expect(page.getByText("Saved. It goes live with the next publish.")).toBeVisible();
  await page.getByRole("button", { name: "Back to pages" }).click();

  await publishAll(page);

  for (const slug of [first, second]) {
    const html = readFileSync(`.e2e-data/site/${slug}/index.html`, "utf8");
    expect(html, slug).toContain("--accent-base:#aa3300");
    expect(html, slug).toContain(`Halyard ${tag}`);
    expect(html, slug).toContain('class="site-logo"');
  }

  const reader = await page.context().newPage();
  await quiet(reader);
  await reader.goto(`${SITE}/${second}/`);
  // Other tests' publishes reload open tabs of the site, which can land between the load and the read.
  await expect
    .poll(() =>
      reader
        .evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--accent").trim())
        .catch(() => ""),
    )
    .toBe("#aa3300");
});

test("a low-contrast palette shows a warning and cannot be saved", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(page.getByText("Text contrast passes WCAG AA")).toBeVisible();

  await page.getByLabel("Text", { exact: true }).fill("#cccccc");
  await expect(page.getByRole("alert")).toContainText("Text on the background in light mode");
  await expect(page.getByRole("button", { name: "Save theme" })).toBeDisabled();

  // The dark palette is checked as well, and fixing the color lifts the warning.
  await page.getByRole("tab", { name: "Dark" }).click();
  await page.getByLabel("Background", { exact: true }).fill("#eeeeee");
  await expect(page.getByRole("alert")).toContainText("dark mode");
  await page.getByRole("button", { name: "Discard" }).click();
  await expect(page.getByText("Text contrast passes WCAG AA")).toBeVisible();
});

test("header links, footer and per-mode colors reach the published site", async ({ page }) => {
  const tag = Math.random().toString(36).slice(2, 8);
  const slug = `brand-${tag}`;
  await newPage(page, slug);

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Add a link" }).first().click();
  await page.getByLabel("Header: label 1").fill("Status");
  await page.getByLabel("Header: address 1").fill("https://status.example.com");
  await page.getByLabel("Text", { exact: true }).fill("#101010");
  await page.getByLabel("Footer text").fill(`Made by ${tag}`);
  await page.getByRole("button", { name: "Add a column" }).click();
  await page.getByLabel("Column 1 title").fill("Product");
  await page.getByRole("button", { name: "Add a link" }).last().click();
  await page.getByLabel("Column 1: label 1").fill("Pricing");
  await page.getByLabel("Column 1: address 1").fill("/pricing/");
  await page.getByRole("button", { name: "Save theme" }).click();
  await expect(page.getByText("Saved. It goes live with the next publish.")).toBeVisible();
  await page.getByRole("button", { name: "Back to pages" }).click();

  await publishAll(page);

  const html = readFileSync(`.e2e-data/site/${slug}/index.html`, "utf8");
  expect(html).toContain('href="https://status.example.com"');
  expect(html).toContain(`Made by ${tag}`);
  expect(html).toContain('href="/pricing/"');
  expect(html).toContain(":root{--text:#101010;--text-body:#101010}");
});

test("custom CSS is loaded after the theme, in the editor and on the published site", async ({ page }) => {
  const tag = Math.random().toString(36).slice(2, 8);
  const slug = `css-${tag}`;
  await newPage(page, slug);

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel(/Stylesheet, loaded after the theme/).fill(`.doc-content { --probe-${tag}: 1; }`);
  await page.getByRole("button", { name: "Save theme" }).click();
  await expect(page.getByText("Saved. It goes live with the next publish.")).toBeVisible();
  await expect
    .poll(() => page.evaluate(() => document.querySelector("style#site-theme")?.textContent ?? ""))
    .toMatch(new RegExp(`--accent-base:#[0-9a-f]{6}\\}.*--probe-${tag}`));
  await page.getByRole("button", { name: "Back to pages" }).click();

  await publishAll(page);

  const html = readFileSync(`.e2e-data/site/${slug}/index.html`, "utf8");
  expect(html.indexOf(`--probe-${tag}`)).toBeGreaterThan(html.indexOf("--accent-base:"));
  expect(html.indexOf("--accent-base:")).toBeGreaterThan(html.indexOf('rel="stylesheet"'));

  // A stylesheet that could close its own element is refused with the reason.
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel(/Stylesheet, loaded after the theme/).fill("</style><script>alert(1)</script>");
  await expect(page.getByRole("alert")).toContainText("cannot contain </style");
  await expect(page.getByRole("button", { name: "Save theme" })).toBeDisabled();
});

test("the published site follows the system theme and has a toggle that is remembered", async ({ page, browser }) => {
  const slug = `theme-${Math.random().toString(36).slice(2, 8)}`;
  await newPage(page, slug);
  await publishAll(page);

  const background = (p: Page) => p.evaluate(() => getComputedStyle(document.body).backgroundColor);
  const dark = await (await browser.newContext({ colorScheme: "dark" })).newPage();
  await quiet(dark);

  await dark.goto(`${SITE}/${slug}/`);
  // Another test's publish reloads open tabs of the site, which can land between the load and the read, so reading is retried.
  const read = () => background(dark).catch(() => "");
  await expect.poll(read).toBe("rgb(18, 20, 22)");

  // Auto → Light: the choice beats the system setting, and survives a reload without a flash of the other theme.
  await dark.getByRole("button", { name: /Change theme/ }).click();
  await expect(dark.locator("html")).toHaveAttribute("data-theme", "light");
  await expect.poll(read).toBe("rgb(255, 255, 255)");
  await dark.reload({ waitUntil: "commit" });
  await expect(dark.locator("html")).toHaveAttribute("data-theme", "light");
});

test("pages are prefetched on hover and fade between each other", async ({ page, request }) => {
  const slug = `fast-${Math.random().toString(36).slice(2, 8)}`;
  await newPage(page, slug);
  await publishAll(page);

  const reader = await page.context().newPage();
  await quiet(reader);
  let href = "";
  await expect(async () => {
    await reader.goto(`${SITE}/${slug}/`);
    const other = reader.locator(`.site-nav > ul:not(.site-nav-sections) a:not([href="/${slug}/"])`).first();
    href = (await other.getAttribute("href")) ?? "";
    await other.hover();
    await expect(reader.locator(`link[rel="prefetch"][href$="${href}"]`)).toHaveCount(1, { timeout: 2000 });
  }).toPass();

  const css = await (
    await request.get(`${SITE}${await reader.locator('link[rel="stylesheet"][href*="site-"]').getAttribute("href")}`)
  ).text();
  expect(css).toContain("@view-transition");
  expect(await reader.locator('script[type="speculationrules"]').count()).toBe(1);
});
