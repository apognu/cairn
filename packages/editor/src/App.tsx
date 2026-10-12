import { useState } from "react";
import { Canvas } from "./Canvas";
import { FixtureView } from "./FixtureView";
import { usePages } from "./queries";
import { Sidebar } from "./Sidebar";

const emptyDoc = { type: "doc", content: [{ type: "paragraph" }] };

export function App() {
  const fixture = new URLSearchParams(window.location.hash.slice(1)).get("fixture");
  if (fixture) return <FixtureView name={fixture} />;

  return <Editor />;
}

function Editor() {
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
          // Documents are not stored yet (M1.11), so every page opens an empty canvas.
          <Canvas
            key={current.id}
            doc={emptyDoc}
            context={{ page: { id: current.id, title: current.title } }}
            embedded
          />
        ) : (
          <p className="muted">Create a page in the sidebar to get started.</p>
        )}
      </main>
    </div>
  );
}
