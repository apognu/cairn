# builder

`build-components <folder> --out <dir>` (`bun src/cli.ts`): bundles the deployer's components and slots with the built-in ones into `ssr.js`, `client.js`, `site.css`, `editor/` and `manifest.json`.
Not obvious: the folder is copied into `.work/` so its imports resolve in the workspace, and `@yourdocs/components/custom` is aliased to a generated module. Every component is test-rendered through the built `ssr.js` in a context with no browser globals.
