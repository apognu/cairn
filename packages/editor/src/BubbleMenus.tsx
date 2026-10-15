import { type Editor, isTextSelection } from "@tiptap/core";
import { useEditorState } from "@tiptap/react";
import { BubbleMenu } from "@tiptap/react/menus";
import { useEffect, useState } from "react";
import { PICKER_EVENT, type PickerRequest, pageUrlRegistry } from "./pageLink";
import { useLibrary, usePages } from "./queries";

/** The toolbars over selected text; each sets its own gap and padding. */
const bubble = "flex items-center border border-input rounded-lg bg-bg shadow-popover text-ui leading-[1.4]";
const bubbleButton =
  "min-w-7 h-7 px-1.5 border-0 rounded-[5px] bg-transparent cursor-pointer hover:bg-hover aria-pressed:bg-hover aria-pressed:text-accent aria-pressed:font-bold";

/** Floating tools: formatting for a text selection, a link the cursor is in. */
export function BubbleMenus({ editor }: { editor: Editor }) {
  return (
    <>
      <BubbleMenu
        editor={editor}
        updateDelay={0}
        // A selected node (a rule, a component) has nothing to format; only selected text does.
        shouldShow={({ editor: e, state }) =>
          isTextSelection(state.selection) &&
          !state.selection.empty &&
          e.isEditable &&
          !e.isActive("codeBlock") &&
          !e.isActive("image")
        }
      >
        <TextTools editor={editor} />
      </BubbleMenu>
      <BubbleMenu
        editor={editor}
        // With words selected the formatting tools show instead; Link there edits the same link.
        shouldShow={({ editor: e, state }) => e.isEditable && state.selection.empty && e.isActive("link")}
        options={{ placement: "bottom-start" }}
      >
        <LinkTools editor={editor} />
      </BubbleMenu>
    </>
  );
}

function TextTools({ editor }: { editor: Editor }) {
  const [marking, setMarking] = useState(false);
  const entries = (useLibrary().data ?? []).filter((e) => e.kind === "term");
  const terms = entries.map((e) => e.key);
  const active = useEditorState({
    editor,
    selector: ({ editor: e }) => ({
      bold: e.isActive("bold"),
      italic: e.isActive("italic"),
      strike: e.isActive("strike"),
      code: e.isActive("code"),
      link: e.isActive("link"),
      term: e.isActive("term"),
      selection: `${e.state.selection.from}-${e.state.selection.to}`,
    }),
  });
  // The term form belongs to the words it was opened on: another selection starts from the tools again.
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset on every new selection
  useEffect(() => setMarking(false), [active.selection]);

  if (marking) {
    const { from, to } = editor.state.selection;
    const words = editor.state.doc.textBetween(from, to).trim().toLowerCase();

    return (
      <form
        className={`${bubble} gap-1 p-1`}
        onKeyDown={(e) => {
          if (e.key !== "Escape") return;
          setMarking(false);
          editor.commands.focus();
        }}
        onSubmit={(e) => {
          e.preventDefault();
          const term = String(new FormData(e.currentTarget).get("term") ?? "");
          if (term) editor.chain().focus().setMark("term", { term }).run();
          setMarking(false);
        }}
      >
        <select
          name="term"
          className="h-7 px-1.5 border border-input rounded-[5px] bg-bg"
          aria-label="Glossary term"
          // The selected words preselect their term, written as the term or as one of its other words.
          defaultValue={entries.find((e) => [e.key, ...e.aliases].some((w) => w.toLowerCase() === words))?.key ?? ""}
        >
          <option value="">{terms.length === 0 ? "No terms defined yet" : "Choose a term…"}</option>
          {terms.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
        <button type="submit" className={bubbleButton}>
          Apply
        </button>
        <button type="button" className={bubbleButton} onClick={() => setMarking(false)}>
          Cancel
        </button>
      </form>
    );
  }

  const tool = (label: string, glyph: string, on: boolean, run: () => void) => (
    <button
      type="button"
      className={bubbleButton}
      aria-label={label}
      aria-pressed={on}
      onClick={run}
      onMouseDown={(e) => e.preventDefault()}
    >
      {glyph}
    </button>
  );

  return (
    <div className={`${bubble} gap-1 p-1`} role="toolbar" aria-label="Text formatting">
      {tool("Bold", "B", active.bold, () => editor.chain().focus().toggleBold().run())}
      {tool("Italic", "I", active.italic, () => editor.chain().focus().toggleItalic().run())}
      {tool("Strikethrough", "S", active.strike, () => editor.chain().focus().toggleStrike().run())}
      {tool("Code", "</>", active.code, () => editor.chain().focus().toggleCode().run())}
      {tool("Glossary term", "Aa", active.term, () =>
        active.term ? editor.chain().focus().unsetMark("term").run() : setMarking(true),
      )}
      {tool("Link", "🔗", active.link, () => {
        const { from, to } = editor.state.selection;
        editor.view.dom.dispatchEvent(
          new CustomEvent<PickerRequest>(PICKER_EVENT, { detail: { mode: "selection", from, to } }),
        );
      })}
    </div>
  );
}

/** Where the link under the cursor goes, with the ways to change it. */
function LinkTools({ editor }: { editor: Editor }) {
  const pages = usePages().data ?? [];
  const link = useEditorState({
    editor,
    selector: ({ editor: e }) => {
      const attrs = e.getAttributes("link");
      return {
        href: String(attrs.href ?? ""),
        pageId: attrs.pageId ? String(attrs.pageId) : null,
        anchor: attrs.anchor ? String(attrs.anchor) : null,
      };
    },
  });
  const page = link.pageId ? pages.find((p) => p.id === link.pageId) : undefined;
  const url = link.pageId ? pageUrlRegistry.urls[link.pageId] : link.href;
  const target = link.pageId && !page ? null : (page?.title ?? link.href);

  const change = () => {
    editor.chain().focus().extendMarkRange("link").run();
    const { from, to } = editor.state.selection;
    editor.view.dom.dispatchEvent(
      new CustomEvent<PickerRequest>(PICKER_EVENT, { detail: { mode: "selection", from, to } }),
    );
  };
  const open = () => {
    if (!url) return;
    if (link.pageId) window.open(`${window.location.pathname}#page=${link.pageId}`, "_blank", "noopener");
    else window.open(url, "_blank", "noopener");
  };

  return (
    <div className={`${bubble} gap-1.5 py-2 px-2.5`} role="toolbar" aria-label="Link">
      {/* One line: the page, then its address, cut off with an ellipsis; the whole address is in the tooltip. */}
      <span
        className="flex items-baseline gap-2 max-w-95 px-1 whitespace-nowrap"
        title={url ? `${url}${link.anchor ? `#${link.anchor}` : ""}` : undefined}
      >
        {target === null ? (
          <span className="text-danger">This page no longer exists</span>
        ) : (
          <>
            <strong
              className={`min-w-0 overflow-hidden text-ellipsis ${link.pageId && url ? "flex-none max-w-50" : ""}`}
            >
              {target}
            </strong>
            {link.pageId && url && (
              <span className="min-w-0 overflow-hidden text-ellipsis text-help text-muted">
                {url}
                {link.anchor ? `#${link.anchor}` : ""}
              </span>
            )}
          </>
        )}
      </span>
      <button type="button" className={bubbleButton} onClick={change} onMouseDown={(e) => e.preventDefault()}>
        Change
      </button>
      {target !== null && (
        <button type="button" className={bubbleButton} onClick={open} onMouseDown={(e) => e.preventDefault()}>
          Open
        </button>
      )}
      <button
        type="button"
        className={bubbleButton}
        onClick={() => editor.chain().focus().extendMarkRange("link").unsetMark("link").run()}
        onMouseDown={(e) => e.preventDefault()}
      >
        Remove
      </button>
    </div>
  );
}
