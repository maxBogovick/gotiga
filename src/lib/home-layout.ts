import type {
    HomeLayoutConfig,
    HomeMainBlockId,
    HomeBandBlockId,
    HomeShelfBlockId,
    HomeBlockId,
    HomeBlockSize,
    HomeElementStyle,
    BlockStyle,
} from '$lib/types/api';

// Shared background palette (same swatches as the figurine DisplayConfig editor).
export const HOME_BG_PRESETS: { id: string; color: string }[] = [
    { id: 'parchment', color: '#f8f1e7' },
    { id: 'aged',      color: '#ede3cf' },
    { id: 'linen',     color: '#f4efe8' },
    { id: 'dark',      color: '#181210' },
    { id: 'slate',     color: '#dce0e4' },
];
import { READING_FONTS } from '$lib/stores/reading-font.svelte';

// Default order of the home page main flow. `returningBand` and `latelyShelves`
// are compound blocks: they travel as one unit (keeping their isReturningVisitor
// gate) and order their own children via bandOrder / shelfOrder.
export const HOME_MAIN_BLOCK_IDS: readonly HomeMainBlockId[] = [
    'hero', 'returningBand', 'gallery', 'authorStory',
    'impressions', 'requestSteps', 'visitorBook', 'latelyShelves',
] as const;
export const HOME_BAND_BLOCK_IDS: readonly HomeBandBlockId[] = ['visitLedger', 'noticeBoard'] as const;
export const HOME_SHELF_BLOCK_IDS: readonly HomeShelfBlockId[] = ['firstLook', 'markedByYou', 'noticedByGuests'] as const;

const FONT_STACK: Record<string, string> = Object.fromEntries(
    READING_FONTS.map(f => [f.id, f.stack])
);

const FONT_SIZES: Record<NonNullable<BlockStyle['fontSize']>, string> = {
    sm:   '0.875rem',
    base: '1rem',
    lg:   '1.125rem',
    xl:   '1.25rem',
};

/**
 * Reconcile a saved order with the canonical block list: drop unknown IDs
 * (removed blocks), append missing ones (blocks added after the config was
 * saved) at their default position relative to the tail.
 */
export function normalizeHomeOrder<T extends HomeBlockId>(
    saved: T[] | undefined,
    all: readonly T[],
): T[] {
    if (!saved?.length) return [...all];
    const known = saved.filter((id) => all.includes(id));
    const missing = all.filter((id) => !known.includes(id));
    return [...known, ...missing];
}

export function isHomeBlockVisible(config: HomeLayoutConfig | null, blockId: HomeBlockId): boolean {
    return !(config?.hiddenBlocks ?? []).includes(blockId);
}

export function homeBlockSize(config: HomeLayoutConfig | null, blockId: HomeBlockId): HomeBlockSize {
    return config?.sizes?.[blockId] ?? 'contained';
}

/**
 * Inline style for a `.hl-block` wrapper: flex `order` plus CSS custom
 * properties consumed by the wrapper rules in +page.svelte (background is
 * painted on the wrapper itself; text overrides cascade to prose via the
 * targeted `.hl-has-*` rules — same pattern as the figurine-detail dc-blocks).
 */
export function homeBlockWrapperStyle(
    config: HomeLayoutConfig | null,
    blockId: HomeBlockId,
    order: number,
): string {
    const parts: string[] = [`order:${order}`];
    const s = config?.blockStyles?.[blockId];
    if (s?.background) parts.push(`--hl-bg:${s.background}`);
    if (s?.color) parts.push(`--hl-color:${s.color}`);
    if (s?.font) parts.push(`--hl-font:${FONT_STACK[s.font] ?? 'inherit'}`);
    if (s?.fontSize && s.fontSize !== 'base') parts.push(`--hl-size:${FONT_SIZES[s.fontSize]}`);
    return parts.join(';');
}

/** Class flags telling the wrapper CSS which overrides are active. */
export function homeBlockClasses(config: HomeLayoutConfig | null, blockId: HomeBlockId): string {
    const s = config?.blockStyles?.[blockId];
    const classes = ['hl-block', `hl-size-${homeBlockSize(config, blockId)}`];
    if (s?.background) classes.push('hl-has-bg');
    if (s?.color) classes.push('hl-has-color');
    if (s?.font) classes.push('hl-has-font');
    if (s?.fontSize && s.fontSize !== 'base') classes.push('hl-has-size');
    if (s?.paddingY && s.paddingY !== 'base') classes.push(`hl-pad-${s.paddingY}`);
    if (s?.divider) classes.push('hl-has-divider');
    for (const device of s?.hideOn ?? []) classes.push(`hl-hide-${device}`);
    return classes.join(' ');
}

/** Inline style for the page container: the whole-page background override. */
export function homePageStyle(config: HomeLayoutConfig | null): string {
    return config?.pageBackground ? `background:${config.pageBackground}` : '';
}

// ── Element registry ─────────────────────────────────────────────────────────
// Fine-grained elements inside each block, addressed by CSS selector under the
// block's `[data-hl]` wrapper — components stay untouched. `orderable` elements
// are direct children of the block's column box and can be rearranged.

export type HomeElementKind = 'text' | 'media' | 'group';

export interface HomeElementDef {
    id: string;
    /** Selector relative to the block wrapper. */
    sel: string;
    kind: HomeElementKind;
    orderable?: boolean;
}

/** The column container that turns into flex-column when elements are reordered. */
export const HOME_BLOCK_BOXES: Partial<Record<HomeBlockId, string>> = {
    hero: '.hero-text',
    gallery: '.context-section',
    authorStory: '.as-copy',
};

export const HOME_BLOCK_ELEMENTS: Partial<Record<HomeBlockId, HomeElementDef[]>> = {
    hero: [
        { id: 'orn',     sel: '.hero-orn',    kind: 'media', orderable: true },
        { id: 'title',   sel: '.hero-title',  kind: 'text',  orderable: true },
        { id: 'lead',    sel: '.hero-lead',   kind: 'text',  orderable: true },
        { id: 'ctas',    sel: '.hero-ctas',   kind: 'group', orderable: true },
        { id: 'proof',   sel: '.hero-proof',  kind: 'text',  orderable: true },
        { id: 'teasers', sel: '.hw-teasers',  kind: 'group', orderable: true },
        { id: 'visual',  sel: '.hero-visual', kind: 'media' },
    ],
    gallery: [
        { id: 'header',  sel: '.context-hd',       kind: 'group', orderable: true },
        { id: 'content', sel: '.work-content',     kind: 'group', orderable: true },
        { id: 'more',    sel: '.work-more-ledger', kind: 'group', orderable: true },
        { id: 'eyebrow', sel: '.eyebrow',          kind: 'text' },
        { id: 'title',   sel: '.context-title',    kind: 'text' },
        { id: 'desc',    sel: '.context-desc',     kind: 'text' },
        { id: 'guideRow', sel: '.context-side-row--guide', kind: 'group' },
    ],
    authorStory: [
        { id: 'eyebrow',  sel: '.eyebrow',     kind: 'text',  orderable: true },
        { id: 'title',    sel: '.as-title',    kind: 'text',  orderable: true },
        { id: 'name',     sel: '.as-name',     kind: 'text',  orderable: true },
        { id: 'body',     sel: '.as-body',     kind: 'text',  orderable: true },
        { id: 'actions',  sel: '.as-actions',  kind: 'group', orderable: true },
        { id: 'portrait', sel: '.as-portrait', kind: 'media' },
    ],
    impressions: [
        { id: 'eyebrow', sel: '.eyebrow',     kind: 'text' },
        { id: 'title',   sel: '.strip-title', kind: 'text' },
        { id: 'quotes',  sel: '.quote-list',  kind: 'group' },
    ],
    requestSteps: [
        { id: 'eyebrow', sel: '.eyebrow',        kind: 'text' },
        { id: 'title',   sel: '.request-title',  kind: 'text' },
        { id: 'steps',   sel: '.request-steps',  kind: 'group' },
    ],
    visitorBook: [
        { id: 'eyebrow', sel: '.eyebrow',    kind: 'text' },
        { id: 'title',   sel: '.book-title', kind: 'text' },
        { id: 'lead',    sel: '.book-lead',  kind: 'text' },
        { id: 'form',    sel: '.sign-form',  kind: 'group' },
    ],
    visitLedger: [
        { id: 'eyebrow', sel: '.ledger-eyebrow', kind: 'text' },
        { id: 'marks',   sel: '.ledger-marks',   kind: 'group' },
        { id: 'vitrine', sel: '.mark-vitrine',   kind: 'group' },
    ],
    noticeBoard: [
        { id: 'eyebrow', sel: '.eyebrow',    kind: 'text' },
        { id: 'title',   sel: '.wall-title', kind: 'text' },
    ],
    firstLook: [
        { id: 'eyebrow', sel: '.eyebrow',  kind: 'text' },
        { id: 'title',   sel: '.fl-title', kind: 'text' },
        { id: 'lead',    sel: '.fl-lead',  kind: 'text' },
        { id: 'grid',    sel: '.fl-grid',  kind: 'group' },
    ],
    markedByYou: [
        { id: 'eyebrow', sel: '.eyebrow',   kind: 'text' },
        { id: 'title',   sel: '.mby-title', kind: 'text' },
        { id: 'lead',    sel: '.mby-lead',  kind: 'text' },
        { id: 'grid',    sel: '.mby-grid',  kind: 'group' },
    ],
    noticedByGuests: [
        { id: 'eyebrow', sel: '.eyebrow',   kind: 'text' },
        { id: 'title',   sel: '.nbg-title', kind: 'text' },
        { id: 'lead',    sel: '.nbg-lead',  kind: 'text' },
        { id: 'grid',    sel: '.nbg-grid',  kind: 'group' },
    ],
};

function elementDef(blockId: string, elementId: string): HomeElementDef | undefined {
    return HOME_BLOCK_ELEMENTS[blockId as HomeBlockId]?.find((d) => d.id === elementId);
}

/** Wide, free ranges for the editor's sliders. */
export const ELEMENT_FONT_RANGE = { min: 8, max: 120 } as const;
export const ELEMENT_WIDTH_RANGE = { min: 15, max: 100 } as const;

/**
 * Global CSS applying element-level overrides (colour / free-range size /
 * visibility / in-block order). Injected on the home page as a <style> tag —
 * `!important` beats the components' scoped styles, so they stay untouched.
 */
export function generateHomeElementCSS(config: HomeLayoutConfig | null): string {
    if (!config) return '';
    const out: string[] = [];

    for (const [key, s] of Object.entries(config.elements ?? {})) {
        const dot = key.indexOf('.');
        if (dot < 0) continue;
        const blockId = key.slice(0, dot);
        const def = elementDef(blockId, key.slice(dot + 1));
        if (!def || !s) continue;
        const sel = `[data-hl="${blockId}"] ${def.sel}`;

        if (s.hidden) {
            out.push(`${sel}{display:none !important}`);
            continue;
        }
        if (s.color) {
            out.push(`${sel},${sel} *{color:${s.color} !important}`);
        }
        if (typeof s.sizePx === 'number' && s.sizePx > 0 && def.kind !== 'media') {
            // Groups force the size into descendants (their children set their
            // own px sizes); plain text elements let spans inherit naturally.
            out.push(def.kind === 'group'
                ? `${sel},${sel} *{font-size:${s.sizePx}px !important}`
                : `${sel}{font-size:${s.sizePx}px !important}`);
        }
        if (typeof s.widthPct === 'number' && s.widthPct > 0 && def.kind === 'media') {
            out.push(`${sel}{width:${s.widthPct}% !important;max-width:${s.widthPct}% !important;margin-inline:auto}`);
        }
    }

    for (const [blockId, order] of Object.entries(config.elementOrder ?? {})) {
        const box = HOME_BLOCK_BOXES[blockId as HomeBlockId];
        if (!box || !order?.length) continue;
        out.push(`[data-hl="${blockId}"] ${box}{display:flex;flex-direction:column}`);
        order.forEach((elementId, i) => {
            const def = elementDef(blockId, elementId);
            if (def?.orderable) out.push(`[data-hl="${blockId}"] ${def.sel}{order:${i}}`);
        });
    }

    return out.join('\n');
}

/** True when the config carries no overrides at all (nothing to apply). */
export function isEmptyHomeLayout(config: HomeLayoutConfig | null): boolean {
    if (!config) return true;
    return !config.blockOrder?.length
        && !config.bandOrder?.length
        && !config.shelfOrder?.length
        && !config.hiddenBlocks?.length
        && !config.pageBackground
        && !Object.keys(config.sizes ?? {}).length
        && !Object.keys(config.blockStyles ?? {}).length
        && !Object.keys(config.elements ?? {}).length
        && !Object.keys(config.elementOrder ?? {}).length;
}
