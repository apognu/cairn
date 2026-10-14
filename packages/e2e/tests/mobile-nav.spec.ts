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
