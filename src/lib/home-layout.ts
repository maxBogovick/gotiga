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
    'hero', 'returningBand', 'gallery', 'authorStory', 'correspondence',
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
    // Same reason as safeColor's own comment: a value holding `;` would smuggle extra
    // declarations into this style attribute.
    const background = safeColor(s?.background);
    const color = safeColor(s?.color);
    if (background) parts.push(`--hl-bg:${background}`);
    if (color) parts.push(`--hl-color:${color}`);
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
    const background = safeColor(config?.pageBackground);
    return background ? `background:${background}` : '';
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
    // Every selector here must exist in the rendered markup. Four of them did not:
    // `.hero-orn`, `.hero-visual`, `.work-more-ledger` and `.context-side-row--guide` were
    // left behind by earlier redesigns, so the editor kept offering their controls and the
    // generated CSS kept targeting nothing — the admin moved a slider and the page did not
    // move. The hero's media is `.cine-frame` now.
    hero: [
        { id: 'title',   sel: '.hero-title',  kind: 'text',  orderable: true },
        { id: 'lead',    sel: '.hero-lead',   kind: 'text',  orderable: true },
        { id: 'ctas',    sel: '.hero-ctas',   kind: 'group', orderable: true },
        { id: 'proof',   sel: '.hero-proof',  kind: 'text',  orderable: true },
        { id: 'teasers', sel: '.hw-teasers',  kind: 'group', orderable: true },
        // Deliberately renamed from `visual` while its selector was being repointed from the
        // long-dead `.hero-visual` to the live `.cine-frame`. Keeping the id would have
        // ARMED every override an admin ever saved for it: the control has been generating
        // CSS that matched nothing for as long as it has existed, so anyone who toggled it
        // off (saw nothing happen) or dragged its width slider left a `hero.visual` entry
        // behind — and the moment the selector started resolving, that entry would hide or
        // shrink the hero photograph, i.e. the page's LCP element, with no admin action.
        // A new id makes those stale entries unresolvable (elementDef returns undefined and
        // generateHomeElementCSS skips them), which is the migration.
        { id: 'photo',   sel: '.cine-frame',  kind: 'media' },
    ],
    gallery: [
        { id: 'header',  sel: '.context-hd',       kind: 'group', orderable: true },
        { id: 'content', sel: '.work-content',     kind: 'group', orderable: true },
        { id: 'eyebrow', sel: '.eyebrow',          kind: 'text' },
        { id: 'title',   sel: '.context-title',    kind: 'text' },
        { id: 'desc',    sel: '.context-desc',     kind: 'text' },
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
 * A colour safe to interpolate into a CSS declaration.
 *
 * Everything in the generated stylesheet below is a literal from the saved config, and a
 * value carrying `}` closes the rule early: `red}html{display:none` would take the whole
 * page down, from a field meant to hold `#c65f3c`. The numeric knobs are already pinned by
 * `typeof === 'number'`; the colour is the one free-text value, so it gets checked here.
 * Anything the browser itself doesn't recognise as a colour is dropped.
 */
function safeColor(value: string | null | undefined): string | null {
    if (typeof value !== 'string') return null;
    const v = value.trim();
    // Parentheses stay legal — rgb()/hsl()/color-mix() need them; the rule-breaking
    // characters do not appear in any colour syntax.
    if (!v || /[;{}<>\\"']/.test(v)) return null;
    // CSS.supports is the browser's own parser — no colour syntax to keep in sync here.
    if (typeof CSS !== 'undefined' && typeof CSS.supports === 'function') {
        return CSS.supports('color', v) ? v : null;
    }
    // No DOM (SSR / prerender): fall back to the shapes the admin editor can actually emit.
    return /^(#[0-9a-f]{3,8}|[a-z]+|(rgb|rgba|hsl|hsla)\([\d\s.,%/-]+\))$/i.test(v) ? v : null;
}

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
        const color = safeColor(s.color);
        if (color) {
            out.push(`${sel},${sel} *{color:${color} !important}`);
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
