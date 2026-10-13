import { type ReactNode, useEffect, useId, useRef, useState } from "react";
import type { Heading } from "./headings";
import { ThemeToggle } from "./ThemeToggle";
import type { NavNode, RenderContext } from "./types";

/** The arrow after a link that leaves the site. */
function ExternalMark() {
  return (
    <svg
      className="external-mark"
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.2"
      strokeLinecap="round"
      role="img"
      aria-label="External link"
    >
      <path d="M7 17L17 7M8 7h9v9" />
    </svg>
  );
}

function NavList({ nodes, currentId }: { nodes: NavNode[]; currentId?: string }) {
  return (
    <ul>
      {nodes.map((node) => (
        <li key={node.id}>
          <a href={node.href} aria-current={node.id === currentId ? "page" : undefined}>
            {node.title}
            {node.external && <ExternalMark />}
          </a>
          {node.children.length > 0 && <NavList nodes={node.children} currentId={currentId} />}
        </li>
      ))}
    </ul>
  );
}

const months = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

/** `2026-10-03` as `3 October 2026`, spelled out here so V8 and every browser agree. */
function longDate(iso: string): string {
  const [year, month, day] = iso.split("-").map(Number);
  return month >= 1 && month <= 12 ? `${day} ${months[month - 1]} ${year}` : iso;
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

/** The section being read: the last heading scrolled past the top of the window, or the first one. */
function useActiveHeading(anchors: string[]): string | undefined {
  const [active, setActive] = useState<string | undefined>(undefined);
  const key = anchors.join(" ");

  // biome-ignore lint/correctness/useExhaustiveDependencies: `key` stands for the list of anchors
  useEffect(() => {
    if (anchors.length === 0) return;

    const update = () => {
      let current = anchors[0];
      for (const anchor of anchors) {
        const top = document.getElementById(anchor)?.getBoundingClientRect().top;
        if (top !== undefined && top <= 140) current = anchor;
      }
      setActive(current);
    };
    update();
    window.addEventListener("scroll", update, { passive: true });

    return () => window.removeEventListener("scroll", update);
  }, [key]);

  return active;
}

/** The site chrome around a page body: header, navigation, title and table of contents. */
export function SiteLayout({
  context,
  headings,
  embedded = false,
  lead,
  children,
}: {
  context: RenderContext;
  headings: Heading[];
  /** Leaves out the site header and navigation, for the editor, which has its own. */
  embedded?: boolean;
  /** Replaces the lead drawn from the page's description: the editor puts a field there to write it in place. */
  lead?: ReactNode;
  children: ReactNode;
}) {
  const toc = headings.filter((h) => h.level >= 2 && h.level <= 3);
  // On narrow screens the navigation is a panel opened from the header; wide screens always show it.
  const [navOpen, setNavOpen] = useState(false);
  const navId = useId();
  const active = useActiveHeading(embedded ? [] : toc.map((h) => h.anchor));
  const [tocOpen, setTocOpen] = useState(false);

  useEffect(() => {
    if (!navOpen) return;

    const onKey = (event: KeyboardEvent) => event.key === "Escape" && setNavOpen(false);
    document.addEventListener("keydown", onKey);
    document.documentElement.style.overflow = "hidden";

    return () => {
      document.removeEventListener("keydown", onKey);
      document.documentElement.style.overflow = "";
    };
  }, [navOpen]);

  return (
    <div className="site" data-embedded={embedded || undefined} data-nav-open={navOpen || undefined}>
      {!embedded && (
        <header className="site-header">
          <button
            type="button"
            className="nav-toggle"
            aria-label="Menu"
            aria-expanded={navOpen}
            aria-controls={navId}
            onClick={() => setNavOpen(!navOpen)}
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d={navOpen ? "M6 6l12 12M18 6L6 18" : "M4 7h16M4 12h16M4 17h16"} strokeLinecap="round" />
            </svg>
          </button>
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
          {context.header.length > 0 && (
            <nav className="site-sections" aria-label="Sections">
              {context.header.map((item) => (
                <a key={item.id} href={item.href} aria-current={item.id === context.section ? "page" : undefined}>
                  {item.title}
                </a>
              ))}
            </nav>
          )}
          <SearchBox />
          <ThemeToggle />
        </header>
      )}
      {!embedded && toc.length > 0 && (
        // Narrow screens have no column for the table of contents: it folds into a bar under the header.
        <details className="toc-bar" open={tocOpen} onToggle={(e) => setTocOpen(e.currentTarget.open)}>
          <summary>
            <span className="muted">On this page:</span>
            <strong>{toc.find((h) => h.anchor === active)?.text ?? toc[0].text}</strong>
          </summary>
          <ul>
            {toc.map((h) => (
              <li key={h.anchor} data-level={h.level}>
                <a
                  href={`#${h.anchor}`}
                  aria-current={h.anchor === active ? "location" : undefined}
                  onClick={() => setTocOpen(false)}
                >
                  {h.text}
                </a>
              </li>
            ))}
          </ul>
        </details>
      )}
      <div className="site-body">
        {!embedded && (
          <nav className="site-nav" id={navId} aria-label="Pages">
            {context.header.length > 0 && (
              <ul className="site-nav-sections">
                {context.header.map((item) => (
                  <li key={item.id}>
                    <a href={item.href} aria-current={item.id === context.section ? "page" : undefined}>
                      {item.title}
                    </a>
                  </li>
                ))}
              </ul>
            )}
            <NavList nodes={context.nav} currentId={context.page.id} />
          </nav>
        )}
        <main className="doc" data-pagefind-body={context.page.id ? "" : undefined}>
          {!embedded && context.breadcrumbs.length > 0 && (
            <nav className="breadcrumbs" aria-label="Breadcrumb" data-pagefind-ignore="">
              {context.breadcrumbs.map((crumb) => (
                <span key={crumb.href}>
                  <a href={crumb.href}>{crumb.title}</a>
                  <span aria-hidden="true"> / </span>
                </span>
              ))}
              <span>{context.page.title}</span>
            </nav>
          )}
          {context.page.title ? <h1 className="doc-title">{context.page.title}</h1> : null}
          {lead ?? (context.page.description ? <p className="doc-lead">{context.page.description}</p> : null)}
          {!embedded && (context.updated || context.markdown) && (
            <div className="doc-meta" data-pagefind-ignore="">
              {context.updated && <span>Last updated {longDate(context.updated)}</span>}
              {context.markdown && <a href={context.markdown}>View as Markdown</a>}
            </div>
          )}
          <div className="doc-content">{children}</div>
          {!embedded && (context.previous || context.next) && (
            <nav className="pager" aria-label="Previous and next page" data-pagefind-ignore="">
              {context.previous ? (
                <a href={context.previous.href} rel="prev">
                  <span>Previous</span>
                  {context.previous.title}
                </a>
              ) : (
                <span />
              )}
              {context.next ? (
                <a href={context.next.href} rel="next" data-side="next">
                  <span>Next</span>
                  {context.next.title}
                </a>
              ) : (
                <span />
              )}
            </nav>
          )}
        </main>
        <aside className="site-toc" aria-label="On this page">
          {toc.length > 0 && (
            <ul>
              {toc.map((h) => (
                <li key={h.anchor} data-level={h.level}>
                  <a href={`#${h.anchor}`} aria-current={h.anchor === active ? "location" : undefined}>
                    {h.text}
                  </a>
                </li>
              ))}
            </ul>
          )}
        </aside>
      </div>
    </div>
  );
}
