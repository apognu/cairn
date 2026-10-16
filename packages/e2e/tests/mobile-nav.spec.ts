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

test("on a phone the header keeps only buttons: the sections and links in a menu, search in a panel, the name in the page tree", async ({
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
  await expect(reader.locator(".site-header .site-name")).toBeHidden();
  await expect(reader.getByRole("navigation", { name: "Sections" }).first()).toBeHidden();

  // The menu lists the sections; Escape closes it and gives the focus back.
  const more = reader.getByRole("button", { name: "Sections and links" });
  await more.click();
  await expect(reader.getByRole("link", { name: section, exact: true })).toBeVisible();
  await reader.keyboard.press("Escape");
  await expect(reader.getByRole("link", { name: section, exact: true })).toBeHidden();
  await expect(more).toBeFocused();

  // Search opens over the page with the field ready to type in.
  await expect(reader.locator(".site-search input")).toBeAttached();
  const search = reader.getByRole("button", { name: "Search", exact: true });
  await search.click();
  const field = reader.locator(".site-search input");
  await expect(field).toBeFocused();
  await field.fill(plain);
  await expect(reader.locator(".pagefind-ui__result-link").first()).toBeVisible();
  await reader.keyboard.press("Escape");
  await expect(field).toBeHidden();
  await expect(search).toBeFocused();

  // The site's name heads the page tree instead.
  await reader.getByRole("button", { name: "Menu" }).click();
  await expect(reader.locator(".site-nav-brand")).toBeVisible();
});
