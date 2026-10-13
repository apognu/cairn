import { useId } from "react";
import type { components } from "./api/schema";

type ModeColors = components["schemas"]["ModeColors"];

const FIELDS: { key: keyof ModeColors; label: string; fallback: { light: string; dark: string } }[] = [
  { key: "background", label: "Background", fallback: { light: "#ffffff", dark: "#121416" } },
  { key: "text", label: "Text", fallback: { light: "#16181a", dark: "#eceeef" } },
  { key: "surface", label: "Surfaces", fallback: { light: "#fafaf9", dark: "#17191c" } },
  { key: "link", label: "Links", fallback: { light: "#0b6e4f", dark: "#5fd3a6" } },
  { key: "code_background", label: "Code blocks", fallback: { light: "#f2f3f0", dark: "#1d2024" } },
];

/** What `site.css` makes of the accent in dark mode (`color-mix(in srgb, accent 55%, white)`). */
function darkAccent(accent: string): string {
  const channel = (i: number) =>
    Math.round(Number.parseInt(accent.slice(1 + 2 * i, 3 + 2 * i), 16) * 0.55 + 255 * 0.45);
  return `#${[0, 1, 2].map((i) => channel(i).toString(16).padStart(2, "0")).join("")}`;
}

/** The optional color overrides of one mode. A color left alone keeps the built-in one; links follow the accent. */
export function ThemeColors({
  mode,
  colors,
  accent,
  onChange,
}: {
  mode: "light" | "dark";
  colors: ModeColors;
  accent: string;
  onChange: (colors: ModeColors) => void;
}) {
  const id = useId();
  const validAccent = /^#[0-9a-f]{6}$/i.test(accent) ? accent.toLowerCase() : null;

  return (
    <div className="color-grid">
      {FIELDS.map(({ key, label, fallback }) => {
        const value = colors[key];
        // The swatch of a color left alone shows what readers get: for links, the accent of this mode.
        const shown =
          key === "link" && validAccent ? (mode === "light" ? validAccent : darkAccent(validAccent)) : fallback[mode];

        return (
          <div className="color-row" key={key}>
            <input
              type="color"
              aria-label={`${label} (${mode}) picker`}
              value={value ?? shown}
              onChange={(e) => onChange({ ...colors, [key]: e.target.value })}
            />
            <label htmlFor={`${id}-${key}`}>{label}</label>
            <input
              id={`${id}-${key}`}
              className="mono"
              placeholder={key === "link" ? "accent" : "default"}
              value={value ?? ""}
              onChange={(e) =>
                onChange({ ...colors, [key]: e.target.value.trim() === "" ? null : e.target.value.trim() })
              }
            />
          </div>
        );
      })}
    </div>
  );
}
