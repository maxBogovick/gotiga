// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      fallback: "app.html",
    }),
    prerender: {
      // The public production origin baked into prerendered HTML wherever
      // `url.origin`/`url.href` appears (og:url, canonical, absolute og:image).
      // Without this it defaults to the `http://sveltekit-prerender` placeholder,
      // which leaks into share previews and breaks them. Override per-deploy via
      // VITE_PUBLIC_ORIGIN; defaults to the live site.
      origin: process.env.VITE_PUBLIC_ORIGIN || 'https://ritunia.com',
      // Only routes that explicitly opt in (figurines archive + details) are
      // prerendered. Crawling is off so the prerenderer never wanders into SPA-only
      // routes (admin, profile, the /admin link in the header, etc.).
      crawl: false,
      // An empty catalog (entries() → []) must not fail the build. A genuinely
      // unreachable API still fails loudly because entries() itself throws.
      handleUnseenRoutes: 'ignore',
    },
    // The home route (and every prerendered route) shipped 7-8 separate small CSS
    // files as render-blocking <link> tags — a page's own node CSS plus every
    // non-trivially-sized component chunk (SealedDoor, HomeFigurineTile, page-turn,
    // ContactMessageForm, AppImage...), each a full round trip the browser must
    // finish before first paint. On mobile latency that waterfall is the single
    // biggest lever on FCP/LCP (measured: ~400-650ms of render-blocking time on a
    // FAST desktop connection per Lighthouse; mobile RTT makes it worse).
    //
    // 35_000 (UTF-16 units of raw, uncompressed CSS) is chosen to sit strictly
    // between HomeFigurineTile.css (~12.5 KB) and NoticedByGuests.css (~43 KB) as
    // measured on the current build. Below the line: small chunks get inlined into
    // the page's own HTML — a few KB of duplication per route is cheap. Above the
    // line: NoticedByGuests.css and the shared root-layout stylesheet (~149 KB
    // raw) stay as separate cached files on purpose — they are large enough, and
    // reused across enough routes (home, /figurines, every /hall/[id]),
    // that duplicating them into every page's HTML would cost more than the
    // request they'd save. Re-check these sizes with `npm run build` before
    // raising this number — the split is a snapshot of today's component sizes,
    // not a law.
    inlineStyleThreshold: 35_000,
  },
};

export default config;
