import { assets } from '$app/paths';

function injectStylesheet(href: string, id: string) {
	if (document.getElementById(id)) return;
	const link = document.createElement('link');
	link.id = id;
	link.rel = 'stylesheet';
	link.href = href;
	document.head.appendChild(link);
}

/**
 * Self-hosted faces used to live in a render-blocking `<link>` in `app.html`.
 * That stylesheet is tiny, but it discovers seven woff2 files (~350 KB) which
 * then share the Slow 4G pipe with the LCP photograph. The stacks already
 * fall back to Georgia / system-ui, so first paint does not need the files.
 *
 * Critical (Fraunces + DM Sans + Newsreader) arrives on `load` — after the
 * hero has had the pipe. The rest (Cormorant, EB Garamond, Instrument Sans)
 * waits for idle.
 */
export function loadSiteFonts(): () => void {
	if (typeof window === 'undefined') return () => {};

	let cancelled = false;
	let restIdle = 0;
	let restTimeout = 0;

	const injectRest = () => {
		if (cancelled) return;
		injectStylesheet(`${assets}/fonts/fonts-rest.css`, 'gotiga-fonts-rest');
	};

	const onLoad = () => {
		if (cancelled) return;
		injectStylesheet(`${assets}/fonts/fonts-critical.css`, 'gotiga-fonts-critical');
		const w = window;
		if (typeof w.requestIdleCallback === 'function') {
			restIdle = w.requestIdleCallback(injectRest, { timeout: 3000 });
		} else {
			restTimeout = w.setTimeout(injectRest, 800);
		}
	};

	if (document.readyState === 'complete') onLoad();
	else window.addEventListener('load', onLoad, { once: true });

	return () => {
		cancelled = true;
		if (restIdle && 'cancelIdleCallback' in window) cancelIdleCallback(restIdle);
		if (restTimeout) clearTimeout(restTimeout);
	};
}
