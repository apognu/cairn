// A stand-in for GitHub that the API reference tests point the server at (GITHUB_BASE_URL and RAW_BASE_URL). It serves latest releases,
// files and release assets from memory, and the tests change them over `/__state/…` to play a new upstream release.
const port = Number(process.env.PORT ?? 3390);

const spec = (title: string, version: string, operations: [method: string, path: string, id: string, tag: string][]) =>
  JSON.stringify({
    openapi: "3.0.3",
    info: { title, version, description: `${title} as of ${version}` },
    servers: [{ url: "https://api.example.com" }],
    paths: operations.reduce<Record<string, Record<string, unknown>>>((paths, [method, path, id, tag]) => {
      paths[path] = {
        ...paths[path],
        [method]: {
          operationId: id,
          tags: [tag],
          summary: `${id} summary`,
          responses: {
            "200": {
              description: "ok",
              content: { "application/json": { schema: { $ref: "#/components/schemas/Thing" } } },
            },
          },
        },
      };
      return paths;
    }, {}),
    components: {
      schemas: { Thing: { type: "object", properties: { id: { type: "string" }, name: { type: "string" } } } },
    },
  });

const defaults = (): { latest: Map<string, string>; files: Map<string, string> } => ({
  latest: new Map([
    ["acme/orders", "v1.0.0"],
    ["acme/billing", "v2.0.0"],
  ]),
  files: new Map([
    [
      "/acme/orders/releases/download/v1.0.0/openapi.json",
      spec("Orders", "1.0.0", [
        ["get", "/orders", "listOrders", "Orders"],
        ["post", "/orders", "createOrder", "Orders"],
      ]),
    ],
    [
      "/acme/billing/v2.0.0/api/billing.json",
      spec("Billing", "2.0.0", [["get", "/invoices", "listInvoices", "Invoices"]]),
    ],
  ]),
});

let state = defaults();

Bun.serve({
  port,
  hostname: "127.0.0.1",
  async fetch(request) {
    const url = new URL(request.url);
    const path = url.pathname;

    if (path === "/__state/reset") {
      state = defaults();
      return new Response("reset");
    }
    if (path.startsWith("/__state/latest/")) {
      state.latest.set(path.slice("/__state/latest/".length), await request.text());
      return new Response("ok");
    }
    if (path === "/__state/file") {
      const target = url.searchParams.get("path") ?? "";
      if (request.method === "DELETE") state.files.delete(target);
      else state.files.set(target, await request.text());
      return new Response("ok");
    }

    const latest = path.match(/^\/([^/]+\/[^/]+)\/releases\/latest$/);
    if (latest) {
      const tag = state.latest.get(latest[1]);
      return tag
        ? new Response(null, { status: 302, headers: { location: `/${latest[1]}/releases/tag/${tag}` } })
        : new Response("not found", { status: 404 });
    }

    const body = state.files.get(path);
    return body === undefined ? new Response("not found", { status: 404 }) : new Response(body);
  },
});
