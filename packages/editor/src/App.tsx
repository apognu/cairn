import { ThemeToggle } from "@cairn/renderer";
import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ApiSettings } from "./ApiSettings";
import { serverUrl } from "./api/base";
import { useEditorEvents } from "./events";
import { FixtureView } from "./FixtureView";
import { HistoryDrawer } from "./HistoryDrawer";
import { Icon } from "./icons";
import { useCloseMenus } from "./menus";
import { PageEditor, type PageStatus } from "./PageEditor";
import { PageSettings } from "./PageSettings";
import { PublishDialog } from "./PublishDialog";
import { pendingCount, useCan, usePages, usePending, useSettings, useSiteInfo } from "./queries";
import { type SettingsSection, SettingsView, sectionTitle } from "./SettingsView";
import { Sidebar } from "./Sidebar";
import { SideDrawer, useModal } from "./sidePanel";
import { firstPage, type Page, pageUrls } from "./tree";
import { UserMenu } from "./UserMenu";
import * as ui from "./ui";
import { useMobile, useNarrow } from "./useNarrow";

const statusText: Record<PageStatus, string> = {
  saved: "Saved",
  dirty: "Unsaved changes",
  saving: "Saving…",
  conflict: "Not saved: conflict",
  // Retried on its own when the server or the network failed; the browser copy keeps the changes in every case.
  error: "Not saved: kept in this browser",
  "read-only": "Read-only",
};

const statusColor: Partial<Record<PageStatus, string>> = {
  conflict: "text-danger",
  error: "text-danger",
  "read-only": "text-warn-icon",
};

/** Where the page actions and the Settings button are: the top bar, the side column over the block settings, or the narrow menu. */
type ActionsAt = "bar" | "column" | "menu";

const actionLook: Record<ActionsAt, string> = {
  bar: `inline-flex items-center gap-1.5 h-8 px-3 rounded-[7px] text-ui font-medium no-underline ${ui.button}`,
  column: `inline-flex flex-[1_1_0] items-center justify-center gap-1.5 min-w-0 h-8 px-2 rounded-[7px] text-[13.5px] font-medium no-underline whitespace-nowrap ${ui.button}`,
  menu: "flex items-center gap-2 py-1.5 px-2.5 border-0 rounded-[5px] bg-transparent text-text text-ui text-left no-underline cursor-pointer hover:bg-hover aria-pressed:bg-hover disabled:text-muted disabled:cursor-default",
};
const actionIcon = "flex-none text-muted";

export function App() {
  const fixture = new URLSearchParams(window.location.hash.slice(1)).get("fixture");
  if (fixture) return <FixtureView name={fixture} />;

  return <Editor />;
}

/** Titles from the top of the tree down to the page. */
function breadcrumb(pages: Page[], page: Page): string[] {
  const trail = [page.title];

  for (
    let parent = pages.find((p) => p.id === page.parent_id);
    parent;
    parent = pages.find((p) => p.id === parent?.parent_id)
  ) {
    trail.unshift(parent.title);
  }
  return trail;
}

function Editor() {
  useCloseMenus();
  useEditorEvents();
  const pages = usePages();
  // The selected page lives in the URL, so a reload comes back to it.
  const [selected, setSelected] = useState(() => new URLSearchParams(window.location.hash.slice(1)).get("page"));
  const [status, setStatus] = useState<PageStatus | null>(null);
  const pending = usePending();
  const [published, setPublished] = useState<number | null>(null);
  const [showPublish, setShowPublish] = useState(false);
  // One side panel at a time; its button opens and closes it.
  const [panel, setPanel] = useState<"history" | "settings" | null>(null);
  const [sideOpen, setSideOpen] = useState(false);
  // On a phone the page tree is a drawer: open from its button in the top bar, closed once something is picked in it.
  const [navOpen, setNavOpen] = useState(false);
  const navButton = useRef<HTMLButtonElement>(null);
  // A page panel opens beside the page, so the side column's drawer (which may hold the button) makes way for it.
  const toggle = (next: "history" | "settings") => {
    setSideOpen(false);
    setPanel((open) => (open === next ? null : next));
  };
  // Bumped when a version is restored, so the editor loads the new draft instead of keeping the old text.
  const [restores, setRestores] = useState(0);
  const settings = useSettings();
  // Settings replace the page tree and the canvas; the section lives in the URL like the selected page does.
  const [section, setSection] = useState<SettingsSection | null>(() => {
    const value = new URLSearchParams(window.location.hash.slice(1)).get("settings");
    if (value === null) return null;
    return Object.hasOwn(sectionTitle, value) ? (value as SettingsSection) : "site";
  });

  // Authors write inside the site's colors: the saved theme is applied to the whole editor, as the published pages get it.
  const themeCss = useQuery({
    queryKey: ["theme-css"],
    queryFn: async () => (await fetch(serverUrl("/api/theme.css"))).text(),
  });
  useEffect(() => {
    const style =
      document.head.querySelector("style#site-theme") ??
      document.head.appendChild(Object.assign(document.createElement("style"), { id: "site-theme" }));
    style.textContent = themeCss.data ?? "";
  }, [themeCss.data]);
  const [panelHost, setPanelHost] = useState<HTMLElement | null>(null);
  const current = pages.data?.find((p) => p.id === selected);
  const trail = current ? breadcrumb(pages.data ?? [], current) : [];

  const select = useCallback((id: string | null) => {
    window.history.replaceState(null, "", id ? `#page=${id}` : window.location.pathname);
    setSelected(id);
  }, []);

  // There is always a page open when there is one: on arrival, and after the open page was deleted.
  // A page just created is selected before the list that holds it comes back: only a settled list says a page is really gone.
  useEffect(() => {
    if (section || !pages.data || pages.isFetching || current) return;
    const first = firstPage(pages.data);
    if (first) select(first.id);
  }, [section, pages.data, pages.isFetching, current, select]);

  // The page as readers see it now: its live address, which is the old one while a move or rename is pending, and none before its
  // first publish.
  const site = useSiteInfo();
  const canEdit = useCan("editor");
  const canPublish = useCan("publisher");
  const canAdmin = useCan("admin");
  const change = pending.data?.changes.find((c) => c.page_id === current?.id);
  const liveUrl = current && (change ? change.old_url : pageUrls(pages.data ?? [])[current.id]);

  const openSettings = (next: SettingsSection | null) => {
    window.history.replaceState(
      null,
      "",
      next ? `#settings=${next}` : selected ? `#page=${selected}` : window.location.pathname,
    );
    setSection(next);
    if (next) setNavOpen(false);
  };

  const narrow = useNarrow();
  const mobile = useMobile();
  const nav = mobile && navOpen;
  const pick = useCallback(
    (id: string | null) => {
      setNavOpen(false);
      select(id);
    },
    [select],
  );
  useEffect(() => {
    if (!nav) return;
    document.querySelector<HTMLElement>(".sidebar :is(input, button)")?.focus();

    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      setNavOpen(false);
      navButton.current?.focus();
    };
    document.addEventListener("keydown", onKey);

    return () => document.removeEventListener("keydown", onKey);
  }, [nav]);
  // Actions on the open page: a header over the settings column when it is shown, the top bar (or its menu) otherwise.
  const pageActions = (at: ActionsAt) =>
    !section &&
    current &&
    current.kind !== "link" && (
      <>
        {liveUrl && site.data ? (
          <a
            className={`${actionLook[at]} hover:bg-hover`}
            href={`${site.data.url}${liveUrl}`}
            target="_blank"
            rel="noopener"
          >
            <Icon name="eye" size={15} className={actionIcon} />
            View as reader
          </a>
        ) : (
          <button type="button" className={actionLook[at]} disabled title="This page has not been published yet">
            <Icon name="eye" size={15} className={actionIcon} />
            View as reader
          </button>
        )}
        {current.kind === "page" && (
          <button
            type="button"
            className={actionLook[at]}
            aria-pressed={panel === "history"}
            onClick={() => toggle("history")}
          >
            <Icon name="history" size={15} className={actionIcon} />
            History
          </button>
        )}
        <button
          type="button"
          className={actionLook[at]}
          aria-pressed={panel === "settings"}
          onClick={() => toggle("settings")}
        >
          <Icon name="sliders" size={15} className={actionIcon} />
          Page settings
        </button>
      </>
    );
  const hasPageActions = Boolean(pageActions("bar"));
  const sideColumn = current?.kind !== "api";
  const sideDrawer = useMemo(() => ({ open: () => setSideOpen(true) }), []);
  const actionsInColumn = sideColumn && !narrow && hasPageActions;
  const siteSettings = (at: ActionsAt) => (
    <button type="button" className={actionLook[at]} onClick={() => openSettings(canAdmin ? "site" : "library")}>
      <Icon name="gear" size={15} className={actionIcon} />
      Settings
    </button>
  );

  return (
    // `topbar` is a hook: the side panels find the top bar. Drawers open under `--drawer-top`.
    <div
      className={`shell grid grid-rows-[52px_1fr] grid-cols-[270px_minmax(0,1fr)_var(--panel-width)] h-screen [--page-actions-height:53px] [--panel-width:420px] narrow:grid-cols-[270px_minmax(0,1fr)] mobile:grid-cols-[minmax(0,1fr)] ${actionsInColumn ? "[--drawer-top:calc(52px+var(--page-actions-height))]" : "[--drawer-top:52px]"}`}
      data-nav-open={nav || undefined}
    >
      <header className="topbar col-span-full flex items-center gap-3.5 px-4 border-b border-border text-ui leading-[1.4] mobile:gap-2.5 mobile:px-3">
        {mobile && (
          <button
            ref={navButton}
            type="button"
            className={`flex flex-none items-center justify-center w-8 h-8 p-0 rounded-[7px] ${ui.button}`}
            aria-label={section ? "Settings sections" : "Pages"}
            title={section ? "Settings sections" : "Pages"}
            aria-expanded={navOpen}
            onClick={() => setNavOpen((open) => !open)}
          >
            <Icon name="menu" size={18} className="flex-none" />
          </button>
        )}
        <span className="flex flex-none items-center gap-2.5 text-[15px]">
          <Icon name="logo" size={20} />
          <strong className="narrow:hidden">{settings.data?.site_name ?? "Documentation"}</strong>
        </span>
        {/* The breadcrumb is what gives way when the bar is short of room; the actions never wrap or shrink. */}
        <span
          className="min-w-0 overflow-hidden text-muted text-help text-ellipsis whitespace-nowrap"
          data-testid="crumbs"
        >
          {section ? `Settings / ${sectionTitle[section]}` : trail.length > 0 ? trail.join(" / ") : "No page selected"}
        </span>
        {status && (
          <output
            className={`flex flex-none items-center gap-1.25 text-[12.5px] ${statusColor[status] ?? "text-muted"}`}
            data-status={status}
          >
            {status === "saved" && <Icon name="check" size={13} className="text-accent" />}
            {status === "read-only" && <Icon name="lock" size={13} />}
            {statusText[status]}
          </output>
        )}
        <div className="flex flex-none items-center gap-3 ml-auto whitespace-nowrap">
          {published !== null && <span className="text-muted narrow:hidden">Published {published} pages</span>}
          <ThemeToggle
            className="w-8 h-8 ml-0 [&_svg]:flex-none [&_svg]:text-muted"
            look={`rounded-[7px] ${ui.button}`}
          />
          <UserMenu />
          {narrow ? (
            <details data-menu className={ui.menu}>
              <summary
                className={`flex items-center justify-center w-8 h-8 p-0 rounded-[7px] list-none [&::-webkit-details-marker]:hidden ${ui.button}`}
                aria-label="More actions"
                title="More actions"
              >
                <Icon name="more" size={18} className="flex-none" />
              </summary>
              {/* Every item leaves the menu: History and the settings open a panel, the reader view another tab. */}
              {/* biome-ignore lint/a11y/useKeyWithClickEvents: closes after the item's own click, which the keyboard triggers too */}
              {/* biome-ignore lint/a11y/noStaticElementInteractions: the items inside are the controls; this only closes the menu */}
              <div
                data-menu-pop
                className={`${ui.menuPopBase} top-9 min-w-45`}
                onClick={(e) => e.currentTarget.closest("details")?.removeAttribute("open")}
              >
                {hasPageActions && (
                  <>
                    {pageActions("menu")}
                    <hr className={ui.menuRule} />
                  </>
                )}
                {siteSettings("menu")}
              </div>
            </details>
          ) : (
            <>
              {!actionsInColumn && pageActions("bar")}
              {siteSettings("bar")}
            </>
          )}
          {/* Editors open the same dialog to deploy a preview, when the target has previews; viewers have neither. */}
          {(canPublish || (canEdit && site.data?.capabilities.preview)) && (
            <button
              type="button"
              className={ui.primary}
              onClick={() => {
                setPublished(null);
                setShowPublish(true);
              }}
            >
              {canPublish ? "Publish" : "Preview"}
              {pendingCount(pending.data) > 0 && ` · ${pendingCount(pending.data)}`}
            </button>
          )}
        </div>
      </header>
      {section ? (
        <SettingsView
          section={section}
          onSection={openSettings}
          onClose={() => openSettings(null)}
          onOpenPage={(id) => {
            setSection(null);
            pick(id);
          }}
        />
      ) : (
        <>
          <Sidebar pages={pages.data ?? []} loading={pages.isPending} selected={selected} onSelect={pick} />
          <SideDrawer.Provider value={narrow ? sideDrawer : null}>
            <main
              className={`canvas relative overflow-y-auto has-[.sticky-actions]:scroll-pb-20 ${current?.kind === "api" ? "col-[2/-1] mobile:col-[1/-1]" : ""}`}
            >
              {current?.kind === "api" ? (
                <div className="py-6.5 px-8 text-ui leading-[1.4] has-[.sticky-actions]:pb-0">
                  <ApiSettings key={current.id} page={current} />
                </div>
              ) : current ? (
                <PageEditor
                  key={`${current.id}-${restores}`}
                  page={current}
                  onStatus={setStatus}
                  panelHost={panelHost}
                  onOpen={select}
                />
              ) : (
                !pages.isPending && (
                  <p className="text-muted px-2.5">
                    {pages.data?.length
                      ? "Choose a page in the sidebar to edit it."
                      : "Create a page in the sidebar to get started."}
                  </p>
                )
              )}
            </main>
          </SideDrawer.Provider>
          {sideColumn &&
            (narrow ? (
              <SideColumnDrawer open={sideOpen} onClose={() => setSideOpen(false)}>
                {hasPageActions && (
                  <nav className={pageActionsBar} aria-label="Page actions">
                    {pageActions("column")}
                  </nav>
                )}
                <aside className={`${ui.props} flex-1`} aria-label="Component settings" ref={setPanelHost} />
              </SideColumnDrawer>
            ) : (
              <div className={`${sideColumnLook} narrow:hidden`}>
                {actionsInColumn && (
                  <nav className={pageActionsBar} aria-label="Page actions">
                    {pageActions("column")}
                  </nav>
                )}
                <aside className={`${ui.props} flex-1`} aria-label="Component settings" ref={setPanelHost} />
              </div>
            ))}
        </>
      )}
      {nav && (
        // Above the page, under the drawer: a tap beside the drawer closes it.
        <button
          type="button"
          className="fixed inset-x-0 top-13 bottom-0 z-29 p-0 border-0 bg-[rgb(16_20_22/35%)] cursor-default"
          aria-label="Close"
          tabIndex={-1}
          onClick={() => setNavOpen(false)}
        />
      )}
      {panel === "history" && current && !section && (
        <HistoryDrawer
          key={current.id}
          page={current}
          onClose={() => setPanel(null)}
          onRestored={() => setRestores((n) => n + 1)}
        />
      )}
      {panel === "settings" && current && !section && (
        <PageSettings key={current.id} page={current} onClose={() => setPanel(null)} />
      )}
      {showPublish && <PublishDialog onClose={() => setShowPublish(false)} onPublished={setPublished} />}
    </div>
  );
}

/** The right column: always there, so opening the settings of a block, Page settings or History never moves the page. */
const sideColumnLook = "side-column flex flex-col min-h-0 border-l border-border bg-side";
const pageActionsBar =
  "page-actions flex flex-none gap-1.5 h-(--page-actions-height) py-2.5 px-3 border-b border-border";

/** The side column as a modal from the right, when the window has no room for it: the same actions and settings, the same layout. */
function SideColumnDrawer({
  open,
  onClose,
  children,
}: {
  open: boolean;
  onClose: () => void;
  children: React.ReactNode;
}) {
  const dialog = useRef<HTMLDialogElement>(null);

  useModal(dialog, open);
  useEffect(() => {
    if (!open && dialog.current?.open) dialog.current.close();
    // It opens for the block's settings: their first field takes the focus, not the page actions above them.
    if (open)
      dialog.current?.querySelector<HTMLElement>(".props :is(input, select, textarea, button, summary)")?.focus();
  }, [open]);

  return (
    <dialog
      ref={dialog}
      className={`${sideColumnLook} fixed inset-y-0 right-0 left-auto w-(--panel-width) max-w-screen h-screen max-h-none m-0 p-0 border-0 border-l border-border text-text shadow-drawer mobile:w-screen mobile:border-l-0 backdrop:bg-[rgb(16_20_22/35%)] [&:not([open])]:hidden`}
      aria-label="Page and block settings"
      onClose={() => {
        onClose();
        // The drawer took the focus from the text; give it back where the cursor was.
        document.querySelector<HTMLElement>("main.canvas .ProseMirror")?.focus();
      }}
    >
      {/* On a phone it covers the whole screen: no backdrop to tap and no Escape key, so it needs its own way out. */}
      <header className="hidden mobile:flex flex-none items-center justify-between h-13 px-4 border-b border-border">
        <h2 className={ui.drawerTitle}>Page and block settings</h2>
        <button type="button" className={ui.icon} aria-label="Close" onClick={() => dialog.current?.close()}>
          ×
        </button>
      </header>
      {children}
    </dialog>
  );
}
