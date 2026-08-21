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
    // 100_000 (UTF-16 units of raw, uncompressed CSS) inlines the home page's own
    // sheet (~81 KB) into its HTML so first paint does not wait on a second CSS
    // request. The shared root-layout stylesheet (~155 KB raw) stays a separate
    // cached file — duplicating it into every prerendered route would cost more
    // than the request it saves. JS `modulepreload` is stripped in hooks.server.ts
    // so that remaining file no longer shares Slow 4G with 48 script chunks.
    // Re-check these sizes with `npm run build` before raising this number.
    inlineStyleThreshold: 100_000,
  },
};

export default config;
