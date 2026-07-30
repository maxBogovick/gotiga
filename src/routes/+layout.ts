// Two build profiles share this codebase:
//   • web (VITE_BUILD_TARGET=web, i.e. `npm run build:web`): enable SSR so public
//     routes can be prerendered for SEO (real HTML with meta/JSON-LD). Non-prerendered
//     routes still ship as SPA via the adapter-static fallback.
//   • anything else — notably `vite dev` — keeps SSR off and runs as a pure SPA, which
//     is what the app is written against (stores read localStorage at init).
// Driving both the layout `ssr` and each public page's `prerender` from the same flag
// avoids relying on a child re-enabling SSR after a parent disabled it.
export const ssr = import.meta.env.VITE_BUILD_TARGET === 'web';
