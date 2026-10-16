import { expect, test } from "@playwright/test";
import { editablePage, nameNewPage, publishAll } from "./helpers";

test("restoring a version creates a new draft and keeps the replaced one, and the trash gives a page its URL back", async ({
  page,
  request,
}) => {
  const title = `Revisions ${Math.random().toString(36).slice(2, 8)}`;

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, title);
  await (await editablePage(page)).click();
  await page.keyboard.type("first version");
  await expect(page.getByRole("status")).toHaveText("Saved");
  const id = new URL(page.url()).hash.replace("#page=", "");

  // Publishing ends the editing session, so the first version stays listed on its own.
  await publishAll(page);
  // Renamed since: restoring with the title ticked brings the old title back, at the page's current URL.
  const renamed = `${title} renamed`;
  await request.post(`/api/pages/${id}`, { data: { title: renamed } });
  await page.reload();
  await expect(page.getByRole("button", { name: renamed, exact: true })).toBeVisible();
  const renamedUrl = (await (await request.get(`/api/pages/${id}/url`)).json()).url;
  await (await editablePage(page)).click();
  await page.keyboard.type(" and then a second");
  await expect(page.getByRole("status")).toHaveText("Saved");
  const versionsBefore = (await (await request.get(`/api/pages/${id}/history`)).json()).length;

  await page.getByRole("button", { name: "History", exact: true }).click();
  const drawer = page.getByRole("dialog", { name: "History" });
  // Other tests publish on the same site, so versions can be listed that this test did not make (an empty first one, say): look for
  // the one whose restore would give "first version".
  const versions = drawer.getByRole("list", { name: "Versions" }).getByRole("button");
  const changes = drawer.getByRole("list", { name: "Changes" });
  await expect(versions.first()).toBeVisible();
  let found = false;
  for (let i = 0; i < (await versions.count()) && !found; i++) {
    await versions.nth(i).click();
    // The list is drawn once the version and the page's current text have both arrived.
    found = await expect
      .poll(async () => (await changes.innerText().catch(() => "")).includes("+ first version"), { timeout: 2500 })
      .toBe(true)
      .then(
        () => true,
        () => false,
      );
  }
  expect(found).toBe(true);
  await expect(changes).toContainText("first version and then a second");
  await drawer.getByRole("checkbox", { name: /Title/ }).check();
  await drawer.getByRole("button", { name: "Restore as draft" }).click();

  // The editor now holds the old text, and nothing was lost: one more version, the replaced draft among them.
  await expect(page.locator(".ProseMirror")).toHaveText("first version");
  await expect(page.getByRole("button", { name: title, exact: true })).toBeVisible();
  expect((await (await request.get(`/api/pages/${id}/url`)).json()).url).toBe(renamedUrl);
  const history = await (await request.get(`/api/pages/${id}/history`)).json();
  expect(history.length).toBe(versionsBefore + 1);

  // Deleting and restoring from the trash brings the page back where it was, at its URL.
  const urlBefore = (await (await request.get(`/api/pages/${id}/url`)).json()).url;
  await page.getByRole("button", { name: title, exact: true }).hover();
  await page.getByLabel(`More actions for ${title}`).click();
  await page.getByRole("button", { name: `Delete ${title}` }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Delete", exact: true }).click();
  await expect(page.getByRole("button", { name: title, exact: true })).toHaveCount(0);

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Trash" }).click();
  await page.getByRole("button", { name: `Restore ${title}` }).click();
  await expect(page.getByRole("status")).toContainText("at its old place");
  expect((await (await request.get(`/api/pages/${id}/url`)).json()).url).toBe(urlBefore);
});
