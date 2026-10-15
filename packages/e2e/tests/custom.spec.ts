import { spawnSync } from "node:child_process";
import { cpSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, type Locator, test } from "@playwright/test";
import { nameNewPage, publishAll } from "./helpers";

test("a custom component appears in the slash menu, renders in the editor and on the published site", async ({
  page,
  request,
}) => {
  const slug = `custom-${Math.random().toString(36).slice(2, 8)}`;
  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, slug);
  await page.getByRole("button", { name: slug, exact: true }).click();

  const editor = page.locator(".ProseMirror");
  await editor.click();
  await page.keyboard.type("/acme");
  await expect(page.getByRole("option", { name: /Acme note/ })).toBeVisible();
  await page.keyboard.press("Enter");

  // Its title is written on the canvas, like the built-in cards'.
  const title = editor.locator(".acme-note .editable");
  await expect(title).toHaveText("Note");
  await title.click();
  await page.keyboard.press("Control+A");
  await page.keyboard.type("Heads up");
  await page.keyboard.press("Enter");
  await expect(title).toHaveText("Heads up");
  // The status still reads "Saved" for the insert until the autosave of the new title starts, so the draft itself is what is waited for.
  await expect
    .poll(async () => {
      const pages = (await (await request.get("/api/pages")).json()) as { id: string; slug: string }[];
      const id = pages.find((p) => p.slug === slug)?.id;
      return JSON.stringify(await (await request.get(`/api/pages/${id}/draft`)).json());
    })
    .toContain("Heads up");

  await publishAll(page);

  const html = readFileSync(`.e2e-custom-data/site/${slug}/index.html`, "utf8");
  expect(html).toContain('<aside class="acme-note"><strong>Heads up</strong>');
  // The deployer's footer slot replaces the default one.
  expect(html).toContain('<footer class="acme-footer">');
  expect(readFileSync(".e2e-bundles/site.css", "utf8")).toContain(".acme-note");
});

test("a custom component styled with utilities alone is styled in the editor and on the published site", async ({
  page,
}) => {
  const slug = `tip-${Math.random().toString(36).slice(2, 8)}`;
  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, slug);
  await page.getByRole("button", { name: slug, exact: true }).click();

  await page.locator(".ProseMirror").click();
  await page.keyboard.type("/acme tip");
  await expect(page.getByRole("option", { name: /Acme tip/ })).toBeVisible();
  await page.keyboard.press("Enter");

  // Its utilities come from the deployer's folder, which the build scans: nothing in Cairn uses a dashed border.
  const look = (tip: Locator) =>
    tip.evaluate((el) => [getComputedStyle(el).borderTopStyle, getComputedStyle(el).borderRadius]);
  await expect.poll(() => look(page.locator(".ProseMirror .acme-tip"))).toEqual(["dashed", "12px"]);
  await expect(page.getByRole("status")).toHaveText("Saved");

  await publishAll(page);

  await page.goto(`http://127.0.0.1:3201/${slug}/`);
  expect(await look(page.locator(".acme-tip"))).toEqual(["dashed", "12px"]);
});

test("bundles built with another SDK version are refused at startup", () => {
  const dir = mkdtempSync(join(tmpdir(), "cairn-sdk-"));
  cpSync(".e2e-bundles", dir, { recursive: true });
  const manifest = JSON.parse(readFileSync(join(dir, "manifest.json"), "utf8"));
  writeFileSync(join(dir, "manifest.json"), JSON.stringify({ ...manifest, sdk_version: manifest.sdk_version + 1 }));

  const run = spawnSync(
    "../../target/debug/cairn",
    ["dev", "--data-dir", join(dir, "data"), "--bind", "127.0.0.1:3210", "--site-bind", "127.0.0.1:3211"],
    {
      env: { ...process.env, BUNDLES_DIR: dir },
      encoding: "utf8",
      timeout: 20_000,
    },
  );

  expect(run.status).not.toBe(0);
  expect(run.stderr).toContain("were built with component SDK version 2");
});
