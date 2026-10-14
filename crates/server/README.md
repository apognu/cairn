# server

Axum app (library): the page tree and publish API, the embedded editor (or the one `build-components` made) and the published site. `run_dev` (two loopback ports, no login) and `run_serve` (one port, OIDC sign-in, configured by `config.rs`) are its entry points.
Not obvious: `dev` refuses non-loopback binds and non-local Host headers; `serve` mounts the editor, API and sign-in under `/_editor` (so site pages can be called `api`) and the self-hosted site at `/`; `serving.rs` has the router, `/_preview/<release>/` and the shared `/_a/` path; `proxy.rs` believes `X-Forwarded-For` only from `TRUSTED_PROXIES`.
`bundles.rs` picks the JavaScript bundles (embedded, or `/app/bundles` / `BUNDLES_DIR`) and refuses an SDK version mismatch. `locks.rs`: a draft save must carry the page lock's token (423 when held by someone else, 410 when released).
`api_pages.rs`: an API page's sources, "Check sources now" (one at a time per page; the last merged spec is kept in memory for the editor's preview) and the Scalar script for it.
`GITHUB_BASE_URL` and `RAW_BASE_URL` move where API sources are read from (tests and mirrors only).
