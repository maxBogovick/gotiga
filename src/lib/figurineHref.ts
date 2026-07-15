import type { Figurine, FigurineListItem } from '$lib/types/api';

/**
 * Build the detail-page path for a work. Prefers the transliterated slug (pretty,
 * shareable, indexable) and falls back to the UUID for works not yet re-saved —
 * the `[id]` route resolves either handle, so both always work.
 */
export function figurineHref(
	f: Pick<Figurine | FigurineListItem, 'id' | 'slug'>
): string {
	return `/figurines/${f.slug ?? f.id}`;
}
