import { useState } from "react";
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

  const renderNode = (node: TreeNode, depth: number) => {
    const { page } = node;
    const isEditing = editing && "id" in editing && editing.id === page.id;

    return (
      <li key={page.id}>
        <div className="row" style={{ paddingLeft: 10 + depth * 16 }} data-selected={selected === page.id}>
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
              <IconButton
                label={`Rename ${page.title}`}
                glyph="✎"
                onClick={() => setEditing({ kind: "rename", id: page.id })}
              />
              <IconButton label={`Move ${page.title} up`} glyph="↑" onClick={() => doMove(page.id, "up")} />
              <IconButton label={`Move ${page.title} down`} glyph="↓" onClick={() => doMove(page.id, "down")} />
              <IconButton label={`Indent ${page.title}`} glyph="→" onClick={() => doMove(page.id, "indent")} />
              <IconButton label={`Outdent ${page.title}`} glyph="←" onClick={() => doMove(page.id, "outdent")} />
              <IconButton
                label={`Delete ${page.title}`}
                glyph="×"
                onClick={() => setEditing({ kind: "delete", id: page.id })}
              />
            </span>
          )}
        </div>
        {(node.children.length > 0 || (editing?.kind === "create" && editing.parent === page.id)) && (
          <ul>
            {node.children.map((child) => renderNode(child, depth + 1))}
            {editing?.kind === "create" && editing.parent === page.id && (
              <li style={{ paddingLeft: 10 + (depth + 1) * 16 }}>
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
        <span>Pages</span>
        <IconButton label="New page" glyph="+" onClick={() => setEditing({ kind: "create", parent: null })} />
      </div>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {loading && <p className="muted pad">Loading…</p>}
      <ul className="tree">
        {buildTree(pages).map((node) => renderNode(node, 0))}
        {editing?.kind === "create" && editing.parent === null && (
          <li style={{ paddingLeft: 10 }}>
            <TitleForm initial="" label="New page title" onSubmit={submit} onCancel={() => setEditing(null)} />
          </li>
        )}
      </ul>
    </nav>
  );
}

function IconButton({ label, glyph, onClick }: { label: string; glyph: string; onClick: () => void }) {
  return (
    <button type="button" className="icon" aria-label={label} title={label} onClick={onClick}>
      <span aria-hidden="true">{glyph}</span>
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
