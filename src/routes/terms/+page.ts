// Web build: prerender to static HTML so the terms are crawlable (the footer
// links here from every public page). Dev stays SPA.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';
