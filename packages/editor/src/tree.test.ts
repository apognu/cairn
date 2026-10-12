import { expect, test } from "bun:test";
import { buildTree, moveTarget, type Page } from "./tree";

const page = (id: string, parent_id: string | null = null): Page => ({
  id,
  parent_id,
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
