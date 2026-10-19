// Drawing the image editor's canvas: the picture, the crop, the annotations and the selection. Canvas pixels are not themed: these
// colors end up in the saved image, and the frame and handles must read on any picture, light or dark.

import {
  type Color,
  fontSize,
  HANDLES,
  handlePoint,
  type Label,
  type Rect,
  type Shape,
  type Size,
  strokeWidth,
  unit,
} from "./imageEdit";

const HANDLE_SIZE = 8;
const FONT_FAMILY = '"Hanken Grotesk", system-ui, sans-serif';

/** What `accent` paints with: the site's accent color, read from the settings. */
export type Palette = { accent: string };

export function paint(color: Color, palette: Palette): string {
  return { accent: palette.accent, red: "#e5484d", black: "#16191b", white: "#ffffff" }[color];
}

export function labelFont(label: Label, image: Size): string {
  return `600 ${fontSize(label.weight, image)}px ${FONT_FAMILY}`;
}

/** Waits for the site font at every size labels use, so measuring and the saved image both use it rather than a fallback. */
export async function loadFonts(image: Size): Promise<void> {
  const sizes = (["small", "medium", "large"] as const).map(
    (weight) => `600 ${fontSize(weight, image)}px ${FONT_FAMILY}`,
  );
  await Promise.all(sizes.map((font) => document.fonts.load(font))).catch(() => undefined);
}

type Scene = {
  image: HTMLImageElement;
  crop: Rect;
  shapes: Shape[];
  palette: Palette;
  /** Image pixels per screen pixel: the frame, handles and selection keep their size on screen. */
  scale: number;
  showHandles: boolean;
  selected: number | null;
  /** A label being typed into is drawn by its text field instead. */
  hidden: number | null;
};

/** Draws the editing view and returns the box of every label, for hit testing. */
export function drawScene(context: CanvasRenderingContext2D, scene: Scene): Map<number, Rect> {
  const { image, crop, scale } = scene;
  const size = { width: image.naturalWidth, height: image.naturalHeight };
  context.clearRect(0, 0, size.width, size.height);
  context.drawImage(image, 0, 0);

  const boxes = drawShapes(context, scene.shapes, size, scene.palette, scene.hidden);

  context.fillStyle = "rgb(0 0 0 / 50%)";
  context.fillRect(0, 0, size.width, crop.y);
  context.fillRect(0, crop.y + crop.h, size.width, size.height - crop.y - crop.h);
  context.fillRect(0, crop.y, crop.x, crop.h);
  context.fillRect(crop.x + crop.w, crop.y, size.width - crop.x - crop.w, crop.h);

  if (scene.showHandles) {
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

  const chosen = scene.shapes.find((shape) => shape.id === scene.selected);
  if (chosen) drawSelection(context, bounds(chosen, size, boxes), scale);

  return boxes;
}

/** Draws the annotations, in order, and returns each label's box. */
export function drawShapes(
  context: CanvasRenderingContext2D,
  shapes: Shape[],
  image: Size,
  palette: Palette,
  hidden: number | null = null,
): Map<number, Rect> {
  const boxes = new Map<number, Rect>();
  for (const shape of shapes) {
    if (shape.kind === "arrow") drawArrow(context, shape, image, palette);
    else boxes.set(shape.id, drawLabel(context, shape, image, palette, shape.id === hidden));
  }

  return boxes;
}

function drawArrow(context: CanvasRenderingContext2D, arrow: Shape & { kind: "arrow" }, image: Size, palette: Palette) {
  const width = strokeWidth(arrow.weight, image);
  const angle = Math.atan2(arrow.to.y - arrow.from.y, arrow.to.x - arrow.from.x);
  const length = Math.hypot(arrow.to.x - arrow.from.x, arrow.to.y - arrow.from.y);
  const head = Math.min(Math.max(width * 3.5, 10 * unit(image)), length);
  // The line stops inside the head, so its round end does not poke through the point.
  const end = { x: arrow.to.x - Math.cos(angle) * head * 0.6, y: arrow.to.y - Math.sin(angle) * head * 0.6 };

  context.strokeStyle = paint(arrow.color, palette);
  context.fillStyle = paint(arrow.color, palette);
  context.lineWidth = width;
  context.lineCap = "round";
  context.beginPath();
  context.moveTo(arrow.from.x, arrow.from.y);
  context.lineTo(end.x, end.y);
  context.stroke();

  const spread = Math.PI / 7;
  context.beginPath();
  context.moveTo(arrow.to.x, arrow.to.y);
  context.lineTo(arrow.to.x - head * Math.cos(angle - spread), arrow.to.y - head * Math.sin(angle - spread));
  context.lineTo(arrow.to.x - head * Math.cos(angle + spread), arrow.to.y - head * Math.sin(angle + spread));
  context.closePath();
  context.fill();
}

/** The padding around a label's text, from its font size. */
export function labelPadding(size: number): { x: number; y: number } {
  return { x: size * 0.4, y: size * 0.22 };
}

function drawLabel(
  context: CanvasRenderingContext2D,
  label: Label,
  image: Size,
  palette: Palette,
  hidden: boolean,
): Rect {
  const size = fontSize(label.weight, image);
  const pad = labelPadding(size);
  context.font = labelFont(label, image);
  context.textBaseline = "middle";
  const box = {
    x: label.at.x,
    y: label.at.y,
    w: context.measureText(label.text).width + pad.x * 2,
    h: size * 1.25 + pad.y * 2,
  };
  if (hidden || !label.text) return box;

  const color = paint(label.color, palette);
  const contrast = label.color === "white" ? paint("black", palette) : "#ffffff";
  const baseline = box.y + box.h / 2;

  if (label.pill) {
    context.fillStyle = color;
    context.beginPath();
    context.roundRect(box.x, box.y, box.w, box.h, size * 0.3);
    context.fill();
    context.fillStyle = contrast;
    context.fillText(label.text, box.x + pad.x, baseline);
  } else {
    // Without a background, a thin outline in the opposite color keeps the text readable on any picture.
    context.lineWidth = size * 0.14;
    context.lineJoin = "round";
    context.strokeStyle = contrast;
    context.strokeText(label.text, box.x + pad.x, baseline);
    context.fillStyle = color;
    context.fillText(label.text, box.x + pad.x, baseline);
  }

  return box;
}

function bounds(shape: Shape, image: Size, boxes: Map<number, Rect>): Rect {
  if (shape.kind === "text") return boxes.get(shape.id) ?? { x: shape.at.x, y: shape.at.y, w: 0, h: 0 };

  const pad = strokeWidth(shape.weight, image) * 2;
  const x = Math.min(shape.from.x, shape.to.x) - pad;
  const y = Math.min(shape.from.y, shape.to.y) - pad;
  return { x, y, w: Math.abs(shape.to.x - shape.from.x) + pad * 2, h: Math.abs(shape.to.y - shape.from.y) + pad * 2 };
}

function drawSelection(context: CanvasRenderingContext2D, box: Rect, scale: number) {
  const gap = 4 * scale;
  context.lineWidth = 1.5 * scale;
  context.setLineDash([]);
  context.strokeStyle = "white";
  context.strokeRect(box.x - gap, box.y - gap, box.w + gap * 2, box.h + gap * 2);
  context.setLineDash([4 * scale, 4 * scale]);
  context.strokeStyle = "black";
  context.strokeRect(box.x - gap, box.y - gap, box.w + gap * 2, box.h + gap * 2);
  context.setLineDash([]);
}

/** The saved image: the cropped part of the picture with the annotations on it. */
export function renderCopy(image: HTMLImageElement, crop: Rect, shapes: Shape[], palette: Palette): HTMLCanvasElement {
  const output = document.createElement("canvas");
  output.width = crop.w;
  output.height = crop.h;
  const context = output.getContext("2d");
  if (!context) return output;

  context.drawImage(image, crop.x, crop.y, crop.w, crop.h, 0, 0, crop.w, crop.h);
  context.translate(-crop.x, -crop.y);
  drawShapes(context, shapes, { width: image.naturalWidth, height: image.naturalHeight }, palette);

  return output;
}
