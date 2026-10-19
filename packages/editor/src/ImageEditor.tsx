import { type KeyboardEvent, type PointerEvent, useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { ImageEditorTools, type Style, type Tool } from "./ImageEditorTools";
import { drawScene, labelPadding, loadFonts, paint, renderCopy } from "./imageDraw";
import {
  between,
  contains,
  fontSize,
  type Handle,
  handleAt,
  type Label,
  MIN_CROP,
  moved,
  type Point,
  type Rect,
  resized,
  rounded,
  type Shape,
  shapeAt,
  shapeMoved,
  unit,
} from "./imageEdit";
import { type Asset, assetLabel, useRefreshAssets } from "./media";
import { useSettings } from "./queries";
import { useModal } from "./sidePanel";
import * as ui from "./ui";
import { uploadImage } from "./upload";

/** How close to a handle or a shape, in screen pixels, a press grabs it. */
const REACH = 10;
const DEFAULT_ACCENT = "#0b6e4f";

type Drag =
  | { kind: "new"; start: Point }
  | { kind: "move"; start: Point; from: Rect }
  | { kind: "resize"; handle: Handle }
  | { kind: "arrow"; id: number }
  | { kind: "shape"; id: number; start: Point; from: Shape };

type Snapshot = { crop: Rect; shapes: Shape[] };

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

const help: Record<Tool, string> = {
  select: "Click an arrow or a label to select it, then drag it, change its color or size, or press Delete.",
  crop: "Drag across the image to crop it, then drag the frame or its handles. With the image focused, the arrow keys move the frame and Alt with the arrows resizes it.",
  arrow: "Drag from where the arrow starts to what it points at.",
  text: "Click where the label goes and type; Enter finishes. Click a label to change its text.",
};

/**
 * Crops and annotates an image, and saves the result as a new image made from it, kept in the media library. The original never
 * changes. Opens over whatever asked for it, outside the canvas so document styles stay out.
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
  const before = useRef<Snapshot | null>(null);
  const history = useRef<Snapshot[]>([]);
  const boxes = useRef(new Map<number, Rect>());
  const nextId = useRef(1);
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [crop, setCrop] = useState<Rect>({ x: 0, y: 0, w: 0, h: 0 });
  const [shapes, setShapes] = useState<Shape[]>([]);
  const [tool, setTool] = useState<Tool>("crop");
  const [style, setStyle] = useState<Style>({ color: "accent", weight: "medium", pill: true });
  const [selected, setSelected] = useState<number | null>(null);
  const [editing, setEditing] = useState<number | null>(null);
  const [undoable, setUndoable] = useState(0);
  const [cursor, setCursor] = useState("crosshair");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const refresh = useRefreshAssets();
  const palette = { accent: useSettings().data?.accent ?? DEFAULT_ACCENT };

  useModal(dialog);

  useEffect(() => {
    const element = new Image();
    element.src = `/_a/${asset.id}`;
    element
      .decode()
      .then(() => loadFonts({ width: element.naturalWidth, height: element.naturalHeight }))
      .then(
        () => {
          setCrop({ x: 0, y: 0, w: element.naturalWidth, h: element.naturalHeight });
          setImage(element);
        },
        () => setError("The image could not be loaded."),
      );
  }, [asset.id]);

  const size = image ? { width: image.naturalWidth, height: image.naturalHeight } : { width: 0, height: 0 };
  const whole = crop.x === 0 && crop.y === 0 && crop.w === size.width && crop.h === size.height;
  const chosen = shapes.find((shape) => shape.id === selected) ?? null;
  const label = shapes.find((shape): shape is Label => shape.kind === "text" && shape.id === editing) ?? null;
  /** Image pixels per screen pixel: the canvas is drawn at the image's size and shown smaller. */
  const scale = () => {
    const element = canvas.current;
    return element ? element.width / element.getBoundingClientRect().width : 1;
  };

  useEffect(() => {
    const element = canvas.current;
    const context = element?.getContext("2d");
    if (!element || !context || !image) return;

    boxes.current = drawScene(context, {
      image,
      crop,
      shapes,
      palette: { accent: palette.accent },
      scale: element.width / element.getBoundingClientRect().width,
      showHandles: tool === "crop",
      selected,
      hidden: editing,
    });
  }, [image, crop, shapes, tool, selected, editing, palette.accent]);

  /** Keeps the state before a change, so Undo can bring it back. */
  const remember = (snapshot: Snapshot = { crop, shapes }) => {
    history.current.push(snapshot);
    setUndoable(history.current.length);
  };

  const undo = () => {
    const last = history.current.pop();
    setUndoable(history.current.length);
    if (!last) return;
    setCrop(last.crop);
    setShapes(last.shapes);
    setSelected(null);
    setEditing(null);
  };

  const update = (id: number, change: (shape: Shape) => Shape) =>
    setShapes((all) => all.map((shape) => (shape.id === id ? change(shape) : shape)));

  const remove = (id: number) => {
    remember();
    setShapes((all) => all.filter((shape) => shape.id !== id));
    setSelected(null);
  };

  /** A new style applies to the selected annotation too. */
  const changeStyle = (next: Style) => {
    setStyle(next);
    if (!chosen) return;
    remember();
    update(chosen.id, (shape) =>
      shape.kind === "text"
        ? { ...shape, color: next.color, weight: next.weight, pill: next.pill }
        : { ...shape, color: next.color, weight: next.weight },
    );
  };

  // Called by the text field losing focus and by a press on the canvas, whichever comes first: the second finds nothing to do.
  const finishEditing = () => {
    const start = before.current;
    before.current = null;
    if (!label) return;
    setEditing(null);
    if (!label.text.trim()) {
      setShapes((all) => all.filter((shape) => shape.id !== label.id));
      setSelected(null);
    }
    // A new label left empty leaves nothing to undo.
    if (start && (label.text.trim() || start.shapes.some((shape) => shape.id === label.id))) remember(start);
  };

  const toImage = (event: PointerEvent<HTMLCanvasElement>): Point => {
    const box = event.currentTarget.getBoundingClientRect();
    const ratio = event.currentTarget.width / box.width;
    return { x: (event.clientX - box.left) * ratio, y: (event.clientY - box.top) * ratio };
  };

  const onPointerDown = (event: PointerEvent<HTMLCanvasElement>) => {
    if (!image) return;
    // The first press after typing a label only ends it.
    if (label) {
      finishEditing();
      return;
    }
    const point = toImage(event);
    const reach = REACH * scale();
    before.current = { crop, shapes };
    event.currentTarget.setPointerCapture(event.pointerId);

    if (tool === "crop") {
      const handle = handleAt(crop, point, reach);
      drag.current = handle
        ? { kind: "resize", handle }
        : contains(crop, point) && !whole
          ? { kind: "move", start: point, from: crop }
          : { kind: "new", start: point };
      return;
    }

    const hit = shapeAt(shapes, point, reach, size, boxes.current);
    if (tool === "text") {
      // The text field that opens keeps the focus.
      event.preventDefault();
      if (hit?.kind === "text") {
        setSelected(hit.id);
        setEditing(hit.id);
        return;
      }
      // The label is placed with the press at the middle of its first line, where the text cursor shows.
      const height = fontSize(style.weight, size) * 1.25;
      const pad = labelPadding(fontSize(style.weight, size));
      const created: Label = {
        id: nextId.current++,
        kind: "text",
        at: { x: point.x - pad.x, y: point.y - height / 2 - pad.y },
        text: "",
        ...style,
      };
      setShapes((all) => [...all, created]);
      setSelected(created.id);
      setEditing(created.id);
      return;
    }

    if (tool === "arrow") {
      const created: Shape = {
        id: nextId.current++,
        kind: "arrow",
        from: point,
        to: point,
        color: style.color,
        weight: style.weight,
      };
      setShapes((all) => [...all, created]);
      setSelected(created.id);
      drag.current = { kind: "arrow", id: created.id };
      return;
    }

    setSelected(hit?.id ?? null);
    if (hit) drag.current = { kind: "shape", id: hit.id, start: point, from: hit };
  };

  const onPointerMove = (event: PointerEvent<HTMLCanvasElement>) => {
    if (!image) return;
    const point = toImage(event);
    const current = drag.current;

    if (!current) {
      const reach = REACH * scale();
      const handle = tool === "crop" ? handleAt(crop, point, reach) : null;
      const movable =
        tool === "select"
          ? shapeAt(shapes, point, reach, size, boxes.current) !== null
          : tool === "crop" && contains(crop, point) && !whole;
      const idle: Record<Tool, string> = { select: "default", crop: "crosshair", arrow: "crosshair", text: "text" };
      setCursor(handle ? cursors[handle] : movable ? "move" : idle[tool]);
      return;
    }

    if (current.kind === "resize") setCrop(resized(crop, current.handle, point, size));
    else if (current.kind === "move")
      setCrop(moved(current.from, point.x - current.start.x, point.y - current.start.y, size));
    else if (current.kind === "new") setCrop(between(current.start, point, size));
    else if (current.kind === "arrow") update(current.id, (shape) => ({ ...shape, to: point }));
    else update(current.id, () => shapeMoved(current.from, point.x - current.start.x, point.y - current.start.y));
  };

  const onPointerUp = () => {
    const current = drag.current;
    drag.current = null;
    // A label being typed keeps its starting state until it is done.
    if (editing !== null) return;
    const start = before.current;
    before.current = null;

    // A click without a drag draws no crop and no arrow.
    if (current?.kind === "new" && (crop.w < MIN_CROP || crop.h < MIN_CROP)) {
      setCrop(start?.crop ?? { x: 0, y: 0, w: size.width, h: size.height });
      return;
    }
    const arrow = current?.kind === "arrow" ? shapes.find((shape) => shape.id === current.id) : undefined;
    if (arrow?.kind === "arrow" && Math.hypot(arrow.to.x - arrow.from.x, arrow.to.y - arrow.from.y) < 6 * unit(size)) {
      setShapes((all) => all.filter((shape) => shape.id !== arrow.id));
      setSelected(null);
      return;
    }

    if (start && (start.crop !== crop || start.shapes !== shapes)) remember(start);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDialogElement>) => {
    if (event.target instanceof HTMLInputElement) return;

    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "z") {
      event.preventDefault();
      undo();
      return;
    }
    if ((event.key === "Delete" || event.key === "Backspace") && chosen) {
      event.preventDefault();
      remove(chosen.id);
      return;
    }

    const step = event.shiftKey ? 10 : 1;
    const steps: Record<string, Point> = {
      ArrowLeft: { x: -step, y: 0 },
      ArrowRight: { x: step, y: 0 },
      ArrowUp: { x: 0, y: -step },
      ArrowDown: { x: 0, y: step },
    };
    const by = steps[event.key];
    if (!by || event.target !== canvas.current) return;
    event.preventDefault();

    remember();
    if (chosen) update(chosen.id, (shape) => shapeMoved(shape, by.x, by.y));
    // Alt moves the bottom right corner, which grows or shrinks the crop; otherwise the crop moves.
    else if (event.altKey) setCrop(resized(crop, "se", { x: crop.x + crop.w + by.x, y: crop.y + crop.h + by.y }, size));
    else setCrop(moved(crop, by.x, by.y, size));
  };

  const save = async () => {
    if (!image) return;
    setSaving(true);
    setError(null);
    try {
      const file = await exportImage(renderCopy(image, rounded(crop), shapes, palette), asset);
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

  const shown = rounded(crop);
  const ratio = image ? scale() : 1;
  const annotations = shapes.length === 1 ? "1 annotation" : `${shapes.length} annotations`;

  return createPortal(
    <dialog
      ref={dialog}
      className={`${ui.dialogBase} w-[min(1100px,calc(100vw-32px))] max-h-[calc(100vh-32px)] overflow-y-auto`}
      aria-labelledby={`${id}-title`}
      onClose={onClose}
      onKeyDown={onKeyDown}
    >
      <h2 id={`${id}-title`} className={`${ui.dialogTitle} truncate`}>
        Edit “{assetLabel(asset)}”
      </h2>

      <ImageEditorTools
        tool={tool}
        onTool={(next) => {
          setTool(next);
          if (next === "crop") setSelected(null);
        }}
        style={
          chosen
            ? { color: chosen.color, weight: chosen.weight, pill: chosen.kind === "text" ? chosen.pill : style.pill }
            : style
        }
        onStyle={changeStyle}
        showPill={tool === "text" || chosen?.kind === "text"}
        palette={palette}
        canUndo={undoable > 0}
        onUndo={undo}
        canReset={!whole}
        onReset={() => {
          remember();
          setCrop({ x: 0, y: 0, w: size.width, h: size.height });
        }}
        canDelete={chosen !== null}
        onDelete={() => chosen && remove(chosen.id)}
      />
      <p className="mt-0 mb-3 text-muted text-help">
        {help[tool]} The original stays as it is: saving makes a new image.
      </p>

      <div className="flex items-center justify-center min-h-60 p-3 rounded-lg bg-hover">
        {image ? (
          <div className="relative max-w-full">
            <canvas
              ref={canvas}
              width={size.width}
              height={size.height}
              className="block max-w-full max-h-[60vh] touch-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
              style={{ cursor }}
              tabIndex={0}
              role="img"
              aria-label={`${assetLabel(asset)}, cropped to ${shown.w} × ${shown.h} pixels, with ${annotations}`}
              onPointerDown={onPointerDown}
              onPointerMove={onPointerMove}
              onPointerUp={onPointerUp}
              onPointerCancel={onPointerUp}
            />
            {label && (
              <LabelInput
                label={label}
                fontSize={fontSize(label.weight, size) / ratio}
                color={paint(label.color, palette)}
                left={label.at.x / ratio}
                top={label.at.y / ratio}
                onChange={(text) => update(label.id, (shape) => ({ ...shape, text }) as Shape)}
                onDone={finishEditing}
              />
            )}
          </div>
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
          {image && (whole ? `${shown.w} × ${shown.h} px` : `Cropped to ${shown.w} × ${shown.h} px`)}
        </span>
        <button type="button" className={ui.dialogButton} onClick={() => dialog.current?.close()}>
          Cancel
        </button>
        <button type="button" className={ui.primary} disabled={(whole && shapes.length === 0) || saving} onClick={save}>
          {saving ? "Saving…" : saveLabel}
        </button>
      </div>
    </dialog>,
    document.body,
  );
}

/** The text field over a label while it is typed, drawn like the label so the picture does not jump when it ends. */
function LabelInput({
  label,
  fontSize: size,
  color,
  left,
  top,
  onChange,
  onDone,
}: {
  label: Label;
  fontSize: number;
  color: string;
  left: number;
  top: number;
  onChange: (text: string) => void;
  onDone: () => void;
}) {
  const pad = labelPadding(size);
  const contrast = label.color === "white" ? "#16191b" : "#ffffff";

  return (
    <input
      aria-label="Label text"
      // biome-ignore lint/a11y/noAutofocus: placing a label is asking to type it.
      autoFocus
      className="absolute m-0 border-0 outline-2 outline-offset-2 outline-dashed outline-accent font-sans font-semibold leading-[1.25]"
      // Placed and drawn from the picture's own sizes and colors, which only exist at run time.
      style={{
        left,
        top,
        fontSize: size,
        padding: `${pad.y}px ${pad.x}px`,
        width: `calc(${Math.max(label.text.length, 6) + 1}ch + ${pad.x * 2}px)`,
        borderRadius: size * 0.3,
        background: label.pill ? color : "transparent",
        color: label.pill ? contrast : color,
      }}
      value={label.text}
      onChange={(e) => onChange(e.target.value)}
      onBlur={onDone}
      onKeyDown={(e) => {
        // Escape ends the label, not the editor.
        if (e.key === "Enter" || e.key === "Escape") {
          e.preventDefault();
          e.currentTarget.blur();
        }
      }}
    />
  );
}

/** The edited image as a file: JPEG photos stay JPEG, everything else becomes PNG so transparency survives. */
function exportImage(output: HTMLCanvasElement, asset: Asset): Promise<File> {
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
