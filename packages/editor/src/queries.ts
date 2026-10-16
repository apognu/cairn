import { keepPreviousData, useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
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

/** How many things a publish would change: the pages, and the site settings as one more. */
export function pendingCount(pending: components["schemas"]["Pending"] | undefined): number {
  return pending ? pending.changes.length + (pending.settings ? 1 : 0) : 0;
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
  return usePageMutation(
    async (body: { parent_id: string | null; title: string; external_url?: string; kind?: "api" }) =>
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

/** The problems that would block a publish now. Asked fresh every time the publish dialog opens. */
export function useChecks() {
  return useQuery({
    queryKey: ["checks"],
    gcTime: 0,
    refetchOnMount: "always",
    queryFn: async () => unwrap(await api.GET("/api/publish/checks")),
  });
}

export function usePublish() {
  const client = useQueryClient();

  return useMutation({
    /** `pages` publishes a hotfix of them; without it, everything pending goes out. */
    mutationFn: async (pages?: string[]) =>
      unwrap(await api.POST("/api/publish", { body: pages ? { pages } : undefined })),
    onSuccess: () =>
      Promise.all([client.invalidateQueries({ queryKey: PENDING }), client.invalidateQueries({ queryKey: ["site"] })]),
  });
}

/** Builds the pending changes without making them live. Only targets with previews offer it. */
export function usePreview() {
  return useMutation({ mutationFn: async () => unwrap(await api.POST("/api/publish/preview")) });
}

/** Makes a preview the live site. */
export function usePromote() {
  const client = useQueryClient();

  return useMutation({
    mutationFn: async (id: string) =>
      unwrap(await api.POST("/api/releases/{id}/promote", { params: { path: { id } } })),
    onSuccess: () =>
      Promise.all([client.invalidateQueries({ queryKey: PENDING }), client.invalidateQueries({ queryKey: ["site"] })]),
  });
}

/** The releases that were the site, newest first. Read again each time Releases opens: publishing elsewhere adds one. */
export function useReleases() {
  return useQuery({ queryKey: ["releases"], gcTime: 0, queryFn: async () => unwrap(await api.GET("/api/releases")) });
}

export function useRollback() {
  const client = useQueryClient();

  return useMutation({
    mutationFn: async (id: string) =>
      unwrap(await api.POST("/api/releases/{id}/rollback", { params: { path: { id } } })),
    onSuccess: () =>
      Promise.all([PENDING, ["site"], ["releases"]].map((queryKey) => client.invalidateQueries({ queryKey }))),
  });
}

/** Where the published site lives. Fixed for the life of the server. */
export function useSiteInfo() {
  return useQuery({
    queryKey: ["site"],
    staleTime: Number.POSITIVE_INFINITY,
    queryFn: async () => unwrap(await api.GET("/api/site")),
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
    onSuccess: (saved) => {
      client.setQueryData(["settings"], saved);
      return Promise.all([
        client.invalidateQueries({ queryKey: ["theme-css"] }),
        client.invalidateQueries({ queryKey: PENDING }),
      ]);
    },
  });
}

/** The contrast problems and the per-mode custom properties of unsaved settings. Kept while the next answer loads, so the preview doesn't flash. */
export function useThemeCheck(settings: components["schemas"]["Settings"] | null) {
  return useQuery({
    queryKey: ["settings-check", settings],
    enabled: settings !== null,
    placeholderData: keepPreviousData,
    queryFn: async () =>
      unwrap(await api.POST("/api/settings/check", { body: settings as components["schemas"]["Settings"] })),
    retry: false,
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

const LIBRARY = ["library"];

/** The variables and snippets of the library. */
export function useLibrary() {
  return useQuery({ queryKey: LIBRARY, queryFn: async () => unwrap(await api.GET("/api/library")) });
}

function useLibraryMutation<V>(run: (vars: V) => Promise<components["schemas"]["LibraryEntry"][]>) {
  const client = useQueryClient();

  return useMutation({
    mutationFn: run,
    onSuccess: (entries) => {
      client.setQueryData(LIBRARY, entries);
      // Pages using a changed entry become pending. Not awaited: the form that saved the entry closes with the saved list, not later.
      void client.invalidateQueries({ queryKey: PENDING });
    },
  });
}

export function useSaveVariable() {
  return useLibraryMutation(async ({ key, value }: { key: string; value: string }) =>
    unwrap(await api.PUT("/api/library/variables/{key}", { params: { path: { key } }, body: { value } })),
  );
}

export function useSaveTerm() {
  return useLibraryMutation(async (body: components["schemas"]["TermBody"]) =>
    unwrap(await api.PUT("/api/library/terms", { body })),
  );
}

export function useSaveSnippet() {
  return useLibraryMutation(async ({ key, doc }: { key: string; doc: unknown }) =>
    unwrap(await api.PUT("/api/library/snippets/{key}", { params: { path: { key } }, body: { doc } })),
  );
}

export function useDeleteEntry() {
  return useLibraryMutation(async (id: string) =>
    unwrap(await api.DELETE("/api/library/{id}", { params: { path: { id } } })),
  );
}

/** The saved versions of a page, newest first. */
export function useHistory(id: string) {
  return useQuery({
    queryKey: ["history", id],
    gcTime: 0,
    queryFn: async () => unwrap(await api.GET("/api/pages/{id}/history", { params: { path: { id } } })),
  });
}

export function useRevision(id: string | null) {
  return useQuery({
    queryKey: ["revision", id],
    enabled: id !== null,
    queryFn: async () => unwrap(await api.GET("/api/revisions/{id}", { params: { path: { id: id as string } } })),
  });
}

/** Makes an old version the page's draft; the replaced draft stays in the history. */
export function useRestoreVersion(pageId: string) {
  const client = useQueryClient();

  return useMutation({
    mutationFn: async (body: components["schemas"]["Restore"]) =>
      unwrap(await api.POST("/api/pages/{id}/restore", { params: { path: { id: pageId } }, body })),
    // The title and description, when restored, show in the tree and the page.
    onSuccess: () =>
      Promise.all(["draft", "history", "pending", "pages"].map((key) => client.invalidateQueries({ queryKey: [key] }))),
  });
}

const TRASH = ["trash"];

export function useTrash() {
  return useQuery({ queryKey: TRASH, gcTime: 0, queryFn: async () => unwrap(await api.GET("/api/trash")) });
}

export function useRestoreFromTrash() {
  const client = useQueryClient();

  return useMutation({
    mutationFn: async (id: string) => unwrap(await api.POST("/api/trash/{id}/restore", { params: { path: { id } } })),
    onSuccess: () => Promise.all([TRASH, PAGES, PENDING].map((queryKey) => client.invalidateQueries({ queryKey }))),
  });
}

type ApiConfig = components["schemas"]["ApiConfig"];

/** The sources of an API reference page. */
export function useApiConfig(id: string) {
  return useQuery({
    queryKey: ["api-config", id],
    queryFn: async () => unwrap(await api.GET("/api/pages/{id}/api-config", { params: { path: { id } } })),
  });
}

export function useSaveApiConfig(id: string) {
  const client = useQueryClient();

  return useMutation({
    mutationFn: async (config: ApiConfig) =>
      unwrap(await api.PUT("/api/pages/{id}/api-config", { params: { path: { id } }, body: config })),
    onSuccess: (saved) => {
      client.setQueryData(["api-config", id], saved);
      return client.invalidateQueries({ queryKey: PENDING });
    },
  });
}

/** Fetches and merges the sources of the form as it stands, the way a publish would, without publishing. */
export function useCheckApiConfig(id: string) {
  const client = useQueryClient();

  return useMutation({
    mutationFn: async (config: ApiConfig) =>
      unwrap(await api.POST("/api/pages/{id}/api-config/check", { params: { path: { id } }, body: config })),
    // The server keeps what a successful check merged: links to operations are offered from it from now on.
    onSuccess: (result) =>
      result.errors.length === 0 ? client.invalidateQueries({ queryKey: ["api-operations", id] }) : undefined,
  });
}

/** The operations of each given API page, for the picker to search across them. Pages still loading have none yet. */
export function useApiOperationsOf<P extends { id: string }>(pages: P[]) {
  const results = useQueries({
    queries: pages.map((page) => ({
      queryKey: ["api-operations", page.id],
      queryFn: async () => unwrap(await api.GET("/api/pages/{id}/operations", { params: { path: { id: page.id } } })),
    })),
  });

  return pages.map((page, i) => ({ page, operations: results[i]?.data ?? [] }));
}

/** The operations of an API page that a link can point at; only asked for once the page is chosen in the picker. */
export function useApiOperations(id: string | null) {
  return useQuery({
    queryKey: ["api-operations", id],
    enabled: id !== null,
    queryFn: async () =>
      unwrap(await api.GET("/api/pages/{id}/operations", { params: { path: { id: id as string } } })),
  });
}

/** The signed-in user. In `cairn dev` there is nobody to sign in, and the email is empty. */
export function useMe() {
  return useQuery({
    queryKey: ["me"],
    staleTime: Number.POSITIVE_INFINITY,
    queryFn: async () => unwrap(await api.GET("/api/me")),
  });
}

/** The user `cairn dev` runs every request as (the server's `local_user_id`). */
const LOCAL_USER = "00000000-0000-0000-0000-000000000001";

/** Whether a person signed in, rather than `cairn dev`'s built-in user. Not judged by the email, which a provider may not give. */
export function useSignedIn(): boolean {
  const me = useMe().data;

  return me !== undefined && me.id !== LOCAL_USER;
}

const TOKENS = ["tokens"];

export function useTokens() {
  return useQuery({ queryKey: TOKENS, queryFn: async () => unwrap(await api.GET("/api/tokens")) });
}

export function useCreateToken() {
  const client = useQueryClient();

  return useMutation({
    mutationFn: async (name: string) => unwrap(await api.POST("/api/tokens", { body: { name } })),
    onSuccess: () => client.invalidateQueries({ queryKey: TOKENS }),
  });
}

export function useRevokeToken() {
  const client = useQueryClient();

  return useMutation({
    mutationFn: async (id: string) => unwrap(await api.DELETE("/api/tokens/{id}", { params: { path: { id } } })),
    onSuccess: () => client.invalidateQueries({ queryKey: TOKENS }),
  });
}

export type PageReview = components["schemas"]["PageReview"];

/** Who looks after a page and when it should be checked again. */
export function usePageReview(id: string) {
  return useQuery({
    queryKey: ["review", id],
    queryFn: async () => unwrap(await api.GET("/api/pages/{id}/review", { params: { path: { id } } })),
  });
}

/** Saves a page's owner and review date at once; they are never published. */
export function useSetPageReview(id: string) {
  const client = useQueryClient();

  return useMutation({
    mutationFn: async (review: PageReview) =>
      unwrap(await api.PUT("/api/pages/{id}/review", { params: { path: { id } }, body: review })),
    onSuccess: (saved) => {
      client.setQueryData(["review", id], saved);
      void client.invalidateQueries({ queryKey: ["stale"] });
    },
  });
}

/** The people who can own a page. */
export function useUsers() {
  return useQuery({ queryKey: ["users"], queryFn: async () => unwrap(await api.GET("/api/users")) });
}

/** The pages whose review date has come. */
export function useStalePages() {
  return useQuery({ queryKey: ["stale"], queryFn: async () => unwrap(await api.GET("/api/reports/stale")) });
}
