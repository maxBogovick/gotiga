import { writable, derived } from 'svelte/store';
import type { CardStyle, CardTarget, GradientStop, ReelTheme } from '$lib/types/api';
import type { TranslationKey } from '$lib/i18n/en';

/**
 * The home reel's appearance.
 *
 * THE DEFAULTS BELOW ARE THE ONLY SOURCE OF TRUTH for how the reel looks. The
 * server stores an all-optional patch on top of them, so a fresh install (empty
 * config) renders exactly the designed page, and any field the admin never
 * touches keeps tracking the design rather than freezing at whatever value
 * happened to be current when they first hit Save.
 */
/** Every field present — the shape the components actually render from. */
export type ResolvedCardStyle = Required<CardStyle>;
export type ResolvedReelTheme = Omit<Required<ReelTheme>, 'hero' | 'work'> & {
    hero: ResolvedCardStyle;
    work: ResolvedCardStyle;
};

/** The opening pane: one big statement, generous type, two decisive buttons. */
export const HERO_DEFAULTS: ResolvedCardStyle = {
    fillKind: 'solid',
    fillType: 'linear',
    fillAngle: 160,
    fillStops: [
        { color: '#faf6ee', position: 0, opacity: 0.2 },
        { color: '#faf6ee', position: 100, opacity: 0.06 },
    ],
    glassTint: '#faf6ee',
    glassOpacity: 0.14,
    glassBlur: 26,
    glassSaturation: 1.25,
    glassRadius: 36,
    glassSheen: 1,
    glassShadow: 0.45,
    shadowColor: '#140b07',

    edgeColor: '#faf6ee',
    edgeHoverColor: '#faf6ee',
    edgeOpacity: 0.28,

    titleColor: '#faf6ee',
    titleSize: 3.1,
    bodyColor: '#faf6ee',
    bodySize: 0.9,
    metaColor: '#faf6ee',
    metaSize: 0.7,

    btnFill: '#faf6ee',
    btnText: '#2c1710',
    btnRadius: 18,
    btnSize: 0.66,
    btnBorder: '#faf6ee',
};

/**
 * The work cards on the home page. These live on the site's own parchment, not
 * on a dark room, so they are PAPER, not glass: opaque warm white, ink type, no
 * frosting (there is nothing behind them to blur). Any dark theme applied from
 * the panel overrides this — and the panel warns when the type stops reading.
 */
export const WORK_DEFAULTS: ResolvedCardStyle = {
    fillKind: 'solid',
    fillType: 'linear',
    fillAngle: 160,
    // Unused until the admin switches the fill to 'gradient' — but a sane starting
    // gradient means that switch shows something designed, not a black rectangle.
    fillStops: [
        { color: '#fdfaf5', position: 0, opacity: 1 },
        { color: '#f2e6d4', position: 100, opacity: 1 },
    ],
    glassTint: '#fdfaf5',
    glassOpacity: 1,
    glassBlur: 0,
    glassSaturation: 1,
    glassRadius: 18,
    glassSheen: 0,
    glassShadow: 0.5,
    shadowColor: '#3c190a',

    edgeColor: '#a0744a',
    edgeHoverColor: '#c0582c',
    edgeOpacity: 0.24,

    titleColor: '#2c1710',
    titleSize: 2.2,
    bodyColor: '#5a3420',
    bodySize: 0.92,
    metaColor: '#a0745a',
    metaSize: 0.82,

    btnFill: '#2c1710',
    btnText: '#faf6ee',
    btnRadius: 4,
    btnSize: 0.68,
    btnBorder: '#2c1710',
};

export const REEL_DEFAULTS: ResolvedReelTheme = {
    hero: HERO_DEFAULTS,
    work: WORK_DEFAULTS,

    // Backdrop
    backdropKind: 'image',
    backgroundImage: '/images/main_2.jpg',
    backgroundImageMobile: '',
    backgroundFit: 'cover',
    // The figures' heads are the one thing that must survive every window shape,
    // so the crop is taken off the bottom.
    backgroundPosition: 'center top',
    backgroundBlur: 3,
    backgroundBrightness: 0.62,
    backgroundSaturation: 0.72,
    backdropColor: '#140b07',
    shadowColor: '#140b07',

    // Overlay
    overlayKind: 'gradient',
    overlayColor: '#140b07',
    overlayOpacity: 0.55,
    gradientType: 'linear',
    gradientAngle: 180,
    gradientStops: [
        { color: '#140b07', position: 0, opacity: 0.55 },
        { color: '#140b07', position: 22, opacity: 0.05 },
        { color: '#140b07', position: 72, opacity: 0.05 },
        { color: '#140b07', position: 100, opacity: 0.55 },
    ],
    vignette: 0.45,
    grain: 0.14,

    performanceMode: false,
    textTone: 'light',

    // Density
    cardGap: 2.25,
    cardWidth: 64,

    // Legacy — never written any more; see resolveReelTheme.
    glassTint: '#faf6ee',
    glassOpacity: 0.14,
    glassBlur: 26,
    glassSaturation: 1.25,
    glassRadius: 36,
    glassSheen: 1,
    glassShadow: 0.45,
    buttonFill: '#faf6ee',
    buttonText: '#2c1710',
};

/** Keys the first version of the panel wrote at the top level of the theme. */
const LEGACY_CARD_KEYS = [
    'glassTint', 'glassOpacity', 'glassBlur', 'glassSaturation',
    'glassRadius', 'glassSheen', 'glassShadow', 'shadowColor',
] as const;

function mergeCard(base: ResolvedCardStyle, patch: CardStyle | undefined): ResolvedCardStyle {
    const out = { ...base };
    for (const [key, value] of Object.entries(patch ?? {})) {
        if (value === null || value === undefined || value === '') continue;
        (out as Record<string, unknown>)[key] = value;
    }
    return out;
}

/** Fill every gap in the stored patch from the defaults. */
export function resolveReelTheme(config: ReelTheme | null | undefined): ResolvedReelTheme {
    const c = config ?? {};
    const out = { ...REEL_DEFAULTS } as ResolvedReelTheme;

    for (const [key, value] of Object.entries(c)) {
        if (key === 'hero' || key === 'work') continue;
        // An empty string is "unset" for the image/colour fields, not a valid value —
        // clearing a field in the admin form must fall back, not blank the page out.
        if (value === null || value === undefined || value === '') continue;
        (out as Record<string, unknown>)[key] = value;
    }

    // A theme saved before the panes were separable carries its glass settings
    // flat at the top level. Fold those into BOTH cards first, so an existing
    // saved look keeps rendering exactly as it did, then let any explicit
    // per-card settings win over them.
    const legacy: CardStyle = {};
    for (const key of LEGACY_CARD_KEYS) {
        const v = c[key];
        if (v !== null && v !== undefined && v !== '') (legacy as Record<string, unknown>)[key] = v;
    }
    if (c.buttonFill) legacy.btnFill = c.buttonFill;
    if (c.buttonText) legacy.btnText = c.buttonText;

    out.hero = mergeCard(mergeCard(HERO_DEFAULTS, legacy), c.hero);
    out.work = mergeCard(mergeCard(WORK_DEFAULTS, legacy), c.work);

    return out;
}

/** #rrggbb + alpha → rgba(). Anything unparseable degrades to the raw string. */
function withAlpha(hex: string, alpha: number): string {
    const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
    if (!m) return hex;
    const n = parseInt(m[1], 16);
    const r = (n >> 16) & 255;
    const g = (n >> 8) & 255;
    const b = n & 255;
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/** #rrggbb → "r g b", for use inside rgb(var(--x) / a). */
function toTriplet(hex: string): string {
    const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
    if (!m) return '20 11 7';
    const n = parseInt(m[1], 16);
    return `${(n >> 16) & 255} ${(n >> 8) & 255} ${n & 255}`;
}

/** Perceived luminance, 0..1 — used to catch light-type-on-light-backdrop. */
export function luminance(hex: string): number {
    const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
    if (!m) return 0;
    const n = parseInt(m[1], 16);
    const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => v / 255);
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** The admin's gradient, as one CSS gradient function. */
function buildGradient(t: ResolvedReelTheme): string | null {
    const stops = [...t.gradientStops]
        .sort((a, b) => a.position - b.position)
        .map((s) => `${withAlpha(s.color, s.opacity)} ${s.position}%`)
        .join(', ');
    if (!stops) return null;

    if (t.gradientType === 'radial') return `radial-gradient(circle at 50% 50%, ${stops})`;
    if (t.gradientType === 'conic') {
        return `conic-gradient(from ${t.gradientAngle}deg at 50% 50%, ${stops})`;
    }
    return `linear-gradient(${t.gradientAngle}deg, ${stops})`;
}

function buildVignette(t: ResolvedReelTheme): string | null {
    if (t.vignette <= 0) return null;
    return `radial-gradient(135% 100% at 50% 45%, transparent 45%, ${withAlpha(t.overlayColor, t.vignette)} 100%)`;
}

/**
 * The veil laid OVER the backdrop.
 *
 * There is one gradient editor on the panel, and it belongs to exactly one
 * surface: if the backdrop IS a gradient, the editor is painting the backdrop,
 * so the veil must not paint the same gradient a second time on top of it (that
 * doubled every stop's alpha and made the controls behave nonsensically). In
 * that mode the veil is the vignette and nothing else.
 */
function buildVeil(t: ResolvedReelTheme): string {
    const layers: string[] = [];

    if (t.backdropKind !== 'gradient') {
        if (t.overlayKind === 'solid') {
            const c = withAlpha(t.overlayColor, t.overlayOpacity);
            layers.push(`linear-gradient(${c}, ${c})`);
        } else if (t.overlayKind === 'gradient') {
            const g = buildGradient(t);
            if (g) layers.push(g);
        }
    }

    const vignette = buildVignette(t);
    if (vignette) layers.push(vignette);

    return layers.length ? layers.join(', ') : 'none';
}

/**
 * The pane's own fill, as one CSS value.
 *
 * `opacity` is the pane's transparency slider and applies in BOTH modes: in solid
 * it scales the tint, in gradient it scales every stop's own alpha. So the slider
 * never stops working just because the admin switched to a gradient, and a
 * gradient of opaque stops can still be faded back to a ghost.
 */
function cardFill(c: ResolvedCardStyle, opacity: number): string {
    if (c.fillKind !== 'gradient') return withAlpha(c.glassTint, opacity);

    const stops = [...c.fillStops]
        .sort((a, b) => a.position - b.position)
        .map((s) => `${withAlpha(s.color, s.opacity * opacity)} ${s.position}%`)
        .join(', ');
    // An emptied stop list would emit `linear-gradient()` and take the whole card
    // down with it; fall back to the flat tint.
    if (!stops) return withAlpha(c.glassTint, opacity);

    if (c.fillType === 'radial') return `radial-gradient(circle at 50% 50%, ${stops})`;
    if (c.fillType === 'conic') return `conic-gradient(from ${c.fillAngle}deg at 50% 50%, ${stops})`;
    return `linear-gradient(${c.fillAngle}deg, ${stops})`;
}

/** One pane's variables. Emitted once per surface, so hero and work differ. */
function cardVars(c: ResolvedCardStyle, performanceMode: boolean): string {
    // Performance mode is the escape hatch for weak machines: frosting two dozen
    // panes with backdrop-filter is what makes the reel stutter, so it drops the
    // filter and leans on a slightly more opaque tint to stay legible.
    const frost =
        performanceMode || c.glassBlur <= 0
            ? 'none'
            : `blur(${c.glassBlur}px) saturate(${c.glassSaturation})`;
    const tintOpacity = performanceMode ? Math.min(1, c.glassOpacity + 0.22) : c.glassOpacity;

    return `  --card-bg: ${cardFill(c, tintOpacity)};
  --card-bg-soft: ${cardFill(c, tintOpacity * 0.62)};
  --card-frost: ${frost};
  --card-radius: ${c.glassRadius}px;
  --card-sheen: ${0.28 * c.glassSheen};
  --card-edge: ${0.5 * c.glassSheen};
  --card-edge-color: ${withAlpha(c.edgeColor, c.edgeOpacity)};
  --card-edge-hover: ${withAlpha(c.edgeHoverColor, Math.min(1, c.edgeOpacity + 0.28))};
  --card-shadow: ${c.glassShadow};
  --card-film: ${toTriplet(c.shadowColor)};

  --card-title: ${c.titleColor};
  --card-title-size: ${c.titleSize}rem;
  --card-body: ${c.bodyColor};
  --card-body-size: ${c.bodySize}rem;
  --card-meta: ${c.metaColor};
  --card-meta-size: ${c.metaSize}rem;

  --card-btn-fill: ${c.btnFill};
  --card-btn-text: ${c.btnText};
  --card-btn-radius: ${c.btnRadius}px;
  --card-btn-size: ${c.btnSize}rem;
  --card-btn-border: ${c.btnBorder};`;
}

export function generateReelCSS(config: ReelTheme | null | undefined): string {
    const t = resolveReelTheme(config);

    const ink = t.textTone === 'dark' ? '44 23 16' : '250 246 238';

    // What the room itself is made of. With a photograph the fill stays
    // transparent so the <img> shows through (and backdropColor is only the
    // letterbox behind it); otherwise the room IS the colour or the gradient.
    const backdrop =
        t.backdropKind === 'image'
            ? 'transparent'
            : t.backdropKind === 'gradient'
              ? (buildGradient(t) ?? t.backdropColor)
              : t.backdropColor;

    // The photograph is a CSS background, not an <img>, so that EVERY part of the
    // room is driven by this one block. When it was an element, the admin's live
    // preview (which only ships CSS) could not make the photo disappear on
    // switching to a flat colour — the preview lied.
    const photo =
        t.backdropKind === 'image' && t.backgroundImage
            ? `url("${t.backgroundImage.replace(/"/g, '\\"')}")`
            : 'none';
    const photoMobile =
        t.backdropKind === 'image' && (t.backgroundImageMobile || t.backgroundImage)
            ? `url("${(t.backgroundImageMobile || t.backgroundImage).replace(/"/g, '\\"')}")`
            : 'none';

    // The site header floats over the room on the home page, so it has to follow
    // the theme: on a pale backdrop, parchment type on a dark scrim is unreadable.
    const headerScrim = t.textTone === 'dark' ? '250 246 238' : toTriplet(t.shadowColor);

    return `:root {
  --reel-header-ink: ${ink};
  --reel-header-scrim: ${headerScrim};
  --reel-photo: ${photo};
  --reel-photo-mobile: ${photoMobile};
  --reel-backdrop-color: ${t.backdropColor};
  --reel-backdrop-fill: ${backdrop};
  --reel-bg-fit: ${t.backgroundFit};
  --reel-bg-position: ${t.backgroundPosition};
  --reel-bg-filter: saturate(${t.backgroundSaturation}) brightness(${t.backgroundBrightness}) blur(${t.backgroundBlur}px);
  /* blur() samples past the element's edge and leaves a soft rim; scale it away. */
  --reel-bg-scale: ${t.backgroundBlur > 0 ? 1.04 : 1};
  --reel-overlay: ${buildVeil(t)};
  --reel-grain: ${t.grain};

  --reel-ink: ${ink};
  --reel-film: ${toTriplet(t.shadowColor)};
  --reel-card-gap: ${t.cardGap}rem;
  --reel-card-width: ${t.cardWidth}rem;
}
/* The opening pane and the work panes are independent surfaces; each gets its
   own copy of the same variable names, so the components never branch. */
.glass-hero {
${cardVars(t.hero, t.performanceMode)}
}
.glass-work,
.glass-archive {
${cardVars(t.work, t.performanceMode)}
}`;
}

// ── Ready-made looks ────────────────────────────────────────────────────────
// Starting points, not straitjackets: each preset is a patch, so it only sets
// what it is about and leaves everything else alone.

export interface Preset<T> {
    id: string;
    label: TranslationKey;
    patch: Partial<T>;
}

export const BACKDROP_PRESETS: Preset<ReelTheme>[] = [
    {
        id: 'room',
        label: 'reelPresetRoom',
        patch: {
            backdropKind: 'image', backgroundBlur: 3, backgroundBrightness: 0.62,
            backgroundSaturation: 0.72, overlayKind: 'gradient', vignette: 0.45, grain: 0.14,
        },
    },
    {
        id: 'sharp',
        label: 'reelPresetSharp',
        patch: {
            backdropKind: 'image', backgroundBlur: 0, backgroundBrightness: 0.85,
            backgroundSaturation: 1, overlayKind: 'solid', overlayOpacity: 0.25, vignette: 0.3,
        },
    },
    {
        id: 'ink',
        label: 'reelPresetInk',
        patch: {
            backdropKind: 'color', backdropColor: '#140b07',
            overlayKind: 'none', vignette: 0.35, grain: 0.1,
        },
    },
    {
        id: 'parchment',
        label: 'reelPresetParchment',
        patch: {
            backdropKind: 'color', backdropColor: '#faf6ee',
            overlayKind: 'none', vignette: 0.2, grain: 0.08, textTone: 'dark',
        },
    },
    {
        id: 'ember',
        label: 'reelPresetEmber',
        patch: {
            backdropKind: 'gradient', gradientType: 'linear', gradientAngle: 160,
            gradientStops: [
                { color: '#2c1710', position: 0, opacity: 1 },
                { color: '#c0582c', position: 60, opacity: 0.55 },
                { color: '#140b07', position: 100, opacity: 1 },
            ],
            vignette: 0.4,
        },
    },
];

export const GLASS_PRESETS: Preset<CardStyle>[] = [
    {
        id: 'frosted',
        label: 'reelPresetFrosted',
        patch: { glassTint: '#faf6ee', glassOpacity: 0.14, glassBlur: 26, glassSaturation: 1.25, glassRadius: 36, glassSheen: 1, glassShadow: 0.45 },
    },
    {
        id: 'clear',
        label: 'reelPresetClear',
        patch: { glassOpacity: 0.06, glassBlur: 8, glassSaturation: 1.1, glassSheen: 0.6, glassShadow: 0.3 },
    },
    {
        id: 'milk',
        label: 'reelPresetMilk',
        patch: { glassTint: '#faf6ee', glassOpacity: 0.42, glassBlur: 40, glassSaturation: 1.4, glassSheen: 1.3, glassShadow: 0.5 },
    },
    {
        // Dark frosted glass. Needs texture behind the pane to read as glass at all
        // — over a flat colour it is just a tinted rectangle.
        id: 'smoked',
        label: 'reelPresetSmoked',
        patch: { glassTint: '#2a2019', glassOpacity: 0.5, glassBlur: 30, glassSaturation: 1.3, glassSheen: 0.5, glassShadow: 0.3 },
    },
    {
        id: 'slab',
        label: 'reelPresetSlab',
        patch: { glassTint: '#140b07', glassOpacity: 0.55, glassBlur: 0, glassRadius: 6, glassSheen: 0, glassShadow: 0.6 },
    },
    {
        id: 'edgeless',
        label: 'reelPresetEdgeless',
        patch: { glassOpacity: 0, glassBlur: 0, glassSheen: 0, glassShadow: 0 },
    },
];

export const BUTTON_PRESETS: Preset<CardStyle>[] = [
    { id: 'pill', label: 'reelPresetPill', patch: { btnFill: '#faf6ee', btnText: '#2c1710', btnRadius: 22, btnSize: 0.66, btnBorder: '#faf6ee' } },
    { id: 'square', label: 'reelPresetSquare', patch: { btnFill: '#faf6ee', btnText: '#2c1710', btnRadius: 2, btnSize: 0.66, btnBorder: '#faf6ee' } },
    { id: 'ember', label: 'reelPresetBtnEmber', patch: { btnFill: '#c0582c', btnText: '#faf6ee', btnRadius: 4, btnSize: 0.68, btnBorder: '#c0582c' } },
    { id: 'outline', label: 'reelPresetOutline', patch: { btnFill: '#00000000', btnText: '#faf6ee', btnRadius: 2, btnSize: 0.66, btnBorder: '#faf6ee' } },
];

export const TYPE_PRESETS: Preset<CardStyle>[] = [
    { id: 'quiet', label: 'reelPresetQuiet', patch: { titleSize: 2.4, bodySize: 0.85, metaSize: 0.8, titleColor: '#faf6ee', bodyColor: '#faf6ee', metaColor: '#faf6ee' } },
    { id: 'grand', label: 'reelPresetGrand', patch: { titleSize: 4, bodySize: 1, metaSize: 0.95 } },
    { id: 'ink', label: 'reelPresetTypeInk', patch: { titleColor: '#2c1710', bodyColor: '#2c1710', metaColor: '#5a3420' } },
    { id: 'ember', label: 'reelPresetTypeEmber', patch: { titleColor: '#faf6ee', bodyColor: '#faf6ee', metaColor: '#c0582c' } },
];

export const CARD_TARGETS: CardTarget[] = ['hero', 'work'];

// ── The house themes ────────────────────────────────────────────────────────
// Five finished looks, each a COMPLETE theme (backdrop, veil, both panes), not a
// patch — applying one replaces everything, so there is no way to end up with
// half of one look wearing half of another.

export const HOUSE_THEMES: Preset<ReelTheme>[] = [
    {
        // The designed default, tightened: a dark room out of focus, thick frosted
        // panes, parchment type. The photograph is depth; the glass is the subject.
        id: 'night-atelier',
        label: 'reelThemeNight',
        patch: {
            backdropKind: 'image',
            backgroundFit: 'cover',
            backgroundPosition: 'center top',
            backgroundBlur: 4,
            backgroundBrightness: 0.58,
            backgroundSaturation: 0.7,
            backdropColor: '#140b07',
            shadowColor: '#140b07',
            overlayKind: 'gradient',
            overlayColor: '#140b07',
            gradientType: 'linear',
            gradientAngle: 180,
            gradientStops: [
                { color: '#140b07', position: 0, opacity: 0.6 },
                { color: '#140b07', position: 24, opacity: 0.05 },
                { color: '#140b07', position: 74, opacity: 0.08 },
                { color: '#140b07', position: 100, opacity: 0.62 },
            ],
            vignette: 0.5,
            grain: 0.16,
            textTone: 'light',
            cardGap: 2.25,
            cardWidth: 64,
            hero: {
                glassTint: '#faf6ee', glassOpacity: 0.13, glassBlur: 30, glassSaturation: 1.3,
                glassRadius: 38, glassSheen: 1.1, glassShadow: 0.5, shadowColor: '#140b07',
                titleColor: '#faf6ee', titleSize: 3.2,
                bodyColor: '#efe6d7', bodySize: 0.92,
                metaColor: '#c9b9a4', metaSize: 0.7,
                btnFill: '#faf6ee', btnText: '#2c1710', btnRadius: 20, btnSize: 0.66, btnBorder: '#faf6ee',
            },
            work: {
                glassTint: '#faf6ee', glassOpacity: 0.1, glassBlur: 26, glassSaturation: 1.2,
                glassRadius: 32, glassSheen: 0.9, glassShadow: 0.45, shadowColor: '#140b07',
                titleColor: '#faf6ee', titleSize: 2.8,
                bodyColor: '#e2d6c5', bodySize: 0.88,
                metaColor: '#a8937c', metaSize: 0.85,
                btnFill: '#faf6ee', btnText: '#2c1710', btnRadius: 20, btnSize: 0.64, btnBorder: '#faf6ee',
            },
        },
    },
    {
        // Museum daylight: no photograph at all, just warm paper. The work images
        // inside the panes carry every bit of colour on the page — which is the most
        // honest thing a portfolio can do.
        id: 'daylight',
        label: 'reelThemeDaylight',
        patch: {
            backdropKind: 'color',
            backdropColor: '#f2ebdd',
            shadowColor: '#8a6a4d',
            overlayKind: 'none',
            vignette: 0.18,
            grain: 0.07,
            textTone: 'dark',
            cardGap: 3,
            cardWidth: 66,
            hero: {
                glassTint: '#ffffff', glassOpacity: 0.5, glassBlur: 14, glassSaturation: 1.1,
                glassRadius: 30, glassSheen: 0.5, glassShadow: 0.16, shadowColor: '#8a6a4d',
                titleColor: '#faf6ee', titleSize: 3,
                bodyColor: '#3a2418', bodySize: 0.92,
                metaColor: '#8a6a4d', metaSize: 0.7,
                btnFill: '#2c1710', btnText: '#faf6ee', btnRadius: 4, btnSize: 0.66, btnBorder: '#2c1710',
            },
            work: {
                glassTint: '#ffffff', glassOpacity: 0.55, glassBlur: 12, glassSaturation: 1.05,
                glassRadius: 26, glassSheen: 0.4, glassShadow: 0.14, shadowColor: '#8a6a4d',
                titleColor: '#2c1710', titleSize: 2.7,
                bodyColor: '#5a3420', bodySize: 0.88,
                metaColor: '#a0745a', metaSize: 0.85,
                btnFill: '#2c1710', btnText: '#faf6ee', btnRadius: 4, btnSize: 0.64, btnBorder: '#2c1710',
            },
        },
    },
    {
        // The vitrine: the photograph stays SHARP and almost undimmed, and the panes
        // nearly vanish — thin outlines and type laid straight onto the room. The
        // riskiest of the five and the most editorial.
        id: 'vitrine',
        label: 'reelThemeVitrine',
        patch: {
            backdropKind: 'image',
            backgroundFit: 'cover',
            backgroundPosition: 'center top',
            backgroundBlur: 0,
            backgroundBrightness: 0.7,
            backgroundSaturation: 0.95,
            backdropColor: '#0d0906',
            shadowColor: '#0d0906',
            overlayKind: 'solid',
            overlayColor: '#0d0906',
            overlayOpacity: 0.3,
            vignette: 0.55,
            grain: 0.1,
            textTone: 'light',
            cardGap: 4.5,
            cardWidth: 70,
            hero: {
                glassTint: '#faf6ee', glassOpacity: 0.05, glassBlur: 4, glassSaturation: 1,
                glassRadius: 2, glassSheen: 0.35, glassShadow: 0.5, shadowColor: '#0d0906',
                titleColor: '#ffffff', titleSize: 3.4,
                bodyColor: '#e8e2d8', bodySize: 0.9,
                metaColor: '#b9ac9a', metaSize: 0.68,
                btnFill: '#ffffff', btnText: '#0d0906', btnRadius: 0, btnSize: 0.64, btnBorder: '#ffffff',
            },
            work: {
                glassTint: '#faf6ee', glassOpacity: 0.04, glassBlur: 3, glassSaturation: 1,
                glassRadius: 2, glassSheen: 0.25, glassShadow: 0.45, shadowColor: '#0d0906',
                titleColor: '#ffffff', titleSize: 3,
                bodyColor: '#ddd5c9', bodySize: 0.86,
                metaColor: '#9c8f7e', metaSize: 0.82,
                btnFill: '#00000000', btnText: '#ffffff', btnRadius: 0, btnSize: 0.62, btnBorder: '#ffffff',
            },
        },
    },
    {
        // No photograph, no glass: slabs of near-black on a gradient of ink and
        // ember. Everything is edge and letterpress — the closest of the five to
        // the rest of the house's typography.
        id: 'ember',
        label: 'reelThemeEmber',
        patch: {
            backdropKind: 'gradient',
            backdropColor: '#140b07',
            shadowColor: '#000000',
            gradientType: 'linear',
            gradientAngle: 155,
            gradientStops: [
                { color: '#1b0f08', position: 0, opacity: 1 },
                { color: '#5e2a14', position: 45, opacity: 1 },
                { color: '#c0582c', position: 70, opacity: 0.85 },
                { color: '#140b07', position: 100, opacity: 1 },
            ],
            overlayKind: 'none',
            vignette: 0.5,
            grain: 0.13,
            textTone: 'light',
            cardGap: 1.75,
            cardWidth: 62,
            hero: {
                glassTint: '#100804', glassOpacity: 0.62, glassBlur: 0, glassSaturation: 1,
                glassRadius: 4, glassSheen: 0.15, glassShadow: 0.65, shadowColor: '#000000',
                titleColor: '#faf6ee', titleSize: 3.3,
                bodyColor: '#e6d9c8', bodySize: 0.92,
                metaColor: '#e08a5c', metaSize: 0.7,
                btnFill: '#c0582c', btnText: '#faf6ee', btnRadius: 3, btnSize: 0.68, btnBorder: '#c0582c',
            },
            work: {
                glassTint: '#100804', glassOpacity: 0.55, glassBlur: 0, glassSaturation: 1,
                glassRadius: 4, glassSheen: 0.12, glassShadow: 0.6, shadowColor: '#000000',
                titleColor: '#faf6ee', titleSize: 2.8,
                bodyColor: '#d8c8b4', bodySize: 0.88,
                metaColor: '#c0582c', metaSize: 0.88,
                btnFill: '#00000000', btnText: '#faf6ee', btnRadius: 3, btnSize: 0.64, btnBorder: '#e08a5c',
            },
        },
    },
    {
        // Deep defocus and thick milk glass — the softest, most expensive-looking of
        // the five. Generous type, wide panes, room to breathe.
        id: 'fog',
        label: 'reelThemeFog',
        patch: {
            backdropKind: 'image',
            backgroundFit: 'cover',
            backgroundPosition: 'center center',
            backgroundBlur: 14,
            backgroundBrightness: 0.75,
            backgroundSaturation: 0.45,
            backdropColor: '#2a2320',
            shadowColor: '#1a1512',
            overlayKind: 'solid',
            overlayColor: '#2a2320',
            overlayOpacity: 0.28,
            vignette: 0.4,
            grain: 0.06,
            textTone: 'light',
            cardGap: 3.5,
            cardWidth: 68,
            hero: {
                glassTint: '#f7f2ea', glassOpacity: 0.24, glassBlur: 44, glassSaturation: 1.5,
                glassRadius: 44, glassSheen: 1.4, glassShadow: 0.4, shadowColor: '#1a1512',
                titleColor: '#ffffff', titleSize: 3.6,
                bodyColor: '#f2ece2', bodySize: 0.98,
                metaColor: '#ddd2c4', metaSize: 0.72,
                btnFill: '#ffffff', btnText: '#2a2320', btnRadius: 28, btnSize: 0.66, btnBorder: '#ffffff',
            },
            work: {
                glassTint: '#f7f2ea', glassOpacity: 0.2, glassBlur: 40, glassSaturation: 1.45,
                glassRadius: 40, glassSheen: 1.2, glassShadow: 0.36, shadowColor: '#1a1512',
                titleColor: '#ffffff', titleSize: 3,
                bodyColor: '#ece5da', bodySize: 0.92,
                metaColor: '#cabfb0', metaSize: 0.86,
                btnFill: '#ffffff', btnText: '#2a2320', btnRadius: 28, btnSize: 0.64, btnBorder: '#ffffff',
            },
        },
    },
    {
        // Cold night. The photograph is drained almost to grey and lit from behind by
        // indigo — the only theme here that isn't warm, which is exactly why it makes
        // the figures' skin tones jump off the glass.
        id: 'midnight',
        label: 'reelThemeMidnight',
        patch: {
            backdropKind: 'image',
            backgroundFit: 'cover',
            backgroundPosition: 'center top',
            backgroundBlur: 6,
            backgroundBrightness: 0.5,
            backgroundSaturation: 0.25,
            backdropColor: '#0b1020',
            shadowColor: '#05070f',
            overlayKind: 'gradient',
            overlayColor: '#0b1020',
            gradientType: 'linear',
            gradientAngle: 200,
            gradientStops: [
                { color: '#0b1020', position: 0, opacity: 0.85 },
                { color: '#1e3a5f', position: 45, opacity: 0.4 },
                { color: '#0b1020', position: 100, opacity: 0.9 },
            ],
            vignette: 0.55,
            grain: 0.12,
            textTone: 'light',
            cardGap: 2.5,
            cardWidth: 64,
            hero: {
                glassTint: '#cfe0f5', glassOpacity: 0.12, glassBlur: 32, glassSaturation: 1.35,
                glassRadius: 26, glassSheen: 1.2, glassShadow: 0.55, shadowColor: '#05070f',
                titleColor: '#eef4fb', titleSize: 3.2,
                bodyColor: '#c3d2e4', bodySize: 0.92,
                metaColor: '#7f96b3', metaSize: 0.7,
                btnFill: '#eef4fb', btnText: '#0b1020', btnRadius: 3, btnSize: 0.66, btnBorder: '#eef4fb',
            },
            work: {
                glassTint: '#cfe0f5', glassOpacity: 0.09, glassBlur: 28, glassSaturation: 1.3,
                glassRadius: 22, glassSheen: 1, glassShadow: 0.5, shadowColor: '#05070f',
                titleColor: '#eef4fb', titleSize: 2.8,
                bodyColor: '#b6c6da', bodySize: 0.88,
                metaColor: '#6d86a5', metaSize: 0.85,
                btnFill: '#00000000', btnText: '#eef4fb', btnRadius: 3, btnSize: 0.64, btnBorder: '#8fb0d4',
            },
        },
    },
    {
        // A plate out of the archive: colour drained to sepia, heavy grain, a hard
        // vignette, and cream cards with square corners — a photograph album, not a
        // website.
        id: 'daguerreotype',
        label: 'reelThemeDaguerreotype',
        patch: {
            backdropKind: 'image',
            backgroundFit: 'cover',
            backgroundPosition: 'center top',
            backgroundBlur: 2,
            backgroundBrightness: 0.72,
            backgroundSaturation: 0.18,
            backdropColor: '#c7b299',
            shadowColor: '#4a3b28',
            overlayKind: 'solid',
            overlayColor: '#8a6a45',
            overlayOpacity: 0.42,
            vignette: 0.75,
            grain: 0.34,
            textTone: 'dark',
            cardGap: 3,
            cardWidth: 62,
            hero: {
                glassTint: '#f5ecd9', glassOpacity: 0.88, glassBlur: 0, glassSaturation: 1,
                glassRadius: 0, glassSheen: 0.2, glassShadow: 0.35, shadowColor: '#4a3b28',
                titleColor: '#faf6ee', titleSize: 2.9,
                bodyColor: '#43331f', bodySize: 0.9,
                metaColor: '#8a6a45', metaSize: 0.68,
                btnFill: '#43331f', btnText: '#f5ecd9', btnRadius: 0, btnSize: 0.64, btnBorder: '#43331f',
            },
            work: {
                glassTint: '#f5ecd9', glassOpacity: 0.9, glassBlur: 0, glassSaturation: 1,
                glassRadius: 0, glassSheen: 0.15, glassShadow: 0.32, shadowColor: '#4a3b28',
                titleColor: '#2e2114', titleSize: 2.6,
                bodyColor: '#5b4730', bodySize: 0.86,
                metaColor: '#9d8058', metaSize: 0.82,
                btnFill: '#00000000', btnText: '#43331f', btnRadius: 0, btnSize: 0.62, btnBorder: '#8a6a45',
            },
        },
    },
    {
        // Light through coloured glass: a radial gold burning behind the panes, and
        // the panes themselves genuinely tinted rather than neutral. The only theme
        // where the glass has a colour of its own.
        id: 'reliquary',
        label: 'reelThemeReliquary',
        patch: {
            backdropKind: 'gradient',
            backdropColor: '#1a0f06',
            shadowColor: '#0a0602',
            gradientType: 'radial',
            // The gold is a LAMP, not a wash: it burns out by a fifth of the way and
            // the rest of the room is dark, or the panes have nothing to sit against.
            gradientStops: [
                { color: '#f0b455', position: 0, opacity: 0.95 },
                { color: '#a85a1e', position: 14, opacity: 0.9 },
                { color: '#3a1c0a', position: 34, opacity: 1 },
                { color: '#140a04', position: 62, opacity: 1 },
                { color: '#0a0502', position: 100, opacity: 1 },
            ],
            overlayKind: 'none',
            vignette: 0.6,
            grain: 0.15,
            textTone: 'light',
            cardGap: 2,
            cardWidth: 62,
            hero: {
                // Barely-tinted glass. Gold panes over a gold lamp read as one
                // orange smear — the pane has to stay glass.
                glassTint: '#fdf3e2', glassOpacity: 0.1, glassBlur: 26, glassSaturation: 1.5,
                glassRadius: 30, glassSheen: 1.4, glassShadow: 0.65, shadowColor: '#0a0602',
                titleColor: '#fff4dd', titleSize: 3.3,
                bodyColor: '#eddcc0', bodySize: 0.94,
                metaColor: '#e8a33d', metaSize: 0.72,
                btnFill: '#e8a33d', btnText: '#2a1408', btnRadius: 24, btnSize: 0.68, btnBorder: '#fff4dd',
            },
            work: {
                glassTint: '#fdf3e2', glassOpacity: 0.08, glassBlur: 24, glassSaturation: 1.45,
                glassRadius: 26, glassSheen: 1.2, glassShadow: 0.6, shadowColor: '#0a0602',
                titleColor: '#fff4dd', titleSize: 2.8,
                bodyColor: '#e3d1b4', bodySize: 0.88,
                metaColor: '#e8a33d', metaSize: 0.86,
                btnFill: '#e8a33d', btnText: '#2a1408', btnRadius: 24, btnSize: 0.64, btnBorder: '#fff4dd',
            },
        },
    },
    {
        // Gallery wall: flat chalk, and graphite slabs sitting on it with no blur, no
        // sheen and no shadow to speak of. Nothing here is decorative — the work is
        // the only thing allowed to be.
        id: 'chalk',
        label: 'reelThemeChalk',
        patch: {
            backdropKind: 'color',
            backdropColor: '#e9e6e0',
            shadowColor: '#5c5850',
            overlayKind: 'none',
            vignette: 0.12,
            grain: 0.05,
            textTone: 'dark',
            cardGap: 3.5,
            cardWidth: 70,
            hero: {
                glassTint: '#2f2d2a', glassOpacity: 0.95, glassBlur: 0, glassSaturation: 1,
                glassRadius: 0, glassSheen: 0, glassShadow: 0.18, shadowColor: '#5c5850',
                titleColor: '#f2f0ec', titleSize: 3,
                bodyColor: '#cfccc6', bodySize: 0.9,
                metaColor: '#8f8b84', metaSize: 0.68,
                btnFill: '#f2f0ec', btnText: '#2f2d2a', btnRadius: 0, btnSize: 0.64, btnBorder: '#f2f0ec',
            },
            work: {
                glassTint: '#2f2d2a', glassOpacity: 0.95, glassBlur: 0, glassSaturation: 1,
                glassRadius: 0, glassSheen: 0, glassShadow: 0.16, shadowColor: '#5c5850',
                titleColor: '#f2f0ec', titleSize: 2.6,
                bodyColor: '#c4c1ba', bodySize: 0.86,
                metaColor: '#8f8b84', metaSize: 0.8,
                btnFill: '#00000000', btnText: '#f2f0ec', btnRadius: 0, btnSize: 0.62, btnBorder: '#8f8b84',
            },
        },
    },
    {
        // The theatre: oxblood and old gold, the deepest and most frankly decorative
        // of the ten. The panes are heavy, the buttons gilded, the type generous.
        id: 'velvet',
        label: 'reelThemeVelvet',
        patch: {
            backdropKind: 'gradient',
            backdropColor: '#2a070c',
            shadowColor: '#120306',
            gradientType: 'linear',
            gradientAngle: 165,
            gradientStops: [
                { color: '#1a0407', position: 0, opacity: 1 },
                { color: '#6e1420', position: 40, opacity: 1 },
                { color: '#3d0a11', position: 75, opacity: 1 },
                { color: '#120306', position: 100, opacity: 1 },
            ],
            overlayKind: 'none',
            vignette: 0.62,
            grain: 0.14,
            textTone: 'light',
            cardGap: 2.75,
            cardWidth: 66,
            hero: {
                glassTint: '#1a0407', glassOpacity: 0.5, glassBlur: 18, glassSaturation: 1.2,
                glassRadius: 14, glassSheen: 0.8, glassShadow: 0.7, shadowColor: '#120306',
                titleColor: '#f3e3c4', titleSize: 3.4,
                bodyColor: '#e0cfb4', bodySize: 0.94,
                metaColor: '#c9a44e', metaSize: 0.72,
                btnFill: '#c9a44e', btnText: '#1a0407', btnRadius: 6, btnSize: 0.68, btnBorder: '#c9a44e',
            },
            work: {
                glassTint: '#1a0407', glassOpacity: 0.45, glassBlur: 16, glassSaturation: 1.15,
                glassRadius: 12, glassSheen: 0.7, glassShadow: 0.65, shadowColor: '#120306',
                titleColor: '#f3e3c4', titleSize: 2.9,
                bodyColor: '#d6c3a6', bodySize: 0.88,
                metaColor: '#c9a44e', metaSize: 0.86,
                btnFill: '#00000000', btnText: '#f3e3c4', btnRadius: 6, btnSize: 0.64, btnBorder: '#c9a44e',
            },
        },
    },

    // ── Warm and contemporary ───────────────────────────────────────────────
    // The five below share one idea: warm off-white air, panes that are barely
    // there, ink-dark type, and one decisive dark button. No photograph competes
    // with the work — the work is the only image on the page.

    {
        // Linen: warm white, generous air, a pane you can only see by its shadow.
        // The closest this site gets to a contemporary studio portfolio.
        id: 'linen',
        label: 'reelThemeLinen',
        patch: {
            backdropKind: 'color',
            backdropColor: '#f4efe8',
            shadowColor: '#a8967f',
            overlayKind: 'none',
            vignette: 0.1,
            grain: 0.04,
            textTone: 'dark',
            cardGap: 4,
            cardWidth: 70,
            hero: {
                glassTint: '#ffffff', glassOpacity: 0.75, glassBlur: 0, glassSaturation: 1,
                glassRadius: 28, glassSheen: 0.25, glassShadow: 0.12, shadowColor: '#a8967f',
                titleColor: '#faf6ee', titleSize: 3.1,
                bodyColor: '#3d3830', bodySize: 0.95,
                metaColor: '#9a9083', metaSize: 0.7,
                btnFill: '#1f1c18', btnText: '#f4efe8', btnRadius: 26, btnSize: 0.66, btnBorder: '#1f1c18',
            },
            work: {
                glassTint: '#ffffff', glassOpacity: 0.8, glassBlur: 0, glassSaturation: 1,
                glassRadius: 26, glassSheen: 0.2, glassShadow: 0.1, shadowColor: '#a8967f',
                titleColor: '#1f1c18', titleSize: 2.7,
                bodyColor: '#5a544a', bodySize: 0.9,
                metaColor: '#a89e8f', metaSize: 0.8,
                btnFill: '#1f1c18', btnText: '#f4efe8', btnRadius: 26, btnSize: 0.64, btnBorder: '#1f1c18',
            },
        },
    },
    {
        // Clay: a flat terracotta wall with cream panes floating on it. Warm, matte,
        // and the one modern theme with real colour in the room.
        id: 'clay',
        label: 'reelThemeClay',
        patch: {
            backdropKind: 'color',
            backdropColor: '#c2725a',
            shadowColor: '#6e3520',
            overlayKind: 'none',
            vignette: 0.22,
            grain: 0.06,
            textTone: 'dark',
            cardGap: 3.5,
            cardWidth: 66,
            hero: {
                glassTint: '#fbf3e9', glassOpacity: 0.82, glassBlur: 6, glassSaturation: 1.1,
                glassRadius: 32, glassSheen: 0.35, glassShadow: 0.28, shadowColor: '#6e3520',
                titleColor: '#fbf3e9', titleSize: 3.1,
                bodyColor: '#4a2a1d', bodySize: 0.94,
                metaColor: '#b06a4e', metaSize: 0.7,
                btnFill: '#4a2a1d', btnText: '#fbf3e9', btnRadius: 30, btnSize: 0.66, btnBorder: '#4a2a1d',
            },
            work: {
                glassTint: '#fbf3e9', glassOpacity: 0.86, glassBlur: 6, glassSaturation: 1.1,
                glassRadius: 30, glassSheen: 0.3, glassShadow: 0.26, shadowColor: '#6e3520',
                titleColor: '#3a2016', titleSize: 2.7,
                bodyColor: '#6b4635', bodySize: 0.9,
                metaColor: '#c2725a', metaSize: 0.82,
                btnFill: '#4a2a1d', btnText: '#fbf3e9', btnRadius: 30, btnSize: 0.64, btnBorder: '#4a2a1d',
            },
        },
    },
    {
        // Honey: a soft amber wash that lifts from the top left, milk-glass panes,
        // espresso type. Warm daylight without a photograph.
        id: 'honey',
        label: 'reelThemeHoney',
        patch: {
            backdropKind: 'gradient',
            backdropColor: '#f7e7c8',
            shadowColor: '#8a6a3a',
            gradientType: 'linear',
            gradientAngle: 145,
            gradientStops: [
                { color: '#fdf6e7', position: 0, opacity: 1 },
                { color: '#f2d9a4', position: 45, opacity: 1 },
                { color: '#e8bf7c', position: 78, opacity: 1 },
                { color: '#f7e7c8', position: 100, opacity: 1 },
            ],
            overlayKind: 'none',
            vignette: 0.18,
            grain: 0.05,
            textTone: 'dark',
            cardGap: 3.25,
            cardWidth: 68,
            hero: {
                glassTint: '#ffffff', glassOpacity: 0.55, glassBlur: 24, glassSaturation: 1.2,
                glassRadius: 34, glassSheen: 0.7, glassShadow: 0.18, shadowColor: '#8a6a3a',
                titleColor: '#fdf6e7', titleSize: 3.2,
                bodyColor: '#4a3a22', bodySize: 0.95,
                metaColor: '#a08348', metaSize: 0.7,
                btnFill: '#33281a', btnText: '#fdf6e7', btnRadius: 28, btnSize: 0.66, btnBorder: '#33281a',
            },
            work: {
                glassTint: '#ffffff', glassOpacity: 0.6, glassBlur: 22, glassSaturation: 1.2,
                glassRadius: 32, glassSheen: 0.6, glassShadow: 0.16, shadowColor: '#8a6a3a',
                titleColor: '#33281a', titleSize: 2.8,
                bodyColor: '#5c4a30', bodySize: 0.9,
                metaColor: '#b0914f', metaSize: 0.82,
                btnFill: '#33281a', btnText: '#fdf6e7', btnRadius: 28, btnSize: 0.64, btnBorder: '#33281a',
            },
        },
    },
    {
        // Apricot: a soft peach-to-sand wash and thick frosted panes on top — the
        // most obviously "of now" of the ten, and the softest.
        id: 'apricot',
        label: 'reelThemeApricot',
        patch: {
            backdropKind: 'gradient',
            backdropColor: '#f6d9c4',
            shadowColor: '#9b6a52',
            gradientType: 'linear',
            gradientAngle: 200,
            gradientStops: [
                { color: '#ffd9bd', position: 0, opacity: 1 },
                { color: '#f3c3ab', position: 34, opacity: 1 },
                { color: '#e8b49f', position: 62, opacity: 1 },
                { color: '#f7e6d6', position: 100, opacity: 1 },
            ],
            overlayKind: 'none',
            vignette: 0.16,
            grain: 0.04,
            textTone: 'dark',
            cardGap: 3.75,
            cardWidth: 68,
            hero: {
                glassTint: '#ffffff', glassOpacity: 0.42, glassBlur: 38, glassSaturation: 1.35,
                glassRadius: 40, glassSheen: 0.9, glassShadow: 0.2, shadowColor: '#9b6a52',
                titleColor: '#fff6ef', titleSize: 3.3,
                bodyColor: '#4a3025', bodySize: 0.96,
                metaColor: '#a8705a', metaSize: 0.72,
                btnFill: '#c0582c', btnText: '#fff6ef', btnRadius: 30, btnSize: 0.66, btnBorder: '#c0582c',
            },
            work: {
                glassTint: '#ffffff', glassOpacity: 0.48, glassBlur: 34, glassSaturation: 1.3,
                glassRadius: 36, glassSheen: 0.8, glassShadow: 0.18, shadowColor: '#9b6a52',
                titleColor: '#3d2419', titleSize: 2.8,
                bodyColor: '#5f4235', bodySize: 0.9,
                metaColor: '#b57b62', metaSize: 0.82,
                btnFill: '#c0582c', btnText: '#fff6ef', btnRadius: 30, btnSize: 0.64, btnBorder: '#c0582c',
            },
        },
    },
    {
        // Espresso on sand: the inverse of Linen — a pale warm room and DARK panes,
        // so the work's own photograph is the only bright thing on the screen.
        //
        // The panes are SMOKED glass, not brown slabs. Frosting is a blur of what is
        // behind the pane, so it needs something behind it: this theme keeps the
        // room photograph, washed out almost to sand (bright, drained, veiled) —
        // enough texture for the glass to refract, not enough to compete.
        id: 'espresso',
        label: 'reelThemeEspresso',
        patch: {
            backdropKind: 'image',
            backgroundFit: 'cover',
            backgroundPosition: 'center top',
            backgroundBlur: 5,
            backgroundBrightness: 1.15,
            backgroundSaturation: 0.22,
            backdropColor: '#e7dccb',
            shadowColor: '#8c7a63',
            overlayKind: 'solid',
            overlayColor: '#e7dccb',
            overlayOpacity: 0.55,
            vignette: 0.2,
            grain: 0.06,
            textTone: 'dark',
            cardGap: 3.25,
            cardWidth: 68,
            hero: {
                glassTint: '#2a2019', glassOpacity: 0.55, glassBlur: 30, glassSaturation: 1.3,
                glassRadius: 30, glassSheen: 0.5, glassShadow: 0.3, shadowColor: '#8c7a63',
                titleColor: '#f6efe4', titleSize: 3.2,
                bodyColor: '#e0d6c8', bodySize: 0.94,
                metaColor: '#c2ac8e', metaSize: 0.7,
                btnFill: '#e8a33d', btnText: '#2a2019', btnRadius: 26, btnSize: 0.66, btnBorder: '#f6efe4',
            },
            work: {
                glassTint: '#2a2019', glassOpacity: 0.5, glassBlur: 28, glassSaturation: 1.25,
                glassRadius: 28, glassSheen: 0.45, glassShadow: 0.28, shadowColor: '#8c7a63',
                titleColor: '#f6efe4', titleSize: 2.8,
                bodyColor: '#dcd1c1', bodySize: 0.9,
                metaColor: '#e8a33d', metaSize: 0.82,
                btnFill: '#e8a33d', btnText: '#2a2019', btnRadius: 26, btnSize: 0.64, btnBorder: '#f6efe4',
            },
        },
    },
];

export const reelTheme = writable<ReelTheme>({});
export const reelCSS = derived(reelTheme, ($t) => generateReelCSS($t));

/** Which image the room actually shows, given the viewport. */
export function reelBackgroundImage(config: ReelTheme | null | undefined, narrow: boolean): string {
    const t = resolveReelTheme(config);
    if (t.backdropKind !== 'image') return '';
    if (narrow && t.backgroundImageMobile) return t.backgroundImageMobile;
    return t.backgroundImage;
}

// ── Live preview, mirroring the theme editor's channel ───────────────────────

const CHANNEL_NAME = 'gotiga_reel_preview';
type PreviewMessage = { type: 'apply'; config: ReelTheme } | { type: 'clear' };

const STYLE_ID = 'gotiga-reel-preview';

let _sender: BroadcastChannel | null = null;

function sender(): BroadcastChannel | null {
    if (typeof BroadcastChannel === 'undefined') return null;
    if (!_sender) _sender = new BroadcastChannel(CHANNEL_NAME);
    return _sender;
}

function applyCSS(css: string | null) {
    if (typeof document === 'undefined') return;
    const existing = document.getElementById(STYLE_ID);
    if (!css) {
        existing?.remove();
        return;
    }
    const style = existing instanceof HTMLStyleElement ? existing : document.createElement('style');
    style.id = STYLE_ID;
    style.textContent = css;
    if (!style.parentNode) document.head.appendChild(style);
}

export function applyReelPreview(config: ReelTheme): void {
    // Svelte state proxies don't survive structured clone — send a plain object.
    const plain = JSON.parse(JSON.stringify(config)) as ReelTheme;
    applyCSS(generateReelCSS(plain));
    try {
        sender()?.postMessage({ type: 'apply', config: plain } satisfies PreviewMessage);
    } catch {
        // Local preview still works even if cross-context delivery fails.
    }
}

export function clearReelPreview(): void {
    applyCSS(null);
    sender()?.postMessage({ type: 'clear' } satisfies PreviewMessage);
}

export function startListeningForReelPreview(): () => void {
    if (typeof BroadcastChannel === 'undefined') return () => {};
    const ch = new BroadcastChannel(CHANNEL_NAME);
    ch.onmessage = (e: MessageEvent<PreviewMessage>) => {
        if (e.data.type === 'apply') applyCSS(generateReelCSS(e.data.config));
        else if (e.data.type === 'clear') applyCSS(null);
    };
    return () => ch.close();
}
