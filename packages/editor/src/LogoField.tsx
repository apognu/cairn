import { useId } from "react";
import { uploadImage } from "./upload";

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
    <div className="form-field">
      <label htmlFor={id}>{label}</label>
      <div className="logo-field" data-theme={dark ? "dark" : "light"}>
        {value ? (
          <img className="logo-preview" src={`/_a/${value}`} alt={`Current ${label.toLowerCase()}`} />
        ) : (
          <span className="muted">{dark ? "The light logo" : "The site name alone"}</span>
        )}
        {/* The browser's own file input cannot be styled; it stays reachable by keyboard, drawn as the button after it. */}
        <input
          id={id}
          className="sr-only"
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
        <label htmlFor={id} className="button">
          {value ? "Replace" : "Upload"}
        </label>
        {value && (
          <button type="button" onClick={() => onChange(null)}>
            Remove
          </button>
        )}
      </div>
    </div>
  );
}
