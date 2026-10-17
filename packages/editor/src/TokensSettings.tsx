import { useId, useRef, useState } from "react";
import type { components } from "./api/schema";
import { atLeast, ROLE_ORDER, type Role, useCreateToken, useMe, useRevokeToken, useTokens } from "./queries";
import { useModal } from "./sidePanel";
import { roleLabel } from "./UsersSettings";
import * as ui from "./ui";

type ApiToken = components["schemas"]["ApiToken"];

const date = (iso: string) =>
  new Date(iso).toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" });

/** Tokens for scripts and CI: they act as their owner, and the secret is shown once, when it is made. */
export function TokensSettings() {
  const id = useId();
  const tokens = useTokens();
  const create = useCreateToken();
  const revoke = useRevokeToken();
  const [name, setName] = useState("");
  const [role, setRole] = useState<Role | "">("");
  const mine = useMe().data?.role;
  const [copied, setCopied] = useState(false);
  const [revoking, setRevoking] = useState<ApiToken | null>(null);
  const field = useRef<HTMLInputElement>(null);

  const error = create.error ?? revoke.error;
  const created = create.data;

  return (
    <div className="max-w-205">
      <h1 className={ui.settingsTitle}>API tokens</h1>
      <p className="text-muted">
        A token lets a script, such as a CI job, call this server as you: send it as{" "}
        <code>Authorization: Bearer &lt;token&gt;</code>. It can do what your role can, or less if you give it a role of
        its own, so keep it secret and delete it when it is no longer used.
      </p>

      <table className={ui.table} aria-label="API tokens">
        <thead>
          <tr>
            <th scope="col" className={ui.th}>
              Name
            </th>
            <th scope="col" className={ui.th}>
              Role
            </th>
            <th scope="col" className={ui.th}>
              Created
            </th>
            <th scope="col" className={ui.th}>
              Last used
            </th>
            <th scope="col" className={ui.th}>
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {tokens.data?.length === 0 && (
            <tr>
              <td colSpan={5} className={`${ui.td} text-muted text-help`}>
                No tokens yet.
              </td>
            </tr>
          )}
          {tokens.data?.map((t) => (
            <tr key={t.id}>
              <td className={ui.td}>
                <strong>{t.name}</strong>
              </td>
              <td className={ui.tdBase}>{t.role ? roleLabel[t.role] : <span className="text-muted">yours</span>}</td>
              <td className={ui.td}>{date(t.created_at)}</td>
              <td className={ui.td}>
                {t.last_used_at ? date(t.last_used_at) : <span className="text-muted">never</span>}
              </td>
              <td className={ui.td}>
                <button
                  type="button"
                  className={ui.icon}
                  aria-label={`Delete token ${t.name}`}
                  disabled={revoke.isPending}
                  onClick={() => setRevoking(t)}
                >
                  ×
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <h2 className={ui.settingsHeading}>Make a token</h2>
      <form
        className="grid gap-1 max-w-130"
        onSubmit={(e) => {
          e.preventDefault();
          setCopied(false);
          create.mutate(
            { name, role: role || null },
            {
              onSuccess: () => {
                setName("");
                field.current?.focus();
              },
            },
          );
        }}
      >
        <div className={ui.field}>
          <label htmlFor={`${id}-name`} className={ui.fieldLabel}>
            Name
          </label>
          <input
            id={`${id}-name`}
            className={ui.input}
            ref={field}
            placeholder="CI"
            maxLength={80}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
        <div className={ui.field}>
          <label htmlFor={`${id}-role`} className={ui.fieldLabel}>
            Role
          </label>
          <select
            id={`${id}-role`}
            className={ui.select}
            value={role}
            onChange={(e) => setRole(e.target.value as Role | "")}
          >
            <option value="">Yours, whatever it is when the token is used</option>
            {ROLE_ORDER.filter((r) => atLeast(mine, r)).map((r) => (
              <option key={r} value={r}>
                {roleLabel[r]}
              </option>
            ))}
          </select>
        </div>
        <button
          type="submit"
          className={`${ui.primary} justify-self-start`}
          disabled={create.isPending || name.trim() === ""}
        >
          Make token
        </button>
      </form>

      {created && (
        <output className="flex flex-col items-start gap-2 mt-4 py-3 px-3.5 border border-input rounded-lg text-help">
          <span>Copy “{created.token.name}” now: it is not shown again.</span>
          <code className="max-w-full wrap-anywhere text-help">{created.secret}</code>
          <button
            type="button"
            className={ui.secondary}
            onClick={() => {
              void navigator.clipboard.writeText(created.secret).then(() => setCopied(true));
            }}
          >
            {copied ? "Copied" : "Copy"}
          </button>
        </output>
      )}
      {error && (
        <p role="alert" className="text-danger">
          {error.message}
        </p>
      )}
      {revoking && (
        <RevokeDialog
          token={revoking}
          onClose={() => setRevoking(null)}
          onConfirm={() => {
            revoke.mutate(revoking.id, {
              // The secret of a token that no longer exists is no use to anyone.
              onSuccess: () => created?.token.id === revoking.id && create.reset(),
            });
            setRevoking(null);
          }}
        />
      )}
    </div>
  );
}

/** Deleting a token breaks whatever uses it at once, so it is confirmed. */
function RevokeDialog({ token, onConfirm, onClose }: { token: ApiToken; onConfirm: () => void; onClose: () => void }) {
  const id = useId();
  const dialog = useRef<HTMLDialogElement>(null);

  useModal(dialog);

  return (
    <dialog ref={dialog} className={ui.dialog} aria-labelledby={`${id}-title`} onClose={onClose}>
      <h2 id={`${id}-title`} className={ui.dialogTitle}>
        Delete “{token.name}”?
      </h2>
      <p className="text-muted">Scripts and CI jobs that use this token stop working at once. This cannot be undone.</p>
      <div className={ui.actions}>
        <button type="button" className={ui.secondary} onClick={() => dialog.current?.close()}>
          Cancel
        </button>
        <button type="button" className={ui.primary} onClick={onConfirm}>
          Delete
        </button>
      </div>
    </dialog>
  );
}
