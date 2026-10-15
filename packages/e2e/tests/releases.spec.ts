import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import { saveDraft } from "./helpers";

const text = (words: string) => ({
  type: "doc",
  content: [{ type: "paragraph", content: [{ type: "text", text: words }] }],
});

// Rolling back changes the live site for every other test, so this runs alone, last (see playwright.config.ts).
test("rolling back brings the earlier site back, and what followed is pending again", async ({ page, request }) => {
  const title = `Rolled ${Math.random().toString(36).slice(2, 8)}`;
  const slug = title.toLowerCase().replace(/\W+/g, "-");
  const published = async () => {
    const response = await request.post("/api/publish");
    expect(response.ok(), await response.text()).toBe(true);
  };

  const created = await (await request.post("/api/pages", { data: { parent_id: null, title } })).json();
  await saveDraft(request, created.id, text("the first words"), null);
  await published();
  const draft = await (await request.get(`/api/pages/${created.id}/draft`)).json();
  await saveDraft(request, created.id, text("the second words"), draft.base_revision_id);
  await published();
  expect(readFileSync(`.e2e-data/site/${slug}/index.html`, "utf8")).toContain("the second words");

  await page.goto("/#settings=releases");
  const releases = page.getByRole("table", { name: "Releases" });
  await expect(releases.getByText("Live", { exact: true })).toHaveCount(1);
  await expect(releases).toContainText(title);

  // The newest row is the live one; the row under it is the release to go back to.
  await page
    .getByRole("button", { name: /^Roll back to #/ })
    .first()
    .click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText("no draft is lost");
  await dialog.getByRole("button", { name: "Roll back" }).click();
  await expect(dialog).toBeHidden();

  await expect(async () => {
    expect(readFileSync(`.e2e-data/site/${slug}/index.html`, "utf8")).toContain("the first words");
  }).toPass({ timeout: 10_000 });

  // The second version is a draft waiting to be published, not lost.
  const pending = await (await request.get("/api/publish/changes")).json();
  expect(pending.changes.map((c: { title: string }) => c.title)).toContain(title);
  expect(JSON.stringify(await (await request.get(`/api/pages/${created.id}/draft`)).json())).toContain(
    "the second words",
  );

  await published();
  expect(readFileSync(`.e2e-data/site/${slug}/index.html`, "utf8")).toContain("the second words");
});
