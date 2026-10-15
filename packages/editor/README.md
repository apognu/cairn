# editor

The editor SPA (React, TanStack Query, Vite), embedded in the server binary with rust-embed.
Not obvious: `src/api/` is generated from the server's OpenAPI spec (`bun run gen:api`); `typecheck` fails when it is stale. `bun run build` must run before `cargo build`. Styles are Tailwind utilities; shared control classes are in `src/ui.ts`.
