# render

Renders ProseMirror documents to HTML in embedded V8 on dedicated threads (one isolate per worker; `ssr_rs` only sets up the platform).
Not obvious: V8 isolates are `!Send`, so each pool thread owns its own instance; a watchdog thread stops a render past its timeout, and a
render near the heap limit is stopped by a near-heap-limit callback (safe Rust: a plain `extern "C" fn`), after which the worker starts a fresh isolate.
Code highlighting is Rust too (syntect with `two-face`): classes and a stylesheet for published pages, colors resolved per token for the editor canvas (`/api/highlight`).
