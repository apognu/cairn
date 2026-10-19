import { type Palette, paint } from "./imageDraw";
import type { Color, Weight } from "./imageEdit";
import * as ui from "./ui";

export type Tool = "select" | "crop" | "arrow" | "text";
/** How new annotations look, and the selected one. */
export type Style = { color: Color; weight: Weight; pill: boolean };

const tools: { tool: Tool; label: string }[] = [
  { tool: "select", label: "Select" },
  { tool: "crop", label: "Crop" },
  { tool: "arrow", label: "Arrow" },
  { tool: "text", label: "Text" },
];

const colors: { color: Color; label: string }[] = [
  { color: "accent", label: "Accent color" },
  { color: "red", label: "Red" },
  { color: "black", label: "Black" },
  { color: "white", label: "White" },
];

const weights: { weight: Weight; label: string; short: string }[] = [
  { weight: "small", label: "Small", short: "S" },
  { weight: "medium", label: "Medium", short: "M" },
  { weight: "large", label: "Large", short: "L" },
];

/** The image editor's tools, the color and size of annotations, and Undo. */
export function ImageEditorTools({
  tool,
  onTool,
  style,
  onStyle,
  showPill,
  palette,
  canUndo,
  onUndo,
  canReset,
  onReset,
  canDelete,
  onDelete,
}: {
  tool: Tool;
  onTool: (tool: Tool) => void;
  style: Style;
  onStyle: (style: Style) => void;
  showPill: boolean;
  palette: Palette;
  canUndo: boolean;
  onUndo: () => void;
  canReset: boolean;
  onReset: () => void;
  canDelete: boolean;
  onDelete: () => void;
}) {
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-2.5 mb-3" role="toolbar" aria-label="Image tools">
      <fieldset className={`${ui.segmented} m-0 border-0`} aria-label="Tool">
        {tools.map((t) => (
          <button
            key={t.tool}
            type="button"
            className={ui.segment}
            aria-pressed={tool === t.tool}
            onClick={() => onTool(t.tool)}
          >
            {t.label}
          </button>
        ))}
      </fieldset>

      {tool === "crop" ? (
        <button type="button" className={ui.smallButton} disabled={!canReset} onClick={onReset}>
          Reset crop
        </button>
      ) : (
        <>
          <fieldset className="flex items-center gap-1.5 m-0 p-0 border-0" aria-label="Color">
            {colors.map((c) => (
              <button
                key={c.color}
                type="button"
                className="w-6 h-6 p-0 border border-input rounded-full cursor-pointer aria-pressed:outline-2 aria-pressed:outline-offset-2 aria-pressed:outline-text"
                // The swatch shows the color itself, which for the accent comes from the site settings.
                style={{ background: paint(c.color, palette) }}
                aria-label={c.label}
                aria-pressed={style.color === c.color}
                onClick={() => onStyle({ ...style, color: c.color })}
              />
            ))}
          </fieldset>
          <fieldset className={`${ui.segmented} m-0 border-0`} aria-label="Size">
            {weights.map((w) => (
              <button
                key={w.weight}
                type="button"
                className={ui.segment}
                aria-label={w.label}
                aria-pressed={style.weight === w.weight}
                onClick={() => onStyle({ ...style, weight: w.weight })}
              >
                {w.short}
              </button>
            ))}
          </fieldset>
          {showPill && (
            <label className="flex items-center gap-2 text-ui font-normal">
              <input
                type="checkbox"
                checked={style.pill}
                onChange={(e) => onStyle({ ...style, pill: e.target.checked })}
              />
              Background
            </label>
          )}
        </>
      )}

      <div className="flex gap-2 ml-auto">
        {canDelete && (
          <button type="button" className={ui.smallButton} onClick={onDelete}>
            Delete
          </button>
        )}
        <button type="button" className={ui.smallButton} disabled={!canUndo} onClick={onUndo}>
          Undo
        </button>
      </div>
    </div>
  );
}
