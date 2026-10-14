import type { DocNode } from "@cairn/renderer";
import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./api/client";
import { discardLocal, type LocalDraft, writeLocal } from "./localDrafts";
import { PENDING } from "./queries";
import { contentSignature } from "./signature";

export type SaveStatus = "saved" | "dirty" | "saving" | "conflict" | "error";

export interface InitialDraft {
  doc: DocNode;
  base: string | null;
  /** The document came from the browser copy and the server has not seen it. */
  dirty: boolean;
  /** That browser copy was edited from an older revision than the server's: someone saved in between, and the user must choose. */
  conflict?: boolean;
}

const LOCAL_DELAY = 300;
const SERVER_DELAY = 2000;
const RETRY_DELAY = 5000;

/**
 * Saves still running for a page, kept across mounts: leaving a page sends its last changes, and coming straight back must wait for
 * that save, or the new editor would read the browser copy with the old base and conflict with its own save.
 */
const running = new Map<string, Promise<void>>();

export function saveSettled(pageId: string): Promise<void> {
  return running.get(pageId) ?? Promise.resolve();
}

/** A refused save that may work later: no answer at all, a server error, or too many requests. A 4xx will not change by itself. */
function worthRetrying(response: Response | undefined): boolean {
  return response === undefined || response.status >= 500 || response.status === 429;
}

/** The page's lock, which every save must name. Must keep its identity across renders: the sync is rebuilt when it changes. */
export interface DraftLock {
  token: () => string | null;
  /** A save was refused for the lock (410 or 423). */
  refused: (status: number) => void;
}

/**
 * Keeps one page's document safe: every change goes to IndexedDB almost at once, and to the server after a pause (or on blur).
 * Saves carry the revision they are based on; a refused save (409) stops the sync until the user chooses what to keep.
 */
export function useDraftSync(pageId: string, initial: InitialDraft, lock: DraftLock) {
  const queryClient = useQueryClient();
  const [status, setStatus] = useState<SaveStatus>(initial.conflict ? "conflict" : initial.dirty ? "dirty" : "saved");
  const sync = useRef({
    base: initial.base,
    doc: initial.doc,
    savedSignature: initial.dirty ? "" : contentSignature(initial.doc),
    inFlight: false,
    conflict: initial.conflict === true,
    /** The page was left: only the last save is finished, nothing is scheduled any more. */
    disposed: false,
    localTimer: undefined as number | undefined,
    serverTimer: undefined as number | undefined,
  });

  const flush = useCallback((): Promise<void> => {
    const s = sync.current;
    window.clearTimeout(s.serverTimer);
    if (s.inFlight || s.conflict) return saveSettled(pageId);
    // Typing undone, or nothing to send after all.
    if (contentSignature(s.doc) === s.savedSignature) {
      if (!s.disposed) setStatus("saved");
      return saveSettled(pageId);
    }

    const sent = s.doc;
    const token = lock.token();
    if (!token) return saveSettled(pageId);
    s.inFlight = true;
    if (!s.disposed) setStatus("saving");

    const save = (async () => {
      const { data, response } = await api
        .PUT("/api/pages/{id}/draft", {
          params: { path: { id: pageId } },
          body: { doc: sent, base_revision_id: s.base, lock_token: token },
        })
        .catch(() => ({ data: undefined, response: undefined }));
      s.inFlight = false;

      if (data) {
        s.base = data.base_revision_id ?? null;
        s.savedSignature = contentSignature(sent);
        const caughtUp = contentSignature(s.doc) === s.savedSignature;
        await writeLocal({ pageId, doc: s.doc, base: s.base, dirty: !caughtUp });
        void queryClient.invalidateQueries({ queryKey: PENDING });

        if (caughtUp) {
          if (!s.disposed) setStatus("saved");
        } else if (s.disposed) {
          // Typed while this save was on its way, then left: send the rest now.
          await flush();
        } else {
          setStatus("dirty");
          s.serverTimer = window.setTimeout(() => void flush(), SERVER_DELAY);
        }
      } else if (response?.status === 409) {
        s.conflict = true;
        if (!s.disposed) setStatus("conflict");
      } else if (response?.status === 410 || response?.status === 423) {
        // Someone took the page or the lock lapsed: the browser copy keeps the changes, and the lock decides what comes next.
        lock.refused(response.status);
        if (!s.disposed) setStatus("error");
        if (response.status === 410 && !s.disposed) s.serverTimer = window.setTimeout(() => void flush(), RETRY_DELAY);
      } else if (!s.disposed) {
        // The browser copy keeps the changes either way; a page left with a failed save is sent again when it is next opened.
        setStatus("error");
        if (worthRetrying(response)) s.serverTimer = window.setTimeout(() => void flush(), RETRY_DELAY);
      }
    })();

    const tracked = save.finally(() => {
      if (running.get(pageId) === tracked) running.delete(pageId);
    });
    running.set(pageId, tracked);
    return tracked;
  }, [pageId, queryClient, lock]);

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

  /**
   * The editor fills in what the stored document leaves implicit (an empty page gets a paragraph, attributes get their defaults):
   * that form is the saved one, so opening a page does not save it and list it as edited.
   */
  const baseline = useCallback(
    (doc: DocNode) => {
      const s = sync.current;
      if (initial.dirty) return;

      if (s.doc === initial.doc) s.doc = doc;
      s.savedSignature = contentSignature(doc);
      // The editor's own first update (block IDs) can come before this and look like an edit: it was not one.
      if (contentSignature(s.doc) === s.savedSignature) {
        window.clearTimeout(s.serverTimer);
        setStatus("saved");
      }
    },
    [initial.dirty, initial.doc],
  );

  /** After a conflict: keep this version, saving it over the one on the server. */
  const overwrite = useCallback(async () => {
    const { data } = await api.GET("/api/pages/{id}/draft", { params: { path: { id: pageId } } });
    if (!data) return;

    sync.current.base = data.base_revision_id ?? null;
    sync.current.conflict = false;
    // The browser copy is now ahead of the server's version, not behind it.
    sync.current.savedSignature = "";
    await flush();
  }, [pageId, flush]);

  /** After a conflict: drop this version and load the one on the server. The browser copy goes too, or it would win again. */
  const discard = useCallback(async () => {
    sync.current.disposed = true;
    window.clearTimeout(sync.current.localTimer);
    window.clearTimeout(sync.current.serverTimer);
    await discardLocal(pageId);
    window.location.reload();
  }, [pageId]);

  // A restored browser copy still has to reach the server; leaving the page must not lose a pending save.
  useEffect(() => {
    const s = sync.current;
    s.disposed = false;
    if (initial.dirty && !initial.conflict) s.serverTimer = window.setTimeout(() => void flush(), SERVER_DELAY);

    return () => {
      if (s.disposed) return;
      s.disposed = true;
      window.clearTimeout(s.localTimer);
      void writeLocal({ pageId, doc: s.doc, base: s.base, dirty: contentSignature(s.doc) !== s.savedSignature });
      void flush();
    };
  }, [pageId, initial.dirty, initial.conflict, flush]);

  return { status, onChange, baseline, flush, overwrite, discard };
}
