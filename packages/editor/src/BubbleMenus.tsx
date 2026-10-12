import type { Editor } from "@tiptap/core";
import { useEditorState } from "@tiptap/react";
import { BubbleMenu } from "@tiptap/react/menus";
import { useId, useState } from "react";

const languages = ["bash", "css", "html", "javascript", "json", "python", "rust", "sql", "toml", "typescript", "yaml"];
const tones = ["note", "tip", "warning"];

/** Floating tools: formatting for a text selection, settings for the code block or callout the cursor is in. */
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
        shouldShow={({ editor: e }) => e.isEditable && e.isActive("callout")}
        options={{ placement: "top-start" }}
      >
        <CalloutTools editor={editor} />
      </BubbleMenu>
    </>
  );
}

function TextTools({ editor }: { editor: Editor }) {
  const [linking, setLinking] = useState(false);
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

  if (linking) {
    return (
      <form
        className="bubble"
        onSubmit={(e) => {
          e.preventDefault();
          const href = String(new FormData(e.currentTarget).get("href") ?? "").trim();
          const chain = editor.chain().focus().extendMarkRange("link");
          (href ? chain.setLink({ href }) : chain.unsetLink()).run();
          setLinking(false);
        }}
      >
        <input
          name="href"
          aria-label="Link address"
          placeholder="https://…"
          defaultValue={String(editor.getAttributes("link").href ?? "")}
        />
        <button type="submit">Apply</button>
        <button type="button" onClick={() => setLinking(false)}>
          Cancel
        </button>
      </form>
    );
  }

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
      {tool("Link", "🔗", active.link, () => setLinking(true))}
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

function CalloutTools({ editor }: { editor: Editor }) {
  const tone = useEditorState({
    editor,
    selector: ({ editor: e }) => String(e.getAttributes("callout").tone ?? "note"),
  });

  return (
    <fieldset className="bubble">
      <legend className="sr-only">Callout</legend>
      <label>
        Type
        <select
          value={tone}
          onChange={(e) => editor.chain().updateAttributes("callout", { tone: e.target.value }).run()}
        >
          {tones.map((t) => (
            <option key={t}>{t}</option>
          ))}
        </select>
      </label>
    </fieldset>
  );
}
