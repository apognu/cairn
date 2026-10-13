import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The Rust server embeds `dist`; in development it serves the API and Vite proxies to it.
export default defineConfig({
  plugins: [react()],
  // `build-components` points the custom components module at the deployer's folder and builds elsewhere than `dist`.
  resolve: { alias: process.env.YOURDOCS_CUSTOM ? { "@yourdocs/components/custom": process.env.YOURDOCS_CUSTOM } : {} },
  server: { proxy: { "/api": "http://127.0.0.1:3000" } },
  build: { outDir: process.env.YOURDOCS_OUT ?? "dist", emptyOutDir: true },
});
