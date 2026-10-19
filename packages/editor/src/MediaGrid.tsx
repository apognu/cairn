import { type ReactNode, useEffect, useId, useState } from "react";
import { type Asset, assetLabel, thumbnail, useAssets } from "./media";
import * as ui from "./ui";

/** Typing pauses this long before the library is searched again. */
const SEARCH_DELAY_MS = 200;

/**
 * The media library as a searchable grid of thumbnails, in Settings and in the image picker. A click chooses an image, a double click
 * picks it at once. `children` go beside the search field.
 */
export function MediaGrid({
  selected,
  onChoose,
  onPick,
  children,
}: {
  selected: string | null;
  onChoose: (asset: Asset) => void;
  onPick?: (asset: Asset) => void;
  children?: ReactNode;
}) {
  const id = useId();
  const [typed, setTyped] = useState("");
  const [search, setSearch] = useState("");
  const assets = useAssets(search);
  const all = assets.data?.pages.flat() ?? [];

  useEffect(() => {
    const timer = setTimeout(() => setSearch(typed.trim()), SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  }, [typed]);

  return (
    <div>
      <div className="flex items-center gap-2 mb-3.5">
        <label htmlFor={id} className="sr-only">
          Search images
        </label>
        <input
          id={id}
          className={`${ui.input} flex-1`}
          type="search"
          placeholder="Search by title or file name"
          value={typed}
          // biome-ignore lint/a11y/noAutofocus: opening the library puts the focus in its first field.
          autoFocus
          onChange={(e) => setTyped(e.target.value)}
        />
        {children}
      </div>

      {assets.isError && (
        <p role="alert" className="text-danger">
          {assets.error.message}
        </p>
      )}
      {assets.isSuccess && all.length === 0 && (
        <p className="text-muted text-help">
          {search ? `No image matches “${search}”.` : "No image has been uploaded yet."}
        </p>
      )}

      <ul className="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-3 m-0 p-0 list-none" aria-label="Images">
        {all.map((asset) => (
          <li key={asset.id} className="min-w-0">
            <button
              type="button"
              className="group/tile flex flex-col w-full p-0 border-0 bg-transparent text-left text-text cursor-pointer"
              aria-pressed={selected === asset.id}
              onClick={() => onChoose(asset)}
              onDoubleClick={() => onPick?.(asset)}
            >
              <span className="flex items-center justify-center w-full aspect-[4/3] overflow-hidden border border-border rounded-lg bg-hover group-hover/tile:border-input group-aria-pressed/tile:border-accent group-aria-pressed/tile:outline-2 group-aria-pressed/tile:outline-accent">
                <img className="w-full h-full object-contain" src={thumbnail(asset)} alt="" loading="lazy" />
              </span>
              <span className={`mt-1.5 text-help truncate ${asset.title ? "" : "text-muted"}`}>
                {assetLabel(asset)}
              </span>
            </button>
          </li>
        ))}
      </ul>

      {assets.hasNextPage && (
        <div className="flex justify-center mt-4">
          <button
            type="button"
            className={ui.secondary}
            disabled={assets.isFetchingNextPage}
            onClick={() => assets.fetchNextPage()}
          >
            {assets.isFetchingNextPage ? "Loading…" : "Show more"}
          </button>
        </div>
      )}
    </div>
  );
}
