import { type RefObject, useEffect } from "react";

/**
 * Opens a `<dialog>` as a side panel beside the canvas: not modal, so the page stays readable and the top bar usable. A modal dialog
 * closes on Escape by itself; this one does when the focus is inside it, which leaves Escape to the canvas otherwise.
 */
export function useSidePanel(dialog: RefObject<HTMLDialogElement | null>) {
  useEffect(() => {
    const element = dialog.current;
    if (!element) return;

    element.show();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) element.close();
    };
    element.addEventListener("keydown", onKey);

    return () => element.removeEventListener("keydown", onKey);
  }, [dialog]);
}
