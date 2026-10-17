import type { DocNode } from "@cairn/renderer";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api/client";
import { unwrap } from "./api/unwrap";
import { Canvas } from "./Canvas";
import { LeadField } from "./LeadField";
import { type LockActions, LockBanner, ViewerBanner } from "./LockBanner";
import { discardLocal, readLocal } from "./localDrafts";
import { atLeast, useMe, usePages } from "./queries";
import { ReviewBanner } from "./ReviewBanner";
import { TitleField } from "./TitleField";
import { type Page, pageUrls } from "./tree";
import * as ui from "./ui";
import { type DraftLock, type InitialDraft, type SaveStatus, saveSettled, useDraftSync } from "./useDraftSync";
import { markCreated, usePageLock } from "./useLock";

/** What the top bar says of the open page: how its saving goes, or that it can only be read. */
export type PageStatus = SaveStatus | "read-only";

interface Props {
  page: Page;
  onStatus: (status: PageStatus | null) => void;
  panelHost: HTMLElement | null;
  /** Opens another page, such as the one made from work kept after a take-over. */
  onOpen?: (id: string) => void;
}

/** Opens the page for editing when its lock can be had, and read-only while someone else holds it or for a viewer. */
export function PageEditor(props: Props) {
  const role = useMe().data?.role;

  if (!role) return <p className="text-muted px-2.5">Loading…</p>;
  return (
    <>
      <ReviewBanner page={props.page} />
      {/* A viewer never takes the lock, so their reading does not keep anyone else from editing. */}
      {atLeast(role, "editor") ? (
        <LockedEditor {...props} />
      ) : (
        <ViewerPage page={props.page} onStatus={props.onStatus} />
      )}
    </>
  );
}

function ViewerPage({ page, onStatus }: Pick<Props, "page" | "onStatus">) {
  useEffect(() => {
    onStatus("read-only");
    return () => onStatus(null);
  }, [onStatus]);

  return <ReadOnlyPage page={page} banner={<ViewerBanner />} />;
}

function LockedEditor({ page, onStatus, panelHost, onOpen }: Props) {
  const role = useMe().data?.role;
  const lock = usePageLock(page.id, page.title);
  const kept = useKeptWork(page, lock.state.kind === "taken", onOpen);
  // The sync must see one object for the page's whole life, and the latest lock functions through it.
  const latest = useRef(lock);
  latest.current = lock;
  const draftLock = useMemo<DraftLock>(
    () => ({ token: () => latest.current.token(), refused: (status) => void latest.current.recheck(status) }),
    [],
  );

  useEffect(() => () => onStatus(null), [onStatus]);
  useEffect(() => {
    if (lock.state.kind !== "mine") onStatus(lock.state.kind === "loading" ? null : "read-only");
  }, [lock.state.kind, onStatus]);

  switch (lock.state.kind) {
    case "loading":
      return <p className="text-muted px-2.5">Loading…</p>;
    case "mine":
      return <EditablePage page={page} onStatus={onStatus} panelHost={panelHost} lock={draftLock} />;
    default: {
      const actions: LockActions = {
        edit: () => void lock.acquire(),
        takeOver: lock.takeOver,
        watching: lock.watching,
        watch: () => void lock.watch(),
        canForce: atLeast(role, "admin"),
        ...kept,
      };
      return <ReadOnlyPage page={page} banner={<LockBanner state={lock.state} actions={actions} />} />;
    }
  }
}

/** The plain text of a document, one block per paragraph: what "Copy my changes" puts on the clipboard. */
function docText(node: DocNode): string {
  if (node.text) return node.text;
  const inner = (node.content ?? []).map(docText);
  return inner.join(node.content?.some((c) => c.text !== undefined) ? "" : "\n\n");
}

/**
 * After the page was taken from this editor: what it had not saved, which stays in the browser copy, and the two ways to keep it. The
 * page itself cannot take it any more (its draft moved on), so it goes to the clipboard or into a new page next to it.
 */
function useKeptWork(page: Page, taken: boolean, onOpen?: (id: string) => void) {
  const queryClient = useQueryClient();
  const [doc, setDoc] = useState<DocNode | null>(null);

  useEffect(() => {
    if (!taken) return;
    // The editor writes its last copy as it closes, just before this runs.
    const timer = window.setTimeout(async () => {
      const local = await readLocal(page.id);
      setDoc(local?.dirty ? local.doc : null);
    }, 100);
    return () => window.clearTimeout(timer);
  }, [taken, page.id]);

  const copy = async () => {
    if (doc) await navigator.clipboard.writeText(docText(doc));
  };

  const saveAsNew = async () => {
    if (!doc) return;
    const created = unwrap(
      await api.POST("/api/pages", {
        body: { parent_id: page.parent_id ?? null, title: `${page.title} (my changes)`, before: null },
      }),
    );
    const { token } = unwrap(await api.POST("/api/pages/{id}/lock", { params: { path: { id: created.id } } }));
    unwrap(
      await api.PUT("/api/pages/{id}/draft", {
        params: { path: { id: created.id } },
        body: { doc, base_revision_id: null, lock_token: token },
      }),
    );
    await api.DELETE("/api/pages/{id}/lock", { params: { path: { id: created.id }, query: { token } } });
    await discardLocal(page.id);
    setDoc(null);
    await queryClient.invalidateQueries({ queryKey: ["pages"] });
    markCreated(created.id);
    onOpen?.(created.id);
  };

  return { unsaved: doc !== null, copy, saveAsNew };
}

/** Loads what to edit: the server's draft, or the browser's copy when it holds typing the server has not seen. */
function EditablePage({ page, onStatus, panelHost, lock }: Props & { lock: DraftLock }) {
  const draft = useQuery({
    queryKey: ["draft", page.id],
    // Reloading while the user is typing would replace their text.
    staleTime: Number.POSITIVE_INFINITY,
    gcTime: 0,
    queryFn: async (): Promise<InitialDraft> => {
      // A save sent when this page was last left must land first, or its own revision would look like someone else's.
      await saveSettled(page.id);
      const server = unwrap(await api.GET("/api/pages/{id}/draft", { params: { path: { id: page.id } } }));
      const serverBase = server.base_revision_id ?? null;
      const local = await readLocal(page.id);

      if (local?.dirty) return { doc: local.doc, base: local.base, dirty: true, conflict: local.base !== serverBase };
      return { doc: server.doc as InitialDraft["doc"], base: serverBase, dirty: false };
    },
  });

  if (draft.isPending) return <p className="text-muted px-2.5">Loading…</p>;
  if (draft.isError)
    return (
      <p role="alert" className={ui.errorBox}>
        {draft.error.message}
      </p>
    );

  return <Editing page={page} initial={draft.data} onStatus={onStatus} panelHost={panelHost} lock={lock} />;
}

/** The page as last saved, refreshed while someone else edits it. */
function ReadOnlyPage({ page, banner }: { page: Page; banner: React.ReactNode }) {
  const saved = useQuery({
    queryKey: ["saved-draft", page.id],
    queryFn: async () => unwrap(await api.GET("/api/pages/{id}/draft", { params: { path: { id: page.id } } })),
    refetchInterval: 15_000,
  });
  const context = useCanvasContext(page);

  return (
    <>
      {banner}
      {saved.data && (
        <div className="opacity-92">
          <Canvas
            // A new save shows as a new canvas: the editor reads its document once.
            key={saved.data.base_revision_id ?? "empty"}
            doc={saved.data.doc as InitialDraft["doc"]}
            context={context}
            embedded
            editable={false}
          />
        </div>
      )}
    </>
  );
}

function useCanvasContext(page: Page) {
  const pages = usePages();

  return {
    page: { id: page.id, title: page.title },
    urls: pageUrls(pages.data ?? []),
    pages: Object.fromEntries(
      (pages.data ?? [])
        .filter((p) => p.kind !== "link")
        .map((p) => [p.id, { title: p.title, description: p.description }]),
    ),
  };
}

const conflictButton = `py-[3px] px-2.5 rounded-md ${ui.button}`;

function Editing({
  page,
  initial,
  onStatus,
  panelHost,
  lock,
}: {
  page: Page;
  initial: InitialDraft;
  onStatus: Props["onStatus"];
  panelHost: HTMLElement | null;
  lock: DraftLock;
}) {
  const { status, onChange, baseline, flush, overwrite, discard } = useDraftSync(page.id, initial, lock);
  const context = useCanvasContext(page);

  useEffect(() => onStatus(status), [status, onStatus]);

  return (
    <>
      {status === "conflict" && (
        <div
          role="alert"
          className="flex items-center gap-2.5 mt-3 mx-12 mb-0 py-2 px-3 border border-danger rounded-lg"
        >
          This page was saved from somewhere else since you opened it.
          <button type="button" className={conflictButton} onClick={() => void overwrite()}>
            Keep my version
          </button>
          <button type="button" className={conflictButton} onClick={() => void discard()}>
            Reload the saved version
          </button>
        </div>
      )}
      <Canvas
        doc={initial.doc}
        context={context}
        embedded
        title={<TitleField page={page} />}
        lead={<LeadField page={page} />}
        onChange={onChange}
        onReady={baseline}
        panelHost={panelHost}
        onBlur={() => void flush()}
      />
    </>
  );
}
