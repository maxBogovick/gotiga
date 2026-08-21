import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { VitePWA } from "vite-plugin-pwa";
import fs from "node:fs";
import path from "node:path";

const isWebBuild = process.env.VITE_BUILD_TARGET === "web";

// Resolve the /admin route's own emitted files (its JS node chunk + CSS) so the service
// worker can drop them from its precache — the panel is SPA-only and behind auth, so
// precaching ~0.5 MB of it for every public visitor is pure waste (see manifestTransforms
// below). This is done WITHOUT a hardcoded hash or node index: SvelteKit keys build output
// by node index, so we find which index is admin by scanning the generated node stubs for
// the one that points at routes/admin, then read that node's real filenames out of Vite's
// build manifest. Any failure returns [] — a precache optimisation must never break a build.
function adminBuildFiles() {
  try {
    // The generated node stubs carry the route→index mapping. Prefer the optimized set
    // (present in a production build); fall back to the plain one.
    const genDir = [
      ".svelte-kit/generated/client-optimized/nodes",
      ".svelte-kit/generated/client/nodes",
    ].find((d) => fs.existsSync(d));
    if (!genDir) return [];
    const stub = fs
      .readdirSync(genDir)
      .find((f) => /routes\/admin\/\+page/.test(fs.readFileSync(path.join(genDir, f), "utf8")));
    if (!stub) return [];
    const nodeIndex = stub.replace(/\.js$/, ""); // e.g. "4"

    const manifest = JSON.parse(
      fs.readFileSync(".svelte-kit/output/client/.vite/manifest.json", "utf8"),
    );
    const entry = Object.values(manifest).find(
      (v) => v.file && v.file.includes(`nodes/${nodeIndex}.`),
    );
    if (!entry) return [];
    // The admin panels are static imports, so Rollup inlines them into this one node
    // chunk (its `imports` are shared vendor/i18n chunks the public pages need too, so
    // those are deliberately NOT excluded). File paths are relative to the client outDir.
    return [entry.file, ...(entry.css ?? [])];
  } catch {
    return [];
  }
}

// A workbox manifestTransform: strip the admin route's own files from the precache list.
/** @type {import('workbox-build').ManifestTransform} */
const adminPrecacheExclusion = (entries) => {
  const admin = adminBuildFiles();
  const manifest = admin.length
    ? entries.filter((e) => !admin.some((f) => e.url.endsWith(f)))
    : entries;
  return { manifest, warnings: [] };
};

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
      // SvelteKit's adapter-static sets Vite's own `base` to a relative "./" so
      // prerendered pages can be deployed under any subpath (see the `../` asset
      // hrefs it emits). vite-plugin-pwa inherits that same `base` by default and
      // bakes it into the register script as `new Workbox("./sw.js", { scope: "./" })`.
      // "./" is resolved by the browser relative to the CURRENT DOCUMENT URL, and
      // SvelteKit's default trailingSlash ("never") means a nested route's URL has
      // no trailing slash — e.g. /figurines/some-slug. Resolving "./sw.js" against
      // that strips the last segment (treated as a filename) and lands one level up,
      // at /figurines/sw.js — a 404, confirmed in production. The site is only ever
      // deployed at the domain root, so force an absolute base here independent of
      // SvelteKit's relative asset paths.
      base: "/",
      scope: "/",
      workbox: {
        globPatterns: ["**/*.{js,css,html,ico,png,webp,svg}"],
        maximumFileSizeToCacheInBytes: 3 * 1024 * 1024,
        // Drop the SPA-only /admin route from the precache. It is never prerendered and
        // pulls in 27 panels — a ~0.5 MB JS chunk plus ~90 KB CSS that the service worker
        // would otherwise precache for EVERY public visitor, none of whom can open the
        // panel (it is behind auth). We do NOT do this with manualChunks: forcing admin
        // into a named chunk also replaces Vite's default vendor splitting and merges the
        // shared graph into one monolith, which regressed the HOME critical path from
        // ~170 KB to ~300 KB gzip. Chunking is left exactly as Vite/SvelteKit produce it;
        // we only edit the precache MANIFEST, resolving the admin route's own files from
        // SvelteKit's build manifest so there is no hardcoded hash or node index.
        manifestTransforms: [adminPrecacheExclusion],
        navigateFallback: null,
        runtimeCaching: [
          {
            urlPattern: /\/fonts\/.*\.woff2$/i,
            handler: "CacheFirst",
            options: {
              cacheName: "self-hosted-fonts",
              expiration: { maxAgeSeconds: 60 * 60 * 24 * 365, maxEntries: 40 },
            },
          },
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
    watch: {
      // The Rust backend lives here; recompiles must not trigger an HMR reload.
      ignored: ["**/src-tauri/**"],
    },
    proxy: apiProxy,
  },
  preview: {
    proxy: apiProxy,
  },
}));
