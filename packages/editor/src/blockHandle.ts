import { Extension } from "@tiptap/core";
import { type EditorState, NodeSelection, Plugin, PluginKey, TextSelection } from "@tiptap/pm/state";
import type { EditorView } from "@tiptap/pm/view";

const GRIP =
  '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><circle cx="9" cy="5" r="1.6"/><circle cx="15" cy="5" r="1.6"/><circle cx="9" cy="12" r="1.6"/><circle cx="15" cy="12" r="1.6"/><circle cx="9" cy="19" r="1.6"/><circle cx="15" cy="19" r="1.6"/></svg>';
const SIZE = 24;
// How far left of the text the pointer may go and still reach the handle.
const REACH = 56;

/**
 * A handle in the margin of the top-level block under the pointer, dragged to move the block elsewhere in the page. ProseMirror does the
 * drop itself, as for any dragged selection. Alt+Shift+Up and Down move the block at the cursor for the keyboard. Not on phones.
 */
export const BlockHandle = Extension.create({
  name: "blockHandle",
  addKeyboardShortcuts() {
    return {
      "Alt-Shift-ArrowUp": () => moveBlock(this.editor.view, -1),
      "Alt-Shift-ArrowDown": () => moveBlock(this.editor.view, 1),
    };
  },
  addProseMirrorPlugins() {
    return [new Plugin({ key: new PluginKey("blockHandle"), view: (view) => handleView(view) })];
  },
});

function handleView(view: EditorView) {
  // The handle sits beside the editor, inside its wrapper: UniqueID only keeps the IDs of blocks dragged from within that wrapper.
  const host = view.dom.parentElement;
  const handle = document.createElement("div");
  handle.className =
    "block-handle absolute z-10 flex items-center justify-center w-5 h-6 rounded text-muted cursor-grab invisible hover:bg-hover hover:text-text active:cursor-grabbing mobile:hidden";
  handle.draggable = true;
  handle.title = "Drag to move";
  handle.innerHTML = GRIP;
  host?.appendChild(handle);

  let block: number | null = null;
  const hide = () => {
    block = null;
    handle.classList.add("invisible");
  };

  const onMove = (event: MouseEvent) => {
    if (!host || view.dragging) return;
    const area = host.getBoundingClientRect();
    if (
      event.clientX < area.left - REACH ||
      event.clientX > area.right ||
      event.clientY < area.top ||
      event.clientY > area.bottom
    )
      return hide();

    const found = blockAt(view, event.clientY);
    if (!found) return hide();

    const rect = found.dom.getBoundingClientRect();
    // Centered on the first line of text; other blocks (tables, components) get it at their top edge.
    const lineHeight = found.textblock ? Number.parseFloat(getComputedStyle(found.dom).lineHeight) : Number.NaN;
    const offset = Number.isFinite(lineHeight) ? (lineHeight - SIZE) / 2 : 0;
    handle.style.left = `${rect.left - area.left - 28}px`;
    handle.style.top = `${rect.top - area.top + offset}px`;
    handle.classList.remove("invisible");
    block = found.pos;
  };

  handle.addEventListener("dragstart", (event) => {
    if (block === null || !event.dataTransfer) return;

    const selection = NodeSelection.create(view.state.doc, block);
    view.dispatch(view.state.tr.setSelection(selection));
    const slice = selection.content();
    const { dom, text } = view.serializeForClipboard(slice);
    event.dataTransfer.clearData();
    event.dataTransfer.setData("text/html", dom.innerHTML);
    event.dataTransfer.setData("text/plain", text);
    // "move", not "copyMove": UniqueID gives a copied block new IDs, and anchors and links to a heading rely on its ID.
    event.dataTransfer.effectAllowed = "move";
    const node = view.nodeDOM(block);
    if (node instanceof HTMLElement) event.dataTransfer.setDragImage(node, 0, 0);
    view.dragging = { slice, move: true };
  });
  handle.addEventListener("dragend", hide);

  document.addEventListener("mousemove", onMove);
  // Typing hides it until the pointer moves again: it would cover the start of the line.
  view.dom.addEventListener("keydown", hide);

  return {
    destroy() {
      document.removeEventListener("mousemove", onMove);
      view.dom.removeEventListener("keydown", hide);
      handle.remove();
    },
  };
}

/** The top-level block at a height of the page: the last one starting above it. */
function blockAt(view: EditorView, y: number): { pos: number; dom: HTMLElement; textblock: boolean } | null {
  let found: { pos: number; dom: HTMLElement; textblock: boolean } | null = null;

  view.state.doc.forEach((node, pos) => {
    const dom = view.nodeDOM(pos);
    if (!(dom instanceof HTMLElement)) return;
    if (dom.getBoundingClientRect().top <= y + 4) found = { pos, dom, textblock: node.isTextblock };
  });
  return found;
}

/** Swaps the top-level block holding the selection with the one above or below it, keeping the selection on it. */
function moveBlock(view: EditorView, direction: -1 | 1): boolean {
  const { state } = view;
  const { doc, selection } = state;
  const index = selection.$from.index(0);
  // A selection over several blocks, or table cells, has no one block to move.
  const within =
    selection instanceof NodeSelection || (selection instanceof TextSelection && selection.$to.index(0) === index);
  if (!within) return false;
  const target = index + direction;
  // Consumed even at the ends, so the keys never fall through to the browser's own selection change.
  if (target < 0 || target >= doc.childCount) return true;

  const start = blockStart(state, index);
  const node = doc.child(index);
  const moved = direction < 0 ? start - doc.child(target).nodeSize : start + doc.child(target).nodeSize;
  const tr = state.tr.delete(start, start + node.nodeSize).insert(moved, node);
  const shift = moved - start;
  tr.setSelection(
    selection instanceof NodeSelection
      ? NodeSelection.create(tr.doc, moved)
      : TextSelection.create(tr.doc, selection.anchor + shift, selection.head + shift),
  );
  view.dispatch(tr.scrollIntoView());
  return true;
}

function blockStart(state: EditorState, index: number): number {
  let pos = 0;
  for (let i = 0; i < index; i++) pos += state.doc.child(i).nodeSize;
  return pos;
}
