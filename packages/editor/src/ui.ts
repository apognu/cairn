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

/** A checkbox or radio with its label beside it; `checkInField` when it is one of a field's choices. */
export const check = "flex items-center gap-2 mb-3.5 text-ui font-normal";
export const checkInField = "flex items-center gap-2 mt-0 mb-1.5 text-ui font-normal";
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

/** A short tag; a change kind (`new`, `edited`, `moved`…) gives it that kind's color. `margin` replaces the space after it. */
export function badge(kind?: string, margin = "mr-1"): string {
  return `inline-block ${margin} py-0.5 px-2 rounded-sm text-label font-semibold ${(kind && badgeColors[kind]) || "bg-hover"}`;
}

export const tinyButton =
  "h-7 px-2.5 border border-input rounded-md bg-bg text-[12.5px] whitespace-nowrap cursor-pointer disabled:opacity-50 disabled:cursor-default";

/** A modal dialog: confirmations, publishing, rolling back. */
export const dialog =
  "w-120 p-6 border border-border rounded-[14px] shadow-dialog bg-bg text-text text-ui leading-[1.4] backdrop:bg-[rgb(0_0_0/40%)]";
export const dialogTitle = "mt-0 mb-4.5 text-[20px] tracking-[-0.01em]";

/** The dot of a page or section with unpublished changes. */
export const dot = "flex-none w-1.5 h-1.5 ml-1 mr-2 rounded-full bg-accent";

/**
 * A `<details data-menu>` menu: its summary, and the panel (`data-menu-pop`) that opens under it. `menus.ts` closes it on a click
 * elsewhere; `placeMenu` sets `data-up` to open the panel above.
 */
export const menu = "group/menu relative";
export const menuSummary =
  "flex items-center justify-center w-6 h-6 rounded-[5px] text-muted cursor-pointer list-none hover:bg-border hover:text-text [&::-webkit-details-marker]:hidden";
/** The panel without its place, for menus that open further down (the top bar's) or wider. */
export const menuPopBase =
  "absolute z-10 right-0 flex flex-col p-1 border border-input rounded-lg bg-bg shadow-popover text-text font-normal";
export const menuPop = `${menuPopBase} top-7 min-w-35 group-data-up/menu:top-auto group-data-up/menu:bottom-7`;
export const menuItem = "py-1.5 px-2.5 border-0 rounded-[5px] bg-transparent text-left cursor-pointer hover:bg-hover";
export const menuRule = "w-full mx-0 my-1 border-0 border-t border-border";

/** A failed request, said in a box. */
export const errorBox = "my-1 mx-2 py-1.5 px-2 border border-danger rounded-md text-danger";

/** The menus that open at the cursor (blocks, emojis): the box takes its width and padding from the menu. */
export const cursorMenu =
  "flex flex-col max-h-[476px] overflow-y-auto border border-input rounded-xl bg-bg shadow-[0_12px_32px_rgb(16_20_22/18%)] text-ui leading-[1.4]";
export const cursorOption =
  "group/option flex items-center gap-3 py-1.5 px-2 border-0 rounded-lg bg-transparent text-left cursor-pointer hover:bg-accent-soft aria-selected:bg-accent-soft";

/**
 * A panel from the right, under the top bar (and under the page actions when the side column shows them: the shell sets
 * `--drawer-top`): page settings and history.
 */
export const drawer =
  "fixed top-(--drawer-top) right-0 bottom-0 left-auto z-30 w-(--panel-width) max-w-screen h-[calc(100vh-var(--drawer-top))] max-h-none m-0 py-5 px-6 border-0 border-l border-border bg-side text-text text-ui leading-[1.4] overflow-y-auto narrow:shadow-drawer";
export const drawerHead = "flex items-center justify-between mb-4";
export const drawerTitle = "m-0 text-[20px]";
/** A section of a drawer, and its heading, the same whether a legend or a heading. */
export const drawerSection = `${fieldBase} mb-4.5`;
export const drawerHeading = "p-0 mt-0 mb-2 text-ui font-semibold";
/** A drawer's direct paragraphs are help text. */
export const drawerNote = "text-muted text-help";

/** The block settings, in the side column or beside a snippet; empty until the cursor is in a component. */
export const props =
  "props pt-4.5 px-4.5 pb-6 overflow-y-auto overflow-x-hidden bg-side text-ui leading-[1.4] empty:before:content-['Select_a_component_to_change_its_settings.'] empty:before:text-muted empty:before:text-help";
