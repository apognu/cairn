import { signIn } from "./api/client";
import { useMe } from "./queries";

/** Who is signed in, and the way out. Absent in `yourdocs dev`, where nobody signs in. */
export function UserMenu() {
  const me = useMe();

  if (!me.data?.email) return null;

  const signOut = async () => {
    await fetch("/auth/logout", { method: "POST" });
    signIn();
  };

  return (
    <details className="menu user-menu">
      <summary aria-label="Account" title={me.data.email}>
        {me.data.name}
      </summary>
      <div className="menu-pop">
        <p className="menu-note">{me.data.email}</p>
        <hr />
        <button type="button" onClick={() => void signOut()}>
          Sign out
        </button>
      </div>
    </details>
  );
}
