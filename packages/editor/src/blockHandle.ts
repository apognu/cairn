import { Extension } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import { NodeSelection, Plugin, PluginKey, type Selection, TextSelection } from "@tiptap/pm/state";
import type { EditorView } from "@tiptap/pm/view";

const GRIP =
  '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><circle cx="9" cy="5" r="1.6"/><circle cx="15" cy="5" r="1.6"/><circle cx="9" cy="12" r="1.6"/><circle cx="15" cy="12" r="1.6"/><circle cx="9" cy="19" r="1.6"/><circle cx="15" cy="19" r="1.6"/></svg>';
const SIZE = 24;
// How far left of the text the pointer may go and still reach the handle, or drop.
const REACH = 56;

/**
 * A handle in the margin of the top-level block under the pointer, dragged to move the block between two others; a line shows where it
 * will land. The drag is ours from start to end: ProseMirror's own drop would put a caret anywhere, inside other blocks too, and remove
 * the original only if the selection still held it. Alt+Shift+Up and Down move the block at the cursor for the keyboard. Not on phones.
 */
export const BlockHandle = Extension.create({
  name: "blockHandle",
  addKeyboardShortcuts() {
    return {
      "Alt-Shift-ArrowUp": () => moveByKey(this.editor.view, -1),
      "Alt-Shift-ArrowDown": () => moveByKey(this.editor.view, 1),
    };
  },
  addProseMirrorPlugins() {
    return [new Plugin({ key: new PluginKey("blockHandle"), view: (view) => handleView(view) })];
  },
});

interface Block {
  index: number;
  node: PMNode;
  dom: HTMLElement;
  rect: DOMRect;
}

function handleView(view: EditorView) {
  const host = view.dom.parentElement;
  const handle = document.createElement("div");
  handle.className =
    "block-handle absolute z-10 flex items-center justify-center w-5 h-6 rounded text-muted cursor-grab invisible hover:bg-hover hover:text-text active:cursor-grabbing mobile:hidden";
  handle.draggable = true;
  handle.title = "Drag to move";
  handle.innerHTML = GRIP;
  const line = document.createElement("div");
  line.className = "block-drop-line absolute z-10 h-0.5 rounded-full bg-accent pointer-events-none hidden";
  host?.append(handle, line);

  // The block under the pointer, and while dragging, the one being moved and the gap it would go into (an index between blocks).
  let hovered: number | null = null;
  let dragged: number | null = null;
  let gap: number | null = null;
  // Held still from the press until the drag starts: moved under the pointer to another block, the browser would start no drag at all.
  let pressed = false;

  const hideHandle = () => {
    hovered = null;
    handle.classList.add("invisible");
  };
  const near = (event: MouseEvent) => {
    if (!host) return false;
    const area = host.getBoundingClientRect();
    return (
      event.clientX >= area.left - REACH &&
      event.clientX <= area.right &&
      event.clientY >= area.top - 16 &&
      event.clientY <= area.bottom + 16
    );
  };

  const onMove = (event: MouseEvent) => {
    if (!host || dragged !== null || pressed) return;
    if (!near(event)) return hideHandle();

    const block = blocks(view)
      .filter((b) => b.rect.top <= event.clientY + 4)
      .at(-1);
    if (!block) return hideHandle();

    const area = host.getBoundingClientRect();
    // Centered on the first line of text; other blocks (tables, components) get it at their top edge.
    const lineHeight = block.node.isTextblock ? Number.parseFloat(getComputedStyle(block.dom).lineHeight) : Number.NaN;
    const offset = Number.isFinite(lineHeight) ? (lineHeight - SIZE) / 2 : 0;
    handle.style.left = `${block.rect.left - area.left - 28}px`;
    handle.style.top = `${block.rect.top - area.top + offset}px`;
    handle.classList.remove("invisible");
    hovered = block.index;
  };

  const endDrag = () => {
    pressed = false;
    dragged = null;
    gap = null;
    line.classList.add("hidden");
    hideHandle();
  };

  handle.addEventListener("dragstart", (event) => {
    if (hovered === null || !event.dataTransfer) return;
    dragged = hovered;
    const block = blocks(view)[dragged];
    // Some browsers start no drag without data. Plain text only: dropped in another field it is the block's words, not a copy of it.
    event.dataTransfer.setData("text/plain", block?.node.textContent ?? "");
    event.dataTransfer.effectAllowed = "move";
    if (block) event.dataTransfer.setDragImage(block.dom, 0, 0);
  });
  handle.addEventListener("dragend", endDrag);
  handle.addEventListener("mousedown", () => {
    pressed = true;
  });
  // A click without a drag lets go of it too.
  handle.addEventListener("mouseup", () => {
    pressed = false;
  });

  // Caught before they reach the editor, so neither ProseMirror's drop nor its drop caret ever see this drag.
  const onDragOver = (event: DragEvent) => {
    if (dragged === null || !host) return;
    event.stopPropagation();
    if (!near(event)) {
      gap = null;
      line.classList.add("hidden");
      return;
    }
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";

    const all = blocks(view);
    const at = all.filter((b) => (b.rect.top + b.rect.bottom) / 2 < event.clientY).length;
    // Before or after itself: nowhere to go.
    if (at === dragged || at === dragged + 1) {
      gap = null;
      line.classList.add("hidden");
      return;
    }
    gap = at;

    const area = host.getBoundingClientRect();
    const above = all[at - 1]?.rect;
    const below = all[at]?.rect;
    const y = above && below ? (above.bottom + below.top) / 2 : below ? below.top - 4 : (above?.bottom ?? 0) + 4;
    const editor = view.dom.getBoundingClientRect();
    line.style.top = `${y - area.top - 1}px`;
    line.style.left = `${editor.left - area.left}px`;
    line.style.width = `${editor.width}px`;
    line.classList.remove("hidden");
  };
  const onDrop = (event: DragEvent) => {
    if (dragged === null) return;
    event.stopPropagation();
    event.preventDefault();
    if (gap !== null) moveBlock(view, dragged, gap);
    endDrag();
  };

  document.addEventListener("mousemove", onMove);
  document.addEventListener("dragover", onDragOver, true);
  document.addEventListener("drop", onDrop, true);
  // Typing hides it until the pointer moves again: it would cover the start of the line.
  view.dom.addEventListener("keydown", hideHandle);

  return {
    destroy() {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("dragover", onDragOver, true);
      document.removeEventListener("drop", onDrop, true);
      view.dom.removeEventListener("keydown", hideHandle);
      handle.remove();
      line.remove();
    },
  };
}

/** The top-level blocks, top to bottom, with where they are on screen. */
function blocks(view: EditorView): Block[] {
  const list: Block[] = [];

  view.state.doc.forEach((node, pos, index) => {
    const dom = view.nodeDOM(pos);
    if (dom instanceof HTMLElement) list.push({ index, node, dom, rect: dom.getBoundingClientRect() });
  });
  return list;
}

/** Where the top-level block at `index` starts. */
function startOf(doc: PMNode, index: number): number {
  let pos = 0;
  for (let i = 0; i < index; i++) pos += doc.child(i).nodeSize;
  return pos;
}

/**
 * Moves the top-level block at `from` into the gap before the block now at `gap` (the end for the last gap), in one transaction: the
 * same node is put back, IDs included, so links to a heading keep working. `select` places the selection in the moved block; by default
 * it goes to its start, or onto it when it holds no text.
 */
function moveBlock(view: EditorView, from: number, gap: number, select?: (after: PMNode, shift: number) => Selection) {
  const { doc } = view.state;
  const node = doc.child(from);
  const start = startOf(doc, from);
  // Counted in the document without the block, once it is taken out.
  const moved = startOf(doc, gap) - (gap > from ? node.nodeSize : 0);

  const tr = view.state.tr.delete(start, start + node.nodeSize).insert(moved, node);
  tr.setSelection(
    select?.(tr.doc, moved - start) ??
      (node.isTextblock ? TextSelection.near(tr.doc.resolve(moved + 1)) : NodeSelection.create(tr.doc, moved)),
  );
  view.dispatch(tr.scrollIntoView());
  view.focus();
}

/** Swaps the top-level block holding the selection with the one above or below it, keeping the selection where it was in it. */
function moveByKey(view: EditorView, direction: -1 | 1): boolean {
  const { doc, selection } = view.state;
  const index = selection.$from.index(0);
  // A selection over several blocks, or table cells, has no one block to move.
  const within =
    selection instanceof NodeSelection || (selection instanceof TextSelection && selection.$to.index(0) === index);
  if (!within) return false;
  // Consumed even at the ends, so the keys never fall through to the browser's own selection change.
  if (index + direction < 0 || index + direction >= doc.childCount) return true;

  moveBlock(view, index, direction < 0 ? index - 1 : index + 2, (after, shift) =>
    selection instanceof NodeSelection
      ? NodeSelection.create(after, selection.from + shift)
      : TextSelection.create(after, selection.anchor + shift, selection.head + shift),
  );
  return true;
}
