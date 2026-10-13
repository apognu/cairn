import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import { api } from "./api/client";
import { unwrap } from "./api/unwrap";
import { Canvas } from "./Canvas";
import { LeadField } from "./LeadField";
import { readLocal } from "./localDrafts";
import { usePages } from "./queries";
import { type Page, pageUrls } from "./tree";
import { type InitialDraft, type SaveStatus, saveSettled, useDraftSync } from "./useDraftSync";

interface Props {
  page: Page;
  onStatus: (status: SaveStatus | null) => void;
  panelHost: HTMLElement | null;
}

/** Loads what to edit: the server's draft, or the browser's copy when it holds typing the server has not seen. */
export function PageEditor({ page, onStatus, panelHost }: Props) {
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

  useEffect(() => () => onStatus(null), [onStatus]);

  if (draft.isPending) return <p className="muted pad">Loading…</p>;
  if (draft.isError)
    return (
      <p role="alert" className="error">
        {draft.error.message}
      </p>
    );

  return <Editing page={page} initial={draft.data} onStatus={onStatus} panelHost={panelHost} />;
}

function Editing({
  page,
  initial,
  onStatus,
  panelHost,
}: {
  page: Page;
  initial: InitialDraft;
  onStatus: Props["onStatus"];
  panelHost: HTMLElement | null;
}) {
  const { status, onChange, baseline, flush, overwrite, discard } = useDraftSync(page.id, initial);
  const pages = usePages();

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
        context={{
          page: { id: page.id, title: page.title },
          urls: pageUrls(pages.data ?? []),
          pages: Object.fromEntries(
            (pages.data ?? [])
              .filter((p) => p.kind !== "link")
              .map((p) => [p.id, { title: p.title, description: p.description }]),
          ),
        }}
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
