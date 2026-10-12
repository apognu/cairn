import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./api/client";
import { unwrap } from "./api/unwrap";
import type { MoveTarget } from "./tree";

const PAGES = ["pages"];

export function usePages() {
  return useQuery({ queryKey: PAGES, queryFn: async () => unwrap(await api.GET("/api/pages")) });
}

function usePageMutation<V, R>(run: (vars: V) => Promise<R>) {
  const client = useQueryClient();
  return useMutation({ mutationFn: run, onSuccess: () => client.invalidateQueries({ queryKey: PAGES }) });
}

export function useCreatePage() {
  return usePageMutation(async (body: { parent_id: string | null; title: string }) =>
    unwrap(await api.POST("/api/pages", { body: { ...body, before: null } })),
  );
}

export function useRenamePage() {
  return usePageMutation(async ({ id, title }: { id: string; title: string }) =>
    unwrap(await api.POST("/api/pages/{id}", { params: { path: { id } }, body: { title } })),
  );
}

export function useMovePage() {
  return usePageMutation(async ({ id, target }: { id: string; target: MoveTarget }) =>
    unwrap(await api.POST("/api/pages/{id}/move", { params: { path: { id } }, body: target })),
  );
}

export function useDeletePage() {
  return usePageMutation(async (id: string) => {
    const { error, response } = await api.DELETE("/api/pages/{id}", { params: { path: { id } } });
    if (error !== undefined)
      throw new Error((error as { message?: string }).message ?? `Request failed (${response.status})`);
  });
}
