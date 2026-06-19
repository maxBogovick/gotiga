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
  },
};

export default config;
