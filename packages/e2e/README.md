# e2e

Playwright smoke tests against the built server.
Not obvious: run with node, not bun; install the browser with `node node_modules/.bin/playwright install`.
The `api` project starts `stand-in.ts`, a fake GitHub (latest releases, files, assets) the tests change over `/__state/…`, and a server pointed at it.
