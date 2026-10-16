import { customSlots } from "@cairn/components";
import type { SlotDefinition, SlotName } from "@cairn/sdk";
import { createElement, type ReactNode, useEffect, useId, useRef, useState } from "react";
import { ExternalMark, HeaderMenu, headerButton, isExternal, SearchBox } from "./header";
import type { Heading } from "./headings";
import { ThemeToggle } from "./ThemeToggle";
import type { NavNode, RenderContext, SiteImage } from "./types";

type Slots = Partial<Record<SlotName, SlotDefinition>>;

/** A part of the layout a deployer may replace: the slot gets the default as its children. Rendered as an element so it can use hooks. */
function Slot({
  name,
  slots,
  context,
  children,
}: {
  name: SlotName;
  slots: Slots;
  context: RenderContext;
  children?: ReactNode;
}) {
  const slot = slots[name];

  if (!slot) return children ?? null;

  return createElement(slot.render, { context }, children ?? null);
}

function SiteLogo({ image, mode }: { image: SiteImage; mode?: "light" | "dark" }) {
  return (
    <img
      // With both logos, each shows only in its own mode.
      className={`site-logo h-6 w-auto ${mode === "dark" ? "hidden dark:block" : mode === "light" ? "dark:hidden" : ""}`}
      data-mode={mode}
      src={image.src}
      width={image.width}
      height={image.height}
      alt=""
    />
  );
}

/** A short, stable name for an announcement, so dismissing one does not hide the next. */
function bannerId(banner: { text: string; url: string }): string {
  let hash = 5381;
  for (const char of `${banner.text}\n${banner.url}`) hash = ((hash * 33) ^ char.charCodeAt(0)) >>> 0;
  return hash.toString(36);
}

function SiteBanner({ banner }: { banner: NonNullable<RenderContext["site"]["banner"]> }) {
  const id = bannerId(banner);

  return (
    <div
      // The open menu hangs under the header at a fixed height: the banner steps aside while it is open.
      className="site-banner flex items-center gap-3 py-2 px-4 bg-accent-soft text-text text-ui in-data-nav-open:hidden"
      data-pagefind-ignore=""
    >
      {/* The head script marks a dismissed announcement on <html> before the first paint; this rule hides it then, with no flash. */}
      <style>{`html[data-banner-dismissed="${id}"] .site-banner{display:none}`}</style>
      <p className="flex-1 m-0 text-center">
        {banner.url ? (
          <a href={banner.url} className="text-text underline">
            {banner.text}
          </a>
        ) : (
          banner.text
        )}
      </p>
      <button
        type="button"
        className="flex items-center justify-center w-7 h-7 p-0 border-0 rounded-md bg-transparent text-muted cursor-pointer hover:bg-hover hover:text-text"
        aria-label="Dismiss the announcement"
        onClick={() => {
          document.documentElement.dataset.bannerDismissed = id;
          try {
            localStorage.setItem("cairn-banner", id);
          } catch {
            // Storage can be blocked: the banner then only stays hidden on this page.
          }
        }}
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          aria-hidden="true"
        >
          <path d="M6 6l12 12M18 6L6 18" />
        </svg>
      </button>
    </div>
  );
}

function SiteFooter({ footer }: { footer: NonNullable<RenderContext["site"]["footer"]> }) {
  if (!footer.text && footer.columns.length === 0) return null;

  return (
    <footer className="site-footer py-8 px-4 border-t border-border bg-side text-muted text-ui" data-pagefind-ignore="">
      {footer.columns.length > 0 && (
        <div className="flex flex-wrap gap-x-16 gap-y-8 mb-5">
          {footer.columns.map((column) => (
            <nav key={column.title} aria-label={column.title}>
              <h2 className="mt-0 mb-2 text-text text-help font-semibold">{column.title}</h2>
              <ul className="m-0 p-0 list-none">
                {column.links.map((link) => (
                  <li key={link.url + link.label} className="my-1 mx-0">
                    <a href={link.url} className="text-muted">
                      {link.label}
                    </a>
                  </li>
                ))}
              </ul>
            </nav>
          ))}
        </div>
      )}
      {footer.text && <p className="m-0">{footer.text}</p>}
    </footer>
  );
}

/** A link of the navigation panel; `look` is its display and anything that sets it apart. */
const navLink = (look = "block") =>
  `${look} my-0.5 mx-0 py-1 px-2.5 rounded-md text-body no-underline hover:bg-hover aria-[current=page]:bg-hover aria-[current=page]:font-semibold tablet:py-1.75`;
const navList = "list-none m-0 p-0";

function NavList({ nodes, currentId, nested = false }: { nodes: NavNode[]; currentId?: string; nested?: boolean }) {
  return (
    <ul className={`${navList} ${nested ? "pl-3.5" : ""}`}>
      {nodes.map((node) => (
        <li key={node.id}>
          <a
            href={node.href}
            className={navLink(node.badge ? "flex items-center gap-2" : "block")}
            aria-current={node.id === currentId ? "page" : undefined}
          >
            {node.title}
            {node.external && <ExternalMark />}
            {node.badge && (
              <span className="ml-auto px-1.75 rounded-full bg-[color-mix(in_srgb,var(--accent)_14%,var(--bg))] text-accent text-[11px] font-semibold leading-[18px]">
                {node.badge}
              </span>
            )}
          </a>
          {node.children.length > 0 && <NavList nodes={node.children} currentId={currentId} nested />}
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

/** An image of the page shown as large as the window allows, over a dimmed page. Any click or Escape closes it. */
function ImageZoom({ image, onClose }: { image: { src: string; alt: string }; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);

  useEffect(() => dialog.current?.showModal(), []);

  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: Escape closes a modal dialog by itself
    <dialog
      ref={dialog}
      className="image-zoom"
      aria-label={image.alt || "Image"}
      onClose={onClose}
      onClick={() => dialog.current?.close()}
    >
      <img src={image.src} alt={image.alt} />
    </dialog>
  );
}

/**
 * The navigation as a panel over the page, under the header, while the menu button has it open on a narrow screen. A grid item keeps its
 * own alignment even when fixed: "start" would size the panel to its content.
 */
const navPanel =
  "tablet:block tablet:fixed tablet:self-stretch tablet:inset-x-0 tablet:top-13 tablet:bottom-0 tablet:z-15 tablet:h-auto tablet:pt-3 tablet:px-4 tablet:pb-8 tablet:border-r-0 tablet:text-[15px]";
const pagerLink =
  "block py-3.5 px-4.5 border border-border rounded-[10px] text-text font-semibold no-underline hover:border-accent";
const pagerLabel = "block text-muted text-help font-normal";

/** The page title's look, shared with the editor's field that renames the page in place. */
export const docTitleClass = "doc-title mt-0 mb-4 text-[40px] leading-[1.15] tracking-[-0.025em]";

/** The site chrome around a page body: header, navigation, title and table of contents. */
export function SiteLayout({
  context,
  headings,
  embedded = false,
  title,
  lead,
  slots = customSlots,
  children,
}: {
  context: RenderContext;
  headings: Heading[];
  /** Leaves out the site header and navigation, for the editor, which has its own. */
  embedded?: boolean;
  /** Replaces the page's title heading: the editor puts a field there to rename the page in place. */
  title?: ReactNode;
  /** Replaces the lead drawn from the page's description: the editor puts a field there to write it in place. */
  lead?: ReactNode;
  /** The layout parts to replace; the deployer's by default. */
  slots?: Slots;
  children: ReactNode;
}) {
  const toc = headings.filter((h) => h.level >= 2 && h.level <= 3);
  // On narrow screens the navigation is a panel opened from the header; wide screens always show it.
  const [navOpen, setNavOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const searchToggle = useRef<HTMLButtonElement>(null);
  const navId = useId();
  const navToggle = useRef<HTMLButtonElement>(null);
  const active = useActiveHeading(embedded ? [] : toc.map((h) => h.anchor));
  const [tocOpen, setTocOpen] = useState(false);
  const [zoomed, setZoomed] = useState<{ src: string; alt: string } | null>(null);

  useEffect(() => {
    if (!navOpen) return;

    // Escape hands the focus back to the button that opened the panel, which a keyboard user would otherwise lose.
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setNavOpen(false);
      navToggle.current?.focus();
    };
    // The toggle closes the panel with its own click.
    const onPointer = (event: PointerEvent) => {
      const target = event.target as Element | null;
      if (target && !target.closest(".site-nav, .nav-toggle")) setNavOpen(false);
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("pointerdown", onPointer);
    document.documentElement.style.overflow = "hidden";

    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("pointerdown", onPointer);
      document.documentElement.style.overflow = "";
    };
  }, [navOpen]);

  const closeSearch = () => {
    setSearchOpen(false);
    searchToggle.current?.focus();
  };
  useEffect(() => {
    if (!searchOpen) return;

    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setSearchOpen(false);
      searchToggle.current?.focus();
    };
    document.addEventListener("keydown", onKey);
    document.documentElement.style.overflow = "hidden";

    return () => {
      document.removeEventListener("keydown", onKey);
      document.documentElement.style.overflow = "";
    };
  }, [searchOpen]);

  // One listener for every image of the body, so the rendered markup stays the same with or without script. The original file, not
  // the copy sized for the column, is the one worth enlarging.
  useEffect(() => {
    if (embedded) return;

    const onClick = (event: MouseEvent) => {
      const image = (event.target as Element | null)?.closest(".doc-content figure img");
      if (image instanceof HTMLImageElement)
        setZoomed({ src: image.getAttribute("src") ?? image.currentSrc, alt: image.alt });
    };
    document.addEventListener("click", onClick);

    return () => document.removeEventListener("click", onClick);
  }, [embedded]);

  const api = Boolean(context.api);
  const headerLinks = (context.site.headerLinks ?? []).length > 0;
  const brand = (
    <>
      {context.site.logo && <SiteLogo image={context.site.logo} mode={context.site.logoDark ? "light" : undefined} />}
      {context.site.logoDark && <SiteLogo image={context.site.logoDark} mode="dark" />}
      {context.site.name}
    </>
  );
  // The page tree and the table of contents beside the reading column; an API reference uses the whole width beside the navigation.
  // In the editor (embedded) there is no tree, and the editor narrows it further itself.
  const columns = embedded
    ? "grid-cols-[minmax(0,1fr)_var(--toc-width)]"
    : `${api ? "grid-cols-[var(--nav-width)_minmax(0,1fr)]" : "grid-cols-[var(--nav-width)_minmax(0,1fr)_var(--toc-width)]"} tablet:grid-cols-[minmax(0,1fr)]`;

  return (
    <div
      // The side columns widen only where the reading column keeps its full width (808px, plus both of them).
      className="site [--nav-width:280px] [--toc-width:240px] compact:[--nav-width:250px] compact:[--toc-width:200px]"
      data-embedded={embedded || undefined}
      data-api={context.api ? "" : undefined}
      data-nav-open={navOpen || undefined}
    >
      {!embedded && context.site.banner && <SiteBanner banner={context.site.banner} />}
      {!embedded && (
        <Slot name="Header" slots={slots} context={context}>
          <header className="site-header sticky top-0 z-20 flex items-center h-13 px-4 border-b border-border bg-bg">
            <button
              ref={navToggle}
              type="button"
              className="nav-toggle hidden tablet:flex items-center justify-center w-9 h-9 mr-2 -ml-2 p-0 border-0 rounded-lg bg-transparent text-text cursor-pointer hover:bg-hover"
              aria-label="Menu"
              aria-expanded={navOpen}
              aria-controls={navId}
              onClick={() => {
                setSearchOpen(false);
                setNavOpen(!navOpen);
              }}
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
            {/* On a narrow screen the name moves to the top of the navigation panel, and the sections and links into a menu. */}
            <a href="/" className="site-name flex items-center gap-2 font-bold text-text no-underline tablet:hidden">
              {brand}
            </a>
            {context.header.length > 0 && (
              <nav className="site-sections flex self-stretch gap-1 ml-6 tablet:hidden" aria-label="Sections">
                {context.header.map((item) => (
                  <a
                    key={item.id}
                    href={item.href}
                    className="flex items-center px-3 text-muted text-[14.5px] no-underline aria-[current=page]:text-text aria-[current=page]:font-semibold aria-[current=page]:shadow-[inset_0_-2px_0_var(--accent)]"
                    aria-current={item.id === context.section ? "page" : undefined}
                  >
                    {item.title}
                  </a>
                ))}
              </nav>
            )}
            {context.site.headerLinks && headerLinks && (
              <nav className="site-links flex gap-1 ml-auto pr-2 tablet:hidden" aria-label="Links">
                {context.site.headerLinks.map((link) => (
                  <a
                    key={link.url + link.label}
                    href={link.url}
                    className="flex items-center gap-1 py-1.5 px-2.5 rounded-md text-muted text-[14.5px] no-underline hover:bg-hover hover:text-text"
                  >
                    {link.label}
                    {isExternal(link.url) && <ExternalMark />}
                  </a>
                ))}
              </nav>
            )}
            <SearchBox afterLinks={headerLinks} open={searchOpen} onClose={closeSearch} />
            <button
              ref={searchToggle}
              type="button"
              className={`search-toggle ${headerButton} hidden tablet:flex ml-auto`}
              aria-label="Search"
              aria-expanded={searchOpen}
              onClick={() => {
                setNavOpen(false);
                setSearchOpen(true);
              }}
            >
              <svg
                width="17"
                height="17"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                aria-hidden="true"
              >
                <circle cx="11" cy="11" r="7" />
                <path d="M20 20l-3.5-3.5" />
              </svg>
            </button>
            {(context.header.length > 0 || headerLinks) && <HeaderMenu context={context} />}
            <ThemeToggle className="w-8.5 h-8.5 ml-3 tablet:ml-2" />
          </header>
        </Slot>
      )}
      {!embedded && toc.length > 0 && (
        // Narrow screens have no column for the table of contents: it folds into a bar under the header.
        <details
          className="toc-bar group/toc hidden tablet:block sticky top-13 z-10 border-b border-border bg-side text-ui"
          open={tocOpen}
          onToggle={(e) => setTocOpen(e.currentTarget.open)}
        >
          {/* A chevron after the heading turns when the list is open. */}
          <summary className="flex items-center gap-1.5 py-2.5 px-4 list-none cursor-pointer [&::-webkit-details-marker]:hidden after:content-[''] after:flex-none after:w-[7px] after:h-[7px] after:-mt-[3px] after:mr-0.5 after:ml-auto after:border-r-[1.5px] after:border-b-[1.5px] after:border-muted after:rotate-45 group-open/toc:after:mt-[3px] group-open/toc:after:-rotate-135">
            <span className="text-muted">On this page:</span>
            <strong className="overflow-hidden text-ellipsis whitespace-nowrap">
              {toc.find((h) => h.anchor === active)?.text ?? toc[0].text}
            </strong>
          </summary>
          <ul className="max-h-[50vh] overflow-y-auto m-0 pt-0 px-4 pb-3 list-none">
            {toc.map((h) => (
              <li key={h.anchor} className={h.level === 3 ? "pl-3" : undefined} data-level={h.level}>
                <a
                  href={`#${h.anchor}`}
                  className="block py-1.5 px-0 text-body no-underline aria-[current=location]:text-accent aria-[current=location]:font-semibold"
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
      <div className={`site-body grid ${columns}`}>
        {!embedded && (
          <nav
            // Stays in view and fills the height under the header; on narrow screens it opens as a panel over the page.
            className={`site-nav sticky top-13 self-start h-[calc(100vh-52px)] overflow-y-auto py-3 px-2 border-r border-border bg-side text-ui ${navOpen ? navPanel : "tablet:hidden"}`}
            id={navId}
            aria-label="Pages"
          >
            {/* The header has no room for the site's name on a narrow screen: the panel opens with it, unless the way back from a
                section already names the site. */}
            {context.sectionBack?.title !== context.site.name && (
              <a
                href="/"
                className="site-nav-brand hidden tablet:flex items-center gap-2 mb-3 pt-1 pb-3 px-2.5 border-b border-border font-bold text-text text-[17px] no-underline"
              >
                {brand}
              </a>
            )}
            {context.sectionBack && (
              // A section's sidebar lists only that section: this is the way back to the rest of the site.
              <a
                className="site-nav-back flex items-center gap-1.5 mt-0.5 mx-0 mb-2 pt-1 tablet:pt-1.75 pb-2 px-2.5 border-b border-border text-muted text-help no-underline hover:text-body"
                href={context.sectionBack.href}
              >
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  aria-hidden="true"
                >
                  <path d="M15 18l-6-6 6-6" />
                </svg>
                {context.sectionBack.title}
              </a>
            )}
            <NavList nodes={context.nav} currentId={context.page.id} />
            <Slot name="SidebarFooter" slots={slots} context={context} />
          </nav>
        )}
        <main
          className={`doc w-full mx-auto ${api ? "max-w-none p-0" : "max-w-[808px] py-10 px-12 tablet:py-6 tablet:px-4"}`}
          data-pagefind-body={context.page.id ? "" : undefined}
        >
          {/* Scalar has its own title, so an API reference hides both (the title is still indexed for search). */}
          {!embedded && context.breadcrumbs.length > 0 && (
            <nav
              className={`breadcrumbs ${api ? "hidden" : "flex"} flex-wrap gap-x-1.5 mb-2 leading-[22px] text-muted text-ui`}
              aria-label="Breadcrumb"
              data-pagefind-ignore=""
            >
              {context.breadcrumbs.map((crumb) => (
                <span key={crumb.href}>
                  <a className="text-muted no-underline hover:text-text" href={crumb.href}>
                    {crumb.title}
                  </a>
                  <span aria-hidden="true"> / </span>
                </span>
              ))}
              <span className="text-body">{context.page.title}</span>
            </nav>
          )}
          {title ??
            (context.page.title ? (
              <h1 className={`${docTitleClass} ${api ? "hidden" : ""}`}>{context.page.title}</h1>
            ) : null)}
          {lead ?? (context.page.description ? <p className="doc-lead">{context.page.description}</p> : null)}
          {!embedded && (context.updated || context.markdown) && (
            // Whole-pixel line heights keep everything below at whole-pixel positions, so text is drawn the same wherever the page is
            // compared.
            <div
              className="doc-meta flex flex-wrap items-center gap-x-4 gap-y-1 mb-2 pb-5 border-b border-border leading-[22px] text-muted text-[13.5px]"
              data-pagefind-ignore=""
            >
              {context.updated && <span>Last updated {longDate(context.updated)}</span>}
              {context.markdown && (
                <a className="ml-auto text-muted" href={context.markdown}>
                  View as Markdown
                </a>
              )}
            </div>
          )}
          <div className="doc-content">
            {context.notFound ? (
              <Slot name="NotFound" slots={slots} context={context}>
                {children}
              </Slot>
            ) : (
              children
            )}
          </div>
          {!embedded && (context.previous || context.next) && (
            <nav
              className="pager grid grid-cols-2 gap-3.5 mt-10"
              aria-label="Previous and next page"
              data-pagefind-ignore=""
            >
              {context.previous ? (
                <a href={context.previous.href} rel="prev" className={pagerLink}>
                  <span className={pagerLabel}>Previous</span>
                  {context.previous.title}
                </a>
              ) : (
                <span />
              )}
              {context.next ? (
                <a
                  href={context.next.href}
                  rel="next"
                  className={`${pagerLink} col-start-2 text-right`}
                  data-side="next"
                >
                  <span className={pagerLabel}>Next</span>
                  {context.next.title}
                </a>
              ) : (
                <span />
              )}
            </nav>
          )}
          {!embedded && <Slot name="PageFooter" slots={slots} context={context} />}
        </main>
        <aside
          // Stays in view while the page scrolls under it.
          className={`site-toc sticky top-13 self-start max-h-[calc(100vh-52px)] overflow-y-auto py-10 px-4 text-[13.5px] ${api ? "hidden" : "tablet:hidden"}`}
          aria-label="On this page"
        >
          {toc.length > 0 && (
            <ul className={navList}>
              {toc.map((h) => (
                <li key={h.anchor} className={h.level === 3 ? "pl-3" : undefined} data-level={h.level}>
                  <a
                    href={`#${h.anchor}`}
                    className="block py-0.5 px-0 text-muted no-underline aria-[current=location]:text-accent aria-[current=location]:font-semibold"
                    aria-current={h.anchor === active ? "location" : undefined}
                  >
                    {h.text}
                  </a>
                </li>
              ))}
            </ul>
          )}
        </aside>
      </div>
      {!embedded && (
        <Slot name="Footer" slots={slots} context={context}>
          {context.site.footer && <SiteFooter footer={context.site.footer} />}
        </Slot>
      )}
      {zoomed && <ImageZoom image={zoomed} onClose={() => setZoomed(null)} />}
    </div>
  );
}
