# server

Axum app (library): the page tree and publish API, the embedded editor (or the one `build-components` made) and, in `dev`, the published site; `run_dev` is its entry point.
Not obvious: `dev` refuses non-loopback binds and non-local Host headers; `bundles.rs` picks the JavaScript bundles (embedded, or `/app/bundles` / `BUNDLES_DIR`) and refuses an SDK version mismatch.
`api_pages.rs`: an API page's sources, "Check sources now" (one at a time per page; the last merged spec is kept in memory for the editor's preview) and the Scalar script for it.
`GITHUB_BASE_URL` and `RAW_BASE_URL` move where API sources are read from (tests and mirrors only).
