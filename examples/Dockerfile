# Build context: this folder. The custom components in `components/` end up in the server image.
FROM cairn/builder:1 AS build
COPY components /components
RUN cairn build-components /components --out /bundles

FROM cairn/runtime:1
COPY --from=build /bundles /app/bundles
