import { expect, test } from "@playwright/test";
import { editablePage, publishFixture, quiet, saveDraft } from "./helpers";

test("tabs render on the server and switch after hydration", async ({ page, request }) => {
  await quiet(page);
  await page.goto(await publishFixture(request, "tabs"));

  const linux = page.getByRole("tab", { name: "Linux" });
  const macos = page.getByRole("tab", { name: "macOS" });

  await expect(linux).toHaveAttribute("aria-selected", "true");
  await expect(page.getByText("Use the package manager.")).toBeVisible();
  await expect(page.getByText("Use Homebrew.")).toBeHidden();

  await macos.click();

  await expect(macos).toHaveAttribute("aria-selected", "true");
  await expect(page.getByText("Use Homebrew.")).toBeVisible();
  await expect(page.getByText("Use the package manager.")).toBeHidden();
});

test("tabs are added, renamed, reordered and removed from the settings panel", async ({ page, request }) => {
  const title = `Tabs ${Math.random().toString(36).slice(2, 8)}`;
  const created = await (await request.post("/api/pages", { data: { parent_id: null, title } })).json();
  const panel = (label: string, text: string) => ({
    type: "tabPanel",
    attrs: { label },
    content: [{ type: "paragraph", content: [{ type: "text", text }] }],
  });
  await saveDraft(
    request,
    created.id,
    {
      type: "doc",
      content: [{ type: "tabs", content: [panel("Linux", "Use apt."), panel("macOS", "Use Homebrew.")] }],
    },
    null,
  );
  await page.goto(`/#page=${created.id}`);
  const editor = await editablePage(page);
  await editor.getByText("Use apt.").click();

  const settings = page.getByRole("complementary", { name: "Component settings" });
  const tabs = editor.getByRole("tab");
  await settings.getByRole("button", { name: "+ Add tab" }).click();
  // The new tab's label is selected, ready to be typed over, and its panel shows.
  await page.keyboard.type("Windows");
  await expect(tabs).toHaveText(["Linux", "macOS", "Windows"]);
  await expect(tabs.nth(2)).toHaveAttribute("aria-selected", "true");

  await settings.getByRole("button", { name: "Move tab Windows up" }).click();
  await expect(tabs).toHaveText(["Linux", "Windows", "macOS"]);
  await settings.getByRole("button", { name: "Remove tab Linux" }).click();
  await expect(tabs).toHaveText(["Windows", "macOS"]);
  await expect(editor).not.toContainText("Use apt.");

  await editor.press("ControlOrMeta+z");
  await expect(tabs).toHaveText(["Linux", "Windows", "macOS"]);
  await expect(settings.getByRole("button", { name: "Remove tab Linux" })).toBeEnabled();
});
