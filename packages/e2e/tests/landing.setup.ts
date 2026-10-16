import { expect, test as setup } from "@playwright/test";

// The editor opens the first page of the tree on arrival at "/", and after the open page is deleted, and takes its lock. Every test is
// the same user, so a test's own page there would lose its lock to any other test arriving meanwhile ("editing this page in another
// tab"). This page, made before every test, stays first (new pages go last) and no test edits it.
setup("a landing page comes first in the tree", async ({ request }) => {
  const pages = (await (await request.get("/api/pages")).json()) as { title: string }[];
  if (pages.some((p) => p.title === "Landing")) return;

  const created = await request.post("/api/pages", { data: { parent_id: null, title: "Landing" } });
  expect(created.ok(), await created.text()).toBe(true);
});
