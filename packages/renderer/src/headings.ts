import type { DocNode } from "./types";

export type Heading = { level: number; text: string; anchor: string };

function textOf(node: DocNode): string {
  return node.text ?? (node.content ?? []).map(textOf).join("");
}

function slug(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/** Headings in document order with unique anchors, keyed by node so renderNode can look its own up. */
export function collectHeadings(doc: DocNode): { list: Heading[]; byNode: WeakMap<DocNode, string> } {
  const list: Heading[] = [];
  const byNode = new WeakMap<DocNode, string>();
  const taken = new Set<string>();

  const visit = (node: DocNode) => {
    if (node.type === "heading") {
      const text = textOf(node);
      const base = slug(text) || "section";
      // The next free number: a heading "Setup 2" may already hold `setup-2` when a second "Setup" comes. `term_anchors` in Rust
      // numbers glossary entries the same way, and must keep doing so.
      let anchor = base;
      for (let n = 2; taken.has(anchor); n++) anchor = `${base}-${n}`;
      taken.add(anchor);

      list.push({ level: Number(node.attrs?.level ?? 1), text, anchor });
      byNode.set(node, anchor);
    }

    node.content?.forEach(visit);
  };
  visit(doc);

  return { list, byNode };
}
