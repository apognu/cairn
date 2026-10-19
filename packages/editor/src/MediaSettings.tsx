import { useId, useState } from "react";
import { MediaDrawer } from "./MediaDrawer";
import { MediaGrid } from "./MediaGrid";
import { useRefreshAssets } from "./media";
import * as ui from "./ui";
import { uploadImage } from "./upload";

const IMAGE_TYPES = "image/png,image/jpeg,image/gif,image/webp";

/** Every uploaded image: found by title or file name, named, and taken out once nothing uses it. */
export function MediaSettings({ onOpenPage }: { onOpenPage: (id: string) => void }) {
  const id = useId();
  const [chosen, setChosen] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const refresh = useRefreshAssets();

  // Images added here are kept in the library even while no page shows them.
  const upload = async (files: File[]) => {
    setError(null);
    const images = files.filter((f) => f.type.startsWith("image/"));
    let last: string | null = null;
    for (const file of images) {
      try {
        last = (await uploadImage(file, { keep: true })).id;
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    }
    await refresh();
    if (last && images.length === 1) setChosen(last);
  };

  return (
    <section
      className={`max-w-205 min-h-full rounded-lg ${dragging ? "outline-2 outline-dashed outline-accent outline-offset-8" : ""}`}
      aria-labelledby={`${id}-title`}
      onDragOver={(e) => {
        if (!e.dataTransfer.types.includes("Files")) return;
        e.preventDefault();
        setDragging(true);
      }}
      onDragLeave={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setDragging(false);
      }}
      onDrop={(e) => {
        e.preventDefault();
        setDragging(false);
        void upload(Array.from(e.dataTransfer.files));
      }}
    >
      <h1 id={`${id}-title`} className={ui.settingsTitle}>
        Media
      </h1>
      <p className="text-muted">
        Images uploaded to the site. Images added here stay until they are deleted, even when no page shows them; images
        pasted into a page go once no page uses them. Drop files here to add them.
      </p>

      {error && (
        <p role="alert" className="text-danger">
          {error}
        </p>
      )}

      <MediaGrid selected={chosen} onChoose={(asset) => setChosen(asset.id)}>
        {/* The browser's own file input cannot be styled; it stays reachable by keyboard, drawn as the button after it. */}
        <input
          id={`${id}-file`}
          className="peer sr-only"
          type="file"
          accept={IMAGE_TYPES}
          multiple
          onChange={(e) => {
            const files = Array.from(e.target.files ?? []);
            e.target.value = "";
            void upload(files);
          }}
        />
        <label
          htmlFor={`${id}-file`}
          className={`${ui.secondary} inline-flex items-center flex-none font-medium peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-accent`}
        >
          Upload
        </label>
      </MediaGrid>

      {chosen && (
        <MediaDrawer
          key={chosen}
          id={chosen}
          onClose={() => setChosen(null)}
          onOpen={setChosen}
          onOpenPage={onOpenPage}
        />
      )}
    </section>
  );
}
