import type { DocNode } from "@yourdocs/renderer";

/** A string that is equal for two documents with the same content, ignoring block IDs the editor adds on its own. */
export function contentSignature(doc: DocNode): string {
  return JSON.stringify(doc, (key, value: unknown) => {
    if (key === "attrs" && value && typeof value === "object") {
      const { id: _id, ...rest } = value as Record<string, unknown>;
      return rest;
    }
    return value;
  });
}
