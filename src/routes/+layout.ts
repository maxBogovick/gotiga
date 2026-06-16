// Two build profiles share this codebase:
//   • web   (VITE_BUILD_TARGET=web): enable SSR so public routes can be prerendered
//     for SEO (real HTML with meta/JSON-LD). Non-prerendered routes still ship as SPA
//     via the adapter-static fallback.
//   • tauri (default / any other value): SSR off — the desktop app has no Node server
//     and gets its data over IPC at runtime, so it stays a pure SPA (current behaviour).
// Driving both the layout `ssr` and each public page's `prerender` from the same flag
// avoids relying on a child re-enabling SSR after a parent disabled it.
export const ssr = import.meta.env.VITE_BUILD_TARGET === 'web';
