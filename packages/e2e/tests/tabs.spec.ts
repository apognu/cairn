import { expect, test } from "@playwright/test";

test("tabs render on the server and switch after hydration", async ({ page }) => {
  await page.goto("/docs/demo");

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
