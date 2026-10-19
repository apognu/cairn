// The geometry of the image editor, in the image's own pixels: what a drag does to the crop, kept apart from the canvas so it can be
// tested.

export type Point = { x: number; y: number };
export type Rect = { x: number; y: number; w: number; h: number };
export type Handle = "nw" | "n" | "ne" | "e" | "se" | "s" | "sw" | "w";
export type Size = { width: number; height: number };

/** A crop is never smaller than this on a side. */
export const MIN_CROP = 8;

/** Corners first, so a press near both a corner and an edge takes the corner. */
export const HANDLES: Handle[] = ["nw", "ne", "se", "sw", "n", "e", "s", "w"];

/** Where a handle sits on the rectangle. */
export function handlePoint(rect: Rect, handle: Handle): Point {
  const x = handle.includes("w") ? rect.x : handle.includes("e") ? rect.x + rect.w : rect.x + rect.w / 2;
  const y = handle.includes("n") ? rect.y : handle.includes("s") ? rect.y + rect.h : rect.y + rect.h / 2;
  return { x, y };
}

/** The handle within `reach` of the point, if any; corners win over edges. */
export function handleAt(rect: Rect, point: Point, reach: number): Handle | null {
  return (
    HANDLES.find((handle) => {
      const at = handlePoint(rect, handle);
      return Math.abs(at.x - point.x) <= reach && Math.abs(at.y - point.y) <= reach;
    }) ?? null
  );
}

export function contains(rect: Rect, point: Point): boolean {
  return point.x >= rect.x && point.x <= rect.x + rect.w && point.y >= rect.y && point.y <= rect.y + rect.h;
}

/** The rectangle between two corners, whichever way it was drawn, inside the image. */
export function between(a: Point, b: Point, image: Size): Rect {
  const x1 = clamp(Math.min(a.x, b.x), 0, image.width);
  const y1 = clamp(Math.min(a.y, b.y), 0, image.height);
  const x2 = clamp(Math.max(a.x, b.x), 0, image.width);
  const y2 = clamp(Math.max(a.y, b.y), 0, image.height);
  return { x: x1, y: y1, w: x2 - x1, h: y2 - y1 };
}

/** The rectangle moved by `dx`, `dy`, stopping at the image's edges. */
export function moved(rect: Rect, dx: number, dy: number, image: Size): Rect {
  return {
    ...rect,
    x: clamp(rect.x + dx, 0, image.width - rect.w),
    y: clamp(rect.y + dy, 0, image.height - rect.h),
  };
}

/** The rectangle with `handle` dragged to `point`: the opposite side stays put, and the size never drops below `MIN_CROP`. */
export function resized(rect: Rect, handle: Handle, point: Point, image: Size): Rect {
  let left = rect.x;
  let top = rect.y;
  let right = rect.x + rect.w;
  let bottom = rect.y + rect.h;
  const x = clamp(point.x, 0, image.width);
  const y = clamp(point.y, 0, image.height);

  if (handle.includes("w")) left = Math.min(x, right - MIN_CROP);
  if (handle.includes("e")) right = Math.max(x, left + MIN_CROP);
  if (handle.includes("n")) top = Math.min(y, bottom - MIN_CROP);
  if (handle.includes("s")) bottom = Math.max(y, top + MIN_CROP);

  return { x: left, y: top, w: right - left, h: bottom - top };
}

/** Whole pixels, so the copy has an exact size. */
export function rounded(rect: Rect): Rect {
  const x = Math.round(rect.x);
  const y = Math.round(rect.y);
  return { x, y, w: Math.round(rect.x + rect.w) - x, h: Math.round(rect.y + rect.h) - y };
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

export type Color = "accent" | "red" | "black" | "white";
export type Weight = "small" | "medium" | "large";

export type Arrow = { id: number; kind: "arrow"; from: Point; to: Point; color: Color; weight: Weight };
/** A line of text whose top left corner is `at`, on a rounded background in its color when `pill` is set. */
export type Label = { id: number; kind: "text"; at: Point; text: string; color: Color; weight: Weight; pill: boolean };
export type Shape = Arrow | Label;

/** Annotations scale with the picture, so a label reads the same on a phone shot and on a wide screenshot. */
export function unit(image: Size): number {
  return Math.max(0.5, Math.max(image.width, image.height) / 1000);
}

const STROKES: Record<Weight, number> = { small: 3, medium: 5, large: 8 };
const FONT_SIZES: Record<Weight, number> = { small: 16, medium: 24, large: 36 };

export function strokeWidth(weight: Weight, image: Size): number {
  return STROKES[weight] * unit(image);
}

export function fontSize(weight: Weight, image: Size): number {
  return FONT_SIZES[weight] * unit(image);
}

/** How far a point is from the segment between `a` and `b`. */
export function distanceToSegment(point: Point, a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const length = dx * dx + dy * dy;
  const t = length === 0 ? 0 : clamp(((point.x - a.x) * dx + (point.y - a.y) * dy) / length, 0, 1);

  return Math.hypot(point.x - (a.x + t * dx), point.y - (a.y + t * dy));
}

/** The topmost shape under the point. Labels are hit by their box, which only the canvas can measure: `boxes` holds them by ID. */
export function shapeAt(
  shapes: Shape[],
  point: Point,
  reach: number,
  image: Size,
  boxes: Map<number, Rect>,
): Shape | null {
  for (const shape of [...shapes].reverse()) {
    if (
      shape.kind === "arrow" &&
      distanceToSegment(point, shape.from, shape.to) <= strokeWidth(shape.weight, image) / 2 + reach
    )
      return shape;
    const box = boxes.get(shape.id);
    if (shape.kind === "text" && box && contains(box, point)) return shape;
  }

  return null;
}

export function shapeMoved(shape: Shape, dx: number, dy: number): Shape {
  if (shape.kind === "text") return { ...shape, at: { x: shape.at.x + dx, y: shape.at.y + dy } };

  return {
    ...shape,
    from: { x: shape.from.x + dx, y: shape.from.y + dy },
    to: { x: shape.to.x + dx, y: shape.to.y + dy },
  };
}
