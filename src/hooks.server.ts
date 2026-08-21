import type { Handle } from '@sveltejs/kit';

/**
 * Prerendered pages paint from HTML+CSS. The 48 `modulepreload` links SvelteKit
 * would otherwise put in `<head>` start fetching JS on the same Slow 4G pipe as
 * the render-blocking stylesheets and the LCP photograph — that was most of the
 * 1.9 s CSS block and the 3.3 s hero download in lab. CSS still preloads;
 * the module graph loads from the single `type="module"` entry after parse.
 */
export const handle: Handle = async ({ event, resolve }) => {
	return resolve(event, {
		preload: ({ type }) => type === 'css' || type === 'font',
	});
};
