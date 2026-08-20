// Web build: prerender to static HTML so the policy is crawlable (the footer
// already links here from every public page). Dev stays SPA.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';
