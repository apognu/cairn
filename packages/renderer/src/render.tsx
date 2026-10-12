import { Callout, Tabs } from "@yourdocs/components";
import { Fragment, type ReactNode } from "react";
import type { DocMark, DocNode, RenderContext } from "./types";

export type NodeContext = { urls: RenderContext["urls"]; anchors: WeakMap<DocNode, string> };

export function renderNodes(nodes: DocNode[] = [], ctx: NodeContext): ReactNode[] {
  return nodes.map((node, i) => renderNode(node, ctx, i));
}

/** Renders one document node. The editor's built-in nodes must produce the same tags and classes. */
export function renderNode(node: DocNode, ctx: NodeContext, key: number): ReactNode {
  // Panels belong to `tabs` and are not standalone nodes, so it renders its own children.
  if (node.type === "tabs") return renderTabs(node, ctx, key);
  if (node.type === "text") return renderText(node, ctx, key);

  const children = renderNodes(node.content, ctx);
  const attrs = node.attrs ?? {};

  switch (node.type) {
    case "doc":
      return <div key={key}>{children}</div>;
    case "paragraph": {
      // ProseMirror shows an empty paragraph as one blank line, and a line ending in a hard break as a new line, by adding a
      // trailing <br>; the published page must too, since a lone trailing <br> collapses.
      const last = node.content?.at(-1);
      const trailing = !last || last.type === "hardBreak";
      return (
        <p key={key}>
          {children}
          {trailing ? <br /> : null}
        </p>
      );
    }
    case "heading": {
      const Tag = `h${Math.min(Math.max(Number(attrs.level ?? 1), 1), 6)}` as "h1";
      return (
        <Tag key={key} id={ctx.anchors.get(node)}>
          {children}
        </Tag>
      );
    }
    case "bulletList":
      return <ul key={key}>{children}</ul>;
    case "orderedList":
      return (
        <ol key={key} start={attrs.start === undefined || attrs.start === 1 ? undefined : Number(attrs.start)}>
          {children}
        </ol>
      );
    case "listItem":
      return <li key={key}>{children}</li>;
    case "blockquote":
      return <blockquote key={key}>{children}</blockquote>;
    case "codeBlock": {
      const language = attrs.language ? String(attrs.language) : undefined;
      return (
        <pre key={key} data-file-name={attrs.fileName ? String(attrs.fileName) : undefined}>
          {/* The server colors this block after rendering (syntect), so React must not reconcile its contents when hydrating. */}
          <code
            className={language ? `language-${language}` : undefined}
            suppressHydrationWarning
            // biome-ignore lint/security/noDangerouslySetInnerHtml: the text is escaped by escapeHtml
            dangerouslySetInnerHTML={{ __html: escapeHtml(textOf(node)) }}
          />
        </pre>
      );
    }
    case "hardBreak":
      return <br key={key} />;
    case "horizontalRule":
      return <hr key={key} />;
    case "image":
      return (
        <figure key={key}>
          {/* TODO(M1.14): resolve through asset metadata (srcset, size); the original is enough until uploads exist */}
          <img src={`/_a/${String(attrs.assetId)}`} alt={String(attrs.alt ?? "")} loading="lazy" />
          {attrs.caption ? <figcaption>{String(attrs.caption)}</figcaption> : null}
        </figure>
      );
    case "table":
      return (
        <table key={key}>
          <tbody>{children}</tbody>
        </table>
      );
    case "tableRow":
      return <tr key={key}>{children}</tr>;
    case "tableHeader":
    case "tableCell": {
      const Cell = node.type === "tableHeader" ? "th" : "td";
      return (
        <Cell key={key} colSpan={spanOf(attrs.colspan)} rowSpan={spanOf(attrs.rowspan)}>
          {children}
        </Cell>
      );
    }
    case "callout":
      return (
        <Callout key={key} tone={attrs.tone ? String(attrs.tone) : undefined}>
          {children}
        </Callout>
      );
    default:
      throw new Error(`unknown node type: ${node.type}`);
  }
}

function textOf(node: DocNode): string {
  return node.text ?? (node.content ?? []).map(textOf).join("");
}

function escapeHtml(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function spanOf(value: unknown): number | undefined {
  const n = Number(value ?? 1);
  return n > 1 ? n : undefined;
}

function renderText(node: DocNode, ctx: NodeContext, key: number): ReactNode {
  // Marks are listed outermost first, so wrap from the last one inwards.
  const wrapped = (node.marks ?? []).reduceRight<ReactNode>(
    (inner, mark) => wrapMark(inner, mark, ctx),
    node.text ?? "",
  );

  return <Fragment key={key}>{wrapped}</Fragment>;
}

function wrapMark(inner: ReactNode, mark: DocMark, ctx: NodeContext): ReactNode {
  const attrs = mark.attrs ?? {};

  switch (mark.type) {
    case "bold":
      return <strong>{inner}</strong>;
    case "italic":
      return <em>{inner}</em>;
    case "underline":
      return <u>{inner}</u>;
    case "strike":
      return <s>{inner}</s>;
    case "code":
      return <code>{inner}</code>;
    case "link": {
      const byId = attrs.pageId ? ctx.urls[String(attrs.pageId)] : undefined;
      const href = byId ?? (attrs.href ? String(attrs.href) : undefined);
      const external = href?.startsWith("http");
      // A link to a page without a URL (unpublished or deleted) renders as plain text, never a dead link.
      if (!href) return inner;

      return (
        <a
          href={attrs.anchor ? `${href}#${String(attrs.anchor)}` : href}
          rel={external ? "noopener noreferrer" : undefined}
        >
          {inner}
        </a>
      );
    }
    default:
      throw new Error(`unknown mark type: ${mark.type}`);
  }
}

function renderTabs(node: DocNode, ctx: NodeContext, key: number): ReactNode {
  const panels = node.content ?? [];
  const labels = panels.map((p) => String(p.attrs?.label ?? ""));

  return (
    <Tabs key={key} labels={labels}>
      {panels.map((p, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: panels are positional
        <Fragment key={i}>{renderNodes(p.content, ctx)}</Fragment>
      ))}
    </Tabs>
  );
}
