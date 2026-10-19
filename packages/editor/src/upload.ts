import type { Editor } from "@tiptap/core";
import { useSyncExternalStore } from "react";
import { serverUrl } from "./api/base";
import { signIn } from "./api/client";
import type { components } from "./api/schema";

type Asset = components["schemas"]["Asset"];

let inFlight = 0;
const listeners = new Set<() => void>();

function track(change: number) {
  inFlight += change;
  for (const listener of listeners) listener();
}

/** How many uploads are running, for the status shown while a large image travels. */
export function useUploads(): number {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => inFlight,
  );
}

/** How an upload is recorded: `keep` adds it to the media library for good, `from` names the image it was edited from. */
export type UploadOptions = { keep?: boolean; from?: string };

/** Sends an image to the server. The same bytes always come back as the same asset. */
export async function uploadImage(file: Blob & { name?: string }, options: UploadOptions = {}): Promise<Asset> {
  track(1);
  try {
    return await send(file, options);
  } finally {
    track(-1);
  }
}

async function send(file: Blob & { name?: string }, { keep, from }: UploadOptions): Promise<Asset> {
  const query = new URLSearchParams();
  if (file.name) query.set("name", file.name);
  if (from) query.set("from", from);
  if (keep) query.set("keep", "true");

  // The generated client has no body type for a raw file, so this one call uses fetch; the response type is still generated.
  const response = await fetch(serverUrl(`/api/assets?${query}`), {
    method: "POST",
    headers: { "Content-Type": "application/octet-stream" },
    body: file,
  });
  if (response.status === 401) signIn();
  const body: unknown = await response.json().catch(() => ({}));

  if (!response.ok) throw new Error((body as { message?: string }).message ?? `Upload failed (${response.status})`);
  return body as Asset;
}

/** Uploads each image file and inserts it as an image node at `position` (the cursor when absent). */
export async function insertImages(editor: Editor, files: File[], position?: number): Promise<void> {
  for (const file of files.filter((f) => f.type.startsWith("image/"))) {
    try {
      const asset = await uploadImage(file);
      const node = { type: "image", attrs: { assetId: asset.id, alt: "", caption: "" } };

      if (position === undefined) editor.chain().focus().insertContent(node).run();
      else editor.chain().focus().insertContentAt(position, node).run();
    } catch (error) {
      window.alert(error instanceof Error ? error.message : String(error));
    }
  }
}

/** Opens the file picker, then inserts what was chosen. */
export function pickAndInsertImages(editor: Editor): void {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = "image/png,image/jpeg,image/gif,image/webp";
  input.multiple = true;
  input.onchange = () => void insertImages(editor, Array.from(input.files ?? []));
  input.click();
}
