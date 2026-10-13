import { useState } from "react";
import { serverUrl } from "./api/base";
import { useMe, useSignedIn } from "./queries";

/** Who is signed in, and the way out. Absent in `yourdocs dev`, where nobody signs in. */
export function UserMenu() {
  const me = useMe();
  const signedIn = useSignedIn();
  const [error, setError] = useState<string | null>(null);

  if (!signedIn || !me.data) return null;

  const signOut = async () => {
    setError(null);
    // Only once the server ended the session: leaving for the signed-out page otherwise would say so while the session lives on.
    const response = await fetch(serverUrl("/auth/logout"), { method: "POST" }).catch(() => null);
    if (!response?.ok) {
      setError("Could not sign out. Try again.");
      return;
    }
    window.location.assign(serverUrl("/auth/signed-out"));
  };

  return (
    <details className="menu user-menu">
      <summary aria-label={`Account: ${me.data.name}`} title={me.data.email || me.data.name}>
        {me.data.name}
      </summary>
      <div className="menu-pop">
        {me.data.email && <p className="menu-note">{me.data.email}</p>}
        {me.data.email && <hr />}
        <button type="button" onClick={() => void signOut()}>
          Sign out
        </button>
        {error && (
          <p role="alert" className="menu-note error-text">
            {error}
          </p>
        )}
      </div>
    </details>
  );
}
