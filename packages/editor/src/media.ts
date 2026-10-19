import { keepPreviousData, useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./api/client";
import type { components } from "./api/schema";
import { unwrap } from "./api/unwrap";

export type Asset = components["schemas"]["Asset"];

const ASSETS = ["assets"];
const PAGE = 48;

/** The media library, newest first, a page at a time; `search` matches part of a title or file name. */
export function useAssets(search: string) {
  return useInfiniteQuery({
    queryKey: [...ASSETS, "list", search],
    initialPageParam: undefined as string | undefined,
    queryFn: async ({ pageParam }) =>
      unwrap(
        await api.GET("/api/assets", { params: { query: { q: search || undefined, before: pageParam, limit: PAGE } } }),
      ),
    getNextPageParam: (last) => (last.length < PAGE ? undefined : last.at(-1)?.id),
    placeholderData: keepPreviousData,
  });
}

export function useAsset(id: string | null) {
  return useQuery({
    queryKey: [...ASSETS, "detail", id],
    enabled: id !== null,
    queryFn: async () => unwrap(await api.GET("/api/assets/{id}", { params: { path: { id: id ?? "" } } })),
  });
}

function useAssetMutation<V, R>(run: (vars: V) => Promise<R>) {
  const client = useQueryClient();

  return useMutation({ mutationFn: run, onSuccess: () => client.invalidateQueries({ queryKey: ASSETS }) });
}

export function useRenameAsset() {
  return useAssetMutation(async ({ id, title }: { id: string; title: string }) =>
    unwrap(await api.PATCH("/api/assets/{id}", { params: { path: { id } }, body: { title } })),
  );
}

export function useDeleteAsset() {
  return useAssetMutation(async (id: string) => {
    const { error, response } = await api.DELETE("/api/assets/{id}", { params: { path: { id } } });
    if (error !== undefined)
      throw new Error((error as { message?: string }).message ?? `Request failed (${response.status})`);
  });
}

/** Lists again after uploads, so the new images show at the top. */
export function useRefreshAssets() {
  const client = useQueryClient();

  return () => client.invalidateQueries({ queryKey: ASSETS });
}

/** A small copy for thumbnails: the 480px variant when there is one, else the narrowest, else the original. */
export function thumbnail(asset: Asset): string {
  const variants = asset.variants ?? [];
  const variant = variants.find((v) => v.width === 480) ?? variants[0];

  return variant ? `/_a/${asset.id}-${variant.width}.webp` : `/_a/${asset.id}`;
}

/** What the library calls an asset: its title, else the file it came from. */
export function assetLabel(asset: Asset): string {
  return asset.title ?? asset.name ?? "Untitled image";
}
