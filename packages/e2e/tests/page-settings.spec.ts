import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import { publishAll } from "./helpers";

const SITE = "http://127.0.0.1:3001";

test("an alias used by another page is refused naming it, and a custom URL redirects the old one", async ({
  page,
  request,
}) => {
  const suffix = Math.random().toString(36).slice(2, 8);
  const [first, second] = [`Rollbacks ${suffix}`, `Webhooks ${suffix}`];
  const slug = (title: string) => title.toLowerCase().replace(/\W+/g, "-");

  await page.goto("/");
  for (const title of [first, second]) {
    await page.getByRole("button", { name: "New page" }).click();
    await page.getByLabel("New page title").fill(title);
    await page.keyboard.press("Enter");
    await expect(page.getByRole("button", { name: title, exact: true })).toBeVisible();
  }

  await page.getByRole("button", { name: second, exact: true }).click();
  await page.getByRole("button", { name: "Page settings" }).click();
  const drawer = page.getByRole("dialog", { name: "Page settings" });

  // Another page's URL is not available as an alias.
  await drawer.getByLabel("New alias").fill(`/${slug(first)}`);
  await drawer.getByRole("button", { name: "Add" }).click();
  await expect(drawer.getByRole("alert")).toContainText(`“${first}”`);

  // A custom URL keeps the old one as an alias, and the site serves both.
  await drawer.getByLabel("Custom URL", { exact: true }).check();
  await drawer.getByLabel("Custom URL", { exact: true }).last().fill(`/hooks-${suffix}`);
  await drawer.getByRole("button", { name: "Save" }).click();
  await expect(drawer.getByRole("list", { name: "Aliases" })).toContainText(`/${slug(second)}/`);
  await drawer.getByRole("button", { name: "Close page settings" }).click();
  await publishAll(page);

  expect((await request.get(`${SITE}/hooks-${suffix}/`, { maxRedirects: 0 })).status()).toBe(200);
  const old = await request.get(`${SITE}/${slug(second)}/`, { maxRedirects: 0 });
  expect(old.status()).toBe(301);
  expect(old.headers().location).toBe(`/hooks-${suffix}/`);
});

test("promoting a page with children limits the published sidebar to that branch", async ({ page }) => {
  const suffix = Math.random().toString(36).slice(2, 8);
  const [section, child, other] = [`Section ${suffix}`, `Child ${suffix}`, `Other ${suffix}`];
  const slug = (title: string) => title.toLowerCase().replace(/\W+/g, "-");

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await page.getByLabel("New page title").fill(section);
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: section, exact: true }).hover();
  await page.getByRole("button", { name: `Add a subpage to ${section}` }).click();
  await page.getByLabel("New page title").fill(child);
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: "New page" }).click();
  await page.getByLabel("New page title").fill(other);
  await page.keyboard.press("Enter");
  await expect(page.getByRole("button", { name: other, exact: true })).toBeVisible();

  await page.getByRole("button", { name: section, exact: true }).click();
  await page.getByRole("button", { name: "Page settings" }).click();
  // The checkbox follows the server's answer, so it only flips once the page list has reloaded.
  const toggle = page.getByRole("dialog", { name: "Page settings" }).getByLabel("Show in header");
  await toggle.click();
  await expect(toggle).toBeChecked();
  await page.getByRole("button", { name: "Close page settings" }).click();
  await publishAll(page);

  const html = (path: string) => readFileSync(`.e2e-data/site/${path}/index.html`, "utf8");
  const sidebar = (text: string) =>
    text.slice(text.indexOf('class="site-nav"'), text.indexOf("</nav>", text.indexOf('class="site-nav"')));

  const inside = html(`${slug(section)}/${slug(child)}`);
  expect(sidebar(inside)).toContain(`/${slug(section)}/${slug(child)}/`);
  expect(sidebar(inside)).not.toContain(`/${slug(other)}/`);
  expect(html(slug(other))).toContain(`href="/${slug(section)}/"`);
  expect(sidebar(html(slug(other)))).not.toContain(`/${slug(section)}/`);
});
