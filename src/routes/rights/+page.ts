// Web build: prerender to static HTML so Google can fetch this URL from ImageObject
// `license` / `acquireLicensePage` without executing the SPA. Dev stays SPA.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';
