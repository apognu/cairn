import { useQuery } from "@tanstack/react-query";
import { ThemeToggle } from "@yourdocs/renderer";
import { useCallback, useEffect, useState } from "react";
import { ApiSettings } from "./ApiSettings";
import { FixtureView } from "./FixtureView";
import { HistoryDrawer } from "./HistoryDrawer";
import { Icon } from "./icons";
import { PageEditor } from "./PageEditor";
import { PageSettings } from "./PageSettings";
import { PublishDialog } from "./PublishDialog";
import { usePages, usePending, useSettings, useSiteInfo } from "./queries";
import { type SettingsSection, SettingsView, sectionTitle } from "./SettingsView";
import { Sidebar } from "./Sidebar";
import { firstPage, type Page, pageUrls } from "./tree";
import type { SaveStatus } from "./useDraftSync";

const statusText: Record<SaveStatus, string> = {
  saved: "Saved",
  dirty: "Unsaved changes",
  saving: "Saving…",
  conflict: "Not saved: conflict",
  // Retried on its own when the server or the network failed; the browser copy keeps the changes in every case.
  error: "Not saved: kept in this browser",
};

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
  const pages = usePages();
  // The selected page lives in the URL, so a reload comes back to it.
  const [selected, setSelected] = useState(() => new URLSearchParams(window.location.hash.slice(1)).get("page"));
  const [status, setStatus] = useState<SaveStatus | null>(null);
  const pending = usePending();
  const [published, setPublished] = useState<number | null>(null);
  const [showPublish, setShowPublish] = useState(false);
  // One side panel at a time; its button opens and closes it.
  const [panel, setPanel] = useState<"history" | "settings" | null>(null);
  const toggle = (next: "history" | "settings") => setPanel((open) => (open === next ? null : next));
  // Bumped when a version is restored, so the editor loads the new draft instead of keeping the old text.
  const [restores, setRestores] = useState(0);
  const settings = useSettings();
  // Settings replace the page tree and the canvas; the section lives in the URL like the selected page does.
  const [section, setSection] = useState<SettingsSection | null>(() => {
    const value = new URLSearchParams(window.location.hash.slice(1)).get("settings");
    return value === "redirects" || value === "library" || value === "trash" ? value : value ? "site" : null;
  });

  // Authors write inside the site's colors: the saved theme is applied to the whole editor, as the published pages get it.
  const themeCss = useQuery({ queryKey: ["theme-css"], queryFn: async () => (await fetch("/api/theme.css")).text() });
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
  const change = pending.data?.changes.find((c) => c.page_id === current?.id);
  const liveUrl = current && (change ? change.old_url : pageUrls(pages.data ?? [])[current.id]);

  const openSettings = (next: SettingsSection | null) => {
    window.history.replaceState(
      null,
      "",
      next ? `#settings=${next}` : selected ? `#page=${selected}` : window.location.pathname,
    );
    setSection(next);
  };

  return (
    <div className="shell">
      <header className="topbar">
        <span className="brand">
          <Icon name="logo" size={20} />
          <strong>{settings.data?.site_name ?? "Documentation"}</strong>
        </span>
        <span className="crumbs">
          {section ? `Settings / ${sectionTitle[section]}` : trail.length > 0 ? trail.join(" / ") : "No page selected"}
        </span>
        {status && (
          <output className="save-status" data-status={status}>
            {status === "saved" && <Icon name="check" size={13} />}
            {statusText[status]}
          </output>
        )}
        <div className="topbar-actions">
          {published !== null && <span className="muted">Published {published} pages</span>}
          <ThemeToggle />
          {!section && current && current.kind !== "link" && (
            <>
              {liveUrl && site.data ? (
                <a className="button" href={`${site.data.url}${liveUrl}`} target="_blank" rel="noopener">
                  View as reader
                </a>
              ) : (
                <button type="button" disabled title="This page has not been published yet">
                  View as reader
                </button>
              )}
              {current.kind === "page" && (
                <button type="button" aria-pressed={panel === "history"} onClick={() => toggle("history")}>
                  History
                </button>
              )}
              <button type="button" aria-pressed={panel === "settings"} onClick={() => toggle("settings")}>
                Page settings
              </button>
            </>
          )}
          <button type="button" onClick={() => openSettings("site")}>
            Settings
          </button>
          <button
            type="button"
            className="primary"
            onClick={() => {
              setPublished(null);
              setShowPublish(true);
            }}
          >
            {pending.data && pending.data.changes.length > 0 ? `Publish · ${pending.data.changes.length}` : "Publish"}
          </button>
        </div>
      </header>
      {section ? (
        <SettingsView section={section} onSection={openSettings} onClose={() => openSettings(null)} />
      ) : (
        <>
          <Sidebar pages={pages.data ?? []} loading={pages.isPending} selected={selected} onSelect={select} />
          <main className="canvas" data-wide={current?.kind === "api" || undefined}>
            {current?.kind === "api" ? (
              <div className="settings-main">
                <ApiSettings key={current.id} page={current} />
              </div>
            ) : current ? (
              <PageEditor key={`${current.id}-${restores}`} page={current} onStatus={setStatus} panelHost={panelHost} />
            ) : (
              !pages.isPending && (
                <p className="muted pad">
                  {pages.data?.length
                    ? "Choose a page in the sidebar to edit it."
                    : "Create a page in the sidebar to get started."}
                </p>
              )
            )}
          </main>
          {current?.kind !== "api" && <aside className="props" aria-label="Component settings" ref={setPanelHost} />}
        </>
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
