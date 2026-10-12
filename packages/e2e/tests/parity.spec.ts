import { expect, type Page, test } from "@playwright/test";

// The same document through two paths: rendered by V8 and hydrated, or built entirely in the browser.
async function open(page: Page, path: string): Promise<string[]> {
  const problems: string[] = [];
  page.on("pageerror", (e) => problems.push(e.message));
  page.on("console", (m) => m.type() === "error" && problems.push(m.text()));

  await page.goto(path);
  await expect(page.getByRole("tab", { name: "Linux" })).toBeVisible();
  // Hydration is done once a click changes the selection.
  await page.getByRole("tab", { name: "macOS" }).click();
  await expect(page.getByRole("tab", { name: "macOS" })).toHaveAttribute("aria-selected", "true");
  await page.getByRole("tab", { name: "Linux" }).click();
  await expect(page.getByRole("tab", { name: "Linux" })).toHaveAttribute("aria-selected", "true");

  return problems;
}

test("V8 render and browser render are identical", async ({ browser }) => {
  const server = await browser.newPage({ viewport: { width: 900, height: 600 } });
  const client = await browser.newPage({ viewport: { width: 900, height: 600 } });

  // Look at the server render before hydration touches it.
  await server.goto("/docs/demo", { waitUntil: "commit" });
  const ssrHtml = await (await server.request.get("/docs/demo")).text();
  expect(ssrHtml).toContain("Use the package manager.");

  const serverProblems = await open(server, "/docs/demo");
  const clientProblems = await open(client, "/docs/demo/client");

  expect(serverProblems, "hydration errors").toEqual([]);
  expect(clientProblems).toEqual([]);
  expect(await server.locator("#root").innerHTML()).toBe(await client.locator("#root").innerHTML());

  for (const tab of ["Linux", "macOS"]) {
    await server.getByRole("tab", { name: tab }).click();
    await client.getByRole("tab", { name: tab }).click();

    const left = await server.screenshot();
    const right = await client.screenshot();
    expect(left.equals(right), `screenshots differ for the ${tab} tab`).toBe(true);
  }
});
