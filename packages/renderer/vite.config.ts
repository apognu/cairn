import { defineConfig } from "vite";

// One self-contained script for embedded V8: `SSR.render(doc)` returns HTML.
export default defineConfig({
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    minify: false,
    lib: { entry: "src/ssr.ts", name: "SSR", formats: ["iife"], fileName: () => "ssr.js" },
  },
});
