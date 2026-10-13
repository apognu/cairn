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

/** The first page (not link) in the order of the tree, the way a reader would meet it. */
export function firstPage(pages: Page[]): Page | undefined {
  const visit = (nodes: TreeNode[]): Page | undefined => {
    for (const node of nodes) {
      if (node.page.kind === "page") return node.page;
      const found = visit(node.children);
      if (found) return found;
    }
    return undefined;
  };

  return visit(buildTree(pages));
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

/** Where on a row a dragged page is held: above it, on it (becomes its child) or below it. */
export type DropZone = "before" | "inside" | "after";

/**
 * What the move API should be called with when `dragId` is dropped on `overId`, or null when the drop does nothing:
 * onto itself or its own subtree, onto its current place, or inside a link (links hold no pages).
 */
export function dropTarget(pages: Page[], dragId: string, overId: string, zone: DropZone): MoveTarget | null {
  const over = pages.find((p) => p.id === overId);
  if (!over || !pages.some((p) => p.id === dragId)) return null;

  for (let p: Page | undefined = over; p; p = pages.find((q) => q.id === p?.parent_id)) {
    if (p.id === dragId) return null;
  }

  const childrenOf = (parent: string | null) =>
    pages.filter((p) => (p.parent_id ?? null) === parent && p.id !== dragId);
  const before = (parent: string | null, sibling: Page | undefined): MoveTarget => ({
    parent_id: parent,
    before: sibling?.id ?? null,
  });

  // Below a page that has children means "first child", as the line drawn under it suggests.
  const effective: DropZone = zone === "inside" && over.kind === "link" ? "after" : zone;
  let target: MoveTarget;

  if (effective === "before") {
    target = before(over.parent_id ?? null, over);
  } else if (effective === "inside") {
    target = before(over.id, undefined);
  } else if (over.kind === "page" && childrenOf(over.id).length > 0) {
    target = before(over.id, childrenOf(over.id)[0]);
  } else {
    const siblings = childrenOf(over.parent_id ?? null);
    target = before(over.parent_id ?? null, siblings[siblings.findIndex((p) => p.id === over.id) + 1]);
  }

  // Already there: same parent and the same next sibling.
  const current = pages.find((p) => p.id === dragId);
  const siblings = pages.filter((p) => (p.parent_id ?? null) === (current?.parent_id ?? null));
  const next = siblings[siblings.findIndex((p) => p.id === dragId) + 1];
  const unchanged = target.parent_id === (current?.parent_id ?? null) && target.before === (next?.id ?? null);

  return unchanged ? null : target;
}

/** The public URL of every page (links have none), as the server derives it: a custom URL, or the parent's followed by the slug. */
export function pageUrls(pages: Page[]): Record<string, string> {
  const byId = new Map(pages.map((p) => [p.id, p]));
  const urls: Record<string, string> = {};

  for (const page of pages.filter((p) => p.kind === "page")) {
    const segments: string[] = [];
    let base = "";

    for (let p: Page | undefined = page; p; p = p.parent_id ? byId.get(p.parent_id) : undefined) {
      if (p.path_override) {
        base = p.path_override.replace(/\/$/, "");
        break;
      }
      segments.unshift(p.slug);
    }
    urls[page.id] = `${base}/${segments.join("/")}`.replace(/\/$/, "") || "/";
  }

  return urls;
}
