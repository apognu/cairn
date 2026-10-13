import { componentsByName, Tabs } from "@yourdocs/components";
import { Fragment, type ReactNode } from "react";
import type { DocMark, DocNode, RenderContext } from "./types";

export type NodeContext = {
  urls: RenderContext["urls"];
  terms?: RenderContext["terms"];
  assets: RenderContext["assets"];
  anchors: WeakMap<DocNode, string>;
};

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
    case "image": {
      // The URL and size come from the asset's metadata, so the image reserves its space before it loads.
      const asset = ctx.assets[String(attrs.assetId)];
      return (
        <figure key={key}>
          <img
            src={asset?.src ?? `/_a/${String(attrs.assetId)}`}
            // A decorative picture has an empty description on purpose; screen readers then skip it.
            alt={attrs.decorative ? "" : String(attrs.alt ?? "")}
            width={asset?.width}
            height={asset?.height}
            // The WebP copies, picked by the browser from the width it needs: the column is at most 760px wide, and a picture is
            // never shown larger than it is.
            srcSet={asset?.srcset || undefined}
            sizes={asset?.srcset ? imageSizes(asset.width) : undefined}
            // The picture's average color shows while it loads.
            style={asset?.placeholder ? { backgroundColor: asset.placeholder } : undefined}
            loading="lazy"
          />
          {attrs.caption ? <figcaption>{String(attrs.caption)}</figcaption> : null}
        </figure>
      );
    }
    case "table":
      return (
        <table key={key}>
          <tbody>{children}</tbody>
        </table>
      );
    case "steps":
      return (
        <ol key={key} className="steps">
          {children}
        </ol>
      );
    case "step":
      return (
        <li key={key} className="step">
          {children}
        </li>
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
    default: {
      const component = componentsByName.get(node.type);
      if (!component) throw new Error(`unknown node type: ${node.type}`);

      // Parsing fills defaults and drops attrs that are not props (block IDs).
      return <Fragment key={key}>{component.render(component.props.parse(attrs), children)}</Fragment>;
    }
  }
}

/** The `sizes` of an image: its own width when that is narrower than the column, else the column (760px) or the viewport. */
function imageSizes(width: number | undefined): string {
  return width && width <= 760 ? `(max-width: ${width}px) 100vw, ${width}px` : "(min-width: 800px) 760px, 100vw";
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
    case "term": {
      // A marked term shows its definition on hover and focus and leads to its entry on the Glossary page; one the glossary no longer
      // has is just its words.
      const entry = ctx.terms?.[String(attrs.term)];
      if (!entry) return inner;

      return (
        <dfn className="term">
          <a href={entry.href} data-definition={entry.definition}>
            {inner}
          </a>
        </dfn>
      );
    }
    case "link": {
      // A link by page ID follows the page: the address saved next to it is only for editors, and goes stale when the page moves.
      const href = attrs.pageId ? ctx.urls[String(attrs.pageId)] : attrs.href ? String(attrs.href) : undefined;
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
    <Tabs key={key} labels={labels} variant={node.attrs?.variant === "code" ? "code" : undefined}>
      {panels.map((p, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: panels are positional
        <Fragment key={i}>{renderNodes(p.content, ctx)}</Fragment>
      ))}
    </Tabs>
  );
}

const blockTypes = new Set([
  "doc",
  "paragraph",
  "heading",
  "bulletList",
  "orderedList",
  "listItem",
  "blockquote",
  "codeBlock",
  "text",
  "hardBreak",
  "horizontalRule",
  "image",
  "table",
  "tableRow",
  "tableHeader",
  "tableCell",
  "steps",
  "step",
  "tabs",
  "tabPanel",
]);
const markTypes = new Set(["bold", "italic", "underline", "strike", "code", "link", "term"]);

/** What would stop this document from rendering: unknown nodes and marks, and component props that do not fit their schema. */
export function validateDoc(doc: DocNode): string[] {
  const problems: string[] = [];

  const visit = (node: DocNode) => {
    const component = componentsByName.get(node.type);

    if (component) {
      const parsed = component.props.safeParse(node.attrs ?? {});
      if (!parsed.success) {
        for (const issue of parsed.error.issues) {
          problems.push(`${component.title}: ${issue.path.join(".") || "props"} ${issue.message}`);
        }
      }
    } else if (!blockTypes.has(node.type)) {
      problems.push(`unknown block "${node.type}"`);
    }

    for (const mark of node.marks ?? []) {
      if (!markTypes.has(mark.type)) problems.push(`unknown mark "${mark.type}"`);
    }
    node.content?.forEach(visit);
  };

  visit(doc);
  return problems;
}
