import { Icon } from "@cairn/components";
import { useEffect, useId, useRef, useState } from "react";
import type { RenderContext } from "./types";

/** The arrow after a link that leaves the site. */
export function ExternalMark() {
  return (
    <svg
      className="ml-1.5 text-muted"
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

export function isExternal(url: string): boolean {
  return /^(https?:)?\/\//.test(url);
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

/**
 * Search over the Pagefind index of the published site. Loaded after hydration, so it never disturbs the server-rendered markup. On a
 * narrow screen the header has no room for the field: it lives in a panel over the page, which the header's search button opens.
 */
export function SearchBox({ afterLinks, open, onClose }: { afterLinks: boolean; open: boolean; onClose: () => void }) {
  const container = useRef<HTMLDivElement>(null);
  const isOpen = useRef(open);
  isOpen.current = open;

  useEffect(() => {
    const element = container.current;
    if (!element) return;

    Promise.all([load("link", "/pagefind/pagefind-ui.css"), load("script", "/pagefind/pagefind-ui.js")])
      .then(() => {
        const UI = (window as unknown as { PagefindUI?: PagefindUI }).PagefindUI;
        if (UI && element.childElementCount === 0) new UI({ element, showSubResults: true, showImages: false });
        // The panel may have been opened before the field existed.
        if (isOpen.current) element.querySelector("input")?.focus();
      })
      // No index (the editor, a fixture): there is simply no search box.
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (open) container.current?.querySelector("input")?.focus();
  }, [open]);

  // Both want the free space of the header: the links take it, and the search follows them. The panel around it is no box on a wide
  // screen.
  return (
    <search
      className={`search-panel contents ${open ? "tablet:flex tablet:flex-col tablet:fixed tablet:inset-0 tablet:z-30 tablet:py-3 tablet:px-4 tablet:bg-bg tablet:overflow-y-auto" : "tablet:hidden"}`}
      aria-label="Search the documentation"
    >
      {open && (
        <button
          type="button"
          className={`${headerButton} hidden tablet:flex self-end flex-none mb-2`}
          aria-label="Close search"
          onClick={onClose}
        >
          <svg
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <path d="M6 6l12 12M18 6L6 18" strokeLinecap="round" />
          </svg>
        </button>
      )}
      <div
        className={`site-search relative w-[min(260px,40vw)] ${afterLinks ? "ml-0" : "ml-auto"} tablet:w-full tablet:ml-0`}
        ref={container}
      />
    </search>
  );
}

/** The header's square buttons beside the theme switch: search and the menu of links on a narrow screen. */
export const headerButton =
  "flex-none items-center justify-center w-8.5 h-8.5 p-0 border border-input rounded-lg bg-bg text-text cursor-pointer hover:bg-hover";

/**
 * The header's links on a narrow screen, where they have no room in it: a menu under a button. It closes on a click elsewhere and on
 * Escape, which gives the focus back to its button. The header's sections go to the navigation menu instead.
 */
export function HeaderMenu({ context }: { context: RenderContext }) {
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  const id = useId();
  const links = context.site.headerLinks ?? [];

  useEffect(() => {
    if (!open) return;

    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setOpen(false);
      button.current?.focus();
    };
    const onPointer = (event: PointerEvent) => {
      if (!(event.target as Element | null)?.closest(".site-menu")) setOpen(false);
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("pointerdown", onPointer);

    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("pointerdown", onPointer);
    };
  }, [open]);

  const item = "flex items-center gap-1 py-2 px-3 rounded-md text-body text-[15px] no-underline hover:bg-hover";

  return (
    <div className="site-menu relative hidden tablet:block ml-2">
      <button
        ref={button}
        type="button"
        className={`${headerButton} flex`}
        aria-label="Links"
        aria-expanded={open}
        aria-controls={id}
        onClick={() => setOpen(!open)}
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
          <circle cx="5" cy="12" r="1.8" />
          <circle cx="12" cy="12" r="1.8" />
          <circle cx="19" cy="12" r="1.8" />
        </svg>
      </button>
      {open && (
        <div
          id={id}
          className="absolute top-11 right-0 z-30 flex flex-col w-max min-w-56 max-w-[calc(100vw-32px)] p-1.5 border border-border rounded-xl bg-bg shadow-popover"
        >
          {links.length > 0 && (
            <nav className="flex flex-col" aria-label="Links">
              {links.map((link) => (
                <a key={link.url + link.label} href={link.url} className={item}>
                  {link.icon && (
                    <span className="flex mr-1.5 text-muted">
                      <Icon name={link.icon} size={18} />
                    </span>
                  )}
                  {link.label}
                  {isExternal(link.url) && <ExternalMark />}
                </a>
              ))}
            </nav>
          )}
        </div>
      )}
    </div>
  );
}
