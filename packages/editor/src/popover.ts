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

/**
 * Scrolls `list` so `option` shows in it. Not `scrollIntoView`: that scrolls every ancestor as well, the canvas and the editor's shell
 * included, which moves the page under a menu that sticks out of it.
 */
export function showInList(list: HTMLElement, option: Element, center = false) {
  const box = list.getBoundingClientRect();
  const at = option.getBoundingClientRect();

  if (center) list.scrollTop += at.top - box.top - (box.height - at.height) / 2;
  else if (at.top < box.top) list.scrollTop -= box.top - at.top;
  else if (at.bottom > box.bottom) list.scrollTop += at.bottom - box.bottom;
}
