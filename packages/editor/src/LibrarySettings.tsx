import type { DocNode } from "@yourdocs/renderer";
import { useId, useRef, useState } from "react";
import { Canvas } from "./Canvas";
import { useDeleteEntry, useLibrary, useSaveSnippet, useSaveVariable } from "./queries";

/** Reusable content: variables written inline and snippets included as blocks, edited once and used on many pages. */
export function LibrarySettings() {
  const library = useLibrary();
  const saveVariable = useSaveVariable();
  const saveSnippet = useSaveSnippet();
  const remove = useDeleteEntry();
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
  const error = saveVariable.error ?? saveSnippet.error ?? remove.error;

  return (
    <div className="settings-form wide">
      <h1>Library</h1>
      <p className="muted">
        Write a value or a block of content once and use it on many pages. Editing an entry updates every page that uses
        it at the next publish.
      </p>

      <h2>Variables</h2>
      <table className="changes" aria-label="Variables">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">Value</th>
            <th scope="col">Used in</th>
            <th scope="col">
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {entries.filter((e) => e.kind === "variable").length === 0 && (
            <tr>
              <td colSpan={4} className="muted">
                No variables yet.
              </td>
            </tr>
          )}
          {entries
            .filter((e) => e.kind === "variable")
            .map((e) => (
              <tr key={e.id}>
                <td>
                  <code>{e.key}</code>
                </td>
                <td>
                  <input
                    aria-label={`Value of ${e.key}`}
                    defaultValue={e.text_value ?? ""}
                    onBlur={(ev) =>
                      ev.target.value !== (e.text_value ?? "") &&
                      saveVariable.mutate({ key: e.key, value: ev.target.value })
                    }
                  />
                </td>
                <td>
                  {e.used_in.length === 0 ? (
                    <span className="muted">no pages</span>
                  ) : (
                    `${e.used_in.length} ${e.used_in.length === 1 ? "page" : "pages"}`
                  )}
                </td>
                <td>
                  <button
                    type="button"
                    className="icon"
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
        className="row-inline"
        onSubmit={(ev) => {
          ev.preventDefault();
          saveVariable.mutate({ key: name, value }, { onSuccess: clearVariableForm });
        }}
      >
        <input
          aria-label="New variable name"
          placeholder="latest_version"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <input
          aria-label="New variable value"
          placeholder="2.4.1"
          value={value}
          onChange={(e) => setValue(e.target.value)}
        />
        <button type="submit" disabled={!name || saveVariable.isPending}>
          Add
        </button>
      </form>

      <h2>Snippets</h2>
      <ul className="snippet-list" aria-label="Snippets">
        {entries.filter((e) => e.kind === "snippet").length === 0 && <li className="muted">No snippets yet.</li>}
        {entries
          .filter((e) => e.kind === "snippet")
          .map((e) => (
            <li key={e.id}>
              <div className="snippet-head">
                <code>{e.key}</code>
                <span className="muted">
                  {e.used_in.length === 0
                    ? "no pages"
                    : `${e.used_in.length} ${e.used_in.length === 1 ? "page" : "pages"}`}
                </span>
                <button type="button" onClick={() => toggle(e.id)}>
                  {editing === e.id ? "Close" : "Edit"}
                </button>
                <button
                  type="button"
                  className="icon"
                  aria-label={`Delete snippet ${e.key}`}
                  onClick={() => remove.mutate(e.id)}
                >
                  ×
                </button>
              </div>
              {editing === e.id && (
                <div className="snippet-editor">
                  <Canvas
                    doc={(e.doc ?? { type: "doc", content: [{ type: "paragraph" }] }) as DocNode}
                    embedded
                    onChange={remember}
                    panelHost={panelHost}
                  />
                  <aside className="props" aria-label="Component settings" ref={setPanelHost} />
                  <div className="dialog-actions">
                    <button
                      type="button"
                      className="primary"
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
        className="row-inline"
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
          placeholder="prerequisites"
          value={snippetName}
          onChange={(e) => setSnippetName(e.target.value)}
        />
        <button type="submit" disabled={!snippetName || saveSnippet.isPending}>
          Add snippet
        </button>
      </form>

      {error && (
        <p role="alert" className="error-text">
          {error.message}
        </p>
      )}
    </div>
  );
}
