import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests",
  use: { baseURL: "http://127.0.0.1:3000" },
  // The binary is built by `cargo build` beforehand; Playwright only starts it, on an empty database.
  webServer: {
    command: "rm -rf .e2e-data && ../../target/debug/yourdocs dev --data-dir .e2e-data",
    url: "http://127.0.0.1:3000/docs/demo",
    reuseExistingServer: !process.env.CI,
  },
});
