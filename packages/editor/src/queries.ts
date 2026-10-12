import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./api/client";
import type { components } from "./api/schema";
import { unwrap } from "./api/unwrap";
import type { MoveTarget } from "./tree";

const PAGES = ["pages"];

export function usePages() {
  return useQuery({ queryKey: PAGES, queryFn: async () => unwrap(await api.GET("/api/pages")) });
}

/** What publishing now would change; refreshed whenever the tree or a draft changes. */
export const PENDING = ["pending"];

export function usePending() {
  return useQuery({ queryKey: PENDING, queryFn: async () => unwrap(await api.GET("/api/publish/changes")) });
}

function usePageMutation<V, R>(run: (vars: V) => Promise<R>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: run,
    onSuccess: () =>
      Promise.all([client.invalidateQueries({ queryKey: PAGES }), client.invalidateQueries({ queryKey: PENDING })]),
  });
}

/** Creates a page, or an external link when `external_url` is given. */
export function useCreatePage() {
  return usePageMutation(async (body: { parent_id: string | null; title: string; external_url?: string }) =>
    unwrap(await api.POST("/api/pages", { body: { ...body, before: null } })),
  );
}

/** Renames a page; for a link, `external_url` changes its target too. `redirect: false` lets the old URL stop working. */
export function useRenamePage() {
  return usePageMutation(
    async ({
      id,
      title,
      external_url,
      redirect,
    }: {
      id: string;
      title: string;
      external_url?: string;
      redirect?: boolean;
    }) =>
      unwrap(await api.POST("/api/pages/{id}", { params: { path: { id } }, body: { title, external_url, redirect } })),
  );
}

/** `redirect: false` (Alt-drop) moves without keeping the old URLs working. */
export function useMovePage() {
  return usePageMutation(async ({ id, target, redirect }: { id: string; target: MoveTarget; redirect?: boolean }) =>
    unwrap(await api.POST("/api/pages/{id}/move", { params: { path: { id } }, body: { ...target, redirect } })),
  );
}

/** Sets the short description link cards show for the page. */
export function useSetDescription() {
  return usePageMutation(async ({ id, description }: { id: string; description: string }) =>
    unwrap(await api.PUT("/api/pages/{id}/description", { params: { path: { id } }, body: { description } })),
  );
}

/** Shows a page in the site header (or takes it out). */
export function useSetHeader() {
  return usePageMutation(async ({ id, in_header }: { id: string; in_header: boolean }) =>
    unwrap(await api.PUT("/api/pages/{id}/header", { params: { path: { id } }, body: { in_header } })),
  );
}

export function useDeletePage() {
  return usePageMutation(async (id: string) => {
    const { error, response } = await api.DELETE("/api/pages/{id}", { params: { path: { id } } });
    if (error !== undefined)
      throw new Error((error as { message?: string }).message ?? `Request failed (${response.status})`);
  });
}

export function usePublish() {
  const client = useQueryClient();

  return useMutation({
    mutationFn: async () => unwrap(await api.POST("/api/publish")),
    onSuccess: () => client.invalidateQueries({ queryKey: PENDING }),
  });
}

export function useSettings() {
  return useQuery({ queryKey: ["settings"], queryFn: async () => unwrap(await api.GET("/api/settings")) });
}

export function useSaveSettings() {
  const client = useQueryClient();

  return useMutation({
    mutationFn: async (settings: components["schemas"]["Settings"]) =>
      unwrap(await api.PUT("/api/settings", { body: settings })),
    onSuccess: (saved) => client.setQueryData(["settings"], saved),
  });
}

const pageUrlKey = (id: string) => ["page-url", id];

/** The URL, custom URL and aliases of one page. */
export function usePageUrl(id: string) {
  return useQuery({
    queryKey: pageUrlKey(id),
    queryFn: async () => unwrap(await api.GET("/api/pages/{id}/url", { params: { path: { id } } })),
  });
}

/** Every URL edit changes the tree's URLs and the pending changes too. */
function useUrlMutation<V>(id: string, run: (vars: V) => Promise<components["schemas"]["PageUrl"]>) {
  const client = useQueryClient();

  return useMutation({
    mutationFn: run,
    onSuccess: (saved) => {
      client.setQueryData(pageUrlKey(id), saved);
      return Promise.all([
        client.invalidateQueries({ queryKey: PAGES }),
        client.invalidateQueries({ queryKey: PENDING }),
      ]);
    },
  });
}

export function useSetCustomUrl(id: string) {
  return useUrlMutation(id, async (body: { custom_url: string | null; redirect?: boolean }) =>
    unwrap(await api.PUT("/api/pages/{id}/url", { params: { path: { id } }, body })),
  );
}

export function useAddAlias(id: string) {
  return useUrlMutation(id, async (path: string) =>
    unwrap(await api.POST("/api/pages/{id}/aliases", { params: { path: { id } }, body: { path } })),
  );
}

export function useRemoveAlias(id: string) {
  return useUrlMutation(id, async (path: string) =>
    unwrap(await api.DELETE("/api/pages/{id}/aliases", { params: { path: { id }, query: { path } } })),
  );
}

const REDIRECTS = ["redirects"];

export function useRedirects() {
  return useQuery({ queryKey: REDIRECTS, queryFn: async () => unwrap(await api.GET("/api/redirects")) });
}

/** Redirect changes also change what publishing would do. */
function useRedirectMutation<V>(run: (vars: V) => Promise<components["schemas"]["RedirectEntry"][]>) {
  const client = useQueryClient();

  return useMutation({
    mutationFn: run,
    onSuccess: (entries) => {
      client.setQueryData(REDIRECTS, entries);
      return client.invalidateQueries({ queryKey: PENDING });
    },
  });
}

export function useAddRedirect() {
  return useRedirectMutation(async (body: { from_path: string; to_page_id?: string; to_url?: string }) =>
    unwrap(await api.POST("/api/redirects", { body })),
  );
}

export function useRemoveRedirect() {
  return useRedirectMutation(async (path: string) =>
    unwrap(await api.DELETE("/api/redirects", { params: { query: { path } } })),
  );
}

/** The pages that link to this one. */
export function useBacklinks(id: string) {
  return useQuery({
    queryKey: ["backlinks", id],
    queryFn: async () => unwrap(await api.GET("/api/pages/{id}/backlinks", { params: { path: { id } } })),
  });
}

/** What deleting a page would affect; read fresh each time the question is asked. */
export function useDeleteImpact(id: string) {
  return useQuery({
    queryKey: ["impact", id],
    gcTime: 0,
    queryFn: async () => unwrap(await api.GET("/api/pages/{id}/impact", { params: { path: { id } } })),
  });
}
