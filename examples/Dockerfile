# Build context: this folder. The custom components in `components/` end up in the server image.
# `cairn-builder` is built from a checkout of the same version as the server: `docker build --target builder -t cairn-builder .`
FROM cairn-builder AS build
COPY components /components
RUN cairn build-components /components --out /bundles

FROM ghcr.io/apognu/cairn:latest
COPY --from=build /bundles /app/bundles
