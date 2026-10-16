import type { Node as PMNode } from "@tiptap/pm/model";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import { api } from "./api/client";
import type { components } from "./api/schema";

type CodeToken = components["schemas"]["CodeToken"];
type CodeTokenStyle = components["schemas"]["CodeTokenStyle"];

const key = new PluginKey<DecorationSet>("codeHighlight");

/** How long typing must pause before a changed block is colored again. */
const DELAY = 250;
const CACHE_SIZE = 300;

// Shared by every editor on the page: the same code in the same language always gets the same colors.
const cache = new Map<string, CodeToken[]>();

function remember(id: string, tokens: CodeToken[]) {
  cache.set(id, tokens);
  // Maps keep insertion order, so the first key is the oldest.
  if (cache.size > CACHE_SIZE) cache.delete(cache.keys().next().value as string);
}

interface Block {
  pos: number;
  node: PMNode;
  language: string;
  code: string;
  id: string;
}

function codeBlocks(doc: PMNode): Block[] {
  const found: Block[] = [];

  doc.descendants((node, pos) => {
    if (node.type.name !== "codeBlock") return true;

    const language = String(node.attrs.language ?? "");
    const code = node.textContent;
    if (language && code) found.push({ pos, node, language, code, id: `${language}\n${code}` });
    return false;
  });

  return found;
}

function variables(mode: "light" | "dark", style: CodeTokenStyle | null | undefined): string {
  if (!style) return "";

  return [
    `--code-${mode}:${style.color}`,
    style.italic ? `--code-${mode}-style:italic` : "",
    style.bold ? `--code-${mode}-weight:600` : "",
  ]
    .filter(Boolean)
    .join(";");
}

function decorate({ pos, node }: Block, tokens: CodeToken[]): Decoration[] {
  const size = node.content.size;

  // Each token carries both modes' colors; `.code-token` in styles.css picks the one for the current mode.
  return tokens
    .filter((t) => t.from < t.to && t.to <= size)
    .map((t) =>
      Decoration.inline(pos + 1 + t.from, pos + 1 + t.to, {
        class: "code-token",
        style: [variables("light", t.light), variables("dark", t.dark)].filter(Boolean).join(";"),
      }),
    );
}

/**
 * Colors code blocks in the canvas with the publish highlighter, through the server. A block keeps its previous colors, moved along with
 * the edits, until the server has colored its new text.
 */
export function codeHighlight(): Plugin<DecorationSet> {
  return new Plugin<DecorationSet>({
    key,
    state: {
      init: () => DecorationSet.empty,
      apply(tr, old) {
        if (!tr.docChanged && !tr.getMeta(key)) return old;

        const moved = old.map(tr.mapping, tr.doc);
        const decorations = codeBlocks(tr.doc).flatMap((block) => {
          const tokens = cache.get(block.id);
          return tokens ? decorate(block, tokens) : moved.find(block.pos, block.pos + block.node.nodeSize);
        });

        return DecorationSet.create(tr.doc, decorations);
      },
    },
    props: {
      decorations: (state) => key.getState(state),
    },
    view(view) {
      const pending = new Set<string>();
      let timer: ReturnType<typeof setTimeout> | undefined;
      let alive = true;

      const request = () => {
        for (const { id, language, code } of codeBlocks(view.state.doc)) {
          if (cache.has(id) || pending.has(id)) continue;
          pending.add(id);

          api
            .POST("/api/highlight", { body: { language, code } })
            .then(({ data, response }) => {
              // A block too long to color stays plain rather than being asked about again on every keystroke.
              if (data) remember(id, data.tokens);
              else if (response.status === 422) remember(id, []);
              else return;

              if (alive) view.dispatch(view.state.tr.setMeta(key, true).setMeta("addToHistory", false));
            })
            .catch(() => undefined)
            .finally(() => pending.delete(id));
        }
      };

      const schedule = (delay: number) => {
        clearTimeout(timer);
        timer = setTimeout(request, delay);
      };
      schedule(0);

      return {
        update(view, previous) {
          if (!view.state.doc.eq(previous.doc)) schedule(DELAY);
        },
        destroy() {
          alive = false;
          clearTimeout(timer);
        },
      };
    },
  });
}
