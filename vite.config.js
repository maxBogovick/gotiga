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
    // The service worker only belongs to the web build. `disable` is the plugin's own switch
    // for this: dropping the plugin from the array instead would leave the
    // `virtual:pwa-register` module that the layout imports unresolvable, and Rollup fails on
    // a dynamic import it cannot resolve even when the call site is guarded at runtime. With
    // `disable` the virtual module still resolves (to a no-op) and no service worker is emitted.
    //
    // The worker is our own file (src/pwa/sw.js, `injectManifest`): offline page, push and an
    // update that waits for the visitor all need code that the generated one cannot hold.
    // It is NOT named src/service-worker.* — SvelteKit would pick that name up and build it
    // a second time.
    VitePWA({
      disable: !isWebBuild,
      strategies: "injectManifest",
      srcDir: "src/pwa",
      filename: "sw.js",
      // "prompt": a new version waits until the visitor agrees (the page asks, see
      // src/lib/stores/pwa.svelte.ts) instead of reloading a tab with an unsent letter.
      registerType: "prompt",
      injectRegister: null,
      manifest: false,
      devOptions: { enabled: false },
      // SvelteKit's adapter-static sets Vite's own `base` to a relative "./" so
      // prerendered pages can be deployed under any subpath. vite-plugin-pwa inherits that
      // same `base` and bakes it into the register script as `new Workbox("./sw.js")`,
      // which the browser resolves against the CURRENT document URL: on a nested route
      // (/figurines/some-slug, no trailing slash) that lands on /figurines/sw.js — a 404,
      // confirmed in production. The site is only ever deployed at the domain root, so
      // force an absolute base.
      base: "/",
      scope: "/",
      injectManifest: {
        rollupFormat: "iife",
        globPatterns: ["**/*.{js,css,html,ico,png,webp,svg}"],
        // Splash screens (16 files, 3 MB) are fetched by iOS itself at launch, and the
        // screenshots only matter to the install dialog: neither belongs in every visitor's
        // precache. The game's artwork (static/battles, ~17 MB) and the tale illustrations
        // (static/tales, ~2 MB) used to be precached too — every first-time visitor
        // downloaded ~24 MB in the background for pages most of them never open. They are
        // cached as they are looked at instead (src/pwa/sw.js, "site-art").
        globIgnores: ["pwa/splash/**", "pwa/screenshots/**", "battles/**", "tales/**"],
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
