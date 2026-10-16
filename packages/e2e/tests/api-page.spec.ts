import { expect, test } from "@playwright/test";
import { dragUnder, nameNewPage } from "./helpers";

test("an API page is created, saved, moved, promoted and restored like a page", async ({ page, request }) => {
  const suffix = Math.random().toString(36).slice(2, 8);
  const [section, title] = [`Reference ${suffix}`, `Orders API ${suffix}`];
  const byTitle = async (name: string) =>
    (await (await request.get("/api/pages")).json()).find((p: { title: string }) => p.title === name);

  await page.goto("/");
  await page.getByRole("button", { name: "New page" }).click();
  await nameNewPage(page, section);

  await page.getByLabel("More ways to add").click();
  await page.getByRole("button", { name: "New API reference" }).click();
  await page.getByLabel("New API reference title").fill(title);
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("crumbs")).toContainText(title);

  // The settings form replaces the canvas; sources are saved explicitly.
  await page.getByRole("button", { name: "+ Add URL source" }).click();
  await page.getByLabel("Spec URL").fill("https://example.com/orders.yaml");
  await page.getByLabel("Tag prefix").fill("orders");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByText("Saved", { exact: true })).toBeVisible();
  const id = (await byTitle(title)).id;
  const config = await (await request.get(`/api/pages/${id}/api-config`)).json();
  expect(config.sources).toEqual([{ type: "url", url: "https://example.com/orders.yaml", prefix: "orders" }]);

  // "Check sources now" reads the form as it stands, and says why a source cannot be used (nothing here is allowed to reach the network).
  await page.getByLabel("Spec URL").fill("https://10.0.0.1/orders.yaml");
  await page.getByRole("button", { name: "Check sources now" }).click();
  await expect(page.getByText("The sources could not be merged.")).toBeVisible();
  await expect(page.getByText("is not an HTTPS address on the public internet")).toBeVisible();

  // A source that is not an https URL is refused with a reason.
  await page.getByLabel("Spec URL").fill("http://example.com/orders.yaml");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("https");

  // Dragged under the section, it takes its place in the tree.
  await dragUnder(page, request, title, section);

  await page.getByRole("button", { name: "Page settings" }).click();
  const toggle = page.getByRole("dialog", { name: "Page settings" }).getByLabel("Show in header");
  await toggle.click();
  await expect(toggle).toBeChecked();
  await page.getByRole("button", { name: "Close page settings" }).click();
  expect((await byTitle(title)).in_header).toBe(true);

  await page.getByRole("button", { name: title, exact: true }).hover();
  await page.getByLabel(`More actions for ${title}`).click();
  await page.getByRole("button", { name: `Delete ${title}` }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Delete", exact: true }).click();
  await expect(page.getByRole("button", { name: title, exact: true })).toHaveCount(0);

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Trash" }).click();
  await page.getByRole("button", { name: `Restore ${title}` }).click();
  await expect(page.getByRole("status")).toContainText("at its old place");
  expect((await (await request.get(`/api/pages/${id}/api-config`)).json()).sources).toHaveLength(1);

  // Every publish reads the sources of every API page, and this one points nowhere: it must not block the other tests' publishes.
  await request.delete(`/api/pages/${id}`);
});
