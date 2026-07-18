import type { Figurine, FigurineListItem } from '$lib/types/api';

/**
 * Build the detail-page path for a work. Prefers the transliterated slug (pretty,
 * shareable, indexable) and falls back to the UUID for works not yet re-saved —
 * the `[id]` route resolves either handle, so both always work.
 *
 * `source`, when given, tags the link with `?src=<source>` — read back by
 * `$lib/analytics`'s `basePayload` as `internalSource`, so admin analytics can
 * tell which on-site block (e.g. "home_afisha", "home_featured") sent the
 * visitor to this card. Omit it for links that aren't part of that
 * measurement (e.g. "related works" on the detail page itself).
 */
export function figurineHref(
	f: Pick<Figurine | FigurineListItem, 'id' | 'slug'>,
	source?: string
): string {
	const base = `/figurines/${f.slug ?? f.id}`;
	return source ? `${base}?src=${encodeURIComponent(source)}` : base;
}
