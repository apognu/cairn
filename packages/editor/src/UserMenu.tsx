import { useQuery } from "@tanstack/react-query";
import { api, signIn } from "./api/client";
import { unwrap } from "./api/unwrap";

/** Who is signed in, and the way out. Absent in `yourdocs dev`, where nobody signs in. */
export function UserMenu() {
  const me = useQuery({
    queryKey: ["me"],
    staleTime: Number.POSITIVE_INFINITY,
    queryFn: async () => unwrap(await api.GET("/api/me")),
  });

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
