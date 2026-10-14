import { copyFileSync } from "node:fs";
import { join } from "node:path";
import { defineConfig, type Plugin } from "vite";

// The API reference script is not part of `client.js`: only API pages load it. It is copied as it is, from the pinned package.
const scalar: Plugin = {
  name: "copy-scalar",
  closeBundle() {
    const from = new URL("./node_modules/@scalar/api-reference/dist/browser/standalone.js", import.meta.url);
    copyFileSync(from, join(process.env.CAIRN_OUT ?? "dist", "scalar.js"));
  },
};

// Browser bundle that hydrates the server-rendered page; emptyOutDir stays off so ssr.js survives.
export default defineConfig({
  plugins: [scalar],
  // `build-components` points the custom components module at the deployer's folder and builds elsewhere than `dist`.
  resolve: { alias: process.env.CAIRN_CUSTOM ? { "@cairn/components/custom": process.env.CAIRN_CUSTOM } : {} },
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
  build: {
    outDir: process.env.CAIRN_OUT ?? "dist",
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
