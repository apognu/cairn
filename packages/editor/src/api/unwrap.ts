/** Throws the server's error message when a request failed, so TanStack Query surfaces it. */
export function unwrap<T>(result: { data?: T; error?: unknown; response: Response }): T {
  if (result.error !== undefined || result.data === undefined) {
    const body = result.error as { message?: string } | undefined;
    throw new Error(body?.message ?? `Request failed (${result.response.status})`);
  }

  return result.data;
}
