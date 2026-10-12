import { useState } from "react";
import { usePages } from "./queries";
import { Sidebar } from "./Sidebar";

export function App() {
  const pages = usePages();
  const [selected, setSelected] = useState<string | null>(null);
  const current = pages.data?.find((p) => p.id === selected);

  return (
    <div className="shell">
      <header className="topbar">
        <strong>yourdocs</strong>
        <span className="muted">{current?.title ?? "No page selected"}</span>
      </header>
      <Sidebar pages={pages.data ?? []} loading={pages.isPending} selected={selected} onSelect={setSelected} />
      <main className="canvas">
        {current ? (
          <>
            <h1>{current.title}</h1>
            <p className="muted">/{current.slug}</p>
          </>
        ) : (
          <p className="muted">Create a page in the sidebar to get started.</p>
        )}
      </main>
    </div>
  );
}
