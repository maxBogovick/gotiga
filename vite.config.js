import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

const apiProxy = {
  "/api": {
    target: process.env.VITE_API_URL || "http://localhost:3000",
    changeOrigin: true,
  },
  "/static": {
    target: process.env.VITE_API_URL || "http://localhost:3000",
    changeOrigin: true,
  },
};

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit(), tailwindcss()],

  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
    proxy: apiProxy,
  },
  preview: {
    proxy: apiProxy,
  },
}));
