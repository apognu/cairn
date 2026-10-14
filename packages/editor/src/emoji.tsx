import { Extension, InputRule } from "@tiptap/core";
import { PluginKey } from "@tiptap/pm/state";
import { ReactRenderer } from "@tiptap/react";
import Suggestion, { type SuggestionKeyDownProps, type SuggestionProps } from "@tiptap/suggestion";
import { forwardRef, useEffect, useImperativeHandle, useState } from "react";
import { place } from "./slash";

interface Emoji {
  emoji: string;
  shortcodes: string[];
  search: string;
  order: number;
}

/** A match, with the one of its shortcodes that fits what was typed. */
export interface EmojiItem {
  emoji: string;
  shortcode: string;
}

// The parts of emojibase's compact dataset that are used here.
type CompactEmoji = {
  hexcode: string;
  label: string;
  unicode: string;
  group?: number;
  order?: number;
  tags?: string[];
};

// Skin tone and hair style modifiers: not emojis on their own.
const COMPONENT_GROUP = 2;
const MAX_RESULTS = 8;

let loading: Promise<Emoji[]> | null = null;
let byShortcode: Map<string, string> | null = null;

/** The emoji list, fetched as a separate chunk the first time it is needed. */
function loadEmojis(): Promise<Emoji[]> {
  loading ??= Promise.all([
    import("emojibase-data/en/compact.json"),
    import("emojibase-data/en/shortcodes/github.json"),
  ]).then(([compact, github]) => {
    const shortcodes = github.default as Record<string, string | string[]>;
    const emojis = (compact.default as CompactEmoji[])
      .filter((e) => e.group !== undefined && e.group !== COMPONENT_GROUP)
      .map((e) => {
        const codes = [shortcodes[e.hexcode] ?? []].flat();
        return {
          emoji: e.unicode,
          // GitHub has no name for the newest emojis; their label stands in.
          shortcodes: codes.length > 0 ? codes : [e.label.toLowerCase().replace(/[^a-z0-9]+/g, "_")],
          search: `${codes.join(" ")} ${e.label} ${(e.tags ?? []).join(" ")}`.toLowerCase(),
          order: e.order ?? 0,
        };
      })
      .sort((a, b) => a.order - b.order);

    byShortcode = new Map(emojis.flatMap((e) => e.shortcodes.map((code) => [code, e.emoji] as const)));
    return emojis;
  });
  return loading;
}

function filterEmojis(emojis: Emoji[], query: string): EmojiItem[] {
  const text = query.toLowerCase();
  // A shortcode that starts with what was typed comes first, then one that contains it, then a name or tag.
  const rank = (e: Emoji) =>
    e.shortcodes.some((c) => c.startsWith(text)) ? 0 : e.shortcodes.some((c) => c.includes(text)) ? 1 : 2;

  return emojis
    .filter((e) => e.search.includes(text))
    .sort((a, b) => rank(a) - rank(b) || a.order - b.order)
    .slice(0, MAX_RESULTS)
    .map((e) => ({
      emoji: e.emoji,
      shortcode:
        e.shortcodes.find((c) => c.startsWith(text)) ?? e.shortcodes.find((c) => c.includes(text)) ?? e.shortcodes[0],
    }));
}

type ListHandle = { onKeyDown: (event: KeyboardEvent) => boolean };

const EmojiList = forwardRef<ListHandle, SuggestionProps<EmojiItem, EmojiItem>>(function EmojiList(
  { items, command },
  ref,
) {
  const [active, setActive] = useState(0);

  // biome-ignore lint/correctness/useExhaustiveDependencies: the highlighted row restarts at the top whenever the list changes
  useEffect(() => setActive(0), [items]);

  useImperativeHandle(ref, () => ({
    onKeyDown(event) {
      if (items.length === 0) return false;
      if (event.key === "ArrowDown") setActive((a) => (a + 1) % items.length);
      else if (event.key === "ArrowUp") setActive((a) => (a + items.length - 1) % items.length);
      else if (event.key === "Enter" || event.key === "Tab") command(items[active]);
      else return false;
      return true;
    },
  }));

  // Nothing at all rather than "no match": a colon is often just a colon, or the start of `:)`.
  if (items.length === 0) return null;

  return (
    <div
      className="slash-menu emoji-menu"
      role="listbox"
      aria-label="Insert an emoji"
      aria-activedescendant={`emoji-${active}`}
      tabIndex={-1}
    >
      {items.map((it, i) => (
        <button
          key={it.shortcode}
          type="button"
          role="option"
          id={`emoji-${i}`}
          aria-selected={i === active}
          // Mouse down, not click: a click would first move focus out of the editor and close the menu.
          onMouseDown={(e) => {
            e.preventDefault();
            command(it);
          }}
        >
          <span className="emoji-glyph">{it.emoji}</span>
          <span>:{it.shortcode}:</span>
        </button>
      ))}
    </div>
  );
});

const MENU_HEIGHT = 320;
const MENU_WIDTH = 260;

/** Typing ":" and part of a name offers matching emojis; a complete `:shortcode:` becomes its emoji. Both insert plain text. */
export const EmojiSuggestion = Extension.create({
  name: "emojiSuggestion",
  addInputRules() {
    return [
      new InputRule({
        find: /(?:^|\s)(:([a-z0-9_+-]+):)$/,
        handler: ({ state, range, match }) => {
          const emoji = byShortcode?.get(match[2]);
          if (!emoji) return null;

          // The match may start with the space before the colon, which stays.
          state.tr.insertText(emoji, range.from + match[0].length - match[1].length, range.to);
        },
      }),
    ];
  },
  addProseMirrorPlugins() {
    return [
      Suggestion<EmojiItem, EmojiItem>({
        editor: this.editor,
        // Its own key: the slash menu already uses the default one.
        pluginKey: new PluginKey("emojiSuggestion"),
        char: ":",
        // Code holds literal text, where a colon is just a colon.
        allow: ({ state, range }) => {
          const $from = state.doc.resolve(range.from);
          return !$from.parent.type.spec.code && !$from.marks().some((m) => m.type.spec.code);
        },
        items: async ({ query }) => (/^[a-z0-9_+-]{2,}$/i.test(query) ? filterEmojis(await loadEmojis(), query) : []),
        command: ({ editor, range, props }) => editor.chain().focus().insertContentAt(range, props.emoji).run(),
        render: () => {
          let renderer: ReactRenderer<ListHandle, SuggestionProps<EmojiItem, EmojiItem>> | null = null;

          return {
            onStart(props) {
              renderer = new ReactRenderer(EmojiList, { props, editor: props.editor });
              renderer.element.classList.add("slash-popup");
              document.body.append(renderer.element);
              place(renderer.element, props.clientRect?.(), MENU_HEIGHT, MENU_WIDTH);
            },
            onUpdate(props) {
              renderer?.updateProps(props);
              if (renderer) place(renderer.element, props.clientRect?.(), MENU_HEIGHT, MENU_WIDTH);
            },
            onKeyDown({ event }: SuggestionKeyDownProps) {
              if (event.key === "Escape" && renderer?.element.firstElementChild) {
                renderer.destroy();
                renderer = null;
                return true;
              }
              return renderer?.ref?.onKeyDown(event) ?? false;
            },
            onExit() {
              renderer?.destroy();
              renderer = null;
            },
          };
        },
      }),
    ];
  },
});
