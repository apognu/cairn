import type { Editor } from "@tiptap/core";
import { closeHistory } from "@tiptap/pm/history";
import { NodeSelection } from "@tiptap/pm/state";
import { useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { ImageEditor } from "./ImageEditor";
import { MediaGrid } from "./MediaGrid";
import { type Asset, assetLabel } from "./media";
import { useModal } from "./sidePanel";
import * as ui from "./ui";
import { uploadImage } from "./upload";

export type MediaPickerRequest =
  /** Insert the chosen image at `at`. */
  | { mode: "insert"; at: number }
  /** Show the chosen image in the image node at `at`, keeping its alt text and caption. */
  | { mode: "replace"; at: number };

export const MEDIA_PICKER_EVENT = "cairn:media-picker";

/** Asks the canvas to open the media library to choose an image. */
export function openMediaPicker(editor: Editor, request: MediaPickerRequest): void {
  editor.view.dom.dispatchEvent(new CustomEvent<MediaPickerRequest>(MEDIA_PICKER_EVENT, { detail: request }));
}

/** Puts the chosen image in the page, as an undo step of its own even right after another change. */
function apply(editor: Editor, request: MediaPickerRequest, asset: Asset): void {
  const chain = editor
    .chain()
    .focus()
    .command(({ tr }) => {
      closeHistory(tr);
      return true;
    });

  if (request.mode === "insert") {
    chain.insertContentAt(request.at, { type: "image", attrs: { assetId: asset.id, alt: "", caption: "" } }).run();
    return;
  }

  chain
    .command(({ tr }) => {
      const node = tr.doc.nodeAt(request.at);
      if (node?.type.name !== "image") return false;
      tr.setNodeMarkup(request.at, undefined, { ...node.attrs, assetId: asset.id });
      tr.setSelection(NodeSelection.create(tr.doc, request.at));
      return true;
    })
    .run();
}

/** Listens for picker requests from the slash menu and the image settings, and shows the picker. */
export function MediaPickerHost({ editor }: { editor: Editor }) {
  const [request, setRequest] = useState<MediaPickerRequest | null>(null);

  useEffect(() => {
    const dom = editor.view.dom;
    const onRequest = (event: Event) => setRequest((event as CustomEvent<MediaPickerRequest>).detail);
    dom.addEventListener(MEDIA_PICKER_EVENT, onRequest);

    return () => dom.removeEventListener(MEDIA_PICKER_EVENT, onRequest);
  }, [editor]);

  if (!request) return null;

  // Outside the canvas, whose document styles (headings, images) would apply inside the dialog.
  return createPortal(
    <MediaPicker
      replacing={request.mode === "replace"}
      onChoose={(asset) => apply(editor, request, asset)}
      onClose={() => {
        setRequest(null);
        editor.commands.focus();
      }}
    />,
    document.body,
  );
}

/** The media library in a dialog: choose an image already uploaded, or upload one. */
function MediaPicker({
  replacing,
  onChoose,
  onClose,
}: {
  replacing: boolean;
  onChoose: (asset: Asset) => void;
  onClose: () => void;
}) {
  const id = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const [chosen, setChosen] = useState<Asset | null>(null);
  const [editing, setEditing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useModal(dialog);

  const choose = (asset: Asset) => {
    onChoose(asset);
    dialog.current?.close();
  };

  return (
    <dialog
      ref={dialog}
      className={`${ui.dialogBase} w-[min(880px,calc(100vw-32px))] max-h-[calc(100vh-64px)] overflow-y-auto`}
      aria-labelledby={`${id}-title`}
      onClose={onClose}
    >
      <h2 id={`${id}-title`} className={ui.dialogTitle}>
        {replacing ? "Replace the image" : "Choose an image"}
      </h2>
      <MediaGrid selected={chosen?.id ?? null} onChoose={setChosen} onPick={choose}>
        {/* The browser's own file input cannot be styled; it stays reachable by keyboard, drawn as the button after it. */}
        <input
          id={`${id}-file`}
          className="peer sr-only"
          type="file"
          accept="image/png,image/jpeg,image/gif,image/webp"
          onChange={(e) => {
            const file = e.target.files?.[0];
            e.target.value = "";
            if (file)
              uploadImage(file).then(choose, (err: Error) => {
                setError(err.message);
              });
          }}
        />
        <label
          htmlFor={`${id}-file`}
          className={`${ui.secondary} inline-flex items-center flex-none font-medium peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-accent`}
        >
          Upload
        </label>
      </MediaGrid>
      {error && (
        <p role="alert" className="text-danger">
          {error}
        </p>
      )}
      <div
        className={`${ui.actions} sticky bottom-0 items-center -mx-6 -mb-6 mt-4 py-3 px-6 border-t border-border bg-bg`}
      >
        <span className="mr-auto text-muted text-help truncate">{chosen ? assetLabel(chosen) : "No image chosen"}</span>
        <button type="button" className={ui.dialogButton} onClick={() => dialog.current?.close()}>
          Cancel
        </button>
        <button
          type="button"
          className={ui.dialogButton}
          disabled={!chosen || chosen.variants?.length === 0}
          onClick={() => setEditing(true)}
        >
          Crop & annotate…
        </button>
        <button type="button" className={ui.primary} disabled={!chosen} onClick={() => chosen && choose(chosen)}>
          {replacing ? "Replace" : "Insert"}
        </button>
      </div>
      {editing && chosen && (
        <ImageEditor
          asset={chosen}
          saveLabel={replacing ? "Save and replace" : "Save and insert"}
          onSaved={choose}
          onClose={() => setEditing(false)}
        />
      )}
    </dialog>
  );
}
