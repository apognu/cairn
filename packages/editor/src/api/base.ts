/**
 * Where the editor is mounted: `/_editor` when the server shares its address with the public site (`yourdocs serve`), nothing in
 * `yourdocs dev`. Every request to the server goes through it.
 */
export const BASE: string = /^\/_editor(\/|$)/.test(window.location.pathname) ? "/_editor" : "";

/** The address of a server path (`/api/…`, `/auth/…`) for this mount point. */
export const serverUrl = (path: string): string => BASE + path;
