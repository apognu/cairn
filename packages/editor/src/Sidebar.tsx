import { useState } from "react";
import { DeleteDialog } from "./DeleteDialog";
import { Icon } from "./icons";
import { useCreatePage, useDeletePage, useMovePage, usePending, useRenamePage } from "./queries";
import { IconButton, MenuItem, placeMenu, rowClass, TitleForm, titleClass } from "./TreeParts";
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
import * as ui from "./ui";
import { markCreated, useLocks } from "./useLock";

interface Props {
  pages: Page[];
  loading: boolean;
  selected: string | null;
  onSelect: (id: string | null) => void;
}

type Editing =
  | { kind: "create"; parent: string | null }
  | { kind: "create-link"; parent: string }
  | { kind: "create-api"; parent: string | null }
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
  const locks = useLocks();
  const lockedBy = new Map((locks.data ?? []).filter((l) => !l.mine).map((l) => [l.page_id, l.user_name]));
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
          markCreated(page.id);
          onSelect(page.id);
        },
        (e: unknown) => setError(e instanceof Error ? e.message : String(e)),
      );
    }
    if (editing?.kind === "create-api") {
      return create.mutateAsync({ parent_id: editing.parent, title, kind: "api" }).then(
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

  // Deleting a page takes its subpages to the trash too: the open page may be one of them.
  const within = (id: string | null, ancestor: string) => {
    for (
      let at = id, steps = 0;
      at && steps <= pages.length;
      at = pages.find((p) => p.id === at)?.parent_id ?? null, steps++
    ) {
      if (at === ancestor) return true;
    }
    return false;
  };
  const doDelete = (id: string) =>
    run(remove.mutateAsync(id), () => {
      setEditing(null);
      if (within(selected, id)) onSelect(null);
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
          className={rowClass({
            selected: selected === page.id,
            depth,
            dragging: drag?.id === page.id,
            drop: drag?.over?.id === page.id && target ? drag.over.zone : undefined,
          })}
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
            // Only a row the page can land on accepts the drop. With Alt held the browser proposes a copy, which a "move" drag
            // refuses, so the drop would never come: the effect is set to move whatever the keys.
            if (dropTarget(pages, drag.id, page.id, zone)) {
              e.preventDefault();
              e.dataTransfer.dropEffect = "move";
            }
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
              askRedirect={page.kind !== "link"}
            />
          ) : (
            <button
              type="button"
              className={titleClass({ selected: selected === page.id, depth })}
              aria-current={selected === page.id}
              // A link has no content to edit: clicking it edits where it goes.
              onClick={() => (page.kind === "link" ? setEditing({ kind: "rename", id: page.id }) : onSelect(page.id))}
            >
              {page.kind === "api" && (
                <span className="mr-1.5 font-mono text-[11px] text-accent" aria-hidden="true">
                  API
                </span>
              )}
              {page.title}
              {page.kind === "link" && <Icon name="external" size={12} className="ml-1.5 text-muted align-[-1px]" />}
            </button>
          )}
          {lockedBy.has(page.id) && (
            <span
              className="flex flex-none mx-1 text-warn-icon"
              role="img"
              aria-label={`Locked by ${lockedBy.get(page.id)}`}
              title={`${lockedBy.get(page.id)} is editing`}
            >
              <Icon name="lock" size={13} />
            </span>
          )}
          {changed.has(page.id) && <span className={ui.dot} role="img" aria-label="Unpublished changes" />}
          {/* Floats over the end of the row so titles keep their width. Hidden by opacity, not display, so keyboard users can tab to it. */}
          <span
            className={`absolute right-1 gap-0.5 pl-1 bg-hover rounded-md opacity-0 pointer-events-none group-hover/row:opacity-100 group-hover/row:pointer-events-auto group-focus-within/row:opacity-100 group-focus-within/row:pointer-events-auto has-[details[open]]:opacity-100 has-[details[open]]:pointer-events-auto ${isEditing && editing.kind === "rename" ? "hidden" : "flex"}`}
          >
            {page.kind !== "link" && (
              <IconButton
                label={`Add a subpage to ${page.title}`}
                glyph="+"
                onClick={() => setEditing({ kind: "create", parent: page.id })}
              />
            )}
            <details data-menu className={ui.menu} onToggle={placeMenu}>
              <summary className={ui.menuSummary} aria-label={`More actions for ${page.title}`} title="More actions">
                ⋯
              </summary>
              <div data-menu-pop className={ui.treeMenuPop}>
                <MenuItem
                  label={`Rename ${page.title}`}
                  text={page.kind === "link" ? "Edit link" : "Rename"}
                  onClick={() => setEditing({ kind: "rename", id: page.id })}
                />
                {page.kind !== "link" && (
                  <>
                    <MenuItem
                      label={`Add an API reference under ${page.title}`}
                      text="Add API reference"
                      onClick={() => setEditing({ kind: "create-api", parent: page.id })}
                    />
                    <MenuItem
                      label={`Add an external link under ${page.title}`}
                      text="Add external link"
                      onClick={() => setEditing({ kind: "create-link", parent: page.id })}
                    />
                  </>
                )}
                <MenuItem label={`Move ${page.title} up`} text="Move up" onClick={() => doMove(page.id, "up")} />
                <MenuItem label={`Move ${page.title} down`} text="Move down" onClick={() => doMove(page.id, "down")} />
                <MenuItem label={`Indent ${page.title}`} text="Indent" onClick={() => doMove(page.id, "indent")} />
                <MenuItem label={`Outdent ${page.title}`} text="Outdent" onClick={() => doMove(page.id, "outdent")} />
                <hr className={ui.menuRule} />
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
          <ul className={list}>
            {node.children.map((child) => renderNode(child, depth + 1))}
            {editing?.kind === "create" && editing.parent === page.id && (
              <li style={{ paddingLeft: rowIndent(depth + 1) }}>
                <TitleForm initial="" label="New page title" onSubmit={submit} onCancel={() => setEditing(null)} />
              </li>
            )}
            {editing?.kind === "create-api" && editing.parent === page.id && (
              <li style={{ paddingLeft: rowIndent(depth + 1) }}>
                <TitleForm
                  initial=""
                  label="New API reference title"
                  onSubmit={submit}
                  onCancel={() => setEditing(null)}
                />
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
    <nav className={sidebar} aria-label="Pages">
      <div className="flex gap-1.5 mx-0.5 mt-0 mb-1.5">
        <input
          type="search"
          className="flex-1 min-w-0 h-8 px-2.5 border border-input rounded-md bg-bg text-[13.5px]"
          aria-label="Find a page"
          placeholder="Find a page"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <button
          type="button"
          className={`flex items-center justify-center w-8 h-8 rounded-md ${ui.button}`}
          aria-label="New page"
          onClick={() => setEditing({ kind: "create", parent: null })}
        >
          <Icon name="plus" size={15} />
        </button>
        <details data-menu className={ui.menu}>
          <summary
            className={`flex items-center justify-center w-8 h-8 rounded-md list-none [&::-webkit-details-marker]:hidden ${ui.button}`}
            aria-label="More ways to add"
            title="More ways to add"
          >
            <Icon name="chevron" size={15} />
          </summary>
          <div data-menu-pop className={ui.treeMenuPop}>
            <MenuItem
              label="New API reference"
              text="API reference"
              onClick={() => setEditing({ kind: "create-api", parent: null })}
            />
          </div>
        </details>
      </div>
      {error && (
        <p role="alert" className={ui.errorBox}>
          {error}
        </p>
      )}
      {loading && <p className="text-muted px-2.5">Loading…</p>}
      <ul className={list}>
        {(search ? search.map((page) => ({ page, children: [] })) : buildTree(pages)).map((node) =>
          renderNode(node, search ? 1 : 0),
        )}
        {editing?.kind === "create" && editing.parent === null && (
          <li style={{ paddingLeft: rowIndent(0) }}>
            <TitleForm initial="" label="New page title" onSubmit={submit} onCancel={() => setEditing(null)} />
          </li>
        )}
        {editing?.kind === "create-api" && editing.parent === null && (
          <li style={{ paddingLeft: rowIndent(0) }}>
            <TitleForm initial="" label="New API reference title" onSubmit={submit} onCancel={() => setEditing(null)} />
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
        <div
          className="fixed bottom-7 left-1/2 -translate-x-1/2 flex items-center gap-3.5 py-3 px-4 rounded-[10px] bg-toast text-toast-text text-ui shadow-[0_10px_30px_rgb(16_20_22/30%)] pointer-events-none"
          aria-live="polite"
        >
          <Icon name="move" size={16} className="text-toast-icon" />
          <div>
            <strong>
              Drop to move “{dragged.title}” under {newParent}
            </strong>
            {urlChange && (
              <div className="mt-0.5 text-help text-toast-muted">
                {drag.alt ? "No redirect: " : "A redirect will be added: "}
                <code className="font-mono text-[12.5px]">
                  {drag.alt ? urlChange.from : `${urlChange.from} → ${urlChange.to}`}
                </code>
                {drag.alt && " will stop working"}
              </div>
            )}
          </div>
          <span className="text-[12.5px] text-toast-muted">
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
  card.className =
    "fixed -top-25 py-1.75 px-3 border border-input rounded-[7px] bg-bg shadow-[0_8px_20px_rgb(16_20_22/18%)] text-ui font-semibold";
  card.textContent = title;
  document.body.append(card);
  setTimeout(() => card.remove(), 0);

  return card;
}

/** `sidebar` is what `placeMenu` measures a menu against. */
/** On a phone it opens over the page, under the top bar, while the shell carries `data-nav-open`. */
export const sidebar =
  "sidebar bg-side border-r border-border overflow-y-auto py-3 px-2 text-ui leading-[1.4] mobile:fixed mobile:top-13 mobile:bottom-0 mobile:left-0 mobile:z-30 mobile:w-[min(300px,85vw)] mobile:shadow-[12px_0_32px_rgb(16_20_22/16%)] mobile:not-in-data-nav-open:hidden";
const list = "list-none m-0 p-0";

/** Top-level pages are the section headings of the sidebar; everything below is indented under them. */
const rowIndent = (depth: number) => (depth === 0 ? 10 : 26 + (depth - 1) * 16);
