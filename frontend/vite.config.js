import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// During `npm run dev`, proxy API calls to the Axum backend so the frontend
// and backend can run on separate ports. In production the Rust binary serves
// the built `dist/` directly, so no proxy is involved.
export default defineConfig({
  plugins: [svelte()],
  server: {
    proxy: {
      "/api": "http://localhost:8080",
    },
  },
});
