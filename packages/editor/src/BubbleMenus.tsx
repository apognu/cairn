import type { Editor } from "@tiptap/core";
import { useEditorState } from "@tiptap/react";
import { BubbleMenu } from "@tiptap/react/menus";
import { useId } from "react";
import { PICKER_EVENT, type PickerRequest } from "./pageLink";

const languages = ["bash", "css", "html", "javascript", "json", "python", "rust", "sql", "toml", "typescript", "yaml"];

/** Floating tools: formatting for a text selection, settings for the code block the cursor is in. */
export function BubbleMenus({ editor }: { editor: Editor }) {
  return (
    <>
      <BubbleMenu
        editor={editor}
        updateDelay={0}
        shouldShow={({ editor: e, from, to }) =>
          from !== to && e.isEditable && !e.isActive("codeBlock") && !e.isActive("image")
        }
      >
        <TextTools editor={editor} />
      </BubbleMenu>
      <BubbleMenu
        editor={editor}
        shouldShow={({ editor: e }) => e.isEditable && e.isActive("codeBlock")}
        options={{ placement: "top-start" }}
      >
        <CodeBlockTools editor={editor} />
      </BubbleMenu>
      <BubbleMenu
        editor={editor}
        shouldShow={({ editor: e }) => e.isEditable && e.isActive("image")}
        options={{ placement: "bottom" }}
      >
        <ImageTools editor={editor} />
      </BubbleMenu>
    </>
  );
}

function ImageTools({ editor }: { editor: Editor }) {
  const attrs = useEditorState({
    editor,
    selector: ({ editor: e }) => ({
      alt: String(e.getAttributes("image").alt ?? ""),
      caption: String(e.getAttributes("image").caption ?? ""),
    }),
  });
  const set = (name: "alt" | "caption", value: string) =>
    editor
      .chain()
      .updateAttributes("image", { [name]: value })
      .run();

  return (
    <fieldset className="bubble">
      <legend className="sr-only">Image</legend>
      <label>
        Alt text
        <input value={attrs.alt} onChange={(e) => set("alt", e.target.value)} />
      </label>
      <label>
        Caption
        <input value={attrs.caption} onChange={(e) => set("caption", e.target.value)} />
      </label>
    </fieldset>
  );
}

function TextTools({ editor }: { editor: Editor }) {
  const active = useEditorState({
    editor,
    selector: ({ editor: e }) => ({
      bold: e.isActive("bold"),
      italic: e.isActive("italic"),
      strike: e.isActive("strike"),
      code: e.isActive("code"),
      link: e.isActive("link"),
    }),
  });

  const tool = (label: string, glyph: string, on: boolean, run: () => void) => (
    <button type="button" aria-label={label} aria-pressed={on} onClick={run} onMouseDown={(e) => e.preventDefault()}>
      {glyph}
    </button>
  );

  return (
    <div className="bubble" role="toolbar" aria-label="Text formatting">
      {tool("Bold", "B", active.bold, () => editor.chain().focus().toggleBold().run())}
      {tool("Italic", "I", active.italic, () => editor.chain().focus().toggleItalic().run())}
      {tool("Strikethrough", "S", active.strike, () => editor.chain().focus().toggleStrike().run())}
      {tool("Code", "</>", active.code, () => editor.chain().focus().toggleCode().run())}
      {tool("Link", "🔗", active.link, () => {
        const { from, to } = editor.state.selection;
        editor.view.dom.dispatchEvent(
          new CustomEvent<PickerRequest>(PICKER_EVENT, { detail: { mode: "selection", from, to } }),
        );
      })}
    </div>
  );
}

function CodeBlockTools({ editor }: { editor: Editor }) {
  const listId = useId();
  const attrs = useEditorState({
    editor,
    selector: ({ editor: e }) => ({
      language: String(e.getAttributes("codeBlock").language ?? ""),
      fileName: String(e.getAttributes("codeBlock").fileName ?? ""),
    }),
  });
  const set = (name: "language" | "fileName", value: string) =>
    editor
      .chain()
      .updateAttributes("codeBlock", { [name]: value || null })
      .run();

  return (
    <fieldset className="bubble">
      <legend className="sr-only">Code block</legend>
      <label>
        Language
        <input list={listId} value={attrs.language} onChange={(e) => set("language", e.target.value)} />
      </label>
      <datalist id={listId}>
        {languages.map((l) => (
          <option key={l} value={l} />
        ))}
      </datalist>
      <label>
        File name
        <input value={attrs.fileName} onChange={(e) => set("fileName", e.target.value)} />
      </label>
    </fieldset>
  );
}
