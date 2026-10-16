import { useState } from "react";
import { serverUrl } from "./api/base";
import { signingOutNow } from "./api/client";
import { useMe, useSignedIn } from "./queries";
import * as ui from "./ui";
import { releaseAllLocks } from "./useLock";

const note = "m-0 py-1.5 px-2.5 text-help wrap-anywhere";

/** Who is signed in, and the way out. Absent in `cairn dev`, where nobody signs in. */
export function UserMenu() {
  const me = useMe();
  const signedIn = useSignedIn();
  const [error, setError] = useState<string | null>(null);

  if (!signedIn || !me.data) return null;

  const signOut = async () => {
    setError(null);
    await releaseAllLocks();
    // Only once the server ended the session: leaving for the signed-out page otherwise would say so while the session lives on.
    const response = await fetch(serverUrl("/auth/logout"), { method: "POST" }).catch(() => null);
    if (!response?.ok) {
      setError("Could not sign out. Try again.");
      return;
    }
    // From here every request is refused, and that is expected.
    signingOutNow();
    window.location.assign(serverUrl("/auth/signed-out"));
  };

  return (
    <details data-menu className={ui.menu}>
      <summary
        className={`block w-auto max-w-45 h-8 px-3 rounded-[7px] font-medium leading-[30px] whitespace-nowrap overflow-hidden text-ellipsis list-none [&::-webkit-details-marker]:hidden ${ui.button}`}
        aria-label={`Account: ${me.data.name}`}
        title={me.data.email || me.data.name}
      >
        {me.data.name}
      </summary>
      <div data-menu-pop className={`${ui.menuPopBase} top-9 min-w-50`}>
        {me.data.email && <p className={`${note} text-muted`}>{me.data.email}</p>}
        {me.data.email && <hr className={ui.menuRule} />}
        <button type="button" className={ui.menuItem} onClick={() => void signOut()}>
          Sign out
        </button>
        {error && (
          <p role="alert" className={`${note} text-danger`}>
            {error}
          </p>
        )}
      </div>
    </details>
  );
}
