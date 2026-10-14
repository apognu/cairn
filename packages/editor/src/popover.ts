/**
 * Where a popover anchored to text in the canvas goes: inside the pane that scrolls that text (`.canvas`, which is positioned), so the
 * browser moves it with the text while scrolling, with nothing to recompute. Null outside such a pane: the popover goes on the page.
 */
export function popoverHost(anchor: Element): HTMLElement | null {
  return anchor.closest<HTMLElement>(".canvas");
}

/** A point of the viewport in the coordinates of `host`'s scrolled content, for an absolutely positioned child of it. */
export function inHost(host: HTMLElement, x: number, y: number) {
  const box = host.getBoundingClientRect();

  return { left: x - box.left + host.scrollLeft, top: y - box.top + host.scrollTop };
}
