import { useEffect, useState } from "react";

// Keep in step with the `narrow` and `mobile` variants in styles.css.
const NARROW = "(max-width: 1380px)";
const MOBILE = "(max-width: 760px)";

function useMedia(media: string): boolean {
  const [matches, setMatches] = useState(() => window.matchMedia(media).matches);

  useEffect(() => {
    const query = window.matchMedia(media);
    const update = () => setMatches(query.matches);
    query.addEventListener("change", update);

    return () => query.removeEventListener("change", update);
  }, [media]);

  return matches;
}

/** Whether the window is too narrow for the editor's side columns: they give way to drawers and menus. */
export function useNarrow(): boolean {
  return useMedia(NARROW);
}

/** Whether the window is too narrow even for the page tree: it opens over the page from a button in the top bar. */
export function useMobile(): boolean {
  return useMedia(MOBILE);
}
