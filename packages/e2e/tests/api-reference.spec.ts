import { expect, test } from "@playwright/test";

const GITHUB = "http://127.0.0.1:3390";
const SITE = "http://127.0.0.1:3301";

const github = (repo: string, path: string, asset?: string, prefix = "") => ({
  type: "github",
  repo,
  path,
  asset: asset ?? null,
  git_ref: { type: "latest_release" },
  prefix,
});

test.describe.configure({ mode: "serial" });

test("an API page shows the merged operations in Scalar inside the site layout, in light and dark", async ({
  page,
  request,
}) => {
  await request.put(`${GITHUB}/__state/reset`);
  const created = await (
    await request.post("/api/pages", { data: { parent_id: null, title: "Orders API", kind: "api" } })
  ).json();
  const saved = await request.put(`/api/pages/${created.id}/api-config`, {
    data: { sources: [github("acme/orders", "", "openapi.json"), github("acme/billing", "api/billing.json")] },
  });
  expect(saved.ok(), await saved.text()).toBe(true);
  const published = await request.post("/api/publish");
  expect(published.ok(), await published.text()).toBe(true);

  await page.goto(`${SITE}/orders-api/`);
  const scalar = page.locator(".api-reference .scalar-api-reference");
  await expect(scalar).toBeVisible({ timeout: 20_000 });
  // Scalar draws the sections as they come into view: the first is enough to know both sources were merged and read.
  await expect(scalar.getByText("listInvoices summary").first()).toBeVisible();
  // The site layout is still around it.
  await expect(page.locator("header.site-header")).toBeVisible();
  // The site's sidebar only links to the page; the tags are in Scalar's own sidebar.
  await expect(page.locator("nav.site-nav").getByRole("link", { name: "Orders API" })).toBeVisible();
  await expect(page.locator("nav.site-nav").getByRole("link", { name: "Invoices" })).toHaveCount(0);
  await expect(scalar.getByRole("link", { name: "Invoices" }).first()).toBeVisible();

  // Scalar takes its colors from the site: its background is the site's, in light and in dark mode.
  const backgrounds = () =>
    page.evaluate(() => {
      const root = document.querySelector(".scalar-api-reference");
      if (!root) return null;
      return {
        scalar: getComputedStyle(root).getPropertyValue("--scalar-background-1").trim(),
        site: getComputedStyle(document.documentElement).getPropertyValue("--bg").trim(),
      };
    });
  const light = await backgrounds();
  expect(light?.scalar).toBe(light?.site);

  await page.emulateMedia({ colorScheme: "dark" });
  await expect(async () => {
    const dark = await backgrounds();
    expect(dark?.scalar).toBe(dark?.site);
    expect(dark?.site).not.toBe(light?.site);
  }).toPass();
  await expect(page.locator("body")).toHaveClass(/dark-mode/);

  // The header toggle wins over the system setting: system (dark) to light.
  await page.getByRole("button", { name: /Theme: Auto/ }).click();
  await expect(page.locator("body")).toHaveClass(/light-mode/);

  // Search finds an operation, and a click anywhere on its row, not only on the title, lands on it.
  await page.getByRole("button", { name: /Theme/ }).first().waitFor();
  await page.locator(".pagefind-ui__search-input").fill("createOrder");
  const row = page.locator(".pagefind-ui__result-nested").first();
  await expect(row).toBeVisible();
  const box = await row.boundingBox();
  await page.mouse.click((box?.x ?? 0) + (box?.width ?? 0) - 20, (box?.y ?? 0) + (box?.height ?? 0) / 2);
  await expect(async () => {
    expect(await page.evaluate(() => location.hash)).toBe("#tag/orders/POST/orders");
    expect(await page.evaluate(() => scrollY)).toBeGreaterThan(500);
  }).toPass();
});
