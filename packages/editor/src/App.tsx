import { useCallback, useState } from "react";
import { FixtureView } from "./FixtureView";
import { PageEditor } from "./PageEditor";
import { usePages } from "./queries";
import { Sidebar } from "./Sidebar";
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

function Editor() {
  const pages = usePages();
  // The selected page lives in the URL, so a reload comes back to it.
  const [selected, setSelected] = useState(() => new URLSearchParams(window.location.hash.slice(1)).get("page"));
  const [status, setStatus] = useState<SaveStatus | null>(null);
  const [panelHost, setPanelHost] = useState<HTMLElement | null>(null);
  const current = pages.data?.find((p) => p.id === selected);

  const select = useCallback((id: string | null) => {
    window.history.replaceState(null, "", id ? `#page=${id}` : window.location.pathname);
    setSelected(id);
  }, []);

  return (
    <div className="shell">
      <header className="topbar">
        <strong>yourdocs</strong>
        <span className="muted">{current?.title ?? "No page selected"}</span>
        {status && <output className="muted">{statusText[status]}</output>}
      </header>
      <Sidebar pages={pages.data ?? []} loading={pages.isPending} selected={selected} onSelect={select} />
      <main className="canvas">
        {current ? (
          <PageEditor key={current.id} page={current} onStatus={setStatus} panelHost={panelHost} />
        ) : (
          <p className="muted pad">Create a page in the sidebar to get started.</p>
        )}
      </main>
      <aside className="props" aria-label="Component settings" ref={setPanelHost} />
    </div>
  );
}
