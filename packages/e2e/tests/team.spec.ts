import { type Browser, expect, type Page, test } from "@playwright/test";
import { nameNewPage } from "./helpers";

// Two editors on `cairn serve` (:3400), signed in as alice and bob through the local Dex.
const SERVER = "http://127.0.0.1:3400";

async function signedIn(browser: Browser, login: string): Promise<Page> {
  const page = await (await browser.newContext({ viewport: { width: 1440, height: 900 } })).newPage();
  await page.goto(`${SERVER}/_editor/`);
  await page.waitForURL(/127\.0\.0\.1:5556/);
  await page.locator("input[name=login]").fill(`${login}@example.com`);
  await page.locator("input[name=password]").fill("password");
  await page.locator("button[type=submit]").click();
  await expect(page.locator(".user-menu summary")).toHaveText(login);

  return page;
}

test("a page someone edits is read-only for others, and free again when they leave", async ({ browser }) => {
  const title = `team-${Math.random().toString(36).slice(2, 8)}`;
  const alice = await signedIn(browser, "alice");
  const bob = await signedIn(browser, "bob");

  await alice.getByRole("button", { name: "New page" }).click();
  await nameNewPage(alice, title);
  await alice.locator(".ProseMirror").click();
  await alice.keyboard.type("Alice was here");
  await expect(alice.getByRole("status").first()).toHaveText("Saved");

  await bob.reload();
  await bob.getByRole("button", { name: title, exact: true }).click();
  const banner = bob.locator(".lock-banner");
  await expect(banner).toContainText("alice is editing this page.");
  await expect(bob.locator(".ProseMirror")).toContainText("Alice was here");
  await expect(bob.locator(".ProseMirror")).toHaveAttribute("contenteditable", "false");
  await expect(bob.getByRole("img", { name: "Locked by alice" })).toBeVisible();
  await bob.screenshot({ path: "test-results/locked-light.png" });
  await bob.emulateMedia({ colorScheme: "dark" });
  await bob.screenshot({ path: "test-results/locked-dark.png" });

  // A save by bob is refused even through the API.
  const id = (await (await bob.request.get(`${SERVER}/_editor/api/pages`)).json()).find(
    (p: { title: string }) => p.title === title,
  ).id;
  const refused = await bob.request.post(`${SERVER}/_editor/api/pages/${id}/lock`, { headers: { origin: SERVER } });
  expect(refused.status()).toBe(423);
  expect((await refused.json()).message).toBe("alice is editing this page");

  // Alice leaves the page: bob may edit it.
  await alice.getByRole("button", { name: "New page" }).click();
  await nameNewPage(alice, `${title}-other`);
  await expect(banner).toContainText("Nobody is editing this page now.", { timeout: 20_000 });
  await banner.getByRole("button", { name: "Edit" }).click();
  await expect(bob.locator(".ProseMirror")).toHaveAttribute("contenteditable", "true");
});
