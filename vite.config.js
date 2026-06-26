import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { VitePWA } from "vite-plugin-pwa";

const host = process.env.TAURI_DEV_HOST;
const isWebBuild = process.env.VITE_BUILD_TARGET === "web";

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
  plugins: [
    sveltekit(),
    tailwindcss(),
    VitePWA({
      registerType: "autoUpdate",
      injectRegister: null,
      manifest: false,
      devOptions: { enabled: false },
      workbox: isWebBuild
        ? {
            globPatterns: ["**/*.{js,css,html,ico,png,webp,svg,woff2}"],
            globIgnores: ["**/bg-main.png"],
            maximumFileSizeToCacheInBytes: 3 * 1024 * 1024,
            navigateFallback: null,
            runtimeCaching: [
              {
                urlPattern: /^https:\/\/fonts\.googleapis\.com\/.*/i,
                handler: "StaleWhileRevalidate",
                options: { cacheName: "gfonts-stylesheets" },
              },
              {
                urlPattern: /^https:\/\/fonts\.gstatic\.com\/.*/i,
                handler: "CacheFirst",
                options: {
                  cacheName: "gfonts-webfonts",
                  expiration: { maxAgeSeconds: 60 * 60 * 24 * 365 },
                },
              },
              {
                urlPattern: /\/api\//,
                handler: "NetworkFirst",
                options: {
                  cacheName: "api-cache",
                  networkTimeoutSeconds: 8,
                  expiration: { maxAgeSeconds: 60 * 5 },
                },
              },
              {
                urlPattern: /\/static\//,
                handler: "CacheFirst",
                options: {
                  cacheName: "backend-static",
                  expiration: { maxAgeSeconds: 60 * 60 * 24 * 7 },
                },
              },
            ],
          }
        : { globPatterns: [] },
    }),
  ],

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
