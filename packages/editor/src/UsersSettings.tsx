import { useId, useRef, useState } from "react";
import type { components } from "./api/schema";
import { type Role, useMe, useSetUserRole, useUsers } from "./queries";
import { useModal } from "./sidePanel";
import * as ui from "./ui";

type User = components["schemas"]["User"];

export const roleLabel: Record<Role, string> = {
  viewer: "Viewer",
  editor: "Editor",
  publisher: "Publisher",
  admin: "Admin",
};

const roleHelp: Record<Role, string> = {
  viewer: "reads drafts, history and settings",
  editor: "also edits pages, the library and redirects, and deploys previews",
  publisher: "also publishes, promotes and rolls back",
  admin: "also changes settings and roles",
};

const date = (iso: string) =>
  new Date(iso).toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" });

/** Everyone who signed in, and their roles. New people start as viewers; the site always keeps an admin. */
export function UsersSettings() {
  const users = useUsers();
  const me = useMe().data;
  const setRole = useSetUserRole();
  // Lowering one's own role cannot be undone without another admin, so it is confirmed first.
  const [demoting, setDemoting] = useState<Role | null>(null);

  const change = (user: User, role: Role) => {
    if (user.id === me?.id && role !== "admin") {
      setDemoting(role);
      return;
    }
    setRole.mutate({ id: user.id, role });
  };

  return (
    <div className="max-w-205">
      <h1 className={ui.settingsTitle}>Users</h1>
      <p className="text-muted">
        Everyone who signed in. People start as viewers the first time they sign in; give them a role here. A change
        applies at once, to their API tokens too.
      </p>
      <ul className="mt-2 mb-0 pl-4.5 text-muted text-help">
        {(Object.keys(roleLabel) as Role[]).map((role) => (
          <li key={role}>
            <strong className="text-text">{roleLabel[role]}</strong> {roleHelp[role]}
          </li>
        ))}
      </ul>

      <table className={ui.table} aria-label="Users">
        <thead>
          <tr>
            <th scope="col" className={ui.th}>
              Name
            </th>
            <th scope="col" className={ui.th}>
              Role
            </th>
            <th scope="col" className={ui.th}>
              Last signed in
            </th>
          </tr>
        </thead>
        <tbody>
          {users.data?.map((u) => (
            <tr key={u.id}>
              <td className={`${ui.td} wrap-anywhere`}>
                <strong>{u.name}</strong>
                {u.id === me?.id && <span className="text-muted"> (you)</span>}
                <br />
                <span className="text-muted text-help">{u.email}</span>
              </td>
              <td className={`${ui.tdBase} w-40`}>
                <select
                  className={ui.select}
                  aria-label={`Role of ${u.name}`}
                  value={u.role}
                  disabled={setRole.isPending}
                  onChange={(e) => change(u, e.target.value as Role)}
                >
                  {(Object.keys(roleLabel) as Role[]).map((role) => (
                    <option key={role} value={role}>
                      {roleLabel[role]}
                    </option>
                  ))}
                </select>
              </td>
              <td className={ui.tdBase}>
                {u.last_login_at ? date(u.last_login_at) : <span className="text-muted">never</span>}
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {setRole.error && (
        <p role="alert" className="text-danger">
          {setRole.error.message}
        </p>
      )}
      {demoting && me && (
        <DemoteDialog
          role={demoting}
          onClose={() => setDemoting(null)}
          onConfirm={() => {
            setRole.mutate({ id: me.id, role: demoting });
            setDemoting(null);
          }}
        />
      )}
    </div>
  );
}

function DemoteDialog({ role, onConfirm, onClose }: { role: Role; onConfirm: () => void; onClose: () => void }) {
  const id = useId();
  const dialog = useRef<HTMLDialogElement>(null);

  useModal(dialog);

  return (
    <dialog ref={dialog} className={ui.dialog} aria-labelledby={`${id}-title`} onClose={onClose}>
      <h2 id={`${id}-title`} className={ui.dialogTitle}>
        Make yourself {roleLabel[role].toLowerCase()}?
      </h2>
      <p className="text-muted">
        You will no longer be able to change settings or roles. Only another admin can give you the admin role back.
      </p>
      <div className={ui.actions}>
        <button type="button" className={ui.secondary} onClick={() => dialog.current?.close()}>
          Cancel
        </button>
        <button type="button" className={ui.primary} onClick={onConfirm}>
          Change my role
        </button>
      </div>
    </dialog>
  );
}
