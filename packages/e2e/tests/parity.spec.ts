import { expect, test } from "@playwright/test";
import { publishFixture, quiet } from "./helpers";

// The page the server rendered in V8 must hydrate in the browser without React finding a difference, and then work.
test("a published page hydrates without mismatches", async ({ page, request }) => {
  const problems: string[] = [];
  page.on("pageerror", (e) => problems.push(e.message));
  // A missing search index (404 for pagefind files) is expected when nothing has been indexed yet, and `quiet` aborts the live reload request.
  page.on(
    "console",
    (m) =>
      m.type() === "error" && !m.text().includes("404") && !m.text().includes("ERR_FAILED") && problems.push(m.text()),
  );
  await quiet(page);

  const address = await publishFixture(request, "tabs");
  expect(await (await request.get(address)).text()).toContain("Use the package manager.");

  await page.goto(address);
  await expect(page.getByRole("tab", { name: "Linux" })).toBeVisible();
  // Hydration is done once a click changes the selection.
  await page.getByRole("tab", { name: "macOS" }).click();
  await expect(page.getByRole("tab", { name: "macOS" })).toHaveAttribute("aria-selected", "true");
  await page.getByRole("tab", { name: "Linux" }).click();
  await expect(page.getByRole("tab", { name: "Linux" })).toHaveAttribute("aria-selected", "true");

  expect(problems, "hydration errors").toEqual([]);
});
