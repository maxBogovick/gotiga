// Web build: prerender to static HTML so "how to acquire" is a real page for
// crawlers, not the SPA 404 shell. The commission form stays SPA-only. Dev stays SPA.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';
