import { readFileSync } from "node:fs";
import { expect, type Page, test } from "@playwright/test";
import { PNG, publishAll, quiet } from "./helpers";

const SITE = "http://127.0.0.1:3001";

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

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel("Site name").fill(`Halyard ${tag}`);
  await page.getByLabel("Accent color (hex)").fill("#aa3300");
  await page.getByLabel("Logo").setInputFiles({ name: "logo.png", mimeType: "image/png", buffer: PNG });
  await expect(page.getByAltText("Current logo")).toBeVisible();
  await page.getByRole("button", { name: "Save", exact: true }).click();
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
  let href = "";
  // A publish by another test reloads open tabs of the site, so the whole look-and-hover is attempted again if the page was replaced.
  await expect(async () => {
    await reader.goto(`${SITE}/${slug}/`);
    const other = reader.locator(`.site-nav a:not([href="/${slug}/"])`).first();
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
