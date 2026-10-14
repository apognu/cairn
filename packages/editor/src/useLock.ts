import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { serverUrl } from "./api/base";
import { api } from "./api/client";
import type { components } from "./api/schema";
import { unwrap } from "./api/unwrap";
import { saveSettled } from "./useDraftSync";

export type LockView = components["schemas"]["LockView"];

export type LockState =
  | { kind: "loading" }
  | { kind: "mine" }
  /** Someone else edits the page (or the user does, in another tab). */
  | { kind: "held"; lock: LockView }
  /** The holder left: the page can be edited again. */
  | { kind: "free" }
  /** The lock was given back after a long time without activity. */
  | { kind: "idle" }
  | { kind: "error"; message: string };

/** The server's lease is 2 minutes; renewing well within it survives a slow request or two. */
const RENEW_EVERY = 30_000;
const IDLE_AFTER = 10 * 60_000;
const CHECK_HELD_EVERY = 15_000;

export const LOCKS = ["locks"];

/** Every page someone is editing, for the tree. */
export function useLocks() {
  return useQuery({
    queryKey: LOCKS,
    queryFn: async () => unwrap(await api.GET("/api/locks")),
    refetchInterval: RENEW_EVERY,
  });
}

/** Gives the lock back even while the tab is closing, where a normal request would be cancelled. */
function releaseNow(pageId: string, token: string) {
  const url = serverUrl(`/api/pages/${encodeURIComponent(pageId)}/lock?token=${encodeURIComponent(token)}`);
  void fetch(url, { method: "DELETE", keepalive: true, credentials: "same-origin" }).catch(() => undefined);
}

/**
 * Holds the page's lock while it is open: taken on opening, renewed every 30 seconds while the tab is visible and its author active,
 * given back after 10 minutes without activity and when the page is left. Saves send `token()`.
 */
export function usePageLock(pageId: string) {
  const queryClient = useQueryClient();
  const [state, setState] = useState<LockState>({ kind: "loading" });
  const token = useRef<string | null>(null);
  const lastActive = useRef(Date.now());

  const lookUp = useCallback(async (): Promise<LockView | null> => {
    const { data } = await api.GET("/api/pages/{id}/lock", { params: { path: { id: pageId } } });
    return data ?? null;
  }, [pageId]);

  const acquire = useCallback(async () => {
    // The lock may lapse between the refusal and the look-up; one more try settles it.
    for (let attempt = 0; attempt < 2; attempt++) {
      const { data, error, response } = await api.POST("/api/pages/{id}/lock", { params: { path: { id: pageId } } });
      void queryClient.invalidateQueries({ queryKey: LOCKS });
      if (data) {
        token.current = data.token;
        lastActive.current = Date.now();
        setState({ kind: "mine" });
        return;
      }

      token.current = null;
      if (response.status !== 423) {
        setState({
          kind: "error",
          message: (error as { message?: string } | undefined)?.message ?? "Could not open the page",
        });
        return;
      }
      const lock = await lookUp();
      if (lock) {
        setState({ kind: "held", lock });
        return;
      }
    }
    setState({ kind: "free" });
  }, [pageId, lookUp, queryClient]);

  /** The server refused a save or a renewal: find out who holds the page now. */
  const recheck = useCallback(
    async (status: number) => {
      if (status === 410) return acquire();
      if (status !== 423) return;

      token.current = null;
      const lock = await lookUp();
      setState(lock ? { kind: "held", lock } : { kind: "free" });
    },
    [acquire, lookUp],
  );

  const release = useCallback(() => {
    const held = token.current;
    token.current = null;
    if (!held) return;

    // The last save goes first: it needs the lock.
    void saveSettled(pageId).then(async () => {
      await api.DELETE("/api/pages/{id}/lock", { params: { path: { id: pageId }, query: { token: held } } });
      void queryClient.invalidateQueries({ queryKey: LOCKS });
    });
  }, [pageId, queryClient]);

  useEffect(() => {
    void acquire();

    // A closed tab runs no cleanup of React's; this is the last chance to free the page for others.
    const onHide = () => {
      if (token.current) releaseNow(pageId, token.current);
    };
    window.addEventListener("pagehide", onHide);

    return () => {
      window.removeEventListener("pagehide", onHide);
      // After the editor's own cleanup has started its last save.
      window.setTimeout(release, 0);
    };
  }, [pageId, acquire, release]);

  useEffect(() => {
    const active = () => {
      lastActive.current = Date.now();
    };
    const events = ["keydown", "pointerdown", "input"] as const;
    for (const event of events) window.addEventListener(event, active, { capture: true });

    return () => {
      for (const event of events) window.removeEventListener(event, active, { capture: true });
    };
  }, []);

  useEffect(() => {
    if (state.kind !== "mine") return;

    const timer = window.setInterval(async () => {
      const held = token.current;
      if (!held) return;
      if (Date.now() - lastActive.current > IDLE_AFTER) {
        release();
        setState({ kind: "idle" });
        return;
      }
      // A hidden tab lets the lease run out, so a forgotten one frees the page within two minutes.
      if (document.visibilityState !== "visible") return;

      const { response } = await api.PUT("/api/pages/{id}/lock", {
        params: { path: { id: pageId } },
        body: { token: held },
      });
      if (!response.ok) await recheck(response.status);
    }, RENEW_EVERY);

    return () => window.clearInterval(timer);
  }, [state.kind, pageId, release, recheck]);

  useEffect(() => {
    if (state.kind !== "held") return;

    const timer = window.setInterval(async () => {
      const lock = await lookUp();
      setState(lock ? { kind: "held", lock } : { kind: "free" });
    }, CHECK_HELD_EVERY);

    return () => window.clearInterval(timer);
  }, [state.kind, lookUp]);

  return { state, token: () => token.current, acquire, recheck };
}
