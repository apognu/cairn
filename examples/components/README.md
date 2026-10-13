# Example custom components

A folder like this one is what `yourdocs build-components` takes. Each file's default export is a component (`defineComponent`, named `scope/name`) or a layout slot (`defineSlot`). Files starting with `_` and test files are helpers.

```dockerfile
FROM yourdocs/builder:1 AS build
COPY components /components
RUN yourdocs build-components /components --out /bundles

FROM yourdocs/runtime:1
COPY --from=build /bundles /app/bundles
```

- `note.tsx`: the component `acme/note`, with a stylesheet and a title written on the canvas (`<Editable>`).
- `footer.tsx`: replaces the footer of the site.
