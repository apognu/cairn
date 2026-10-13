# server

Axum app (library): the page tree and publish API, the embedded editor (or the one `build-components` made) and, in `dev`, the published site; `run_dev` is its entry point.
Not obvious: `dev` refuses non-loopback binds and non-local Host headers; `bundles.rs` picks the JavaScript bundles (embedded, or `/app/bundles` / `BUNDLES_DIR`) and refuses an SDK version mismatch.
