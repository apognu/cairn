import type { DocNode } from "@cairn/renderer";
import { useId, useRef, useState } from "react";
import { Canvas } from "./Canvas";
import { useDeleteEntry, useLibrary, useSaveSnippet, useSaveTerm, useSaveVariable } from "./queries";
import { TermForm } from "./TermForm";
import * as ui from "./ui";

const list = "mt-0 mb-2.5 p-0 list-none border border-input rounded-lg";
const listItem = "py-2 px-3 border-b border-border last:border-b-0";
const head = "flex items-center gap-3";

/** Reusable content: variables written inline and snippets included as blocks, edited once and used on many pages. */
export function LibrarySettings() {
  const library = useLibrary();
  const saveVariable = useSaveVariable();
  const saveSnippet = useSaveSnippet();
  const saveTerm = useSaveTerm();
  const remove = useDeleteEntry();
  const [term, setTerm] = useState("");
  // The term whose form is open: an entry's ID, or `new` for the one named in the add form.
  const [editingTerm, setEditingTerm] = useState<string | null>(null);
  const id = useId();
  const [name, setName] = useState("");
  const [value, setValue] = useState("");
  const [snippetName, setSnippetName] = useState("");
  const [editing, setEditing] = useState<string | null>(null);
  const draft = useRef<DocNode | null>(null);
  // Components inside a snippet have their settings panel next to the snippet.
  const [panelHost, setPanelHost] = useState<HTMLElement | null>(null);

  const clearVariableForm = () => {
    setName("");
    setValue("");
  };
  const toggle = (entry: string) => {
    draft.current = null;
    setEditing(editing === entry ? null : entry);
  };
  const remember = (doc: DocNode) => {
    draft.current = doc;
  };

  const entries = library.data ?? [];
  const error = saveVariable.error ?? saveSnippet.error ?? saveTerm.error ?? remove.error;

  return (
    <div className="max-w-205">
      <h1 className={ui.settingsTitle}>Library</h1>
      <p className="text-muted">
        Write a value or a block of content once and use it on many pages. Editing an entry updates every page that uses
        it at the next publish.
      </p>

      <h2 className={ui.settingsHeading}>Variables</h2>
      <table className={ui.table} aria-label="Variables">
        <thead>
          <tr>
            <th scope="col" className={ui.th}>
              Name
            </th>
            <th scope="col" className={ui.th}>
              Value
            </th>
            <th scope="col" className={ui.th}>
              Used in
            </th>
            <th scope="col" className={ui.th}>
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {entries.filter((e) => e.kind === "variable").length === 0 && (
            <tr>
              <td colSpan={4} className={`${ui.td} text-muted text-help`}>
                No variables yet.
              </td>
            </tr>
          )}
          {entries
            .filter((e) => e.kind === "variable")
            .map((e) => (
              <tr key={e.id}>
                <td className={ui.td}>
                  <code>{e.key}</code>
                </td>
                <td className={ui.td}>
                  <input
                    aria-label={`Value of ${e.key}`}
                    defaultValue={e.text_value ?? ""}
                    onBlur={(ev) =>
                      ev.target.value !== (e.text_value ?? "") &&
                      saveVariable.mutate({ key: e.key, value: ev.target.value })
                    }
                  />
                </td>
                <td className={ui.td}>
                  {e.used_in.length === 0 ? (
                    <span className="text-muted">no pages</span>
                  ) : (
                    `${e.used_in.length} ${e.used_in.length === 1 ? "page" : "pages"}`
                  )}
                </td>
                <td className={ui.td}>
                  <button
                    type="button"
                    className={ui.icon}
                    aria-label={`Delete variable ${e.key}`}
                    onClick={() => remove.mutate(e.id)}
                  >
                    ×
                  </button>
                </td>
              </tr>
            ))}
        </tbody>
      </table>
      <form
        className={ui.inline}
        onSubmit={(ev) => {
          ev.preventDefault();
          saveVariable.mutate({ key: name, value }, { onSuccess: clearVariableForm });
        }}
      >
        <input
          className={ui.rowInput}
          aria-label="New variable name"
          placeholder="latest_version"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <input
          className={ui.rowInput}
          aria-label="New variable value"
          placeholder="2.4.1"
          value={value}
          onChange={(e) => setValue(e.target.value)}
        />
        <button type="submit" className={ui.rowSubmit} disabled={!name || saveVariable.isPending}>
          Add
        </button>
      </form>

      <h2 className={ui.settingsHeading}>Snippets</h2>
      <ul className={list} aria-label="Snippets">
        {entries.filter((e) => e.kind === "snippet").length === 0 && (
          <li className={`${listItem} text-muted`}>No snippets yet.</li>
        )}
        {entries
          .filter((e) => e.kind === "snippet")
          .map((e) => (
            <li key={e.id} className={listItem}>
              <div className={head}>
                <code>{e.key}</code>
                <span className="mr-auto text-muted">
                  {e.used_in.length === 0
                    ? "no pages"
                    : `${e.used_in.length} ${e.used_in.length === 1 ? "page" : "pages"}`}
                </span>
                <button type="button" className={ui.smallButton} onClick={() => toggle(e.id)}>
                  {editing === e.id ? "Close" : "Edit"}
                </button>
                <button
                  type="button"
                  className={ui.icon}
                  aria-label={`Delete snippet ${e.key}`}
                  onClick={() => remove.mutate(e.id)}
                >
                  ×
                </button>
              </div>
              {editing === e.id && (
                <div className="snippet-editor mt-2.5 py-2 px-3 border border-border rounded-lg text-[16px]">
                  <Canvas
                    doc={(e.doc ?? { type: "doc", content: [{ type: "paragraph" }] }) as DocNode}
                    embedded
                    onChange={remember}
                    panelHost={panelHost}
                  />
                  <aside
                    className={`${ui.props} w-(--panel-width) border-l border-border`}
                    aria-label="Component settings"
                    ref={setPanelHost}
                  />
                  <div className={ui.actions}>
                    <button
                      type="button"
                      className={ui.primary}
                      disabled={saveSnippet.isPending}
                      onClick={() =>
                        draft.current &&
                        saveSnippet.mutate({ key: e.key, doc: draft.current }, { onSuccess: () => setEditing(null) })
                      }
                    >
                      Save snippet
                    </button>
                  </div>
                </div>
              )}
            </li>
          ))}
      </ul>
      <form
        className={ui.inline}
        onSubmit={(ev) => {
          ev.preventDefault();
          saveSnippet.mutate(
            { key: snippetName, doc: { type: "doc", content: [{ type: "paragraph" }] } },
            { onSuccess: () => setSnippetName("") },
          );
        }}
      >
        <label htmlFor={`${id}-snippet`} className="sr-only">
          New snippet name
        </label>
        <input
          id={`${id}-snippet`}
          className={ui.rowInput}
          placeholder="prerequisites"
          value={snippetName}
          onChange={(e) => setSnippetName(e.target.value)}
        />
        <button type="submit" className={ui.rowSubmit} disabled={!snippetName || saveSnippet.isPending}>
          Add snippet
        </button>
      </form>

      <h2 className={ui.settingsHeading}>Glossary terms</h2>
      <p className="text-muted">
        Mark words in a page as a term to show this definition on hover and link them to the Glossary page, which lists
        every term.
      </p>
      <ul className={list} aria-label="Glossary terms">
        {entries.filter((e) => e.kind === "term").length === 0 && editingTerm !== "new" && (
          <li className={`${listItem} text-muted`}>No terms yet.</li>
        )}
        {entries
          .filter((e) => e.kind === "term")
          .map((e) => (
            <li key={e.id} className={listItem}>
              <div className={head}>
                <strong>{e.key}</strong>
                <span className="flex-1 min-w-0 overflow-hidden text-help text-ellipsis whitespace-nowrap">
                  {e.text_value}
                  {e.aliases.length > 0 && <span className="text-muted"> · also {e.aliases.join(", ")}</span>}
                </span>
                <span className="mr-auto text-muted">
                  {e.used_in.length === 0
                    ? "no pages"
                    : `${e.used_in.length} ${e.used_in.length === 1 ? "page" : "pages"}`}
                </span>
                <button
                  type="button"
                  className={ui.smallButton}
                  onClick={() => setEditingTerm(editingTerm === e.id ? null : e.id)}
                >
                  {editingTerm === e.id ? "Close" : "Edit"}
                </button>
                <button
                  type="button"
                  className={ui.icon}
                  aria-label={`Delete term ${e.key}`}
                  onClick={() => remove.mutate(e.id)}
                >
                  ×
                </button>
              </div>
              {editingTerm === e.id && <TermForm entry={e} term={e.key} onDone={() => setEditingTerm(null)} />}
            </li>
          ))}
        {editingTerm === "new" && (
          <li className={listItem}>
            <div className={head}>
              <strong>{term}</strong>
            </div>
            <TermForm
              term={term}
              onDone={() => {
                setEditingTerm(null);
                setTerm("");
              }}
            />
          </li>
        )}
      </ul>
      <form
        className={ui.inline}
        onSubmit={(ev) => {
          ev.preventDefault();
          if (term.trim()) setEditingTerm("new");
        }}
      >
        <label htmlFor={`${id}-term`} className="sr-only">
          New term
        </label>
        <input
          id={`${id}-term`}
          className={ui.rowInput}
          placeholder="Blue-green deploy"
          value={term}
          disabled={editingTerm === "new"}
          onChange={(e) => setTerm(e.target.value)}
        />
        <button type="submit" className={ui.rowSubmit} disabled={!term.trim() || editingTerm === "new"}>
          Add term
        </button>
      </form>

      {error && (
        <p role="alert" className="text-danger">
          {error.message}
        </p>
      )}
    </div>
  );
}
