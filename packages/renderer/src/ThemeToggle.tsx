import { useEffect, useState } from "react";
import { applyTheme, nextTheme, readTheme, type ThemeChoice } from "./theme";

const labels: Record<ThemeChoice, string> = { system: "Auto", light: "Light", dark: "Dark" };

/** Half a disc for "as the system", a sun, a moon. */
const glyphs: Record<ThemeChoice, string> = {
  system: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zm0 2v14a7 7 0 0 1 0-14z",
  light:
    "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4",
  dark: "M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z",
};

/** Cycles system, light and dark. The server renders "system"; the saved choice shows once the page is running. */
export function ThemeToggle() {
  const [choice, setChoice] = useState<ThemeChoice>("system");
  useEffect(() => setChoice(readTheme()), []);
  const label = `Theme: ${labels[choice]}. Change theme`;

  return (
    <button
      type="button"
      className="theme-toggle"
      aria-label={label}
      title={label}
      onClick={() => {
        const next = nextTheme(choice);
        applyTheme(next);
        setChoice(next);
      }}
    >
      <svg
        width="17"
        height="17"
        viewBox="0 0 24 24"
        aria-hidden="true"
        fill={choice === "light" ? "none" : "currentColor"}
        stroke={choice === "light" ? "currentColor" : "none"}
        strokeWidth="2"
        strokeLinecap="round"
      >
        <path d={glyphs[choice]} fillRule="evenodd" />
      </svg>
    </button>
  );
}
