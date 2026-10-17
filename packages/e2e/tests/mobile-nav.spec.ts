import { expect, test } from "@playwright/test";
import { nameNewPage, publishAll, quiet, SITE } from "./helpers";

test("on a phone the menu button opens the page tree, and it closes after navigating and on Escape", async ({
  page,
  browser,
}) => {
  const tag = Math.random().toString(36).slice(2, 8);
  const [first, second] = [`Phone one ${tag}`, `Phone two ${tag}`];
  await page.goto("/");
  for (const title of [first, second]) {
    await page.getByRole("button", { name: "New page" }).click();
    await nameNewPage(page, title);
  }
  await publishAll(page);

  const reader = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await quiet(reader);
  await reader.goto(`${SITE}/phone-one-${tag}/`);
  const menu = reader.getByRole("button", { name: "Menu" });
  const tree = reader.getByRole("navigation", { name: "Pages" });

  // Below 900px the tree is hidden until the menu opens it.
  await expect(tree).toBeHidden();
  await menu.click();
  await expect(menu).toHaveAttribute("aria-expanded", "true");
  await tree.getByRole("link", { name: second }).click();

  await expect(reader).toHaveURL(`${SITE}/phone-two-${tag}/`);
  await expect(menu).toHaveAttribute("aria-expanded", "false");
  await expect(tree).toBeHidden();

  // Escape closes it too, and the focus goes back to the button.
  await menu.click();
  await expect(tree).toBeVisible();
  await reader.keyboard.press("Escape");
  await expect(tree).toBeHidden();
  await expect(menu).toBeFocused();
});

test("on a phone the header keeps the name on one line, the sections atop the page menu, and search in a panel", async ({
  page,
  request,
  browser,
}) => {
  const tag = Math.random().toString(36).slice(2, 8);
  const [plain, section] = [`Phone plain ${tag}`, `Phone section ${tag}`];
  await page.goto("/");
  for (const title of [plain, section]) {
    await page.getByRole("button", { name: "New page" }).click();
    await nameNewPage(page, title);
  }
  const pages = await (await request.get("/api/pages")).json();
  const id = pages.find((p: { title: string }) => p.title === section).id;
  await request.put(`/api/pages/${id}/header`, { data: { in_header: true } });
  await publishAll(page);

  const reader = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await quiet(reader);
  await reader.goto(`${SITE}/phone-plain-${tag}/`);
  // The name stays on one line: as tall as the logo or a line of text, whatever its length.
  const name = reader.locator(".site-header .site-name");
  await expect(name).toBeVisible();
  expect((await name.boundingBox())?.height).toBeLessThanOrEqual(32);
  await expect(reader.getByRole("navigation", { name: "Sections" }).first()).toBeHidden();

  // The sections head the page menu, apart from the header links (this site has none, so no links menu either).
  await expect(reader.getByRole("button", { name: "Links", exact: true })).toHaveCount(0);
  await reader.getByRole("button", { name: "Menu", exact: true }).click();
  const sections = reader.getByRole("list", { name: "Sections" });
  await expect(sections.getByRole("link", { name: section, exact: true })).toBeVisible();
  await expect(sections.getByRole("link", { name: section, exact: true })).toHaveCSS("font-weight", "600");
  await reader.getByRole("button", { name: "Menu", exact: true }).click();
  await expect(sections).toBeHidden();

  // Search opens over the page with the field ready to type in.
  await expect(reader.locator(".site-search input")).toBeAttached();
  const search = reader.getByRole("button", { name: "Search", exact: true });
  await search.click();
  const field = reader.locator(".site-search input");
  await expect(field).toBeFocused();
  await field.fill(plain);
  // Other tests publish meanwhile, so what the index holds can change: the panel only has to answer.
  await expect(reader.locator(".pagefind-ui__message")).toBeVisible();
  await reader.keyboard.press("Escape");
  await expect(field).toBeHidden();
  await expect(search).toBeFocused();
});
