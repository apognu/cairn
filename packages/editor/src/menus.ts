import { useEffect } from "react";

/** An open menu (`details.menu`) closes on a click anywhere else and on Escape. Mounted once, near the root. */
export function useCloseMenus(): void {
  useEffect(() => {
    const close = (except?: Element | null) => {
      for (const menu of document.querySelectorAll<HTMLDetailsElement>("details.menu[open]")) {
        if (menu !== except) menu.removeAttribute("open");
      }
    };
    const onPointer = (e: PointerEvent) => close((e.target as Element | null)?.closest("details.menu"));
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();

    document.addEventListener("pointerdown", onPointer);
    document.addEventListener("keydown", onKey);

    return () => {
      document.removeEventListener("pointerdown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, []);
}
