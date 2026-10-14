import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { serverUrl } from "./api/base";
import { api } from "./api/client";
import type { components } from "./api/schema";
import { unwrap } from "./api/unwrap";
import { onEditorEvent } from "./events";
import { saveSettled } from "./useDraftSync";

export type LockView = components["schemas"]["LockView"];

export type LockState =
  | { kind: "loading" }
  | { kind: "mine" }
  /** Someone else edits the page (or the user does, in another tab). */
  | { kind: "held"; lock: LockView }
  /** Someone took the page while this editor had it; what it had not saved is in the browser copy. */
  | { kind: "taken"; lock: LockView | null }
  /** The holder left: the page can be edited again. */
  | { kind: "free" }
  /** The lock was given back after a long time without activity. */
  | { kind: "idle" }
  | { kind: "error"; message: string };

/** The server's lease is 2 minutes; renewing well within it survives a slow request or two. */
const RENEW_EVERY = 30_000;
const IDLE_AFTER = 10 * 60_000;
const REPORT_EVERY = 20_000;
/** Lock events arrive at once; this only covers a stream that dropped. */
const CHECK_HELD_EVERY = 30_000;

export const LOCKS = ["locks"];

/** Every page someone is editing, for the tree. Lock events refresh it; the interval covers a dropped stream. */
export function useLocks() {
  return useQuery({
    queryKey: LOCKS,
    queryFn: async () => unwrap(await api.GET("/api/locks")),
    refetchInterval: 60_000,
  });
}

/** Gives the lock back even while the tab is closing, where a normal request would be cancelled. */
function releaseNow(pageId: string, token: string) {
  const url = serverUrl(`/api/pages/${encodeURIComponent(pageId)}/lock?token=${encodeURIComponent(token)}`);
  void fetch(url, { method: "DELETE", keepalive: true, credentials: "same-origin" }).catch(() => undefined);
}

const message = (error: unknown, fallback: string) => (error as { message?: string } | undefined)?.message ?? fallback;

/**
 * Holds the page's lock while it is open: taken on opening, renewed every 30 seconds while the tab is visible and its author active,
 * given back after 10 minutes without activity and when the page is left. Saves send `token()`.
 */
export function usePageLock(pageId: string, title: string) {
  const queryClient = useQueryClient();
  const [state, setState] = useState<LockState>({ kind: "loading" });
  const [watching, setWatching] = useState(false);
  const token = useRef<string | null>(null);
  /** The last keystroke or click, 0 before the first: a page only opened is not being worked on. */
  const lastActive = useRef(0);
  const openedAt = useRef(Date.now());
  const kind = useRef(state.kind);
  kind.current = state.kind;

  const lookUp = useCallback(async (): Promise<LockView | null> => {
    const { data } = await api.GET("/api/pages/{id}/lock", { params: { path: { id: pageId } } });
    return data ?? null;
  }, [pageId]);

  const holding = useCallback(
    (held: string) => {
      token.current = held;
      openedAt.current = Date.now();
      // Before the render: the event this acquire sends can arrive first, and must not read as someone else's lock.
      kind.current = "mine";
      setState({ kind: "mine" });
      void queryClient.invalidateQueries({ queryKey: LOCKS });
    },
    [queryClient],
  );

  const acquire = useCallback(async () => {
    // The lock may lapse between the refusal and the look-up; one more try settles it.
    for (let attempt = 0; attempt < 2; attempt++) {
      const { data, error, response } = await api.POST("/api/pages/{id}/lock", { params: { path: { id: pageId } } });
      if (data) return holding(data.token);

      token.current = null;
      if (response.status !== 423) {
        setState({ kind: "error", message: message(error, "Could not open the page") });
        return;
      }
      const lock = await lookUp();
      if (lock) {
        setState({ kind: "held", lock });
        return;
      }
    }
    setState({ kind: "free" });
  }, [pageId, lookUp, holding]);

  /** Takes the page from its holder. Without `force` it fails (with the server's reason) while they are active. */
  const takeOver = useCallback(
    async (force: boolean): Promise<string | null> => {
      const { data, error } = await api.POST("/api/pages/{id}/lock/take-over", {
        params: { path: { id: pageId } },
        body: { force },
      });
      if (!data) return message(error, "Could not take over the page");

      holding(data.token);
      return null;
    },
    [pageId, holding],
  );

  /** The server refused a save or a renewal, or said the page changed hands: find out who holds it now. */
  const recheck = useCallback(
    async (status: number) => {
      if (status === 410) return acquire();
      if (status !== 423) return;

      const wasMine = kind.current === "mine";
      token.current = null;
      const lock = await lookUp();
      if (wasMine) setState({ kind: "taken", lock });
      else setState(lock ? { kind: "held", lock } : { kind: "free" });
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

  const renew = useCallback(async () => {
    const held = token.current;
    if (!held) return;

    const { response } = await api.PUT("/api/pages/{id}/lock", {
      params: { path: { id: pageId } },
      // Before any activity this is the whole epoch, which the server reads as idle.
      body: { token: held, idle_ms: Math.max(0, Date.now() - lastActive.current) },
    });
    if (!response.ok) await recheck(response.status);
  }, [pageId, recheck]);

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

  // Work starting is told at once, not at the next renewal: until then the lock looks idle, and the user's other tab (or anyone taking
  // over) could have it.
  useEffect(() => {
    let reported = 0;
    const active = () => {
      lastActive.current = Date.now();
      if (token.current && lastActive.current - reported > REPORT_EVERY) {
        reported = lastActive.current;
        void renew();
      }
    };
    const events = ["keydown", "pointerdown", "input"] as const;
    for (const event of events) window.addEventListener(event, active, { capture: true });

    return () => {
      for (const event of events) window.removeEventListener(event, active, { capture: true });
    };
  }, [renew]);

  // Changes of hands are told at once: the editor that lost the page stops before it tries to save again.
  useEffect(
    () =>
      onEditorEvent(async (event) => {
        if (event.type === "published" || event.page_id !== pageId) return;
        if (kind.current === "mine" || token.current) {
          // The user's own other tab taking the page is no take-over for the server, only a change: renewing finds out at once.
          await renew();
          return;
        }
        if (kind.current !== "held" && kind.current !== "free") return;

        const lock = await lookUp();
        // The user may have taken the page while the look-up was on its way; their own lock is not someone else's.
        if (!token.current) setState(lock ? { kind: "held", lock } : { kind: "free" });
      }),
    [pageId, renew, lookUp],
  );

  useEffect(() => {
    if (state.kind !== "mine") return;

    const timer = window.setInterval(() => {
      if (Date.now() - Math.max(lastActive.current, openedAt.current) > IDLE_AFTER) {
        release();
        setState({ kind: "idle" });
        return;
      }
      // A hidden tab lets the lease run out, so a forgotten one frees the page within two minutes.
      if (document.visibilityState === "visible") void renew();
    }, RENEW_EVERY);

    return () => window.clearInterval(timer);
  }, [state.kind, release, renew]);

  useEffect(() => {
    if (state.kind !== "held") return;

    const timer = window.setInterval(async () => {
      const lock = await lookUp();
      if (!token.current) setState(lock ? { kind: "held", lock } : { kind: "free" });
    }, CHECK_HELD_EVERY);

    return () => window.clearInterval(timer);
  }, [state.kind, lookUp]);

  // "Notify me when free": a system notification when the page frees up, for a tab the user is not looking at.
  useEffect(() => {
    if (!watching || state.kind !== "free") return;
    setWatching(false);
    if ("Notification" in window && Notification.permission === "granted")
      new Notification(`${title} is free to edit`, { body: "Nobody is editing it any more." });
  }, [watching, state.kind, title]);

  const watch = useCallback(async () => {
    if ("Notification" in window && Notification.permission === "default") await Notification.requestPermission();
    setWatching(true);
  }, []);

  return { state, token: () => token.current, acquire, takeOver, recheck, watching, watch };
}
