import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import { api } from "./api/client";
import { unwrap } from "./api/unwrap";
import { Canvas } from "./Canvas";
import { readLocal } from "./localDrafts";
import { usePages } from "./queries";
import { type Page, pageUrls } from "./tree";
import { type InitialDraft, type SaveStatus, useDraftSync } from "./useDraftSync";

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
      const server = unwrap(await api.GET("/api/pages/{id}/draft", { params: { path: { id: page.id } } }));
      const local = await readLocal(page.id);

      if (local?.dirty) return { doc: local.doc, base: local.base, dirty: true };
      return { doc: server.doc as InitialDraft["doc"], base: server.base_revision_id ?? null, dirty: false };
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
  const { status, onChange, flush, overwrite } = useDraftSync(page.id, initial);
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
          <button type="button" onClick={() => window.location.reload()}>
            Reload the saved version
          </button>
        </div>
      )}
      <Canvas
        doc={initial.doc}
        context={{ page: { id: page.id, title: page.title }, urls: pageUrls(pages.data ?? []) }}
        embedded
        onChange={onChange}
        panelHost={panelHost}
        onBlur={() => void flush()}
      />
    </>
  );
}
