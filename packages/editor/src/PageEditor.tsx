import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useRef } from "react";
import { api } from "./api/client";
import { unwrap } from "./api/unwrap";
import { Canvas } from "./Canvas";
import { LeadField } from "./LeadField";
import { LockBanner } from "./LockBanner";
import { readLocal } from "./localDrafts";
import { usePages } from "./queries";
import { type Page, pageUrls } from "./tree";
import { type DraftLock, type InitialDraft, type SaveStatus, saveSettled, useDraftSync } from "./useDraftSync";
import { usePageLock } from "./useLock";

/** What the top bar says of the open page: how its saving goes, or that it can only be read. */
export type PageStatus = SaveStatus | "read-only";

interface Props {
  page: Page;
  onStatus: (status: PageStatus | null) => void;
  panelHost: HTMLElement | null;
}

/** Opens the page for editing when its lock can be had, and read-only while someone else holds it. */
export function PageEditor({ page, onStatus, panelHost }: Props) {
  const lock = usePageLock(page.id);
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
      return <p className="muted pad">Loading…</p>;
    case "mine":
      return <EditablePage page={page} onStatus={onStatus} panelHost={panelHost} lock={draftLock} />;
    default:
      return <ReadOnlyPage page={page} banner={<LockBanner state={lock.state} onEdit={() => void lock.acquire()} />} />;
  }
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

  if (draft.isPending) return <p className="muted pad">Loading…</p>;
  if (draft.isError)
    return (
      <p role="alert" className="error">
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
        <div className="read-only">
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
        <div role="alert" className="conflict">
          This page was saved from somewhere else since you opened it.
          <button type="button" onClick={() => void overwrite()}>
            Keep my version
          </button>
          <button type="button" onClick={() => void discard()}>
            Reload the saved version
          </button>
        </div>
      )}
      <Canvas
        doc={initial.doc}
        context={context}
        embedded
        lead={<LeadField page={page} />}
        onChange={onChange}
        onReady={baseline}
        panelHost={panelHost}
        onBlur={() => void flush()}
      />
    </>
  );
}
