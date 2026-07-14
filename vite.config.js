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
    // The service worker only belongs to the web build. This used to be expressed as
    // `workbox: { globPatterns: [] }` on the Tauri target — but an empty precache manifest
    // with no runtimeCaching is not a valid workbox config, so `npm run build` (the Tauri
    // profile) died in the service-worker step with "Couldn't find configuration for either
    // precaching or runtime caching". That break is on master, independent of any perf work.
    //
    // `disable` is the plugin's own switch for this and is the right tool: dropping the
    // plugin from the array instead would leave the `virtual:pwa-register` module that
    // +layout.svelte imports unresolvable, and Rollup fails on a dynamic import it cannot
    // resolve even when the call site is guarded at runtime. With `disable` the virtual
    // module still resolves (to a no-op) and no service worker is emitted.
    VitePWA({
      disable: !isWebBuild,
      registerType: "autoUpdate",
      injectRegister: null,
      manifest: false,
      devOptions: { enabled: false },
      workbox: {
        globPatterns: ["**/*.{js,css,html,ico,png,webp,svg,woff2}"],
        // (The `globIgnores: ["**/bg-main.png"]` that used to sit here is gone with the
        // file: bg-main.png was a 2.4 MB PNG that nothing referenced. The ignore existed
        // only to keep it out of the precache — deleting the asset makes the rule moot.)
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
          // Backgrounds must come BEFORE the /static/ rule below (workbox takes the
          // first matching route) and must NOT be CacheFirst. Every other file under
          // /static/ is content-addressed — a fresh uuid per upload — so caching it
          // hard is free. The background is the one exception: the handler overwrites
          // cabinet-bg.jpg in place, so the URL is stable while the bytes change.
          // CacheFirst would pin the old photo in the service worker for a week, and
          // the SW answers before the network, so nginx's `no-cache` on this path
          // would never even be consulted. StaleWhileRevalidate paints instantly from
          // cache and picks the new background up on the very next view.
          {
            urlPattern: /\/static\/backgrounds\//,
            handler: "StaleWhileRevalidate",
            options: { cacheName: "backend-background" },
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
      },
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
