import { expect, test } from "@playwright/test";
import { nameNewPage, PNG } from "./helpers";

test("a page with a problem blocks publishing, naming the page, until it is fixed", async ({ page, request }) => {
  const title = `Checked ${Math.random().toString(36).slice(2, 8)}`;
  let pageId: string | undefined;

  try {
    await page.goto("/");
    await page.getByRole("button", { name: "New page" }).click();
    await nameNewPage(page, title);
    await page.getByRole("button", { name: title, exact: true }).click();
    pageId = (await (await request.get("/api/pages")).json()).find((p: { title: string }) => p.title === title).id;

    // An image without a description.
    const asset = await (
      await request.post("/api/assets", {
        data: Buffer.concat([PNG, Buffer.from(Math.random().toString())]),
        headers: { "content-type": "application/octet-stream" },
      })
    ).json();
    await page.locator(".ProseMirror").click();
    await page.keyboard.type("Before the picture");
    await expect(page.getByRole("status")).toHaveText("Saved");
    const draft = await (await request.get(`/api/pages/${pageId}/draft`)).json();
    await request.put(`/api/pages/${pageId}/draft`, {
      data: {
        base_revision_id: draft.base_revision_id,
        doc: { type: "doc", content: [{ type: "image", attrs: { assetId: asset.id, alt: "", caption: "" } }] },
      },
    });

    // The dialog names the page and offers no way through.
    await page
      .getByRole("button", { name: /^Publish/ })
      .first()
      .click();
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByRole("list", { name: "Problems" })).toContainText(title);
    await expect(dialog.getByRole("list", { name: "Problems" })).toContainText("no alt text");
    await expect(dialog.getByRole("button", { name: /^Publish/ })).toBeDisabled();

    // The server refuses too, with the page in the message.
    const refused = await request.post("/api/publish");
    expect(refused.status()).toBe(422);
    expect((await refused.json()).details.join(" ")).toContain(title);
    await dialog.getByRole("button", { name: "Cancel" }).click();

    // Marking the image decorative fixes it.
    const latest = await (await request.get(`/api/pages/${pageId}/draft`)).json();
    await request.put(`/api/pages/${pageId}/draft`, {
      data: {
        base_revision_id: latest.base_revision_id,
        doc: {
          type: "doc",
          content: [{ type: "image", attrs: { assetId: asset.id, alt: "", caption: "", decorative: true } }],
        },
      },
    });
    expect((await request.post("/api/publish")).status()).toBe(200);
  } finally {
    // Whatever happened, leave nothing behind that blocks the next publish.
    if (pageId) await request.delete(`/api/pages/${pageId}`);
  }
});
