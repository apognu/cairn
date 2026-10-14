import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests",
  // Publishing takes everything pending, whoever made it. Tests that read the list of pending changes run after the others finish,
  // so no other test's publish can swallow their changes in between.
  projects: [
    { name: "main", testIgnore: /(redirects|checks|custom|api-reference|serve)\.spec/ },
    // A third server whose GitHub is `stand-in.ts`, so API pages read specs the tests control.
    { name: "api", testMatch: /api-reference\.spec/, use: { baseURL: "http://127.0.0.1:3300" } },
    // A second server running with bundles `build-components` made from `examples/components`.
    { name: "custom", testMatch: /custom\.spec/, use: { baseURL: "http://127.0.0.1:3200" } },
    // `serve` with a login, against a local Dex on :5556 (`examples/dex`).
    { name: "serve", testMatch: /serve\.spec/, use: { baseURL: "http://127.0.0.1:3400" } },
    { name: "pending-changes", testMatch: /redirects\.spec/, dependencies: ["main"] },
    // A page with a problem blocks every publish, so this runs alone, last, and removes the page again.
    { name: "checks", testMatch: /checks\.spec/, dependencies: ["pending-changes"] },
  ],
  use: { baseURL: "http://127.0.0.1:3000" },
  // The binary is built by `cargo build` beforehand; Playwright only starts it, on an empty database.
  webServer: [
    {
      command: "bun stand-in.ts",
      url: "http://127.0.0.1:3390/__state/reset",
      reuseExistingServer: !process.env.CI,
    },
    {
      command:
        "rm -rf .e2e-api-data && ../../target/debug/cairn dev --data-dir .e2e-api-data --bind 127.0.0.1:3300 --site-bind 127.0.0.1:3301",
      env: {
        PAGEFIND_BIN: "../../node_modules/.bin/pagefind",
        GITHUB_BASE_URL: "http://127.0.0.1:3390",
        RAW_BASE_URL: "http://127.0.0.1:3390",
      },
      url: "http://127.0.0.1:3300/api/pages",
      reuseExistingServer: !process.env.CI,
    },
    {
      command: "rm -rf .e2e-serve-data && ../../target/debug/cairn serve",
      env: {
        PAGEFIND_BIN: "../../node_modules/.bin/pagefind",
        PUBLIC_URL: "http://127.0.0.1:3400",
        BIND: "127.0.0.1:3400",
        DATA_DIR: ".e2e-serve-data",
        OIDC_ISSUER: "http://127.0.0.1:5556/dex",
        OIDC_CLIENT_ID: "cairn",
        OIDC_CLIENT_SECRET: "dev-secret",
        OIDC_ALLOW_ANY_USER: "true",
      },
      url: "http://127.0.0.1:3400/_editor/",
      reuseExistingServer: !process.env.CI,
    },
    {
      command: "rm -rf .e2e-data && ../../target/debug/cairn dev --data-dir .e2e-data",
      env: { PAGEFIND_BIN: "../../node_modules/.bin/pagefind" },
      url: "http://127.0.0.1:3000/api/pages",
      reuseExistingServer: !process.env.CI,
    },
    {
      command:
        "rm -rf .e2e-bundles .e2e-custom-data && bun ../builder/src/cli.ts build-components ../../examples/components --out .e2e-bundles && ../../target/debug/cairn dev --data-dir .e2e-custom-data --bind 127.0.0.1:3200 --site-bind 127.0.0.1:3201",
      env: { PAGEFIND_BIN: "../../node_modules/.bin/pagefind", BUNDLES_DIR: ".e2e-bundles" },
      url: "http://127.0.0.1:3200/api/pages",
      timeout: 180_000,
      reuseExistingServer: !process.env.CI,
    },
  ],
});
