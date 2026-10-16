import { expect, test } from "@playwright/test";
import { editablePage, nameNewPage, publishAll, quiet } from "./helpers";

// `cairn serve` on :3400 against a local Dex (`examples/dex`, user alice@example.com / password): the editor under /_editor behind a
// sign-in, and the published site at the root of the same address.
const SERVER = "http://127.0.0.1:3400";

test("sign in, write, publish, read anonymously, and sign out", async ({ page, browser }) => {
  const slug = `serve-${Math.random().toString(36).slice(2, 8)}`;

  // The editor needs a login: an anonymous visit goes to the provider, and the API refuses.
  expect((await page.request.get(`${SERVER}/_editor/api/pages`)).status()).toBe(401);
  await page.goto(`${SERVER}/_editor/`);
  await page.waitForURL(/127\.0\.0\.1:5556/);
  await page.locator("input[name=login]").fill("alice@example.com");
  await page.locator("input[name=password]").fill("password");
  await page.locator("button[type=submit]").click();

  await expect(page.getByLabel(/^Account: /)).toHaveText("alice");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, slug);
  await page.getByRole("button", { name: slug, exact: true }).click();
  await (await editablePage(page)).click();
  await page.keyboard.type("Written behind a login");
  await expect(page.getByRole("status")).toHaveText("Saved");
  await publishAll(page);

  // Readers need nothing: the page is on the same address, and what the editor offers is not.
  const reader = await browser.newPage();
  await quiet(reader);
  await reader.goto(`${SERVER}/${slug}/`);
  await expect(reader.locator(".doc-content")).toContainText("Written behind a login");
  expect((await reader.request.get(`${SERVER}/_editor/api/pages`)).status()).toBe(401);

  // Renaming the page keeps its old address working: the self-hosted site applies the redirect.
  const origin = { origin: SERVER };
  const pages = (await (await page.request.get(`${SERVER}/_editor/api/pages`)).json()) as {
    id: string;
    title: string;
  }[];
  const id0 = pages.find((p) => p.title === slug)?.id;
  const renamed = `${slug}-renamed`;
  expect(
    (
      await page.request.post(`${SERVER}/_editor/api/pages/${id0}`, {
        headers: origin,
        data: { title: renamed, redirect: true },
      })
    ).status(),
  ).toBe(200);
  expect((await page.request.post(`${SERVER}/_editor/api/publish`, { headers: origin })).status()).toBe(200);
  const old = await reader.request.get(`${SERVER}/${slug}/`, { maxRedirects: 0 });
  expect([301, 302, 308]).toContain(old.status());
  expect(old.headers().location).toBe(`/${renamed}/`);
  await reader.goto(`${SERVER}/${slug}/`);
  await expect(reader).toHaveURL(`${SERVER}/${renamed}/`);

  // A build can be looked at by a signed-in editor, and by nobody else.
  const preview = await page.request.post(`${SERVER}/_editor/api/publish/preview`, { headers: { origin: SERVER } });
  expect(preview.status()).toBe(200);
  const { id } = (await preview.json()) as { id: string };
  expect((await page.request.get(`${SERVER}/_preview/${id}/${renamed}/`)).status()).toBe(200);
  expect((await reader.request.get(`${SERVER}/_preview/${id}/${renamed}/`, { maxRedirects: 0 })).status()).toBe(303);

  // A preview is not made live over what changed after it: promoting says which pages differ, and publishing builds from staging.
  const later = `Later ${Math.random().toString(36).slice(2, 8)}`;
  const created = await page.request.post(`${SERVER}/_editor/api/pages`, {
    headers: { origin: SERVER },
    data: { parent_id: null, title: later },
  });
  expect(created.ok()).toBe(true);
  const stale = await page.request.post(`${SERVER}/_editor/api/releases/${id}/promote`, {
    headers: { origin: SERVER },
  });
  expect(stale.status()).toBe(409);
  expect(((await stale.json()) as { message: string }).message).toContain(later);

  // A request from another origin is refused even with the session cookie.
  const forged = await page.request.post(`${SERVER}/_editor/api/publish`, {
    headers: { origin: "https://evil.example" },
  });
  expect(forged.status()).toBe(403);

  // Signing out ends the session.
  await page.getByLabel(/^Account: /).click();
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page.getByText("You are signed out")).toBeVisible();
  expect((await page.request.get(`${SERVER}/_editor/api/pages`)).status()).toBe(401);
});
