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

/** How many works the home reel shows before the archive takes over. */
export const GALLERY_LIMIT = 16;

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

/** Newest work by `createdAt`, including pieces still on the bench. */
export function pickLatestAddedWork(items: FigurineListItem[]): FigurineListItem | null {
    let best: FigurineListItem | null = null;
    let bestMs = Number.NEGATIVE_INFINITY;
    for (const item of items) {
        const ms = createdAtMs(item.createdAt);
        if (!best || ms > bestMs || (ms === bestMs && item.id.localeCompare(best.id) > 0)) {
            best = item;
            bestMs = ms;
        }
    }
    return best;
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

export type HomePlacementState = 'shown' | 'beyond' | 'hidden' | 'inProgress' | 'firstLook';

export interface HomePlacement {
    state: HomePlacementState;
    /** 1-based place among the works the home page lists; null when it is not listed. */
    position: number | null;
    /** How many works the home page lists (with this one, when it is listed). */
    total: number;
    /** `firstLookUntil` of the work, for the "opens on …" line. */
    until: string | null;
}

/** What the admin form edits — the fields that decide whether and where the home lists a work. */
export type PlacementDraft = Pick<FigurineListItem, 'id' | 'name' | 'status' | 'sortOrder'> & {
    isVisible?: boolean;
    firstLookUntil?: string | null;
    createdAt?: string | null;
};

/**
 * Where the home page would put a work, using the same rules as the page itself:
 * the server hides unseen works (hidden, or still inside their first-look window),
 * `visibleWorks` drops in-progress ones, `sortWorks` orders the rest, and the reel
 * shows the first GALLERY_LIMIT. `draft` replaces the saved copy of itself in
 * `items`, so the answer follows the form while it is being edited.
 */
export function homePlacement(
    items: FigurineListItem[],
    draft: PlacementDraft,
    now: number = Date.now(),
): HomePlacement {
    const unseenUntil = (f: { firstLookUntil?: string | null }) => {
        const ms = f.firstLookUntil ? new Date(f.firstLookUntil).getTime() : NaN;
        return Number.isFinite(ms) && ms > now ? ms : null;
    };
    const isPublic = (f: { isVisible?: boolean; firstLookUntil?: string | null }) =>
        f.isVisible !== false && unseenUntil(f) === null;

    const until = draft.firstLookUntil ?? null;
    if (draft.isVisible === false) return { state: 'hidden', position: null, total: 0, until };
    if (unseenUntil(draft) !== null) return { state: 'firstLook', position: null, total: 0, until };

    const others = visibleWorks(items.filter((f) => f.id !== draft.id && isPublic(f)));
    if (draft.status === 'in_progress') {
        return { state: 'inProgress', position: null, total: others.length, until };
    }
    const me = {
        ...draft,
        createdAt: draft.createdAt ?? new Date(now).toISOString(),
    } as FigurineListItem;
    const ordered = sortWorks([...others, me]);
    const position = ordered.findIndex((f) => f.id === draft.id) + 1;
    return {
        state: position <= GALLERY_LIMIT ? 'shown' : 'beyond',
        position,
        total: ordered.length,
        until,
    };
}
