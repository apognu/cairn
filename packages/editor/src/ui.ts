// Utility classes of the controls every screen shares, so they look alike everywhere: text fields are 34px high, buttons 32px.

/** A label above its control, with help text under it. */
export const fieldBase = "flex flex-col gap-1.25 min-w-0";
export const field = `${fieldBase} mb-3.5`;
export const fieldLabel = "text-[12.5px] font-semibold";

export const input = "w-full h-8.5 px-2.5 border border-input rounded-md bg-bg";
/** Draws like the text fields beside it, with a chevron in the theme's color rather than the system's arrow. */
export const select = "w-full h-8.5 pl-2.5 pr-7.5 border border-input rounded-md bg-bg cursor-pointer select-chevron";
export const textarea = "py-2 px-2.5 leading-[1.45] border border-input rounded-md bg-bg resize-y";
export const codeInput =
  "w-full py-2 px-2.5 border border-input rounded-md bg-bg text-text font-mono text-[12.5px] leading-normal resize-y";

/** A checkbox or radio with its label beside it. */
export const check = "flex items-center gap-2 mb-3.5 text-ui font-normal";
export const inline = "flex gap-2";

export const primary =
  "h-8 px-3.5 border-0 rounded-[7px] bg-text text-bg text-[13.5px] font-semibold cursor-pointer disabled:opacity-60 disabled:cursor-default";
export const secondary =
  "h-8 px-3 border border-input rounded-[7px] bg-bg cursor-pointer disabled:opacity-50 disabled:cursor-default";
export const linkButton = "border-0 bg-transparent text-accent underline cursor-pointer";
export const actions = "flex justify-end gap-2";
/** The save buttons of a long settings form stay in reach while it scrolls; the screen around them looks for `sticky-actions`. */
export const stickyActions = "sticky-actions sticky bottom-0 z-1 mt-5 pt-3 pb-5 border-t border-border bg-bg";

export const mono = "font-mono text-label";
export const segmented = "flex p-0.5 rounded-[7px] bg-hover";
export const segment =
  "h-6.5 px-2.5 border-0 rounded-[5px] bg-transparent text-muted text-label cursor-pointer aria-selected:bg-bg aria-selected:text-text aria-selected:font-semibold aria-selected:shadow-segment aria-pressed:bg-bg aria-pressed:text-text aria-pressed:font-semibold aria-pressed:shadow-segment";

/** A settings screen: its form column and headings. */
export const settingsForm = "max-w-130";
export const settingsTitle = "mt-0 mb-4.5 text-[24px] tracking-[-0.015em]";
export const settingsHeading = "mt-6.5 mb-2.5 text-[17px]";
export const sectionHead = "flex items-center justify-between mt-6.5 mb-2.5";

/** A text field and its button on one line, as in the "add" forms under a list. */
export const rowInput = "flex-1 min-w-0 h-8.5 px-2.5 border border-input rounded-md bg-bg";
export const rowSubmit = "h-8.5 px-3 border border-input rounded-md bg-bg cursor-pointer";
export const smallButton = "h-7 px-2.5 border border-input rounded-md bg-bg cursor-pointer";
export const icon =
  "w-6 h-6 border-0 rounded-[5px] bg-transparent text-muted cursor-pointer hover:bg-border hover:text-text";

/** Tables of changes, releases, redirects and library entries. */
export const table = "w-full mt-3.5 mb-4.5 border-collapse";
export const th =
  "pt-0 pr-3 pb-2 pl-0 border-b border-input text-label font-semibold tracking-[0.06em] text-left uppercase text-muted";
export const td = "py-2.5 pr-3 pl-0 border-b border-border align-top first:w-37.5";

const badgeColors: Record<string, string> = {
  new: "bg-badge-new-bg text-badge-new",
  edited: "bg-badge-edit-bg text-badge-edit",
  renamed: "bg-badge-move-bg text-badge-move",
  moved: "bg-badge-move-bg text-badge-move",
  header: "bg-badge-move-bg text-badge-move",
  deleted: "bg-badge-del-bg text-badge-del",
};

/** A short tag; a change kind (`new`, `edited`, `moved`…) gives it that kind's color. */
export function badge(kind?: string): string {
  return `inline-block mr-1 py-0.5 px-2 rounded-sm text-label font-semibold ${(kind && badgeColors[kind]) || "bg-hover"}`;
}

export const tinyButton =
  "h-7 px-2.5 border border-input rounded-md bg-bg text-[12.5px] whitespace-nowrap cursor-pointer disabled:opacity-50 disabled:cursor-default";

/** A modal dialog: confirmations, publishing, rolling back. */
export const dialog =
  "w-120 p-6 border border-border rounded-[14px] shadow-dialog bg-bg text-text text-ui leading-[1.4] backdrop:bg-[rgb(0_0_0/40%)]";
export const dialogTitle = "mt-0 mb-4.5 text-[20px] tracking-[-0.01em]";
