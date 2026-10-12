import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests",
  use: { baseURL: "http://127.0.0.1:3000" },
  // The server is built by `cargo build` beforehand; Playwright only starts it.
  webServer: {
    command: "../../target/debug/yourdocs-server",
    url: "http://127.0.0.1:3000/docs/demo",
    reuseExistingServer: true,
  },
});
