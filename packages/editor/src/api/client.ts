import createClient from "openapi-fetch";
import { BASE, serverUrl } from "./base";
import type { paths } from "./schema";

/** Sends the browser through the sign-in and back to where it was. Nothing runs after it: the page is replaced. */
export function signIn(): void {
  const here = window.location.pathname + window.location.search + window.location.hash;
  window.location.assign(serverUrl(`/auth/login?next=${encodeURIComponent(here)}`));
}

export const api = createClient<paths>({ baseUrl: BASE });

let signingOut = false;

/** The user is signing out: requests still on their way will be refused, and must not send the browser to sign in again. */
export function signingOutNow(): void {
  signingOut = true;
}

// A session that ended (idle, or past its lifetime) shows up as a 401 on whatever request comes next.
api.use({
  onResponse({ response }) {
    if (response.status === 401 && !signingOut) signIn();
    return undefined;
  },
});
