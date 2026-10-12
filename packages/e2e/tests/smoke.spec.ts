import { expect, type Page, test } from "@playwright/test";

// The whole path of the M1 acceptance criteria: write three nested pages with an image and a callout, publish, and read them on the
// published site with working search.

const SITE = "http://127.0.0.1:3001";
const PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg==",
  "base64",
);

async function createPage(page: Page, title: string, parent?: string) {
  if (parent) {
    await page.getByRole("button", { name: parent, exact: true }).hover();
    await page.getByRole("button", { name: `Add a subpage to ${parent}` }).click();
  } else {
    await page.getByRole("button", { name: "New page" }).click();
  }
  await page.getByLabel("New page title").fill(title);
  await page.keyboard.press("Enter");
  await expect(page.getByRole("button", { name: title, exact: true })).toBeVisible();
}

test("write, publish and read three nested pages with an image and a callout", async ({ page, browser }) => {
  const tag = Math.random().toString(36).slice(2, 8);
  const [guide, install, linux] = [`Guide ${tag}`, `Install ${tag}`, `Linux ${tag}`];

  await page.goto("/");
  await createPage(page, guide);
  await createPage(page, install, guide);
  await createPage(page, linux, install);

  await page.getByRole("button", { name: linux, exact: true }).click();
  const editor = page.locator(".ProseMirror");
  await editor.click();
  await page.keyboard.type(`Platypus installation steps for ${tag}`);
  await page.keyboard.press("Enter");

  await page.keyboard.type("/callout");
  await page.keyboard.press("Enter");
  await page.keyboard.type("Run as an administrator.");
  await page.keyboard.press("ArrowDown");

  const chooser = page.waitForEvent("filechooser");
  await page.keyboard.type("/image");
  await page.keyboard.press("Enter");
  await (await chooser).setFiles({ name: "step.png", mimeType: "image/png", buffer: PNG });
  await expect(editor.locator("figure img")).toBeVisible();
  await expect(page.getByRole("status")).toHaveText("Saved");

  await page.getByRole("button", { name: "Publish", exact: true }).click();
  await expect(page.getByText(/Published \d+ pages/)).toBeVisible();

  const reader = await browser.newPage();
  await reader.goto(`${SITE}/guide-${tag}/install-${tag}/linux-${tag}/`);

  await expect(reader.locator(".doc-title")).toHaveText(linux);
  await expect(reader.locator("aside.callout")).toContainText("Run as an administrator.");
  const image = reader.locator(".doc-content figure img");
  await expect(image).toHaveAttribute("src", /^\/_a\/[0-9a-f-]{36}\.png$/);
  await expect(image).toHaveJSProperty("naturalWidth", 1);

  // The navigation shows the nesting.
  await expect(reader.locator(".site-nav ul ul ul a", { hasText: linux })).toBeVisible();

  await expect(async () => {
    await reader.goto(`${SITE}/guide-${tag}/`);
    await reader.getByPlaceholder("Search").fill("platypus");
    await expect(reader.locator(".pagefind-ui__result-link", { hasText: linux })).toBeVisible({ timeout: 2000 });
  }).toPass();
});

test("the editor refuses requests that are not for localhost", async ({ request }) => {
  // What a DNS-rebinding page would send: a name that resolves to this machine, but is not a local one.
  const response = await request.get("/api/pages", { headers: { Host: "attacker.example" } });

  expect(response.status()).toBe(403);
});
