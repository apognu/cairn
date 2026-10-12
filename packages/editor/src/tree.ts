import type { components } from "./api/schema";

export type Page = components["schemas"]["Page"];

export interface TreeNode {
  page: Page;
  children: TreeNode[];
}

export type MoveAction = "up" | "down" | "indent" | "outdent";

export interface MoveTarget {
  parent_id: string | null;
  /** Insert before this sibling; the page goes last when null. */
  before: string | null;
}

/** Builds the tree from the flat list. Sibling order is the order of `pages`, which the server sorts by position. */
export function buildTree(pages: Page[]): TreeNode[] {
  const nodes = new Map(pages.map((page) => [page.id, { page, children: [] as TreeNode[] }]));
  const roots: TreeNode[] = [];

  for (const node of nodes.values()) {
    const parent = node.page.parent_id ? nodes.get(node.page.parent_id) : undefined;
    (parent ? parent.children : roots).push(node);
  }

  return roots;
}

function siblingsOf(pages: Page[], parentId: string | null): Page[] {
  return pages.filter((p) => (p.parent_id ?? null) === parentId);
}

/** What the move API should be called with for a button press, or null when the move is not possible. */
export function moveTarget(pages: Page[], id: string, action: MoveAction): MoveTarget | null {
  const page = pages.find((p) => p.id === id);
  if (!page) return null;

  const parentId = page.parent_id ?? null;
  const siblings = siblingsOf(pages, parentId);
  const index = siblings.findIndex((p) => p.id === id);

  switch (action) {
    case "up":
      return index > 0 ? { parent_id: parentId, before: siblings[index - 1].id } : null;
    case "down":
      return index < siblings.length - 1 ? { parent_id: parentId, before: siblings[index + 2]?.id ?? null } : null;
    case "indent":
      // Becomes the last child of the sibling above.
      return index > 0 ? { parent_id: siblings[index - 1].id, before: null } : null;
    case "outdent": {
      if (parentId === null) return null;
      const parent = pages.find((p) => p.id === parentId);
      if (!parent) return null;

      // Lands right after its former parent.
      const uncles = siblingsOf(pages, parent.parent_id ?? null);
      const next = uncles[uncles.findIndex((p) => p.id === parentId) + 1];
      return { parent_id: parent.parent_id ?? null, before: next?.id ?? null };
    }
  }
}
