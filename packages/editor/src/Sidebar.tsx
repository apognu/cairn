import { useEffect, useState } from "react";
import { DeleteDialog } from "./DeleteDialog";
import { Icon } from "./icons";
import { useCreatePage, useDeletePage, useMovePage, usePending, useRenamePage } from "./queries";
import { IconButton, MenuItem, TitleForm } from "./TreeParts";
import {
  buildTree,
  type DropZone,
  dropTarget,
  type MoveAction,
  moveTarget,
  type Page,
  pageUrls,
  type TreeNode,
} from "./tree";

interface Props {
  pages: Page[];
  loading: boolean;
  selected: string | null;
  onSelect: (id: string | null) => void;
}

type Editing =
  | { kind: "create"; parent: string | null }
  | { kind: "create-link"; parent: string }
  | { kind: "rename"; id: string }
  | { kind: "delete"; id: string };

export function Sidebar({ pages, loading, selected, onSelect }: Props) {
  const [editing, setEditing] = useState<Editing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [drag, setDrag] = useState<{ id: string; over: { id: string; zone: DropZone } | null; alt: boolean } | null>(
    null,
  );
  const pending = usePending();
  const changed = new Set(pending.data?.changes.filter((c) => !c.kinds.includes("deleted")).map((c) => c.page_id));
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

  // An open row menu closes on a click anywhere else, on Escape, and when focus leaves it.
  useEffect(() => {
    const close = (except?: Element | null) => {
      for (const menu of document.querySelectorAll<HTMLDetailsElement>("details.menu[open]")) {
        if (menu !== except) menu.removeAttribute("open");
      }
    };
    const onPointer = (e: PointerEvent) => close((e.target as Element | null)?.closest("details.menu"));
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();

    document.addEventListener("pointerdown", onPointer);
    document.addEventListener("keydown", onKey);

    return () => {
      document.removeEventListener("pointerdown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, []);

  const submit = (title: string, url?: string, keepOldUrl?: boolean) => {
    // The form that was submitted closes when its request is done, unless the user has since opened another one.
    const submitted = editing;
    const closeIfSame = () => setEditing((now) => (now === submitted ? null : now));

    if (editing?.kind === "create") {
      // A page made on purpose is a page to write: open it.
      return create.mutateAsync({ parent_id: editing.parent, title }).then(
        (page) => {
          setError(null);
          closeIfSame();
          onSelect(page.id);
        },
        (e: unknown) => setError(e instanceof Error ? e.message : String(e)),
      );
    }
    if (editing?.kind === "create-link" && url) {
      return run(create.mutateAsync({ parent_id: editing.parent, title, external_url: url }), closeIfSame);
    }
    if (editing?.kind === "rename") {
      return run(rename.mutateAsync({ id: editing.id, title, external_url: url, redirect: keepOldUrl }), closeIfSame);
    }
  };

  const target = drag?.over ? dropTarget(pages, drag.id, drag.over.id, drag.over.zone) : null;
  const dragged = pages.find((p) => p.id === drag?.id);
  const newParent =
    target && (target.parent_id ? pages.find((p) => p.id === target.parent_id)?.title : "the top level");

  // Where the dragged page would live, to say what happens to its old URL.
  const urlChange = (() => {
    if (!drag || !target) return null;
    const moved = pages.map((p) => (p.id === drag.id ? { ...p, parent_id: target.parent_id } : p));
    const [from, to] = [pageUrls(pages)[drag.id], pageUrls(moved)[drag.id]];

    return from && to && from !== to ? { from, to } : null;
  })();

  // The Alt key is read at the drop itself, so releasing it on the way is respected.
  const drop = (alt: boolean) => {
    if (drag && target) void run(move.mutateAsync({ id: drag.id, target, redirect: !alt }));
    setDrag(null);
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
        {/* biome-ignore lint/a11y/noStaticElementInteractions: dragging is a pointer shortcut; the row menu has the same moves for the keyboard */}
        <div
          className="row"
          style={{ paddingLeft: rowIndent(depth) }}
          data-depth={Math.min(depth, 1)}
          data-selected={selected === page.id}
          data-dragging={drag?.id === page.id}
          data-drop={drag?.over?.id === page.id && target ? drag.over.zone : undefined}
          draggable={!isEditing}
          onDragStart={(e) => {
            e.dataTransfer.effectAllowed = "move";
            e.dataTransfer.setData("text/plain", page.id);
            e.dataTransfer.setDragImage(dragCard(page.title), 12, 12);
            setDrag({ id: page.id, over: null, alt: false });
          }}
          onDragOver={(e) => {
            if (!drag) return;
            const box = e.currentTarget.getBoundingClientRect();
            const at = (e.clientY - box.top) / box.height;
            const zone: DropZone = at < 0.3 ? "before" : at > 0.7 ? "after" : "inside";

            if (drag.over?.id !== page.id || drag.over.zone !== zone || drag.alt !== e.altKey) {
              setDrag({ ...drag, over: { id: page.id, zone }, alt: e.altKey });
            }
            // Only a row the page can land on accepts the drop.
            if (dropTarget(pages, drag.id, page.id, zone)) e.preventDefault();
          }}
          onDrop={(e) => {
            e.preventDefault();
            drop(e.altKey);
          }}
          onDragEnd={() => setDrag(null)}
        >
          {isEditing && editing.kind === "rename" ? (
            <TitleForm
              initial={page.title}
              initialUrl={page.external_url ?? undefined}
              label={page.kind === "link" ? "Link title" : "Page title"}
              onSubmit={submit}
              onCancel={() => setEditing(null)}
              askRedirect={page.kind === "page"}
            />
          ) : (
            <button
              type="button"
              className="title"
              aria-current={selected === page.id}
              // A link has no content to edit: clicking it edits where it goes.
              onClick={() => (page.kind === "link" ? setEditing({ kind: "rename", id: page.id }) : onSelect(page.id))}
            >
              {page.title}
              {page.kind === "link" && <Icon name="external" size={12} />}
            </button>
          )}
          {changed.has(page.id) && <span className="dot" role="img" aria-label="Unpublished changes" />}
          <span className="actions">
            {page.kind === "page" && (
              <IconButton
                label={`Add a subpage to ${page.title}`}
                glyph="+"
                onClick={() => setEditing({ kind: "create", parent: page.id })}
              />
            )}
            <details className="menu">
              <summary aria-label={`More actions for ${page.title}`} title="More actions">
                ⋯
              </summary>
              <div className="menu-pop">
                <MenuItem
                  label={`Rename ${page.title}`}
                  text={page.kind === "link" ? "Edit link" : "Rename"}
                  onClick={() => setEditing({ kind: "rename", id: page.id })}
                />
                {page.kind === "page" && (
                  <MenuItem
                    label={`Add an external link under ${page.title}`}
                    text="Add external link"
                    onClick={() => setEditing({ kind: "create-link", parent: page.id })}
                  />
                )}
                <MenuItem label={`Move ${page.title} up`} text="Move up" onClick={() => doMove(page.id, "up")} />
                <MenuItem label={`Move ${page.title} down`} text="Move down" onClick={() => doMove(page.id, "down")} />
                <MenuItem label={`Indent ${page.title}`} text="Indent" onClick={() => doMove(page.id, "indent")} />
                <MenuItem label={`Outdent ${page.title}`} text="Outdent" onClick={() => doMove(page.id, "outdent")} />
                <hr />
                <MenuItem
                  danger
                  label={`Delete ${page.title}`}
                  text="Delete…"
                  onClick={() => setEditing({ kind: "delete", id: page.id })}
                />
              </div>
            </details>
          </span>
        </div>
        {(node.children.length > 0 || (editing && "parent" in editing && editing.parent === page.id)) && (
          <ul>
            {node.children.map((child) => renderNode(child, depth + 1))}
            {editing?.kind === "create" && editing.parent === page.id && (
              <li style={{ paddingLeft: rowIndent(depth + 1) }}>
                <TitleForm initial="" label="New page title" onSubmit={submit} onCancel={() => setEditing(null)} />
              </li>
            )}
            {editing?.kind === "create-link" && editing.parent === page.id && (
              <li style={{ paddingLeft: rowIndent(depth + 1) }}>
                <TitleForm
                  initial=""
                  initialUrl=""
                  label="New link title"
                  onSubmit={submit}
                  onCancel={() => setEditing(null)}
                />
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
      {editing?.kind === "delete" && pages.find((p) => p.id === editing.id) && (
        <DeleteDialog
          page={pages.find((p) => p.id === editing.id) as Page}
          onClose={() => setEditing(null)}
          onConfirm={() => void doDelete(editing.id)}
        />
      )}
      {drag && target && dragged && (
        <div className="drag-hint" aria-live="polite">
          <Icon name="move" size={16} />
          <div>
            <strong>
              Drop to move “{dragged.title}” under {newParent}
            </strong>
            {urlChange && (
              <div className="drag-urls">
                {drag.alt ? "No redirect: " : "A redirect will be added: "}
                <code>{drag.alt ? urlChange.from : `${urlChange.from} → ${urlChange.to}`}</code>
                {drag.alt && " will stop working"}
              </div>
            )}
          </div>
          <span>
            {drag.alt ? "Release ⌥ to keep a redirect" : "Hold ⌥ to move without a redirect"}
            <br />
            Esc to cancel
          </span>
        </div>
      )}
    </nav>
  );
}

/** What follows the pointer while a page is dragged. The browser snapshots the element, so it can go right after. */
function dragCard(title: string): HTMLElement {
  const card = document.createElement("div");
  card.className = "drag-card";
  card.textContent = title;
  document.body.append(card);
  setTimeout(() => card.remove(), 0);

  return card;
}

/** Top-level pages are the section headings of the sidebar; everything below is indented under them. */
const rowIndent = (depth: number) => (depth === 0 ? 10 : 26 + (depth - 1) * 16);
