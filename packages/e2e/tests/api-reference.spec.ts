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

test("a link to one operation of an API page is checked against the build's spec", async ({ page, request }) => {
  const byTitle = async (title: string) =>
    (await (await request.get("/api/pages")).json()).find((p: { title: string }) => p.title === title);
  const guide = await (await request.post("/api/pages", { data: { parent_id: null, title: "Guide" } })).json();
  const api = await byTitle("Orders API");

  // The operations a link can point at are those the sources gave at the last check or publish.
  const operations = await (await request.get(`/api/pages/${api.id}/operations`)).json();
  expect(operations.map((o: { anchor: string }) => o.anchor)).toContain("tag/orders/POST/orders");

  await page.goto(`/#page=${guide.id}`);
  const editor = page.locator(".ProseMirror").first();
  await editor.click();
  await page.keyboard.type("create an order");
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.press("ControlOrMeta+k");
  const picker = page.getByRole("dialog", { name: "Link to" });
  await picker.getByRole("combobox").fill("Orders API");
  await picker
    .getByRole("button", { name: /Orders API/ })
    .first()
    .click();
  await picker.getByRole("button", { name: /POST \/orders/ }).click();

  await expect(async () => {
    const draft = await (await request.get(`/api/pages/${guide.id}/draft`)).json();
    expect(JSON.stringify(draft.doc)).toContain('"anchor":"tag/orders/POST/orders"');
  }).toPass();
  const first = await request.post("/api/publish");
  expect(first.ok(), await first.text()).toBe(true);

  // Upstream drops the operation: the link is reported before anything is published.
  await request.put(
    `${GITHUB}/__state/file?path=${encodeURIComponent("/acme/orders/releases/download/v1.0.0/openapi.json")}`,
    {
      data: JSON.stringify({
        openapi: "3.0.3",
        info: { title: "Orders", version: "1.0.1" },
        paths: { "/orders": { get: { tags: ["Orders"], responses: { "200": { description: "ok" } } } } },
      }),
    },
  );
  const blocked = await request.post("/api/publish");
  expect(blocked.status()).toBe(422);
  expect(JSON.stringify(await blocked.json())).toContain("POST /orders");
});
