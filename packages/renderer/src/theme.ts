export type ThemeChoice = "system" | "light" | "dark";

const KEY = "cairn-theme";

// Storage can be blocked (private windows, blocked site data): the theme then simply follows the system.
export function readTheme(): ThemeChoice {
  try {
    const stored = localStorage.getItem(KEY);
    return stored === "light" || stored === "dark" ? stored : "system";
  } catch {
    return "system";
  }
}

/** Shows the chosen theme and remembers it; "system" removes the override so the system setting decides. */
export function applyTheme(choice: ThemeChoice): void {
  if (choice === "system") document.documentElement.removeAttribute("data-theme");
  else document.documentElement.dataset.theme = choice;

  try {
    if (choice === "system") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, choice);
  } catch {
    // see above
  }
}

export const nextTheme = (choice: ThemeChoice): ThemeChoice =>
  choice === "system" ? "light" : choice === "light" ? "dark" : "system";
