import type { FigurineListItem, HomeContent } from '$lib/types/api';

/**
 * The home page reads the WHOLE visible collection, uncapped — and that is deliberate.
 *
 * It used to ask for `?limit=30`. The server ignored the parameter (it deserializes
 * `perPage`), so the page was handed everything anyway and every consumer on it was quietly
 * written against a complete list. Honouring the cap — which is what the parameter was meant
 * to do all along — silently broke them all: the hero and the vitrine resolve the admin's
 * PINNED work by id (and the admin's picker offers the entire catalogue, so a pin on a work
 * sorting past the cap simply vanished and the hero fell back to another piece), the
 * visitor's private marks resolve their ids against this list (a mark on work #31 disappeared
 * from "Отмеченное вами"), and the notice board scans it for showings about to open.
 *
 * So the cap was never a page-size — it was a lookup table masquerading as one. The page
 * needs every work it might have to name, and it slices what it actually SHOWS from that
 * (GALLERY_LIMIT). If this collection ever grows big enough for the payload to hurt, the fix
 * is a /home endpoint that resolves the pins, the marks and the showings server-side and
 * returns only the reel — not a cap that makes the page quietly wrong.
 */

/** Works the home page shows at all: the in-progress ones live on /upcoming. */
export function visibleWorks(items: FigurineListItem[]): FigurineListItem[] {
    return items.filter((f) => f.status !== 'in_progress');
}

/**
 * The reel's order: the author's own, and nothing else.
 *
 * This used to pin the 2 newest works by createdAt and then rotate the rest by a daily
 * offset. Both overrode `sortOrder`, and the rotation did so destructively: the offset
 * is (day number % list length), so on a day when it came out to 16, the reel started at
 * the sixteenth work and everything the author had deliberately put FIRST was rotated
 * onto the tail, past the 16 the home page shows.
 */
export function sortWorks(items: FigurineListItem[]): FigurineListItem[] {
    return items.slice().sort((a, b) => {
        const order = (a.sortOrder ?? 0) - (b.sortOrder ?? 0);
        if (order !== 0) return order;
        // Same sortOrder (the admin form lets that happen freely): newest first, so a
        // fresh work at least leads its own tie instead of landing arbitrarily.
        const da = createdAtMs(a.createdAt);
        const db = createdAtMs(b.createdAt);
        if (db !== da) return db - da;
        return a.id.localeCompare(b.id);
    });
}

function createdAtMs(iso?: string | null): number {
    if (!iso) return 0;
    const t = new Date(iso).getTime();
    return Number.isFinite(t) ? t : 0;
}

/**
 * The work the hero shows. `works` must already be sorted (sortWorks).
 *
 * Deliberately DETERMINISTIC — no daily rotation, no showing-window gate. The home page
 * is prerendered and the hero is its LCP element: whatever load() resolves at BUILD time
 * is baked into the <img src>. If the runtime pick can disagree with the baked one, the
 * browser downloads a large photo, paints it, and then throws it away for a second large
 * photo — every visit, and the `fetchpriority="high"` goes to the wrong file. A pick that
 * depends on today's date or on the showing rooms disagrees BY CONSTRUCTION.
 */
export function pickHeroFigurine(works: FigurineListItem[], content: HomeContent): FigurineListItem | null {
    const byId = (id: string | null | undefined) =>
        (id ? works.find((w) => w.id === id) : undefined) ?? null;

    return byId(content.heroFigurineId)
        ?? byId(content.vitrineFigurineId)
        ?? works.find((w) => w.status === 'available')
        ?? works[0]
        ?? null;
}

/** The hero's photograph: an admin-uploaded background always overrides the work. */
export function heroImageUrl(
    background: string | null,
    hero: FigurineListItem | null,
    fallback: string,
): string {
    if (background) return background;
    return hero?.faceImageLargeUrl?.trim() || hero?.faceImageUrl?.trim() || fallback;
}
