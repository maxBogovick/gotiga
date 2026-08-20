// Web build: prerender to static HTML so the Book of Impressions is crawlable.
// The form itself submits client-side; the invitation copy is what bots need. Dev stays SPA.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';
