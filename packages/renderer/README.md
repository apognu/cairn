# renderer

Shared renderNode, Page and SiteLayout; built into ssr.js for V8 and client.js for the browser, plus `scalar.js` (the pinned Scalar bundle, copied as it is).
Not obvious: V8 has no TextEncoder or MessageChannel; see src/polyfills.ts. ssr.ts must export only render.
`render` takes `{ doc, context }` and returns the whole page (site layout included); Rust only wraps it in `<html>`. Styling is Tailwind: `src/tailwind.css` is the shared theme (utilities read the tokens; no palette, no preflight), `src/site.css` the tokens, document content, components and Pagefind/Scalar rules, shared with the editor.
An API page (`context.api`) renders `ApiReference`: a mount point that starts Scalar in an effect, and a hidden block of operations for the search.
