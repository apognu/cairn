# components

Built-in React components.
Not obvious: components render purely from props so they match in V8 and the browser.
Icons are Lucide (`lucide-static`), drawn from a registry: `ssr.js` and the editor register the whole set, a published page only the icons `UsedIcons` listed in it, so the browser bundle carries none.
