import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { serverUrl } from "./api/base";
import type { components } from "./api/schema";
import { PENDING } from "./queries";

export type EditorEvent = components["schemas"]["EditorEvent"];
type Listener = (event: EditorEvent) => void;

const listeners = new Set<Listener>();

/** Calls `listener` for every event the server sends while the editor is open. Returns the function that stops it. */
export function onEditorEvent(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** A tab nobody uses lets its stream go, so a forgotten editor does not keep a server that scales to zero awake. */
const IDLE_AFTER = 10 * 60_000;
const HIDDEN_AFTER = 60_000;

/**
 * Opens the one event stream of the editor and keeps the shared queries fresh with it. Events are hints: after the stream drops and
 * comes back, what it may have missed is read again. The stream closes when the tab is hidden or unused for a while, and opens again
 * on the next visit or keystroke.
 */
export function useEditorEvents() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let source: EventSource | null = null;
    let dropped = false;
    let lastActive = Date.now();
    let hiddenSince: number | null = null;

    const open = () => {
      if (source) return;
      const next = new EventSource(serverUrl("/api/events"));
      next.onmessage = (message) => {
        const event = JSON.parse(message.data) as EditorEvent;
        if (event.type === "published") {
          void queryClient.invalidateQueries({ queryKey: PENDING });
          void queryClient.invalidateQueries({ queryKey: ["reader"] });
        } else void queryClient.invalidateQueries({ queryKey: ["locks"] });
        for (const listener of listeners) listener(event);
      };
      next.onerror = () => {
        dropped = true;
      };
      next.onopen = () => {
        if (!dropped) return;
        dropped = false;
        void queryClient.invalidateQueries({ queryKey: ["locks"] });
        void queryClient.invalidateQueries({ queryKey: PENDING });
      };
      source = next;
    };

    const close = () => {
      source?.close();
      source = null;
      // Whatever happens while closed is read again when the stream opens.
      dropped = true;
    };

    const active = () => {
      lastActive = Date.now();
      if (!source && document.visibilityState === "visible") open();
    };
    const visibility = () => {
      hiddenSince = document.visibilityState === "hidden" ? Date.now() : null;
      if (hiddenSince === null) active();
    };
    const check = window.setInterval(() => {
      const now = Date.now();
      if (source && (now - lastActive > IDLE_AFTER || (hiddenSince !== null && now - hiddenSince > HIDDEN_AFTER)))
        close();
    }, 15_000);

    const activity = ["pointerdown", "keydown", "wheel"] as const;
    for (const name of activity) window.addEventListener(name, active, { passive: true });
    document.addEventListener("visibilitychange", visibility);
    if (document.visibilityState === "hidden") hiddenSince = Date.now();
    open();

    return () => {
      window.clearInterval(check);
      for (const name of activity) window.removeEventListener(name, active);
      document.removeEventListener("visibilitychange", visibility);
      source?.close();
    };
  }, [queryClient]);
}
