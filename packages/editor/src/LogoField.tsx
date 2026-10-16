import { useId } from "react";
import * as ui from "./ui";
import { uploadImage } from "./upload";

const logoButton = `inline-flex items-center h-8 px-3 rounded-[7px] text-ui font-medium ${ui.button}`;

/** One logo: the current image, and buttons to replace or remove it. */
export function LogoField({
  label,
  value,
  onChange,
  onError,
  dark = false,
}: {
  label: string;
  value: string | null;
  onChange: (id: string | null) => void;
  onError: (message: string) => void;
  /** Draws the box in dark colors, as the logo will appear in dark mode. */
  dark?: boolean;
}) {
  const id = useId();

  return (
    <div className={ui.field}>
      <label htmlFor={id} className={ui.fieldLabel}>
        {label}
      </label>
      <div
        className="flex items-center gap-2 min-h-14 py-2.5 px-3 border border-input rounded-lg bg-bg text-text"
        data-theme={dark ? "dark" : "light"}
      >
        {value ? (
          <img className="mr-auto max-h-9 max-w-50" src={`/_a/${value}`} alt={`Current ${label.toLowerCase()}`} />
        ) : (
          <span className="mr-auto text-muted">{dark ? "The light logo" : "The site name alone"}</span>
        )}
        {/* The browser's own file input cannot be styled; it stays reachable by keyboard, drawn as the button after it. */}
        <input
          id={id}
          className="peer sr-only"
          type="file"
          accept="image/png,image/jpeg,image/gif,image/webp"
          onChange={(e) => {
            const file = e.target.files?.[0];
            e.target.value = "";
            if (file)
              uploadImage(file).then(
                (asset) => onChange(asset.id),
                (err: Error) => onError(err.message),
              );
          }}
        />
        <label
          htmlFor={id}
          className={`${logoButton} peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-accent`}
        >
          {value ? "Replace" : "Upload"}
        </label>
        {value && (
          <button type="button" className={logoButton} onClick={() => onChange(null)}>
            Remove
          </button>
        )}
      </div>
    </div>
  );
}
