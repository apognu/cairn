import { type RefObject, useEffect } from "react";

/**
 * Opens a `<dialog>` as a side panel beside the canvas: not modal, so the page stays readable and the top bar usable. A modal dialog
 * closes on Escape by itself; this one does when the focus is inside it, which leaves Escape to the canvas otherwise. A click elsewhere
 * closes it too, except in the top bar and the page actions, whose buttons open and close the panels themselves, and in another dialog
 * opened over it.
 */
export function useSidePanel(dialog: RefObject<HTMLDialogElement | null>) {
  useEffect(() => {
    const element = dialog.current;
    if (!element) return;

    element.show();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) element.close();
    };
    const onPointer = (event: PointerEvent) => {
      const target = event.target as Element | null;
      if (target && !element.contains(target) && !target.closest(".topbar, .page-actions, dialog[open]"))
        element.close();
    };
    element.addEventListener("keydown", onKey);
    document.addEventListener("pointerdown", onPointer);

    return () => {
      element.removeEventListener("keydown", onKey);
      document.removeEventListener("pointerdown", onPointer);
    };
  }, [dialog]);
}

/** Opens a modal `<dialog>` that also closes on a click on its backdrop. */
export function useModal(dialog: RefObject<HTMLDialogElement | null>, open = true) {
  useEffect(() => {
    const element = dialog.current;
    if (!element || !open) return;

    element.showModal();

    // A click on the backdrop targets the dialog itself, as does one on its padding: only a point outside its box is the backdrop.
    // The press must start there too, so selecting text inside and releasing outside keeps the dialog open.
    const outside = (event: MouseEvent) => {
      const box = element.getBoundingClientRect();
      return (
        event.target === element &&
        (event.clientX < box.left || event.clientX > box.right || event.clientY < box.top || event.clientY > box.bottom)
      );
    };
    let pressed = false;
    const onPointer = (event: PointerEvent) => {
      pressed = outside(event);
    };
    const onClick = (event: MouseEvent) => {
      if (pressed && outside(event)) element.close();
      pressed = false;
    };
    element.addEventListener("pointerdown", onPointer);
    element.addEventListener("click", onClick);

    return () => {
      element.removeEventListener("pointerdown", onPointer);
      element.removeEventListener("click", onClick);
    };
  }, [dialog, open]);
}
