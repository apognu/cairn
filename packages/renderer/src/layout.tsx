import { type ReactNode, useEffect, useRef } from "react";
import type { Heading } from "./headings";
import { ThemeToggle } from "./ThemeToggle";
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

type PagefindUI = new (options: { element: HTMLElement; showSubResults: boolean; showImages: boolean }) => unknown;

function load(tag: "script" | "link", url: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const element = document.createElement(tag);
    if (element instanceof HTMLScriptElement) element.src = url;
    else if (element instanceof HTMLLinkElement) {
      element.rel = "stylesheet";
      element.href = url;
    }
    element.onload = () => resolve();
    element.onerror = () => reject(new Error(`cannot load ${url}`));
    document.head.append(element);
  });
}

/** Search over the Pagefind index of the published site. Loaded after hydration, so it never disturbs the server-rendered markup. */
function SearchBox() {
  const container = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const element = container.current;
    if (!element) return;

    Promise.all([load("link", "/pagefind/pagefind-ui.css"), load("script", "/pagefind/pagefind-ui.js")])
      .then(() => {
        const UI = (window as unknown as { PagefindUI?: PagefindUI }).PagefindUI;
        if (UI && element.childElementCount === 0) new UI({ element, showSubResults: true, showImages: false });
      })
      // No index (the editor, a fixture): there is simply no search box.
      .catch(() => {});
  }, []);

  return <div className="site-search" ref={container} />;
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
            {context.site.logo && (
              <img
                className="site-logo"
                src={context.site.logo.src}
                width={context.site.logo.width}
                height={context.site.logo.height}
                alt=""
              />
            )}
            {context.site.name}
          </a>
          <SearchBox />
          <ThemeToggle />
        </header>
      )}
      <div className="site-body">
        {!embedded && (
          <nav className="site-nav" aria-label="Pages">
            <NavList nodes={context.nav} currentId={context.page.id} />
          </nav>
        )}
        <main className="doc" data-pagefind-body={context.page.id ? "" : undefined}>
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
