import { type Browser, expect, type Page, test } from "@playwright/test";
import { editablePage, nameNewPage } from "./helpers";

// Two editors on `cairn serve` (:3400), signed in as alice and bob through the local Dex.
const SERVER = "http://127.0.0.1:3400";

async function signedIn(browser: Browser, login: string): Promise<Page> {
  const page = await (await browser.newContext({ viewport: { width: 1440, height: 900 } })).newPage();
  await page.goto(`${SERVER}/_editor/`);
  // Reached through a chain of redirects, any of which can cut short the load of the one before: waiting for the URL alone, then for
  // the form, cannot see an aborted navigation.
  await page.waitForURL(/127\.0\.0\.1:5556/, { waitUntil: "commit" });
  await page.locator("input[name=login]").fill(`${login}@example.com`);
  await page.locator("input[name=password]").fill("password");
  await page.locator("button[type=submit]").click();
  await expect(page.getByLabel(/^Account: /)).toHaveText(login);

  return page;
}

test("a page someone edits is read-only for others, and free again when they leave", async ({ browser }) => {
  const title = `team-${Math.random().toString(36).slice(2, 8)}`;
  const alice = await signedIn(browser, "alice");
  const bob = await signedIn(browser, "bob");

  await alice.getByRole("button", { name: "New page" }).click();
  await nameNewPage(alice, title);
  await (await editablePage(alice)).click();
  await alice.keyboard.type("Alice was here");
  await expect(alice.getByRole("status").first()).toHaveText("Saved");

  await bob.reload();
  await bob.getByRole("button", { name: title, exact: true }).click();
  const banner = bob.getByTestId("lock-banner");
  await expect(banner).toContainText("alice is editing this page.");
  await expect(bob.locator(".ProseMirror")).toContainText("Alice was here");
  await expect(bob.locator(".ProseMirror")).toHaveAttribute("contenteditable", "false");
  // The innermost list item holding the page: its own row, not a parent's.
  const row = bob
    .getByRole("listitem")
    .filter({ has: bob.getByRole("button", { name: title, exact: true }) })
    .last();
  await expect(row.getByRole("img", { name: "Locked by alice" })).toBeVisible();
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

test("taking over stops the holder's editor at once, and their unsaved work can become a page", async ({ browser }) => {
  const title = `takeover-${Math.random().toString(36).slice(2, 8)}`;
  const alice = await signedIn(browser, "alice");
  const bob = await signedIn(browser, "bob");

  await alice.getByRole("button", { name: "New page" }).click();
  await nameNewPage(alice, title);
  await (await editablePage(alice)).click();
  await alice.keyboard.type("Saved part.");
  await expect(alice.getByRole("status").first()).toHaveText("Saved");
  // Alice's next words never reach the server.
  await alice.route("**/api/pages/*/draft", (route) =>
    route.request().method() === "PUT" ? route.abort() : route.fallback(),
  );
  await alice.keyboard.type(" Unsaved part.");

  // Alice is active, so taking over asks first.
  await bob.reload();
  await bob.getByRole("button", { name: title, exact: true }).click();
  const banner = bob.getByTestId("lock-banner");
  await banner.getByRole("button", { name: "Take over…" }).click();
  await expect(banner).toContainText("alice is still working on this page.");
  await banner.getByRole("button", { name: "Take over anyway" }).click();
  await expect(bob.locator(".ProseMirror")).toHaveAttribute("contenteditable", "true");

  // Alice's editor is told without waiting for its next renewal, and offers to keep what she wrote.
  const told = alice.getByTestId("lock-banner");
  await expect(told).toContainText("bob took over editing this page.", { timeout: 5_000 });
  await expect(alice.locator(".ProseMirror")).toHaveAttribute("contenteditable", "false");
  await alice.unroute("**/api/pages/*/draft");
  await told.getByRole("button", { name: "Save as a new page" }).click();
  await expect(alice.getByRole("button", { name: `${title} (my changes)`, exact: true })).toBeVisible();
  await expect(alice.locator(".ProseMirror")).toContainText("Saved part. Unsaved part.");
  await expect(alice.locator(".ProseMirror")).toHaveAttribute("contenteditable", "true");
});

test("publishing while someone edits a page names them first", async ({ browser }) => {
  const title = `editing-${Math.random().toString(36).slice(2, 8)}`;
  const alice = await signedIn(browser, "alice");
  const bob = await signedIn(browser, "bob");

  await alice.getByRole("button", { name: "New page" }).click();
  await nameNewPage(alice, title);
  await (await editablePage(alice)).click();
  await alice.keyboard.type("Half of a sentence");
  await expect(alice.getByRole("status").first()).toHaveText("Saved");

  await bob.reload();
  await bob
    .getByRole("button", { name: /^Publish/ })
    .first()
    .click();
  const dialog = bob.getByRole("dialog");
  const first = dialog.getByRole("alert").first();
  await expect(first).toContainText(/being edited right now/);
  await expect(first.getByRole("list", { name: "Pages being edited" })).toContainText(`${title} alice · last saved`);

  // The row says it too, with who wrote it and how much.
  const row = dialog.getByRole("row", { name: new RegExp(title) });
  await expect(row).toContainText("being edited");
  await expect(row).toContainText("alice ·");
  await row.getByRole("button", { name: "View diff" }).click();
  await expect(dialog.getByRole("list", { name: `Changes to ${title}` })).toContainText("+ Half of a sentence");
  await dialog.screenshot({ path: "test-results/publish-light.png" });
  await bob.emulateMedia({ colorScheme: "dark" });
  await dialog.screenshot({ path: "test-results/publish-dark.png" });
});

test("a hotfix of my changes leaves everyone else's drafts pending", async ({ browser }) => {
  const suffix = Math.random().toString(36).slice(2, 8);
  const [theirs, mine] = [`theirs-${suffix}`, `mine-${suffix}`];
  const alice = await signedIn(browser, "alice");
  const bob = await signedIn(browser, "bob");

  for (const [page, title, text] of [
    [alice, theirs, "Unfinished work"],
    [bob, mine, "The urgent fix"],
  ] as const) {
    await page.getByRole("button", { name: "New page" }).click();
    await nameNewPage(page, title);
    await (await editablePage(page)).click();
    await page.keyboard.type(text);
    await expect(page.getByRole("status").first()).toHaveText("Saved");
  }

  await bob
    .getByRole("button", { name: /^Publish/ })
    .first()
    .click();
  const dialog = bob.getByRole("dialog");
  await dialog.getByRole("button", { name: "Selected pages (hotfix)" }).click();
  await dialog.getByRole("button", { name: /^My changes/ }).click();
  await expect(dialog.getByRole("checkbox", { name: `Include ${mine}` })).toBeChecked();
  await expect(dialog.getByRole("checkbox", { name: `Include ${theirs}` })).not.toBeChecked();
  await dialog.screenshot({ path: "test-results/hotfix.png" });
  await dialog.getByRole("button", { name: "Publish 1 change" }).click();
  await expect(dialog).toBeHidden({ timeout: 30_000 });

  const reader = await browser.newPage();
  expect((await reader.request.get(`${SERVER}/${mine}/`)).status()).toBe(200);
  expect((await reader.request.get(`${SERVER}/${theirs}/`)).status()).toBe(404);
  const pending = await (await bob.request.get(`${SERVER}/_editor/api/publish/changes`)).json();
  const titles = pending.changes.map((c: { title: string }) => c.title);
  expect(titles).toContain(theirs);
  expect(titles).not.toContain(mine);
});
