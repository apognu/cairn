import { defineConfig } from "vite";

// Browser bundle that hydrates the server-rendered page; emptyOutDir stays off so ssr.js survives.
export default defineConfig({
  // `build-components` points the custom components module at the deployer's folder and builds elsewhere than `dist`.
  resolve: { alias: process.env.YOURDOCS_CUSTOM ? { "@yourdocs/components/custom": process.env.YOURDOCS_CUSTOM } : {} },
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
  build: {
    outDir: process.env.YOURDOCS_OUT ?? "dist",
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
