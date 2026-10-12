import { useState } from "react";
import { Icon } from "./icons";
import { useCreatePage, useDeletePage, useMovePage, useRenamePage } from "./queries";
import { buildTree, type MoveAction, moveTarget, type Page, type TreeNode } from "./tree";

interface Props {
  pages: Page[];
  loading: boolean;
  selected: string | null;
  onSelect: (id: string | null) => void;
}

type Editing =
  | { kind: "create"; parent: string | null }
  | { kind: "rename"; id: string }
  | { kind: "delete"; id: string };

export function Sidebar({ pages, loading, selected, onSelect }: Props) {
  const [editing, setEditing] = useState<Editing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const create = useCreatePage();
  const rename = useRenamePage();
  const move = useMovePage();
  const remove = useDeletePage();

  const run = (action: Promise<unknown>, done?: () => void) =>
    action.then(
      () => {
        setError(null);
        done?.();
      },
      (e: unknown) => setError(e instanceof Error ? e.message : String(e)),
    );

  const submit = (title: string) => {
    if (editing?.kind === "create") {
      return run(create.mutateAsync({ parent_id: editing.parent, title }), () => setEditing(null));
    }
    if (editing?.kind === "rename") {
      return run(rename.mutateAsync({ id: editing.id, title }), () => setEditing(null));
    }
  };

  const doMove = (id: string, action: MoveAction) => {
    const target = moveTarget(pages, id, action);
    if (target) void run(move.mutateAsync({ id, target }));
  };

  const doDelete = (id: string) =>
    run(remove.mutateAsync(id), () => {
      setEditing(null);
      if (selected === id) onSelect(null);
    });

  const needle = query.trim().toLowerCase();
  const search = needle ? pages.filter((p) => p.title.toLowerCase().includes(needle)) : null;

  const renderNode = (node: TreeNode, depth: number) => {
    const { page } = node;
    const isEditing = editing && "id" in editing && editing.id === page.id;

    return (
      <li key={page.id}>
        <div
          className="row"
          style={{ paddingLeft: rowIndent(depth) }}
          data-depth={Math.min(depth, 1)}
          data-selected={selected === page.id}
        >
          {isEditing && editing.kind === "rename" ? (
            <TitleForm initial={page.title} label="Page title" onSubmit={submit} onCancel={() => setEditing(null)} />
          ) : (
            <button
              type="button"
              className="title"
              aria-current={selected === page.id}
              onClick={() => onSelect(page.id)}
            >
              {page.title}
            </button>
          )}
          {isEditing && editing.kind === "delete" ? (
            <span className="confirm">
              Delete with subpages?
              <button type="button" onClick={() => void doDelete(page.id)}>
                Delete
              </button>
              <button type="button" onClick={() => setEditing(null)}>
                Cancel
              </button>
            </span>
          ) : (
            <span className="actions">
              <IconButton
                label={`Add a subpage to ${page.title}`}
                glyph="+"
                onClick={() => setEditing({ kind: "create", parent: page.id })}
              />
              <details className="menu">
                <summary aria-label={`More actions for ${page.title}`} title="More actions">
                  ⋯
                </summary>
                <div className="menu-pop">
                  <MenuItem
                    label={`Rename ${page.title}`}
                    text="Rename"
                    onClick={() => setEditing({ kind: "rename", id: page.id })}
                  />
                  <MenuItem label={`Move ${page.title} up`} text="Move up" onClick={() => doMove(page.id, "up")} />
                  <MenuItem
                    label={`Move ${page.title} down`}
                    text="Move down"
                    onClick={() => doMove(page.id, "down")}
                  />
                  <MenuItem label={`Indent ${page.title}`} text="Indent" onClick={() => doMove(page.id, "indent")} />
                  <MenuItem label={`Outdent ${page.title}`} text="Outdent" onClick={() => doMove(page.id, "outdent")} />
                  <MenuItem
                    label={`Delete ${page.title}`}
                    text="Delete…"
                    onClick={() => setEditing({ kind: "delete", id: page.id })}
                  />
                </div>
              </details>
            </span>
          )}
        </div>
        {(node.children.length > 0 || (editing?.kind === "create" && editing.parent === page.id)) && (
          <ul>
            {node.children.map((child) => renderNode(child, depth + 1))}
            {editing?.kind === "create" && editing.parent === page.id && (
              <li style={{ paddingLeft: rowIndent(depth + 1) }}>
                <TitleForm initial="" label="New page title" onSubmit={submit} onCancel={() => setEditing(null)} />
              </li>
            )}
          </ul>
        )}
      </li>
    );
  };

  return (
    <nav className="sidebar" aria-label="Pages">
      <div className="sidebar-head">
        <input
          type="search"
          aria-label="Find a page"
          placeholder="Find a page"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <button
          type="button"
          className="new-page"
          aria-label="New page"
          onClick={() => setEditing({ kind: "create", parent: null })}
        >
          <Icon name="plus" size={15} />
        </button>
      </div>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {loading && <p className="muted pad">Loading…</p>}
      <ul className="tree">
        {(search ? search.map((page) => ({ page, children: [] })) : buildTree(pages)).map((node) =>
          renderNode(node, search ? 1 : 0),
        )}
        {editing?.kind === "create" && editing.parent === null && (
          <li style={{ paddingLeft: rowIndent(0) }}>
            <TitleForm initial="" label="New page title" onSubmit={submit} onCancel={() => setEditing(null)} />
          </li>
        )}
      </ul>
    </nav>
  );
}

/** Top-level pages are the section headings of the sidebar; everything below is indented under them. */
const rowIndent = (depth: number) => (depth === 0 ? 10 : 26 + (depth - 1) * 16);

function IconButton({ label, glyph, onClick }: { label: string; glyph: string; onClick: () => void }) {
  return (
    <button type="button" className="icon" aria-label={label} title={label} onClick={onClick}>
      <span aria-hidden="true">{glyph}</span>
    </button>
  );
}

function MenuItem({ label, text, onClick }: { label: string; text: string; onClick: () => void }) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={(e) => {
        e.currentTarget.closest("details")?.removeAttribute("open");
        onClick();
      }}
    >
      {text}
    </button>
  );
}

function TitleForm({
  initial,
  label,
  onSubmit,
  onCancel,
}: {
  initial: string;
  label: string;
  onSubmit: (title: string) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState(initial);

  return (
    <form
      className="title-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (value.trim()) onSubmit(value.trim());
      }}
    >
      <input
        aria-label={label}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => e.key === "Escape" && onCancel()}
      />
    </form>
  );
}
