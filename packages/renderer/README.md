# renderer

Shared renderNode, Page and SiteLayout; built into ssr.js for V8 and client.js for the browser.
Not obvious: V8 has no TextEncoder or MessageChannel; see src/polyfills.ts. ssr.ts must export only render.
