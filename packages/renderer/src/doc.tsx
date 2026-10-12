import { Tabs } from "@yourdocs/components";
import type { ReactNode } from "react";

export type DocNode = {
  type: string;
  attrs?: Record<string, unknown>;
  content?: DocNode[];
  text?: string;
};

export function renderNodes(nodes: DocNode[] = []): ReactNode[] {
  return nodes.map((node, i) => renderNode(node, i));
}

export function renderNode(node: DocNode, key: number): ReactNode {
  // Panels belong to `tabs` and are not standalone nodes, so it renders its own children.
  if (node.type === "tabs") return renderTabs(node, key);

  const children = renderNodes(node.content);

  switch (node.type) {
    case "doc":
      return <div key={key}>{children}</div>;
    case "paragraph":
      return <p key={key}>{children}</p>;
    case "heading": {
      const level = Math.min(Math.max(Number(node.attrs?.level ?? 1), 1), 6);
      const Tag = `h${level}` as "h1";
      return <Tag key={key}>{children}</Tag>;
    }
    case "text":
      return node.text;
    default:
      throw new Error(`unknown node type: ${node.type}`);
  }
}

function renderTabs(node: DocNode, key: number): ReactNode {
  const panels = node.content ?? [];
  const labels = panels.map((p) => String(p.attrs?.label ?? ""));

  return (
    <Tabs key={key} labels={labels}>
      {panels.map((p, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: panels are positional
        <div key={i}>{renderNodes(p.content)}</div>
      ))}
    </Tabs>
  );
}

export function Doc({ doc }: { doc: DocNode }) {
  return <main className="doc-content">{renderNode(doc, 0)}</main>;
}
