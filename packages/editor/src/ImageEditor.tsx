import { type KeyboardEvent, type PointerEvent, useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import {
  between,
  contains,
  HANDLES,
  type Handle,
  handleAt,
  handlePoint,
  MIN_CROP,
  moved,
  type Point,
  type Rect,
  resized,
  rounded,
} from "./imageEdit";
import { type Asset, assetLabel, useRefreshAssets } from "./media";
import { useModal } from "./sidePanel";
import * as ui from "./ui";
import { uploadImage } from "./upload";

/** How close to a handle, in screen pixels, a press grabs it. */
const REACH = 10;
const HANDLE_SIZE = 8;

type Drag =
  | { kind: "new"; start: Point }
  | { kind: "move"; start: Point; from: Rect }
  | { kind: "resize"; handle: Handle };

const cursors: Record<Handle, string> = {
  nw: "nwse-resize",
  se: "nwse-resize",
  ne: "nesw-resize",
  sw: "nesw-resize",
  n: "ns-resize",
  s: "ns-resize",
  e: "ew-resize",
  w: "ew-resize",
};

/**
 * Crops an image and saves the result as a new image made from it, kept in the media library. The original never changes. Opens over
 * whatever asked for it, outside the canvas so document styles stay out.
 */
export function ImageEditor({
  asset,
  saveLabel = "Save as copy",
  onSaved,
  onClose,
}: {
  asset: Asset;
  saveLabel?: string;
  onSaved: (copy: Asset) => void;
  onClose: () => void;
}) {
  const id = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const drag = useRef<Drag | null>(null);
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [crop, setCrop] = useState<Rect | null>(null);
  const [cursor, setCursor] = useState("crosshair");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const refresh = useRefreshAssets();

  useModal(dialog);

  useEffect(() => {
    const element = new Image();
    element.src = `/_a/${asset.id}`;
    element.decode().then(
      () => {
        setImage(element);
        setCrop({ x: 0, y: 0, w: element.naturalWidth, h: element.naturalHeight });
      },
      () => setError("The image could not be loaded."),
    );
  }, [asset.id]);

  useEffect(() => {
    const element = canvas.current;
    const context = element?.getContext("2d");
    if (!element || !context || !image || !crop) return;

    draw(context, image, crop, element.width / element.getBoundingClientRect().width);
  }, [image, crop]);

  const size = image ? { width: image.naturalWidth, height: image.naturalHeight } : { width: 0, height: 0 };
  const whole = !crop || (crop.x === 0 && crop.y === 0 && crop.w === size.width && crop.h === size.height);

  /** The pointer in the image's pixels; the canvas is drawn at the image's size and shown smaller. */
  const toImage = (event: PointerEvent<HTMLCanvasElement>): { point: Point; scale: number } => {
    const box = event.currentTarget.getBoundingClientRect();
    const scale = event.currentTarget.width / box.width;
    return { point: { x: (event.clientX - box.left) * scale, y: (event.clientY - box.top) * scale }, scale };
  };

  const onPointerDown = (event: PointerEvent<HTMLCanvasElement>) => {
    if (!crop) return;
    const { point, scale } = toImage(event);
    const handle = handleAt(crop, point, REACH * scale);

    drag.current = handle
      ? { kind: "resize", handle }
      : contains(crop, point) && !whole
        ? { kind: "move", start: point, from: crop }
        : { kind: "new", start: point };
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const onPointerMove = (event: PointerEvent<HTMLCanvasElement>) => {
    if (!crop) return;
    const { point, scale } = toImage(event);
    const current = drag.current;

    if (!current) {
      const handle = handleAt(crop, point, REACH * scale);
      setCursor(handle ? cursors[handle] : contains(crop, point) && !whole ? "move" : "crosshair");
      return;
    }

    if (current.kind === "resize") setCrop(resized(crop, current.handle, point, size));
    else if (current.kind === "move")
      setCrop(moved(current.from, point.x - current.start.x, point.y - current.start.y, size));
    else setCrop(between(current.start, point, size));
  };

  const onPointerUp = () => {
    const current = drag.current;
    drag.current = null;
    // A click without a drag draws no crop: the whole image stays.
    if (current?.kind === "new" && crop && (crop.w < MIN_CROP || crop.h < MIN_CROP))
      setCrop({ x: 0, y: 0, w: size.width, h: size.height });
  };

  const onKeyDown = (event: KeyboardEvent<HTMLCanvasElement>) => {
    const step = event.shiftKey ? 10 : 1;
    const delta: Record<string, Point> = {
      ArrowLeft: { x: -step, y: 0 },
      ArrowRight: { x: step, y: 0 },
      ArrowUp: { x: 0, y: -step },
      ArrowDown: { x: 0, y: step },
    };
    const by = delta[event.key];
    if (!by || !crop) return;
    event.preventDefault();

    // Alt moves the bottom right corner, which grows or shrinks the crop; otherwise the crop moves.
    if (event.altKey) setCrop(resized(crop, "se", { x: crop.x + crop.w + by.x, y: crop.y + crop.h + by.y }, size));
    else setCrop(moved(crop, by.x, by.y, size));
  };

  const save = async () => {
    if (!image || !crop) return;
    setSaving(true);
    setError(null);
    try {
      const file = await exportImage(image, rounded(crop), asset);
      const copy = await uploadImage(file, { keep: true, from: asset.id });
      await refresh();
      onSaved(copy);
      dialog.current?.close();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const shown = crop ? rounded(crop) : null;

  return createPortal(
    <dialog
      ref={dialog}
      className={`${ui.dialogBase} w-[min(1100px,calc(100vw-32px))] max-h-[calc(100vh-32px)] overflow-y-auto`}
      aria-labelledby={`${id}-title`}
      onClose={onClose}
    >
      <div className="flex flex-wrap items-center gap-2 mb-3.5">
        <h2 id={`${id}-title`} className="min-w-0 my-0 mr-auto text-[20px] tracking-[-0.01em] truncate">
          Edit “{assetLabel(asset)}”
        </h2>
        <button
          type="button"
          className={ui.secondary}
          disabled={whole}
          onClick={() => setCrop({ x: 0, y: 0, w: size.width, h: size.height })}
        >
          Reset crop
        </button>
      </div>
      <p className="mt-0 mb-3 text-muted text-help">
        Drag across the image to crop it, then drag the frame or its handles. With the image focused, the arrow keys
        move the frame and Alt with the arrows resizes it. The original stays as it is: saving makes a new image.
      </p>

      <div className="flex items-center justify-center min-h-60 p-3 rounded-lg bg-hover">
        {image ? (
          <canvas
            ref={canvas}
            width={size.width}
            height={size.height}
            className="block max-w-full max-h-[60vh] touch-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
            style={{ cursor }}
            tabIndex={0}
            role="img"
            aria-label={`${assetLabel(asset)}, cropped to ${shown?.w} × ${shown?.h} pixels`}
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerUp}
            onPointerCancel={onPointerUp}
            onKeyDown={onKeyDown}
          />
        ) : (
          !error && <span className="text-muted">Loading…</span>
        )}
      </div>

      {error && (
        <p role="alert" className="text-danger">
          {error}
        </p>
      )}

      <div className={`${ui.actions} flex-wrap items-center mt-4 whitespace-nowrap`}>
        <span className="mr-auto text-muted text-help" aria-live="polite">
          {shown && (whole ? `${shown.w} × ${shown.h} px` : `Cropped to ${shown.w} × ${shown.h} px`)}
        </span>
        <button type="button" className={ui.dialogButton} onClick={() => dialog.current?.close()}>
          Cancel
        </button>
        <button type="button" className={ui.primary} disabled={whole || saving} onClick={save}>
          {saving ? "Saving…" : saveLabel}
        </button>
      </div>
    </dialog>,
    document.body,
  );
}

/** The image with what lies outside the crop dimmed, and the crop's frame and handles at a constant size on screen. */
function draw(context: CanvasRenderingContext2D, image: HTMLImageElement, crop: Rect, scale: number) {
  const { naturalWidth: width, naturalHeight: height } = image;
  context.clearRect(0, 0, width, height);
  context.drawImage(image, 0, 0);

  // Canvas pixels are not themed: the dimming and the frame read on any picture, light or dark.
  context.fillStyle = "rgb(0 0 0 / 50%)";
  context.fillRect(0, 0, width, crop.y);
  context.fillRect(0, crop.y + crop.h, width, height - crop.y - crop.h);
  context.fillRect(0, crop.y, crop.x, crop.h);
  context.fillRect(crop.x + crop.w, crop.y, width - crop.x - crop.w, crop.h);

  context.lineWidth = scale;
  context.strokeStyle = "white";
  context.strokeRect(crop.x, crop.y, crop.w, crop.h);

  const side = HANDLE_SIZE * scale;
  context.fillStyle = "white";
  context.strokeStyle = "rgb(0 0 0 / 60%)";
  for (const handle of HANDLES) {
    const at = handlePoint(crop, handle);
    context.fillRect(at.x - side / 2, at.y - side / 2, side, side);
    context.strokeRect(at.x - side / 2, at.y - side / 2, side, side);
  }
}

/** The cropped image as a file: JPEG photos stay JPEG, everything else becomes PNG so transparency survives. */
function exportImage(image: HTMLImageElement, crop: Rect, asset: Asset): Promise<File> {
  const output = document.createElement("canvas");
  output.width = crop.w;
  output.height = crop.h;
  output.getContext("2d")?.drawImage(image, crop.x, crop.y, crop.w, crop.h, 0, 0, crop.w, crop.h);

  const jpeg = asset.mime === "image/jpeg";
  const type = jpeg ? "image/jpeg" : "image/png";
  const base = (asset.name ?? "image").replace(/\.[^.]+$/, "");
  const name = `${base}-edited.${jpeg ? "jpg" : "png"}`;

  return new Promise((resolve, reject) =>
    output.toBlob(
      (blob) => (blob ? resolve(new File([blob], name, { type })) : reject(new Error("The image could not be saved."))),
      type,
      0.92,
    ),
  );
}
