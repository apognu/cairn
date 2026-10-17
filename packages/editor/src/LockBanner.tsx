import { useState } from "react";
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
  const idle = lock.idle ? `, no activity since ${clock(lock.active_at)}` : "";
  return `Editing since ${clock(lock.acquired_at)}. ${saved}${idle}.`;
}

/** What the user can do about a page they cannot edit. */
export interface LockActions {
  edit: () => void;
  /** Resolves to the server's refusal, or null once the page is the user's. */
  takeOver: (force: boolean) => Promise<string | null>;
  watching: boolean;
  watch: () => void;
  /** Whether the user may take the page from someone still working on it: admins only. */
  canForce: boolean;
  /** This browser kept work after the page was taken; false when everything was saved. */
  unsaved: boolean;
  copy: () => Promise<void>;
  saveAsNew: () => Promise<void>;
}

interface Props {
  state: Exclude<LockState, { kind: "loading" } | { kind: "mine" }>;
  actions: LockActions;
}

/** A viewer reads every page; changing one needs the editor role. */
export function ViewerBanner() {
  return (
    <Banner title="You can read this page." lead="Your role is viewer.">
      To change pages, ask an admin for the editor role.
    </Banner>
  );
}

/** Why the page is read-only, and what can be done about it. */
export function LockBanner({ state, actions }: Props) {
  switch (state.kind) {
    case "held":
      if (state.lock.mine)
        return (
          <Banner
            title="You are editing this page in another tab."
            buttons={<Primary onClick={() => void actions.takeOver(true)}>Edit here</Primary>}
          >
            Editing here stops the other tab's editor; what it has not saved stays in that tab's browser copy.
          </Banner>
        );
      return <Held lock={state.lock} actions={actions} />;
    case "taken":
      return (
        <Banner
          title={
            state.lock?.mine
              ? "You are now editing this page in another tab."
              : `${state.lock?.user_name ?? "Someone"} took over editing this page.`
          }
          lead="You can read it, but not change it."
          buttons={
            actions.unsaved && (
              <>
                <Secondary onClick={() => void actions.copy()}>Copy my changes</Secondary>
                <Primary onClick={() => void actions.saveAsNew()}>Save as a new page</Primary>
              </>
            )
          }
        >
          {actions.unsaved
            ? "What you had not saved is kept in this browser: copy it, or save it as a page of its own."
            : "Everything you wrote had been saved."}
        </Banner>
      );
    case "free":
      return (
        <Banner title="Nobody is editing this page now." buttons={<Primary onClick={actions.edit}>Edit</Primary>}>
          The page you see is its last saved version.
        </Banner>
      );
    case "idle":
      return (
        <Banner
          title="You stopped editing this page."
          buttons={<Primary onClick={actions.edit}>Resume editing</Primary>}
        >
          After 10 minutes without activity the page is freed for others. Your saved work is here.
        </Banner>
      );
    case "error":
      return (
        <Banner title={state.message} buttons={<Primary onClick={actions.edit}>Try again</Primary>}>
          The page is shown read-only until it can be opened for editing.
        </Banner>
      );
  }
}

function Held({ lock, actions }: { lock: LockView; actions: LockActions }) {
  const [confirming, setConfirming] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  const first = lock.user_name.split(/\s+/)[0];

  const take = async (force: boolean) => {
    const refused = await actions.takeOver(force);
    setRefusal(refused);
    if (refused && !force) setConfirming(true);
  };

  if (confirming)
    return (
      <Banner
        title={`${lock.user_name} is still working on this page.`}
        avatar={initials(lock.user_name)}
        buttons={
          <>
            <Secondary onClick={() => setConfirming(false)}>Cancel</Secondary>
            <Primary onClick={() => void take(true)}>Take over anyway</Primary>
          </>
        }
      >
        Taking over stops {first}'s editor at once. Anything {first} has not saved stays in {first}'s browser.
        {refusal && confirming && <span> {refusal}</span>}
      </Banner>
    );

  return (
    <Banner
      title={`${lock.user_name} is editing this page.`}
      lead="You can read it, but not change it."
      avatar={initials(lock.user_name)}
      buttons={
        <>
          <Secondary onClick={actions.watch} disabled={actions.watching}>
            {actions.watching ? "You will be notified" : "Notify me when free"}
          </Secondary>
          {lock.idle ? (
            <Primary onClick={() => void take(false)}>Take over editing</Primary>
          ) : (
            actions.canForce && <Secondary onClick={() => setConfirming(true)}>Take over…</Secondary>
          )}
        </>
      }
    >
      {activity(lock)}
      {lock.idle && ` Taking over notifies ${first}; anything not saved stays in ${first}'s browser.`}
      {!lock.idle && !actions.canForce && ` You can take over once ${first} stops working on it.`}
    </Banner>
  );
}

function Primary({ onClick, children }: { onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      type="button"
      className="h-8 px-3.5 border-0 rounded-[7px] bg-warn-icon text-bg text-[13.5px] font-semibold cursor-pointer"
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function Secondary({
  onClick,
  disabled,
  children,
}: {
  onClick: () => void;
  disabled?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      className="h-8 px-3 border border-warn-border rounded-[7px] bg-transparent text-inherit cursor-pointer disabled:opacity-50 disabled:cursor-default"
      onClick={onClick}
      disabled={disabled}
    >
      {children}
    </button>
  );
}

function Banner({
  title,
  lead,
  avatar,
  buttons,
  children,
}: {
  title: string;
  lead?: string;
  avatar?: string;
  buttons?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    // The buttons go to a line of their own when the canvas is too narrow for them beside the text.
    <output
      className="flex flex-wrap items-center gap-x-3.5 gap-y-2.5 py-3 px-6 border-b border-warn-border bg-warn-bg text-warn-text text-ui [@media(max-width:700px)]:px-4"
      data-testid="lock-banner"
    >
      {avatar && (
        <span
          className="flex flex-none items-center justify-center w-7.5 h-7.5 rounded-full bg-accent text-bg text-[11.5px] font-bold"
          aria-hidden="true"
        >
          {avatar}
        </span>
      )}
      <div className="flex-[1_1_340px] min-w-0">
        <div>
          <strong>{title}</strong>
          {lead && ` ${lead}`}
        </div>
        <div className="mt-0.5 text-help opacity-85">{children}</div>
      </div>
      {buttons && <div className="flex flex-none flex-wrap gap-2 ml-auto">{buttons}</div>}
    </output>
  );
}
