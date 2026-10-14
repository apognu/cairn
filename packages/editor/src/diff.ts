import type { DocNode } from "@cairn/renderer";

const textOf = (node: DocNode): string => node.text ?? (node.content ?? []).map(textOf).join("");

/** One line of text per top-level block, enough to see what changed between two versions of a page. */
export function blockLines(doc: DocNode): string[] {
  return (doc.content ?? []).flatMap((node): string[] => {
    switch (node.type) {
      case "heading":
        return [`${"#".repeat(Number(node.attrs?.level ?? 2))} ${textOf(node)}`];
      case "bulletList":
      case "orderedList":
        return (node.content ?? []).map((item) => `• ${textOf(item)}`);
      case "codeBlock":
        return [`code: ${textOf(node)}`];
      case "paragraph":
      case "blockquote":
        return [textOf(node)];
      default:
        // A component or media block: its kind and any words in it, or its settings when there are none.
        return [`[${node.type}] ${textOf(node) || JSON.stringify(node.attrs ?? {}).slice(0, 120)}`];
    }
  });
}

export type DiffLine = { kind: "same" | "added" | "removed"; text: string };

/** Lines of `to` compared with `from`: what a change from one to the other adds and removes (longest common subsequence). */
export function diffLines(from: string[], to: string[]): DiffLine[] {
  const table = Array.from({ length: from.length + 1 }, () => new Array<number>(to.length + 1).fill(0));
  for (let i = from.length - 1; i >= 0; i--) {
    for (let j = to.length - 1; j >= 0; j--) {
      table[i][j] = from[i] === to[j] ? table[i + 1][j + 1] + 1 : Math.max(table[i + 1][j], table[i][j + 1]);
    }
  }

  const out: DiffLine[] = [];
  let i = 0;
  let j = 0;
  while (i < from.length && j < to.length) {
    if (from[i] === to[j]) {
      out.push({ kind: "same", text: from[i] });
      i++;
      j++;
    } else if (table[i + 1][j] >= table[i][j + 1]) {
      out.push({ kind: "removed", text: from[i++] });
    } else {
      out.push({ kind: "added", text: to[j++] });
    }
  }
  while (i < from.length) out.push({ kind: "removed", text: from[i++] });
  while (j < to.length) out.push({ kind: "added", text: to[j++] });

  return out;
}
