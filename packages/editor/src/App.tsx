import { ThemeToggle } from "@yourdocs/renderer";
import { useCallback, useEffect, useState } from "react";
import { FixtureView } from "./FixtureView";
import { Icon } from "./icons";
import { PageEditor } from "./PageEditor";
import { PageSettings } from "./PageSettings";
import { PublishDialog } from "./PublishDialog";
import { usePages, usePending, useSettings } from "./queries";
import { type SettingsSection, SettingsView, sectionTitle } from "./SettingsView";
import { Sidebar } from "./Sidebar";
import type { Page } from "./tree";
import type { SaveStatus } from "./useDraftSync";

const statusText: Record<SaveStatus, string> = {
  saved: "Saved",
  dirty: "Unsaved changes",
  saving: "Saving…",
  conflict: "Not saved: conflict",
  error: "Not saved: retrying",
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
  const [showPageSettings, setShowPageSettings] = useState(false);
  const settings = useSettings();
  // Settings replace the page tree and the canvas; the section lives in the URL like the selected page does.
  const [section, setSection] = useState<SettingsSection | null>(() => {
    const value = new URLSearchParams(window.location.hash.slice(1)).get("settings");
    return value === "redirects" || value === "library" ? value : value ? "site" : null;
  });

  // Authors see the site's accent color while they write.
  useEffect(() => {
    if (settings.data) document.documentElement.style.setProperty("--accent-base", settings.data.accent);
  }, [settings.data]);
  const [panelHost, setPanelHost] = useState<HTMLElement | null>(null);
  const current = pages.data?.find((p) => p.id === selected);
  const trail = current ? breadcrumb(pages.data ?? [], current) : [];

  const select = useCallback((id: string | null) => {
    window.history.replaceState(null, "", id ? `#page=${id}` : window.location.pathname);
    setSelected(id);
  }, []);

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
          {!section && current?.kind === "page" && (
            <button type="button" onClick={() => setShowPageSettings(true)}>
              Page settings
            </button>
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
          <main className="canvas">
            {current ? (
              <PageEditor key={current.id} page={current} onStatus={setStatus} panelHost={panelHost} />
            ) : (
              <p className="muted pad">Create a page in the sidebar to get started.</p>
            )}
          </main>
          <aside className="props" aria-label="Component settings" ref={setPanelHost} />
        </>
      )}
      {showPageSettings && current && <PageSettings page={current} onClose={() => setShowPageSettings(false)} />}
      {showPublish && <PublishDialog onClose={() => setShowPublish(false)} onPublished={setPublished} />}
    </div>
  );
}
