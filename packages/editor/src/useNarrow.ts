import { useEffect, useState } from "react";

// Keep in step with the 1100px breakpoint in styles.css, where the side columns give way to drawers and menus.
const NARROW = "(max-width: 1100px)";

/** Whether the window is too narrow for the editor's side columns. */
export function useNarrow(): boolean {
  const [narrow, setNarrow] = useState(() => window.matchMedia(NARROW).matches);

  useEffect(() => {
    const query = window.matchMedia(NARROW);
    const update = () => setNarrow(query.matches);
    query.addEventListener("change", update);

    return () => query.removeEventListener("change", update);
  }, []);

  return narrow;
}
