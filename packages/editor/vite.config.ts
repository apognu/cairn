import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The Rust server embeds `dist`; in development it serves the API and Vite proxies to it.
export default defineConfig({
  plugins: [react()],
  server: { proxy: { "/api": "http://127.0.0.1:3000" } },
  build: { outDir: "dist", emptyOutDir: true },
});
