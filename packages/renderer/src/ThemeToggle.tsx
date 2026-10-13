import { useEffect, useState } from "react";
import { applyTheme, nextTheme, readTheme, type ThemeChoice } from "./theme";

const labels: Record<ThemeChoice, string> = { system: "Auto", light: "Light", dark: "Dark" };
const glyphs: Record<ThemeChoice, string> = { system: "◐", light: "☀", dark: "☾" };

/** Cycles system, light and dark. The server renders "system"; the saved choice shows once the page is running. */
export function ThemeToggle() {
  const [choice, setChoice] = useState<ThemeChoice>("system");
  useEffect(() => setChoice(readTheme()), []);

  return (
    <button
      type="button"
      className="theme-toggle"
      aria-label={`Theme: ${labels[choice]}. Change theme`}
      onClick={() => {
        const next = nextTheme(choice);
        applyTheme(next);
        setChoice(next);
      }}
    >
      <span aria-hidden="true">{glyphs[choice]}</span>
      <span className="theme-label">{` ${labels[choice]}`}</span>
    </button>
  );
}
