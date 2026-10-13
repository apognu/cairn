# Build context: this folder. The custom components in `components/` end up in the server image.
FROM yourdocs/builder:1 AS build
COPY components /components
RUN yourdocs build-components /components --out /bundles

FROM yourdocs/runtime:1
COPY --from=build /bundles /app/bundles
