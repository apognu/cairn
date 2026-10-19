import { expect, test } from "bun:test";
import {
  between,
  distanceToSegment,
  handleAt,
  MIN_CROP,
  moved,
  type Rect,
  resized,
  rounded,
  shapeAt,
} from "./imageEdit";

const image = { width: 100, height: 80 };
const rect: Rect = { x: 20, y: 20, w: 40, h: 30 };

test("dragging a handle moves only its sides, inside the image and never below the smallest crop", () => {
  const cases: [Parameters<typeof resized>[1], { x: number; y: number }, Rect][] = [
    ["se", { x: 70, y: 60 }, { x: 20, y: 20, w: 50, h: 40 }],
    ["nw", { x: 10, y: 5 }, { x: 10, y: 5, w: 50, h: 45 }],
    ["e", { x: 500, y: 0 }, { x: 20, y: 20, w: 80, h: 30 }],
    ["w", { x: 90, y: 0 }, { x: 60 - MIN_CROP, y: 20, w: MIN_CROP, h: 30 }],
    ["n", { x: 0, y: -40 }, { x: 20, y: 0, w: 40, h: 50 }],
  ];
  for (const [handle, point, expected] of cases) expect(resized(rect, handle, point, image)).toEqual(expected);
});

test("a crop drawn backwards or past the edges stays a rectangle inside the image", () => {
  expect(between({ x: 90, y: 70 }, { x: 10, y: 10 }, image)).toEqual({ x: 10, y: 10, w: 80, h: 60 });
  expect(between({ x: -5, y: 10 }, { x: 150, y: 90 }, image)).toEqual({ x: 0, y: 10, w: 100, h: 70 });
  expect(moved(rect, 100, -100, image)).toEqual({ x: 60, y: 0, w: 40, h: 30 });
});

test("corners are found before edges, and the copy gets whole pixels", () => {
  expect(handleAt(rect, { x: 21, y: 19 }, 4)).toBe("nw");
  expect(handleAt(rect, { x: 40, y: 50 }, 4)).toBe("s");
  expect(handleAt(rect, { x: 40, y: 35 }, 4)).toBeNull();
  expect(rounded({ x: 0.4, y: 1.6, w: 10.3, h: 5.2 })).toEqual({ x: 0, y: 2, w: 11, h: 5 });
});

test("the topmost shape under the pointer is found: arrows by their line, labels by their box", () => {
  const big = { width: 1000, height: 500 };
  const arrow = {
    id: 1,
    kind: "arrow",
    from: { x: 0, y: 0 },
    to: { x: 100, y: 100 },
    color: "red",
    weight: "small",
  } as const;
  const label = {
    id: 2,
    kind: "text",
    at: { x: 40, y: 40 },
    text: "Here",
    color: "black",
    weight: "small",
    pill: true,
  } as const;
  const boxes = new Map([[2, { x: 40, y: 40, w: 60, h: 20 }]]);

  expect(shapeAt([arrow, label], { x: 50, y: 48 }, 4, big, boxes)?.id).toBe(2);
  expect(shapeAt([label, arrow], { x: 50, y: 48 }, 4, big, boxes)?.id).toBe(1);
  expect(shapeAt([arrow], { x: 80, y: 20 }, 4, big, boxes)).toBeNull();
  expect(distanceToSegment({ x: 200, y: 0 }, { x: 0, y: 0 }, { x: 100, y: 0 })).toBe(100);
});
