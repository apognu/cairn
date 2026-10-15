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

/** Pages this tab just created: opening one is working on it, so its lock does not start out idle. */
const created = new Set<string>();

/** Marks a page the user just made in this tab. */
export function markCreated(pageId: string) {
  created.add(pageId);
}

/** The locks this tab holds, page to token, so signing out can give them back while the session still works. */
const heldLocks = new Map<string, string>();

/** Gives back every lock this tab holds; others can edit those pages at once instead of waiting for the leases to run out. */
export async function releaseAllLocks(): Promise<void> {
  await Promise.all(
    [...heldLocks].map(([pageId, token]) =>
      api.DELETE("/api/pages/{id}/lock", { params: { path: { id: pageId }, query: { token } } }).catch(() => undefined),
    ),
  );
  heldLocks.clear();
}

/** Locks this tab is giving back. Coming straight back to a page must wait for its release, or the tab would find its own lock held. */
const releasing = new Map<string, Promise<void>>();

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
  const openedAt = useRef(Date.now());
  /**
   * The last activity on this page: in the canvas or its side panel, or (taken with the lock) just before it was opened. Work in the
   * tree or the top bar is not work on the page the tab happens to show.
   */
  const pageActive = useRef(0);
  const kind = useRef(state.kind);
  kind.current = state.kind;

  const lookUp = useCallback(async (): Promise<LockView | null> => {
    const { data } = await api.GET("/api/pages/{id}/lock", { params: { path: { id: pageId } } });
    return data ?? null;
  }, [pageId]);

  const holding = useCallback(
    (held: string) => {
      token.current = held;
      heldLocks.set(pageId, held);
      openedAt.current = Date.now();
      // Before the render: the event this acquire sends can arrive first, and must not read as someone else's lock.
      kind.current = "mine";
      setState({ kind: "mine" });
      void queryClient.invalidateQueries({ queryKey: LOCKS });
    },
    [pageId, queryClient],
  );

  const acquire = useCallback(async () => {
    await releasing.get(pageId);
    // The lock may lapse between the refusal and the look-up; one more try settles it.
    for (let attempt = 0; attempt < 2; attempt++) {
      const { data, error, response } = await api.POST("/api/pages/{id}/lock", {
        params: { path: { id: pageId } },
        body: { idle_ms: created.has(pageId) ? 0 : null },
      });
      if (data) {
        if (created.delete(pageId)) pageActive.current = Date.now();
        return holding(data.token);
      }

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

      // As on the server: taking over is work on the page.
      pageActive.current = Date.now();
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
    if (!held) return;

    // Registered at once, so coming straight back waits for it. The editor's own cleanup runs in the same tick and starts its last save
    // with the token, so the token is only dropped on the next one, and the lock only after that save.
    const done = new Promise((resolve) => window.setTimeout(resolve, 0))
      .then(() => {
        if (token.current === held) token.current = null;
        if (heldLocks.get(pageId) === held) heldLocks.delete(pageId);
        return saveSettled(pageId);
      })
      .then(async () => {
        await api.DELETE("/api/pages/{id}/lock", { params: { path: { id: pageId }, query: { token: held } } });
        void queryClient.invalidateQueries({ queryKey: LOCKS });
      })
      .catch(() => undefined)
      .finally(() => {
        if (releasing.get(pageId) === done) releasing.delete(pageId);
      });
    releasing.set(pageId, done);
  }, [pageId, queryClient]);

  const renew = useCallback(async () => {
    const held = token.current;
    if (!held) return;

    const { response } = await api.PUT("/api/pages/{id}/lock", {
      params: { path: { id: pageId } },
      // Before any activity this is the whole epoch, which the server reads as idle.
      body: { token: held, idle_ms: Math.max(0, Date.now() - pageActive.current) },
    });
    if (!response.ok) await recheck(response.status);
  }, [pageId, recheck]);

  useEffect(() => {
    // Leaving the page before the lock arrived: the cleanup below had no token to give back, so the lock is given back on arrival, or it
    // would stay held by an editor that is gone.
    let gone = false;
    void acquire().then(() => {
      if (gone) release();
    });

    // A closed tab runs no cleanup of React's; this is the last chance to free the page for others.
    const onHide = () => {
      if (token.current) releaseNow(pageId, token.current);
    };
    window.addEventListener("pagehide", onHide);

    return () => {
      gone = true;
      window.removeEventListener("pagehide", onHide);
      release();
    };
  }, [pageId, acquire, release]);

  // Work starting is told at once, not at the next renewal: until then the lock looks idle, and the user's other tab (or anyone taking
  // over) could have it.
  useEffect(() => {
    let reported = 0;
    const active = (event: Event) => {
      if (!(event.target instanceof Element && event.target.closest("main.canvas, .side-column"))) return;
      const now = Date.now();
      pageActive.current = now;
      if (token.current && now - reported > REPORT_EVERY) {
        reported = now;
        void renew();
      }
    };
    // Typing is working on the page; a click is often only looking at it.
    const events = ["keydown", "input"] as const;
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
      if (Date.now() - Math.max(pageActive.current, openedAt.current) > IDLE_AFTER) {
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
