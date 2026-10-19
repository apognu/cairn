import { expect, test } from "bun:test";
import { blockLines, diffLines } from "./diff";
import { buildTree, type DropZone, dropTarget, moveTarget, type Page, pageUrls } from "./tree";

const page = (id: string, parent_id: string | null = null, kind: Page["kind"] = "page"): Page => ({
  id,
  kind,
  parent_id,
  external_url: kind === "link" ? "https://example.com" : null,
  path_override: null,
  in_header: false,
  hidden: false,
  header_position: null,
  position: id,
  slug: id,
  title: id,
  created_by: "u",
  created_at: "",
  updated_at: "",
});

// a, b(b1, b2), c
const pages = [page("a"), page("b"), page("b1", "b"), page("b2", "b"), page("c")];

test("builds nested nodes in order", () => {
  const tree = buildTree(pages);
  expect(tree.map((n) => n.page.id)).toEqual(["a", "b", "c"]);
  expect(tree[1].children.map((n) => n.page.id)).toEqual(["b1", "b2"]);
});

test("button presses map to move requests", () => {
  const cases: [string, Parameters<typeof moveTarget>[2], ReturnType<typeof moveTarget>][] = [
    ["a", "up", null],
    ["b", "up", { parent_id: null, before: "a" }],
    ["a", "down", { parent_id: null, before: "c" }],
    ["b", "down", { parent_id: null, before: null }],
    ["c", "down", null],
    ["b", "indent", { parent_id: "a", before: null }],
    ["a", "indent", null],
    ["b1", "outdent", { parent_id: null, before: "c" }],
    ["b2", "up", { parent_id: "b", before: "b1" }],
    ["a", "outdent", null],
  ];

  for (const [id, action, expected] of cases) {
    expect(moveTarget(pages, id, action), `${id} ${action}`).toEqual(expected);
  }
});

test("drops map to move requests", () => {
  // a, b(b1, b2, link), c
  const tree = [page("a"), page("b"), page("b1", "b"), page("b2", "b"), page("l", "b", "link"), page("c")];
  const cases: [string, string, DropZone, ReturnType<typeof dropTarget>][] = [
    ["a", "c", "before", { parent_id: null, before: "c" }],
    ["a", "c", "after", { parent_id: null, before: null }],
    ["a", "c", "inside", { parent_id: "c", before: null }],
    // Under a page that has children: first child.
    ["a", "b", "after", { parent_id: "b", before: "b1" }],
    ["c", "b2", "after", { parent_id: "b", before: "l" }],
    ["c", "l", "inside", { parent_id: "b", before: null }],
    // Nothing to do.
    ["a", "a", "inside", null],
    ["b", "b1", "before", null],
    ["b", "b2", "inside", null],
    ["a", "b", "before", null],
    ["b1", "b2", "before", null],
    ["b1", "b", "after", null],
  ];

  for (const [drag, over, zone, expected] of cases) {
    expect(dropTarget(tree, drag, over, zone)).toEqual(expected);
  }
});

test("urls follow slugs and custom paths", () => {
  const tree = [
    { ...page("guide") },
    { ...page("hooks", "guide"), path_override: "/hooks/" },
    { ...page("retries", "hooks") },
    { ...page("home-link", null, "link") },
  ];
  expect(pageUrls(tree)).toEqual({ guide: "/guide", hooks: "/hooks", retries: "/hooks/retries" });
});

test("diffs list what a restore adds and removes", () => {
  const doc = (...texts: string[]) => ({
    type: "doc",
    content: texts.map((text) => ({ type: "paragraph", content: [{ type: "text", text }] })),
  });
  // The page now: a, b, d. Restoring the version a, c, d removes b and adds c.
  const current = blockLines(doc("a", "b", "d"));
  const version = blockLines(doc("a", "c", "d"));

  expect(diffLines(current, version)).toEqual([
    { kind: "same", text: "a" },
    { kind: "removed", text: "b" },
    { kind: "added", text: "c" },
    { kind: "same", text: "d" },
  ]);
});
