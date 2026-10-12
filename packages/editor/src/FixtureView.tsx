import type { DocNode } from "@yourdocs/renderer";
import { useEffect, useState } from "react";
import { Canvas } from "./Canvas";

/** Shows a sample document read-only in the full site layout, to compare with its published render (`/docs/<name>`). */
export function FixtureView({ name }: { name: string }) {
  const [doc, setDoc] = useState<DocNode | null>(null);

  useEffect(() => {
    fetch(`/api/docs/${encodeURIComponent(name)}`)
      .then((r) => r.json())
      .then(setDoc);
  }, [name]);

  return doc ? <Canvas doc={doc} editable={false} /> : null;
}
