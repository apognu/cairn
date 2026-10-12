import { defineConfig } from "vite";

// Browser bundle that hydrates the server-rendered page; emptyOutDir stays off so ssr.js survives.
export default defineConfig({
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
  build: {
    outDir: "dist",
    emptyOutDir: false,
    // Fonts become data URLs in site.css: the publish pipeline writes that one file.
    assetsInlineLimit: 100_000,
    lib: {
      entry: "src/client.tsx",
      name: "Client",
      formats: ["iife"],
      fileName: () => "client.js",
      cssFileName: "site",
    },
  },
});
