import type { DocNode } from "@cairn/renderer";
import { Extension } from "@tiptap/core";
import { EditorContent, useEditor, useEditorState } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { useId, useRef, useState } from "react";
import type { components } from "./api/schema";
import { useSaveTerm } from "./queries";
import { PageField } from "./SchemaForm";

type Entry = components["schemas"]["LibraryEntry"];

const EMPTY: DocNode = { type: "doc", content: [{ type: "paragraph" }] };

/**
 * Edits one glossary term: its short definition (text with bold, italic and code), the other words for it, and the page that explains it
 * in full. `entry` is absent for a new term, named `term`.
 */
export function TermForm({ entry, term, onDone }: { entry?: Entry; term: string; onDone: () => void }) {
  const id = useId();
  const save = useSaveTerm();
  const definition = useRef<DocNode>((entry?.doc as DocNode | null) ?? EMPTY);
  const [aliases, setAliases] = useState((entry?.aliases ?? []).join(", "));
  const [page, setPage] = useState(entry?.page_id ?? "");

  const submit = () =>
    save.mutate(
      {
        term,
        definition: definition.current,
        aliases: aliases
          .split(",")
          .map((a) => a.trim())
          .filter(Boolean),
        page_id: page || null,
      },
      { onSuccess: onDone },
    );

  return (
    <form
      className="term-form"
      aria-label={`Term ${term}`}
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
      onKeyDown={(e) => {
        if (e.key !== "Escape") return;
        e.preventDefault();
        onDone();
      }}
    >
      <div className="form-field">
        <span className="form-label" id={`${id}-definition`}>
          Definition
        </span>
        <DefinitionField
          labelledBy={`${id}-definition`}
          initial={definition.current}
          onChange={(doc) => {
            definition.current = doc;
          }}
          onSubmit={submit}
        />
        <small className="muted">
          A sentence or two, shown when readers hover the term. Shift+Enter starts a new line.
        </small>
      </div>
      <div className="form-field">
        <label htmlFor={`${id}-aliases`}>Also called</label>
        <input
          id={`${id}-aliases`}
          placeholder="Red-black deploy, zero-downtime deploy"
          value={aliases}
          onChange={(e) => setAliases(e.target.value)}
        />
        <small className="muted">Other words for the same thing, separated by commas.</small>
      </div>
      <div className="form-field">
        <label htmlFor={`${id}-page`}>Full explanation</label>
        <PageField id={`${id}-page`} value={page} onChange={setPage} />
      </div>
      {save.error && (
        <p role="alert" className="error-text">
          {save.error.message}
        </p>
      )}
      <div className="dialog-actions">
        <button type="button" onClick={onDone}>
          Cancel
        </button>
        <button type="submit" className="primary" disabled={save.isPending}>
          {entry ? "Save term" : "Add term"}
        </button>
      </div>
    </form>
  );
}

/** Enter submits instead of starting a paragraph; `submit` holds the current handler. */
function submitOnEnter(submit: { current: () => void }) {
  return Extension.create({
    name: "submitOnEnter",
    addKeyboardShortcuts: () => ({
      Enter: () => {
        submit.current();
        return true;
      },
    }),
  });
}

/** A small rich-text field: paragraphs with bold, italic and code. Enter submits the form, as in other one-line fields. */
function DefinitionField({
  labelledBy,
  initial,
  onChange,
  onSubmit,
}: {
  labelledBy: string;
  initial: DocNode;
  onChange: (doc: DocNode) => void;
  onSubmit: () => void;
}) {
  // The editor keeps the first callbacks it was given; these refs reach the current ones.
  const submit = useRef(onSubmit);
  submit.current = onSubmit;

  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        heading: false,
        bulletList: false,
        orderedList: false,
        listItem: false,
        listKeymap: false,
        blockquote: false,
        codeBlock: false,
        horizontalRule: false,
        link: false,
        underline: false,
        strike: false,
        dropcursor: false,
        gapcursor: false,
        trailingNode: false,
      }),
      submitOnEnter(submit),
    ],
    content: initial,
    autofocus: "end",
    editorProps: { attributes: { role: "textbox", "aria-multiline": "true", "aria-labelledby": labelledBy } },
    onUpdate: ({ editor: e }) => onChange(e.getJSON() as DocNode),
  });
  const active = useEditorState({
    editor,
    selector: ({ editor: e }) => ({
      bold: e?.isActive("bold") ?? false,
      italic: e?.isActive("italic") ?? false,
      code: e?.isActive("code") ?? false,
    }),
  });

  const tool = (label: string, glyph: string, on: boolean, run: () => void) => (
    <button type="button" aria-label={label} aria-pressed={on} onClick={run} onMouseDown={(e) => e.preventDefault()}>
      {glyph}
    </button>
  );

  return (
    <div className="definition-field">
      <div className="definition-tools" role="toolbar" aria-label="Definition formatting">
        {tool("Bold", "B", active?.bold ?? false, () => editor?.chain().focus().toggleBold().run())}
        {tool("Italic", "I", active?.italic ?? false, () => editor?.chain().focus().toggleItalic().run())}
        {tool("Code", "</>", active?.code ?? false, () => editor?.chain().focus().toggleCode().run())}
      </div>
      <EditorContent editor={editor} />
    </div>
  );
}
