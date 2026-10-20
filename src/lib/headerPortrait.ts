import type { HeaderPortraitEdge } from '$lib/types/api';

export const NO_EDGE: HeaderPortraitEdge = { gapLeft: 0, gapRight: 0, fadeLeft: 0, fadeRight: 0 };

/**
 * Маска, растворяющая левый и правый края картинки в то, что за ней (бумагу
 * шапки). Одна на шапку и на предпросмотр в админке: две копии разошлись бы
 * на первой правке. `null` — края не растворяются.
 */
export function edgeMask(edge: Partial<HeaderPortraitEdge> | null | undefined): string | null {
    const l = Math.max(0, Math.min(50, edge?.fadeLeft ?? 0));
    const r = Math.max(0, Math.min(50, edge?.fadeRight ?? 0));
    if (!l && !r) return null;
    return `linear-gradient(to right, transparent 0, #000 ${l}%, #000 ${100 - r}%, transparent 100%)`;
}
