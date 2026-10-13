import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import { publishAll } from "./helpers";

test("a variable and a snippet are used on a page, and a snippet cannot include itself", async ({ page }) => {
  const tag = Math.random().toString(36).slice(2, 8);
  const [variable, snippet, title] = [`ver${tag}`, `pre${tag}`, `Library ${tag}`];

  await page.goto("/#settings=library");
  await page.getByLabel("New variable name").fill(variable);
  await page.getByLabel("New variable value").fill("9.9.9");
  await page.getByRole("button", { name: "Add", exact: true }).click();
  await expect(page.getByLabel(`Value of ${variable}`)).toHaveValue("9.9.9");

  await page.getByLabel("New snippet name").fill(snippet);
  await page.getByRole("button", { name: "Add snippet" }).click();
  const row = page.getByRole("list", { name: "Snippets" }).getByRole("listitem").filter({ hasText: snippet });
  await row.getByRole("button", { name: "Edit" }).click();
  await row.locator(".ProseMirror").click();
  await page.keyboard.type(`Install the tool before ${tag}.`);
  await row.getByRole("button", { name: "Save snippet" }).click();
  await expect(row.getByRole("button", { name: "Edit" })).toBeVisible();

  // A snippet that includes itself is refused when saved.
  await row.getByRole("button", { name: "Edit" }).click();
  await row.locator(".ProseMirror").click();
  await page.keyboard.press("Enter");
  await page.keyboard.type("/snippet");
  await page.keyboard.press("Enter");
  await row.locator(".snippet").click();
  await row.getByRole("complementary", { name: "Component settings" }).getByLabel("Snippet").selectOption(snippet);
  await row.getByRole("button", { name: "Save snippet" }).click();
  await expect(page.getByRole("alert")).toContainText("would include itself");
  await row.getByRole("button", { name: "Close" }).click();

  // Both are used on a page, and reach the published HTML.
  await page.getByRole("button", { name: "Back to pages" }).click();
  await page.getByRole("button", { name: "New page" }).click();
  await page.getByLabel("New page title").fill(title);
  await page.keyboard.press("Enter");
  await page.locator(".ProseMirror").click();
  await page.keyboard.type("Version ");
  await page.keyboard.type("/variable");
  await page.keyboard.press("Enter");
  const panel = page.getByRole("complementary", { name: "Component settings" });
  await page.locator(".ProseMirror .variable").click();
  await panel.getByLabel("Variable").selectOption(variable);
  await expect(page.locator(".ProseMirror .variable")).toHaveText("9.9.9");
  await page.keyboard.press("End");
  await page.keyboard.press("Enter");
  await page.keyboard.type("/snippet");
  await page.keyboard.press("Enter");
  await page.locator(".ProseMirror .snippet").click();
  await panel.getByLabel("Snippet").selectOption(snippet);
  await expect(page.locator(".ProseMirror .snippet")).toContainText(`Install the tool before ${tag}.`);
  await expect(page.getByRole("status")).toHaveText("Saved");

  await publishAll(page);
  const html = readFileSync(`.e2e-data/site/${title.toLowerCase().replace(/\W+/g, "-")}/index.html`, "utf8");
  expect(html).toContain("9.9.9");
  expect(html).toContain(`Install the tool before ${tag}.`);
});
