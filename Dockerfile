# syntax=docker/dockerfile:1
#
# Two images come out of this file:
#   docker build --target builder -t cairn-builder .   the toolchain that bundles custom components
#   docker build --target runtime -t cairn .           the server (COPY a /app/bundles from the builder to add custom components)
#
# CI publishes the server as ghcr.io/apognu/cairn. It builds the binary in its own job and passes it with
# `--build-context server=<folder holding cairn>`, which replaces the `server` stage, so nothing is compiled twice.

# The JavaScript workspace, installed. The base of the builder image.
FROM oven/bun:1 AS js
WORKDIR /opt/cairn
COPY package.json bun.lock bunfig.toml ./
COPY packages ./packages
# Pagefind's native binary (the `pagefind` command in node_modules is a Node launcher), for the runtime image.
RUN bun install --frozen-lockfile \
 && cp "$(find node_modules/.bun/@pagefind+linux-x64*/ -type f \( -name pagefind -o -name pagefind_extended \) | head -1)" /opt/pagefind

# `cairn build-components <folder> --out <dir>`
FROM js AS builder
RUN printf '#!/bin/sh\nexec bun /opt/cairn/packages/builder/src/cli.ts "$@"\n' > /usr/local/bin/cairn && chmod +x /usr/local/bin/cairn
WORKDIR /work

# The editor, `ssr.js`, `client.js` and `site.css` the binary embeds.
FROM js AS web
RUN bun run build

# Debian bookworm on both sides, so the binary finds the libc it was linked against.
FROM rust:1-bookworm AS server
WORKDIR /src
COPY Cargo.toml Cargo.lock .rustfmt.toml ./
COPY crates ./crates
COPY --from=web /opt/cairn/packages/editor/dist packages/editor/dist
COPY --from=web /opt/cairn/packages/renderer/dist packages/renderer/dist
# The caches keep the downloaded crates and the compiled dependencies (V8 above all) between builds; the binary is copied out because
# the cache mount is not part of the image.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release -p cairn-cli && cp target/release/cairn /cairn

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libstdc++6 && rm -rf /var/lib/apt/lists/*
COPY --from=server /cairn /usr/local/bin/cairn
COPY --from=js /opt/pagefind /usr/local/bin/pagefind
ENV PAGEFIND_BIN=/usr/local/bin/pagefind
# Bundles made by `cairn build-components` go here; without them the binary's own are used.
RUN useradd --system --uid 10001 --home-dir /data cairn \
 && mkdir -p /app/bundles /data \
 && chown cairn /data
USER cairn
WORKDIR /data
# The database, uploads and the published site: the one thing to keep (a named volume starts out owned by the user above).
VOLUME /data
ENV DATA_DIR=/data BIND=0.0.0.0:8080
EXPOSE 8080
# Plain HTTP on one port; HTTPS comes from a reverse proxy in front (see docs/DEPLOY.md). `cairn dev` is for trying things on a laptop.
ENTRYPOINT ["cairn"]
CMD ["serve"]
