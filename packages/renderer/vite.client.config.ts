import { defineConfig } from "vite";

// Browser bundle that hydrates the server-rendered page; emptyOutDir stays off so ssr.js survives.
export default defineConfig({
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
  build: {
    outDir: "dist",
    emptyOutDir: false,
    lib: { entry: "src/client.tsx", name: "Client", formats: ["iife"], fileName: () => "client.js" },
  },
});
