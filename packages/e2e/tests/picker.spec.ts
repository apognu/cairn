import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import { dragUnder, editablePage, nameNewPage, publishAll, slashCommand } from "./helpers";

test("a page created from the picker is in the tree and its link survives a move", async ({ page, request }) => {
  const suffix = Math.random().toString(36).slice(2, 8);
  const [home, section, topic] = [`Home ${suffix}`, `Section ${suffix}`, `Topic ${suffix}`];
  const slug = (title: string) => title.toLowerCase().replace(/\W+/g, "-");

  await page.goto("/");
  for (const title of [home, section]) {
    await page.getByRole("button", { name: "New page" }).click();
    await nameNewPage(page, title);
    await expect(page.getByRole("button", { name: title, exact: true })).toBeVisible();
  }

  await page.getByRole("button", { name: home, exact: true }).click();
  await (await editablePage(page)).click();
  await page.keyboard.type("See [[");

  const picker = page.getByRole("dialog", { name: "Link to" });
  await picker.getByRole("combobox").fill(topic);
  await expect(picker.getByText(`Create page “${topic}”`)).toBeVisible();
  await page.keyboard.press("Enter");

  // The new page is in the tree, and the link is in the text with the brackets gone.
  await expect(page.getByRole("button", { name: topic, exact: true })).toBeVisible();
  await expect(page.locator(".ProseMirror a", { hasText: topic })).toHaveAttribute("data-page-id", /.+/);
  await expect(page.locator(".ProseMirror")).not.toContainText("[[");
  await expect(page.getByRole("status")).toHaveText("Saved");

  // Moving the page under another one changes its URL, not the link.
  await dragUnder(page, request, topic, section);

  await publishAll(page);
  const html = readFileSync(`.e2e-data/site/${slug(home)}/index.html`, "utf8");
  expect(html).toContain(`href="/${slug(section)}/${slug(topic)}/"`);
});

test("deleting a linked page lists every page that links to it", async ({ page }) => {
  const suffix = Math.random().toString(36).slice(2, 8);
  const [target, first, second] = [`Target ${suffix}`, `First ${suffix}`, `Second ${suffix}`];

  await page.goto("/");
  for (const title of [target, first, second]) {
    await page.getByRole("button", { name: "New page" }).click();
    await nameNewPage(page, title);
    await expect(page.getByRole("button", { name: title, exact: true })).toBeVisible();
  }

  for (const source of [first, second]) {
    await page.getByRole("button", { name: source, exact: true }).click();
    await (await editablePage(page)).click();
    await page.keyboard.type("Read [[");
    await page.getByRole("dialog", { name: "Link to" }).getByRole("combobox").fill(target);
    await page
      .getByRole("option", { name: new RegExp(target) })
      .getByRole("button")
      .click();
    await expect(page.getByRole("status")).toHaveText("Saved");
  }

  // The target lists both in its settings, and the delete dialog repeats them.
  await page.getByRole("button", { name: target, exact: true }).click();
  await page.getByRole("button", { name: "Page settings" }).click();
  const linkedFrom = page.getByRole("dialog", { name: "Page settings" }).getByRole("list", { name: "Linked from" });
  await expect(linkedFrom).toContainText(first);
  await expect(linkedFrom).toContainText(second);
  await page.getByRole("button", { name: "Close page settings" }).click();

  await page.getByRole("button", { name: target, exact: true }).hover();
  await page.getByLabel(`More actions for ${target}`).click();
  await page.getByRole("button", { name: `Delete ${target}` }).click();
  const confirm = page.getByRole("dialog", { name: `Delete “${target}”?` });
  await expect(confirm.getByRole("list", { name: "Linked from" })).toContainText(first);
  await expect(confirm.getByRole("list", { name: "Linked from" })).toContainText(second);
  await confirm.getByRole("button", { name: "Cancel" }).click();
});

test("a link card shows the title and description of the page it points to", async ({ page }) => {
  const suffix = Math.random().toString(36).slice(2, 8);
  const [target, host] = [`Card target ${suffix}`, `Card host ${suffix}`];

  await page.goto("/");
  for (const title of [target, host]) {
    await page.getByRole("button", { name: "New page" }).click();
    await nameNewPage(page, title);
    await expect(page.getByRole("button", { name: title, exact: true })).toBeVisible();
  }

  // Written in place, under the title, where readers see it as the lead.
  await page.getByRole("button", { name: target, exact: true }).click();
  await page.getByRole("main").first().getByLabel("Description").fill("What the target page is about");
  await page.keyboard.press("Enter");

  await page.getByRole("button", { name: host, exact: true }).click();
  await (await editablePage(page)).click();
  await slashCommand(page, "link card", "Link card");

  // A selected atom shows its settings.
  await page.locator(".ProseMirror .link-card").click();
  const panel = page.getByRole("complementary", { name: "Component settings" });
  await panel.getByRole("button", { name: "Page", exact: true }).click();
  await panel.getByRole("combobox").fill(target);
  await page
    .getByRole("option", { name: new RegExp(target) })
    .getByRole("button")
    .click();

  const card = page.locator(".ProseMirror .link-card");
  await expect(card).toContainText(target);
  await expect(card).toContainText("What the target page is about");
});
