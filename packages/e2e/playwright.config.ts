import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests",
  // Publishing takes everything pending, whoever made it. Tests that read the list of pending changes run after the others finish,
  // so no other test's publish can swallow their changes in between.
  projects: [
    { name: "main", testIgnore: /(redirects|checks)\.spec/ },
    { name: "pending-changes", testMatch: /redirects\.spec/, dependencies: ["main"] },
    // A page with a problem blocks every publish, so this runs alone, last, and removes the page again.
    { name: "checks", testMatch: /checks\.spec/, dependencies: ["pending-changes"] },
  ],
  use: { baseURL: "http://127.0.0.1:3000" },
  // The binary is built by `cargo build` beforehand; Playwright only starts it, on an empty database.
  webServer: {
    command: "rm -rf .e2e-data && ../../target/debug/yourdocs dev --data-dir .e2e-data",
    env: { PAGEFIND_BIN: "../../node_modules/.bin/pagefind" },
    url: "http://127.0.0.1:3000/docs/demo",
    reuseExistingServer: !process.env.CI,
  },
});
