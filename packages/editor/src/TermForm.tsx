import type { DocNode } from "@cairn/renderer";
import { Extension } from "@tiptap/core";
import { EditorContent, useEditor, useEditorState } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { useId, useRef, useState } from "react";
import type { components } from "./api/schema";
import { useSaveTerm } from "./queries";
import { PageField } from "./SchemaForm";
import * as ui from "./ui";

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
      className="mt-2.5 p-3 border border-border rounded-lg"
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
      <div className={ui.field}>
        <span className={ui.fieldLabel} id={`${id}-definition`}>
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
        <small className="text-muted">
          A sentence or two, shown when readers hover the term. Shift+Enter starts a new line.
        </small>
      </div>
      <div className={ui.field}>
        <label htmlFor={`${id}-aliases`} className={ui.fieldLabel}>
          Also called
        </label>
        <input
          id={`${id}-aliases`}
          className={ui.input}
          placeholder="Red-black deploy, zero-downtime deploy"
          value={aliases}
          onChange={(e) => setAliases(e.target.value)}
        />
        <small className="text-muted">Other words for the same thing, separated by commas.</small>
      </div>
      {/* TODO(M6b.6): `form-field` still styles the page field inside until SchemaForm moves to utilities. */}
      <div className={`${ui.field} form-field`}>
        <label htmlFor={`${id}-page`} className={ui.fieldLabel}>
          Full explanation
        </label>
        <PageField id={`${id}-page`} value={page} onChange={setPage} />
      </div>
      {save.error && (
        <p role="alert" className="text-danger">
          {save.error.message}
        </p>
      )}
      <div className={ui.actions}>
        <button type="button" className={ui.secondary} onClick={onDone}>
          Cancel
        </button>
        <button type="submit" className={ui.primary} disabled={save.isPending}>
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
    editorProps: {
      attributes: {
        role: "textbox",
        "aria-multiline": "true",
        "aria-labelledby": labelledBy,
        class: "min-h-8.5 py-1.75 px-2.5 text-ui leading-[1.45] outline-none",
      },
    },
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
    <button
      type="button"
      className="min-w-7 h-6 px-1.5 border-0 rounded-[5px] bg-transparent text-label cursor-pointer hover:bg-hover aria-pressed:bg-hover aria-pressed:text-accent aria-pressed:font-bold"
      aria-label={label}
      aria-pressed={on}
      onClick={run}
      onMouseDown={(e) => e.preventDefault()}
    >
      {glyph}
    </button>
  );

  return (
    <div className="border border-input rounded-md bg-bg focus-within:outline-2 focus-within:outline-offset-1 focus-within:outline-accent [&_p]:m-0">
      <div className="flex gap-0.5 p-[3px] border-b border-border" role="toolbar" aria-label="Definition formatting">
        {tool("Bold", "B", active?.bold ?? false, () => editor?.chain().focus().toggleBold().run())}
        {tool("Italic", "I", active?.italic ?? false, () => editor?.chain().focus().toggleItalic().run())}
        {tool("Code", "</>", active?.code ?? false, () => editor?.chain().focus().toggleCode().run())}
      </div>
      <EditorContent editor={editor} />
    </div>
  );
}
