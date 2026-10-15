# Example custom components

A folder like this one is what `cairn build-components` takes. Each file's default export is a component (`defineComponent`, named `scope/name`) or a layout slot (`defineSlot`). Files starting with `_` and test files are helpers.

```dockerfile
FROM cairn-builder AS build
COPY components /components
RUN cairn build-components /components --out /bundles

FROM ghcr.io/apognu/cairn:latest
COPY --from=build /bundles /app/bundles
```

- `note.tsx`: the component `acme/note`, with a stylesheet and a title written on the canvas (`<Editable>`).
- `tip.tsx`: the component `acme/tip`, styled with Tailwind utilities alone.

Components style themselves with the theme, so they follow the site's accent and dark mode: a stylesheet uses its custom properties
(`var(--accent)`, `var(--bg-side)`, …), and markup can use the Tailwind utilities made from them (`text-accent`, `bg-side`,
`border-border`…) with Tailwind's spacing, sizes and radii. Tailwind's color palette is left out, and a fixed color, inline or as
`text-[#00f]`, fails the build. The site's breakpoints are `phone:`, `tablet:` and `compact:` (600, 900 and 1360px and narrower).
- `footer.tsx`: replaces the footer of the site.
