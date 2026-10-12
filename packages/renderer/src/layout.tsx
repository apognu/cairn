import type { ReactNode } from "react";
import type { Heading } from "./headings";
import type { NavNode, RenderContext } from "./types";

function NavList({ nodes, currentId }: { nodes: NavNode[]; currentId?: string }) {
  return (
    <ul>
      {nodes.map((node) => (
        <li key={node.id}>
          <a href={node.href} aria-current={node.id === currentId ? "page" : undefined}>
            {node.title}
          </a>
          {node.children.length > 0 && <NavList nodes={node.children} currentId={currentId} />}
        </li>
      ))}
    </ul>
  );
}

/** The site chrome around a page body: header, navigation, title and table of contents. */
export function SiteLayout({
  context,
  headings,
  embedded = false,
  children,
}: {
  context: RenderContext;
  headings: Heading[];
  /** Leaves out the site header and navigation, for the editor, which has its own. */
  embedded?: boolean;
  children: ReactNode;
}) {
  const toc = headings.filter((h) => h.level >= 2 && h.level <= 3);

  return (
    <div className="site" data-embedded={embedded || undefined}>
      {!embedded && (
        <header className="site-header">
          <a href="/" className="site-name">
            {context.site.name}
          </a>
        </header>
      )}
      <div className="site-body">
        {!embedded && (
          <nav className="site-nav" aria-label="Pages">
            <NavList nodes={context.nav} currentId={context.page.id} />
          </nav>
        )}
        <main className="doc">
          {context.page.title ? <h1 className="doc-title">{context.page.title}</h1> : null}
          <div className="doc-content">{children}</div>
        </main>
        <aside className="site-toc" aria-label="On this page">
          {toc.length > 0 && (
            <ul>
              {toc.map((h) => (
                <li key={h.anchor} data-level={h.level}>
                  <a href={`#${h.anchor}`}>{h.text}</a>
                </li>
              ))}
            </ul>
          )}
        </aside>
      </div>
    </div>
  );
}
