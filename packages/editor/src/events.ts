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

/**
 * Opens the one event stream of the editor and keeps the shared queries fresh with it. Events are hints: after the stream drops and
 * comes back, what it may have missed is read again.
 */
export function useEditorEvents() {
  const queryClient = useQueryClient();

  useEffect(() => {
    const source = new EventSource(serverUrl("/api/events"));
    let dropped = false;

    source.onmessage = (message) => {
      const event = JSON.parse(message.data) as EditorEvent;
      if (event.type === "published") void queryClient.invalidateQueries({ queryKey: PENDING });
      else void queryClient.invalidateQueries({ queryKey: ["locks"] });
      for (const listener of listeners) listener(event);
    };
    source.onerror = () => {
      dropped = true;
    };
    source.onopen = () => {
      if (!dropped) return;
      dropped = false;
      void queryClient.invalidateQueries({ queryKey: ["locks"] });
      void queryClient.invalidateQueries({ queryKey: PENDING });
    };

    return () => source.close();
  }, [queryClient]);
}
