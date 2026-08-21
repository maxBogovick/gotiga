/** Families that actually ship in `static/fonts/fonts.css`.
 *
 *  The visitor typeface picker is gone. Admin block styles may still store
 *  old reading-font ids (`spectral`, `caveat`, …); `fontStack()` maps those
 *  onto the nearest remaining face so saved layouts do not go inherit. */
export interface SiteFont {
	id: string;
	name: string;
	stack: string;
}

export const SITE_FONTS: SiteFont[] = [
	{ id: 'fraunces', name: 'Fraunces', stack: "'Fraunces', Georgia, serif" },
	{ id: 'newsreader', name: 'Newsreader', stack: "'Newsreader', Georgia, serif" },
	{ id: 'cormorant', name: 'Cormorant', stack: "'Cormorant Garamond', Georgia, serif" },
	{ id: 'garamond', name: 'Garamond', stack: "'EB Garamond', Georgia, serif" },
	{ id: 'instrument', name: 'Instrument Sans', stack: "'Instrument Sans', system-ui, sans-serif" },
	{ id: 'dm-sans', name: 'DM Sans', stack: "'DM Sans', system-ui, sans-serif" },
];

const STACK_BY_ID: Record<string, string> = {
	...Object.fromEntries(SITE_FONTS.map((f) => [f.id, f.stack])),
	spectral: "'EB Garamond', Georgia, serif",
	lora: "'EB Garamond', Georgia, serif",
	yeseva: "'Fraunces', Georgia, serif",
	caveat: "'Cormorant Garamond', Georgia, serif",
};

export function fontStack(id: string | undefined | null): string {
	if (!id) return 'inherit';
	return STACK_BY_ID[id] ?? 'inherit';
}
