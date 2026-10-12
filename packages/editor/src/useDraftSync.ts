import { useQueryClient } from "@tanstack/react-query";
import type { DocNode } from "@yourdocs/renderer";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./api/client";
import { type LocalDraft, writeLocal } from "./localDrafts";
import { PENDING } from "./queries";
import { contentSignature } from "./signature";

export type SaveStatus = "saved" | "dirty" | "saving" | "conflict" | "error";

export interface InitialDraft {
  doc: DocNode;
  base: string | null;
  /** The document came from the browser copy and the server has not seen it. */
  dirty: boolean;
}

const LOCAL_DELAY = 300;
const SERVER_DELAY = 2000;
const RETRY_DELAY = 5000;

/**
 * Keeps one page's document safe: every change goes to IndexedDB almost at once, and to the server after a pause (or on blur).
 * Saves carry the revision they are based on; a refused save (409) stops the sync until the user chooses what to keep.
 */
export function useDraftSync(pageId: string, initial: InitialDraft) {
  const queryClient = useQueryClient();
  const [status, setStatus] = useState<SaveStatus>(initial.dirty ? "dirty" : "saved");
  const sync = useRef({
    base: initial.base,
    doc: initial.doc,
    savedSignature: initial.dirty ? "" : contentSignature(initial.doc),
    inFlight: false,
    conflict: false,
    localTimer: undefined as number | undefined,
    serverTimer: undefined as number | undefined,
  });

  const flush = useCallback(async () => {
    const s = sync.current;
    window.clearTimeout(s.serverTimer);
    if (s.inFlight || s.conflict || contentSignature(s.doc) === s.savedSignature) return;

    const sent = s.doc;
    s.inFlight = true;
    setStatus("saving");

    const { data, response } = await api
      .PUT("/api/pages/{id}/draft", { params: { path: { id: pageId } }, body: { doc: sent, base_revision_id: s.base } })
      .catch(() => ({ data: undefined, response: undefined }));
    s.inFlight = false;

    if (data) {
      s.base = data.base_revision_id ?? null;
      s.savedSignature = contentSignature(sent);
      const caughtUp = contentSignature(s.doc) === s.savedSignature;
      await writeLocal({ pageId, doc: s.doc, base: s.base, dirty: !caughtUp });

      void queryClient.invalidateQueries({ queryKey: PENDING });
      if (caughtUp) setStatus("saved");
      else {
        setStatus("dirty");
        s.serverTimer = window.setTimeout(() => void flush(), SERVER_DELAY);
      }
    } else if (response?.status === 409) {
      s.conflict = true;
      setStatus("conflict");
    } else {
      setStatus("error");
      s.serverTimer = window.setTimeout(() => void flush(), RETRY_DELAY);
    }
  }, [pageId, queryClient]);

  const onChange = useCallback(
    (doc: DocNode) => {
      const s = sync.current;
      s.doc = doc;
      if (contentSignature(doc) === s.savedSignature) return;

      if (!s.conflict) setStatus("dirty");
      window.clearTimeout(s.localTimer);
      s.localTimer = window.setTimeout(() => {
        const local: LocalDraft = { pageId, doc: s.doc, base: s.base, dirty: true };
        void writeLocal(local);
      }, LOCAL_DELAY);

      window.clearTimeout(s.serverTimer);
      s.serverTimer = window.setTimeout(() => void flush(), SERVER_DELAY);
    },
    [pageId, flush],
  );

  /** After a conflict: keep this version, saving it over the one on the server. */
  const overwrite = useCallback(async () => {
    const { data } = await api.GET("/api/pages/{id}/draft", { params: { path: { id: pageId } } });
    if (!data) return;

    sync.current.base = data.base_revision_id ?? null;
    sync.current.conflict = false;
    await flush();
  }, [pageId, flush]);

  // A restored browser copy still has to reach the server; leaving the page must not lose a pending save.
  useEffect(() => {
    const s = sync.current;
    if (initial.dirty) s.serverTimer = window.setTimeout(() => void flush(), SERVER_DELAY);

    return () => {
      window.clearTimeout(s.localTimer);
      void writeLocal({ pageId, doc: s.doc, base: s.base, dirty: contentSignature(s.doc) !== s.savedSignature });
      void flush();
    };
  }, [pageId, initial.dirty, flush]);

  return { status, onChange, flush, overwrite };
}
