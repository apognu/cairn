import { useEffect, useId, useRef, useState } from "react";
import type { components } from "./api/schema";
import { useCreateToken, useRevokeToken, useTokens } from "./queries";

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
  const [copied, setCopied] = useState(false);
  const [revoking, setRevoking] = useState<ApiToken | null>(null);
  const field = useRef<HTMLInputElement>(null);

  const error = create.error ?? revoke.error;
  const created = create.data;

  return (
    <div className="settings-form wide">
      <h1>API tokens</h1>
      <p className="muted">
        A token lets a script, such as a CI job, call this server as you: send it as{" "}
        <code>Authorization: Bearer &lt;token&gt;</code>. It can do everything you can, so keep it secret and delete it
        when it is no longer used.
      </p>

      <table className="changes" aria-label="API tokens">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">Created</th>
            <th scope="col">Last used</th>
            <th scope="col">
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {tokens.data?.length === 0 && (
            <tr>
              <td colSpan={4} className="muted">
                No tokens yet.
              </td>
            </tr>
          )}
          {tokens.data?.map((t) => (
            <tr key={t.id}>
              <td>
                <strong>{t.name}</strong>
              </td>
              <td>{date(t.created_at)}</td>
              <td>{t.last_used_at ? date(t.last_used_at) : <span className="muted">never</span>}</td>
              <td>
                <button
                  type="button"
                  className="icon"
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

      <h2>Make a token</h2>
      <form
        className="redirect-form"
        onSubmit={(e) => {
          e.preventDefault();
          setCopied(false);
          create.mutate(name, {
            onSuccess: () => {
              setName("");
              field.current?.focus();
            },
          });
        }}
      >
        <div className="form-field">
          <label htmlFor={`${id}-name`}>Name</label>
          <input
            id={`${id}-name`}
            ref={field}
            placeholder="CI"
            maxLength={80}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
        <button type="submit" className="primary" disabled={create.isPending || name.trim() === ""}>
          Make token
        </button>
      </form>

      {created && (
        <output className="token-secret">
          <span>Copy “{created.token.name}” now: it is not shown again.</span>
          <code>{created.secret}</code>
          <button
            type="button"
            onClick={() => {
              void navigator.clipboard.writeText(created.secret).then(() => setCopied(true));
            }}
          >
            {copied ? "Copied" : "Copy"}
          </button>
        </output>
      )}
      {error && (
        <p role="alert" className="error-text">
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

  useEffect(() => dialog.current?.showModal(), []);

  return (
    <dialog ref={dialog} className="settings" aria-labelledby={`${id}-title`} onClose={onClose}>
      <h2 id={`${id}-title`}>Delete “{token.name}”?</h2>
      <p className="muted">Scripts and CI jobs that use this token stop working at once. This cannot be undone.</p>
      <div className="dialog-actions">
        <button type="button" onClick={() => dialog.current?.close()}>
          Cancel
        </button>
        <button type="button" className="primary" onClick={onConfirm}>
          Delete
        </button>
      </div>
    </dialog>
  );
}
