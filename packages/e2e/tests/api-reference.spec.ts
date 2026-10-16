import { type APIRequestContext, expect, test } from "@playwright/test";
import { editablePage } from "./helpers";

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
  const editor = await editablePage(page);
  await editor.click();
  await page.keyboard.type("create an order");
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.press("ControlOrMeta+k");
  const picker = page.getByRole("dialog", { name: "Link to" });
  // An operation is found by what it does, listed with the page it belongs to.
  await picker.getByRole("combobox").fill("POST /orders");
  await picker.getByRole("button", { name: /^POST \/orders Orders API/ }).click();

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

  // The broken link would block every publish after this one.
  await request.delete(`/api/pages/${guide.id}`);
});

const putFile = (request: APIRequestContext, path: string, body: unknown) =>
  request.put(`${GITHUB}/__state/file?path=${encodeURIComponent(path)}`, { data: JSON.stringify(body) });

const orders = (version: string, operationId: string, path = "/orders") => ({
  openapi: "3.0.3",
  info: { title: "Orders", version },
  paths: {
    [path]: {
      get: {
        operationId,
        tags: ["Orders"],
        summary: `${operationId} summary`,
        responses: { "200": { description: "ok" } },
      },
    },
  },
});

test("a duplicate operation fails the publish naming both sources, and a prefix fixes it", async ({ request }) => {
  await request.put(`${GITHUB}/__state/reset`);
  await request.put(`${GITHUB}/__state/latest/acme/dup`, { data: "v1" });
  await putFile(request, "/acme/dup/v1/dup.json", orders("1", "listOrders", "/dup"));

  const created = await (
    await request.post("/api/pages", { data: { parent_id: null, title: "Dup API", kind: "api" } })
  ).json();
  const config = (prefix: string) => ({
    sources: [github("acme/orders", "", "openapi.json"), github("acme/dup", "dup.json", undefined, prefix)],
  });
  // Both sources define the operation ID `listOrders`.
  await request.put(`/api/pages/${created.id}/api-config`, { data: config("") });

  const failed = await request.post("/api/publish");
  expect(failed.status()).toBe(422);
  const text = JSON.stringify(await failed.json());
  expect(text).toContain("operationId listOrders");
  expect(text).toContain("acme/orders openapi.json (v1.0.0)");
  expect(text).toContain("acme/dup dup.json (v1)");

  const fixed = await request.put(`/api/pages/${created.id}/api-config`, { data: config("Dup") });
  expect(fixed.ok()).toBe(true);
  const published = await request.post("/api/publish");
  expect(published.ok(), await published.text()).toBe(true);

  await request.delete(`/api/pages/${created.id}`);
});

test("a new upstream release is picked up by the next publish, with nothing done in the editor", async ({
  request,
}) => {
  await request.put(`${GITHUB}/__state/reset`);
  const specUrl = async () => {
    const html = await (await request.get(`${SITE}/orders-api/`)).text();
    return html.match(/\/_api\/orders-api\.\w+\.json/)?.[0] ?? "";
  };

  expect((await request.post("/api/publish")).ok()).toBe(true);
  const before = await specUrl();
  expect(before).not.toBe("");
  expect(await (await request.get(`${SITE}${before}`)).text()).not.toContain("listOrdersV2");

  await request.put(`${GITHUB}/__state/latest/acme/orders`, { data: "v1.1.0" });
  await putFile(request, "/acme/orders/releases/download/v1.1.0/openapi.json", orders("1.1.0", "listOrdersV2"));
  expect((await request.post("/api/publish")).ok()).toBe(true);

  const after = await specUrl();
  expect(after).not.toBe(before);
  expect(await (await request.get(`${SITE}${after}`)).text()).toContain("listOrdersV2");
});
