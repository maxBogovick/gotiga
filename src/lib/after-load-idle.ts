/**
 * Run `fn` after `window` `load`, then on idle (or a short timeout).
 *
 * First paint and LCP on a Slow 4G pipe lose to anything that starts fetching
 * during hydration: fonts, bird frames, workshop reels, below-fold islands.
 * Waiting for `load` lets the hero photograph finish; idle then keeps the work
 * off a busy main thread.
 */
export function afterLoadIdle(fn: () => void, timeout = 2500): () => void {
	if (typeof window === 'undefined') return () => {};

	let cancelled = false;
	let idleId = 0;
	let timeoutId = 0;

	const run = () => {
		if (cancelled) return;
		const w = window;
		if (typeof w.requestIdleCallback === 'function') {
			idleId = w.requestIdleCallback(
				() => {
					if (!cancelled) fn();
				},
				{ timeout },
			);
		} else {
			timeoutId = w.setTimeout(fn, Math.min(timeout, 400));
		}
	};

	if (document.readyState === 'complete') run();
	else window.addEventListener('load', run, { once: true });

	return () => {
		cancelled = true;
		if (idleId && 'cancelIdleCallback' in window) cancelIdleCallback(idleId);
		if (timeoutId) clearTimeout(timeoutId);
	};
}
