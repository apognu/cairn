# server

Axum app (library): page tree API, the render spike routes and the embedded editor; `run_dev` is its entry point.
Not obvious: the spike's documents are still in memory until M1.11; `dev` refuses non-loopback binds and non-local Host headers.
