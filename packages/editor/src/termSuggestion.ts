import { Extension } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import { type EditorState, Plugin, PluginKey } from "@tiptap/pm/state";
import type { EditorView } from "@tiptap/pm/view";
import { popoverHost } from "./popover";
import { place } from "./slash";
import * as ui from "./ui";

/** A glossary term and the other words that mean it. */
export type GlossaryTerm = { key: string; aliases: string[] };

let glossary: GlossaryTerm[] = [];

/** The terms to look for while typing; the canvas keeps it in step with the library. */
export function setGlossary(terms: GlossaryTerm[]) {
  glossary = terms;
}

type Offer = { term: string; words: string; from: number; to: number };

const key = new PluginKey<{ dismissed: Set<string> }>("termSuggestion");
// What ends a word just typed: the offer comes once the word is complete, so a longer word that starts the same is never cut.
const BOUNDARY = /[\s.,;:!?)]/;

/** The term whose name or other word was just typed before the cursor, if it is worth offering. */
function offerAt(state: EditorState, dismissed: Set<string>): Offer | null {
  const { selection } = state;
  if (!selection.empty || glossary.length === 0) return null;
  const $at = selection.$from;
  if ($at.parent.type.spec.code || $at.marks().some((m) => m.type.spec.code)) return null;

  const before = $at.parent.textBetween(0, $at.parentOffset, undefined, "￼");
  if (before.length < 2 || !BOUNDARY.test(before.at(-1) ?? "")) return null;
  const typed = before.slice(0, -1).toLowerCase();

  let best: Offer | null = null;
  for (const term of glossary) {
    if (dismissed.has(term.key)) continue;
    for (const words of [term.key, ...term.aliases]) {
      const w = words.toLowerCase();
      if (!w || !typed.endsWith(w) || (best && best.words.length >= w.length)) continue;
      // Whole words only: "key" in "monkey" is not the term "key".
      const start = typed.length - w.length;
      if (start > 0 && /[\p{L}\p{N}_-]/u.test(typed[start - 1])) continue;
      const to = $at.pos - 1;
      best = { term: term.key, words: before.slice(start, start + w.length), from: to - w.length, to };
    }
  }
  if (!best) return null;

  const termMark = state.schema.marks.term;
  if (!termMark || state.doc.rangeHasMark(best.from, best.to, termMark)) return null;
  // Readers get the definition at the first use on a page: once the term is marked somewhere, the rest can stay plain.
  if (marksTerm(state.doc, best.term)) return null;

  return best;
}

function marksTerm(doc: PMNode, term: string): boolean {
  let found = false;
  doc.descendants((node) => {
    if (found) return false;
    if (node.marks.some((m) => m.type.name === "term" && m.attrs.term === term)) found = true;
    return !found;
  });
  return found;
}

function accept(view: EditorView, offer: Offer) {
  const mark = view.state.schema.marks.term.create({ term: offer.term });
  view.dispatch(view.state.tr.addMark(offer.from, offer.to, mark));
}

function dismiss(view: EditorView, offer: Offer) {
  view.dispatch(view.state.tr.setMeta(key, offer.term));
}

const MENU_WIDTH = 300;
const MENU_HEIGHT = 48;

/**
 * Typing a glossary term, or one of its other words, offers to mark it: Tab or a click marks the words, Escape stops offering that term
 * on this page. Enter is left alone, so ending a paragraph on the word never marks it.
 */
export const TermSuggestion = Extension.create({
  name: "termSuggestion",
  addProseMirrorPlugins() {
    return [
      new Plugin({
        key,
        state: {
          init: () => ({ dismissed: new Set<string>() }),
          apply: (tr, value) => {
            const term = tr.getMeta(key) as string | undefined;
            return term ? { dismissed: new Set([...value.dismissed, term]) } : value;
          },
        },
        props: {
          handleKeyDown(view, event) {
            if (event.key !== "Tab" && event.key !== "Escape") return false;
            const offer = offerAt(view.state, key.getState(view.state)?.dismissed ?? new Set());
            if (!offer) return false;
            if (event.key === "Tab") accept(view, offer);
            else dismiss(view, offer);
            return true;
          },
        },
        view(editorView) {
          const element = document.createElement("div");
          element.className = "fixed z-20";
          element.hidden = true;
          const menu = document.createElement("div");
          menu.className = `${ui.cursorMenu} p-1`;
          const button = document.createElement("button");
          button.type = "button";
          button.className = ui.cursorOption;
          menu.append(button);
          element.append(menu);
          const host = popoverHost(editorView.dom);
          (host ?? document.body).append(element);

          let current: Offer | null = null;
          // Mouse down, not click: a click would first move the focus out of the editor and the offer with it.
          button.addEventListener("mousedown", (event) => {
            event.preventDefault();
            if (current) accept(editorView, current);
          });

          const update = (view: EditorView) => {
            current = view.hasFocus() ? offerAt(view.state, key.getState(view.state)?.dismissed ?? new Set()) : null;
            element.hidden = current === null;
            if (!current) return;
            button.textContent = "";
            const label = document.createElement("span");
            label.textContent = `Mark “${current.words}” as the glossary term “${current.term}”`;
            const hint = document.createElement("kbd");
            hint.className = "ml-auto text-muted text-label";
            hint.textContent = "Tab";
            button.append(label, hint);
            const at = view.coordsAtPos(current.from);
            place(
              element,
              new DOMRect(at.left, at.top, at.right - at.left, at.bottom - at.top),
              MENU_HEIGHT,
              MENU_WIDTH,
              host,
            );
          };

          return { update, destroy: () => element.remove() };
        },
      }),
    ];
  },
});
