import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests",
  // Publishing takes everything pending, whoever made it. Tests that read the list of pending changes run after the others finish,
  // so no other test's publish can swallow their changes in between.
  projects: [
    { name: "main", testIgnore: /(redirects|checks|custom)\.spec/ },
    // A second server running with bundles `build-components` made from `examples/components`.
    { name: "custom", testMatch: /custom\.spec/, use: { baseURL: "http://127.0.0.1:3200" } },
    { name: "pending-changes", testMatch: /redirects\.spec/, dependencies: ["main"] },
    // A page with a problem blocks every publish, so this runs alone, last, and removes the page again.
    { name: "checks", testMatch: /checks\.spec/, dependencies: ["pending-changes"] },
  ],
  use: { baseURL: "http://127.0.0.1:3000" },
  // The binary is built by `cargo build` beforehand; Playwright only starts it, on an empty database.
  webServer: [
    {
      command: "rm -rf .e2e-data && ../../target/debug/yourdocs dev --data-dir .e2e-data",
      env: { PAGEFIND_BIN: "../../node_modules/.bin/pagefind" },
      url: "http://127.0.0.1:3000/api/pages",
      reuseExistingServer: !process.env.CI,
    },
    {
      command:
        "rm -rf .e2e-bundles .e2e-custom-data && bun ../builder/src/cli.ts build-components ../../examples/components --out .e2e-bundles && ../../target/debug/yourdocs dev --data-dir .e2e-custom-data --bind 127.0.0.1:3200 --site-bind 127.0.0.1:3201",
      env: { PAGEFIND_BIN: "../../node_modules/.bin/pagefind", BUNDLES_DIR: ".e2e-bundles" },
      url: "http://127.0.0.1:3200/api/pages",
      timeout: 180_000,
      reuseExistingServer: !process.env.CI,
    },
  ],
});
