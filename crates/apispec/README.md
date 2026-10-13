# apispec

Reads, bundles, validates and merges OpenAPI 3.0 and 3.1 documents as plain `serde_json::Value`; no I/O of its own.
Not obvious: external `$ref`s are fetched through the `Fetch` trait and inlined under `components` with generated names, so a spec is one self-contained document.
Maps are sorted (no `preserve_order`), so the same inputs always give the same bytes and the same hash.
