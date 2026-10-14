import type { LockState, LockView } from "./useLock";

/** "3 minutes ago", "just now": how long ago the holder last did something. */
export function ago(iso: string, now = Date.now()): string {
  const minutes = Math.floor((now - new Date(iso).getTime()) / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} minute${minutes === 1 ? "" : "s"} ago`;
  const hours = Math.floor(minutes / 60);
  return `${hours} hour${hours === 1 ? "" : "s"} ago`;
}

export const clock = (iso: string) => new Date(iso).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });

function initials(name: string): string {
  const words = name.split(/\s+/).filter(Boolean);
  return (words.length > 1 ? words[0][0] + words[words.length - 1][0] : name.slice(0, 2)).toUpperCase();
}

/** What the holder last did, as the banner's second line. */
function activity(lock: LockView): string {
  const saved = lock.last_saved_at ? `Last saved ${ago(lock.last_saved_at)}` : "Not saved yet";
  return `Editing since ${clock(lock.acquired_at)}. ${saved}.`;
}

interface Props {
  state: Exclude<LockState, { kind: "loading" } | { kind: "mine" }>;
  onEdit: () => void;
}

/** Why the page is read-only, and what can be done about it. */
export function LockBanner({ state, onEdit }: Props) {
  switch (state.kind) {
    case "held":
      if (state.lock.mine)
        return (
          <Banner title="You are editing this page in another tab." onEdit={onEdit} action="Edit here">
            Editing here closes the other tab's editor; what it has not saved stays in that browser.
          </Banner>
        );
      return (
        <Banner
          title={`${state.lock.user_name} is editing this page.`}
          lead="You can read it, but not change it."
          avatar={initials(state.lock.user_name)}
        >
          {activity(state.lock)}
        </Banner>
      );
    case "free":
      return (
        <Banner title="Nobody is editing this page now." onEdit={onEdit} action="Edit">
          The page you see is its last saved version.
        </Banner>
      );
    case "idle":
      return (
        <Banner title="You stopped editing this page." onEdit={onEdit} action="Resume editing">
          After 10 minutes without activity the page is freed for others. Your saved work is here.
        </Banner>
      );
    case "error":
      return (
        <Banner title={state.message} onEdit={onEdit} action="Try again">
          The page is shown read-only until it can be opened for editing.
        </Banner>
      );
  }
}

function Banner({
  title,
  lead,
  avatar,
  action,
  onEdit,
  children,
}: {
  title: string;
  lead?: string;
  avatar?: string;
  action?: string;
  onEdit?: () => void;
  children: React.ReactNode;
}) {
  return (
    <output className="lock-banner">
      {avatar && (
        <span className="avatar" aria-hidden="true">
          {avatar}
        </span>
      )}
      <div className="lock-text">
        <div>
          <strong>{title}</strong>
          {lead && ` ${lead}`}
        </div>
        <div className="lock-detail">{children}</div>
      </div>
      {action && onEdit && (
        <div className="lock-actions">
          <button type="button" className="primary" onClick={onEdit}>
            {action}
          </button>
        </div>
      )}
    </output>
  );
}
