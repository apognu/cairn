import type { DocNode } from "@cairn/renderer";
import { Canvas } from "./Canvas";

/**
 * Shows a document read-only in the full site layout, to compare with the same document published. The document is read from the
 * browser's `localStorage` under `fixture:<name>`, where the comparison test puts it.
 */
export function FixtureView({ name }: { name: string }) {
  const stored = window.localStorage.getItem(`fixture:${name}`);

  return stored ? <Canvas doc={JSON.parse(stored) as DocNode} editable={false} /> : null;
}
