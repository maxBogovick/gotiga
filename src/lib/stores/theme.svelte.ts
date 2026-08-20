import { writable, derived } from 'svelte/store';
import type { ThemeConfig } from '$lib/types/api';

export const DEFAULT_COLORS: Record<string, string> = {
    'canvas-base':    '#FAF6EE',
    'canvas-raised':  '#FDFAF5',
    'canvas-sunken':  '#F0E9DA',
    'canvas-deep':    '#E8D9C4',
    'ink-primary':    '#2C1710',
    'ink-secondary':  '#5A3420',
    'ink-tertiary':   '#7A5035',
    'ink-muted':      '#A0745A',
    'ink-disabled':   '#C4A088',
    'ember-subtle':   '#FCE8DF',
    'ember-light':    '#F5C5AD',
    'ember-mid':      '#D97B52',
    'ember':          '#C0582C',
    'ember-deep':     '#9A4120',
    'ember-ink':      '#72300E',
    'ochre-subtle':   '#FBF0D4',
    'ochre-light':    '#F5D98A',
    'ochre':          '#B08820',
    'ochre-deep':     '#856615',
    'ochre-ink':      '#5A4310',
    'sage-subtle':    '#EAF0E4',
    'sage':           '#6B8A56',
    'sage-ink':       '#344529',
};

export const DEFAULT_FONTS = {
    display: 'Fraunces',
    body: 'DM Sans',
    serif: 'EB Garamond',
    mono: 'ui-monospace',
};

export const FONT_FALLBACKS: Record<string, string> = {
    display: 'Georgia, serif',
    body: 'system-ui, sans-serif',
    serif: 'Georgia, serif',
    mono: 'monospace',
};

export const DEFAULT_MOTION = {
    durationFast:    '150ms',
    durationDefault: '350ms',
    durationSlow:    '600ms',
    durationGlacial: '1000ms',
};

// Global "keyhole" darkness — depth of the shadow over sealed cards (0..1).
// Mirrors the hard-coded fallback in KeyholeVeil so an unset theme looks the same.
export const DEFAULT_EFFECTS = {
    keyholeDarkness: 0.88,
    keyholeDwellReveal: 0, // seconds of hover before a sealed card self-reveals; 0 = off
    birdCircleColor: '#D4860A',
    birdWalkInterval: 60, // seconds between walking-bird cameos
};

export function makeDefaultConfig(): ThemeConfig {
    return {
        colors: { ...DEFAULT_COLORS },
        fonts: { ...DEFAULT_FONTS } as ThemeConfig['fonts'],
        motion: { ...DEFAULT_MOTION } as ThemeConfig['motion'],
        effects: { ...DEFAULT_EFFECTS, birdCircleColor: DEFAULT_EFFECTS.birdCircleColor } as ThemeConfig['effects'],
    };
}

export function toPlainThemeConfig(config: ThemeConfig): ThemeConfig {
    return {
        colors: { ...(config.colors ?? {}) },
        fonts: { ...(config.fonts ?? {}) } as ThemeConfig['fonts'],
        motion: { ...(config.motion ?? {}) } as ThemeConfig['motion'],
        effects: { ...DEFAULT_EFFECTS, ...(config.effects ?? {}) } as ThemeConfig['effects'],
    };
}

export function generateThemeCSS(config: ThemeConfig): string {
    if (!config) return '';
    const lines: string[] = [':root {'];

    for (const [name, value] of Object.entries(config.colors ?? {})) {
        if (value && value !== DEFAULT_COLORS[name]) {
            lines.push(`  --color-${name}: ${value};`);
        }
    }

    // Legacy aliases used by older page/component CSS. Keep them tied to the
    // editable design tokens so preview changes affect the real interface.
    if (Object.entries(config.colors ?? {}).some(([name, value]) => value && value !== DEFAULT_COLORS[name])) {
        lines.push('  --cream: var(--color-canvas-base);');
        lines.push('  --cream2: var(--color-canvas-raised);');
        lines.push('  --ink: var(--color-ink-primary);');
        lines.push('  --brown: var(--color-ink-primary);');
        lines.push('  --mid: var(--color-ember-deep);');
        lines.push('  --tan: var(--color-ember-ink);');
        lines.push('  --copper: var(--color-ember);');
        lines.push('  --gold: var(--color-ochre);');
        lines.push('  --muted: color-mix(in srgb, var(--color-ink-secondary) 68%, transparent);');
        lines.push('  --muted2: color-mix(in srgb, var(--color-ink-secondary) 40%, transparent);');
        lines.push('  --border: color-mix(in srgb, var(--color-ink-primary) 10%, transparent);');
        lines.push('  --border2: color-mix(in srgb, var(--color-ink-primary) 18%, transparent);');
    }

    const fonts = config.fonts ?? {};
    const retired: Record<string, string> = {
        Inter: 'Instrument Sans',
        'JetBrains Mono': 'ui-monospace',
    };
    for (const role of ['display', 'body', 'serif', 'mono'] as const) {
        const raw = fonts[role];
        const family = raw ? (retired[raw] ?? raw) : raw;
        if (family && family !== DEFAULT_FONTS[role]) {
            lines.push(`  --font-${role}: '${family}', ${FONT_FALLBACKS[role]};`);
        }
    }

    const motion = config.motion ?? {};
    const motionMap: Record<string, string> = {
        durationFast:    'duration-fast',
        durationDefault: 'duration-default',
        durationSlow:    'duration-slow',
        durationGlacial: 'duration-glacial',
    };
    for (const [key, cssName] of Object.entries(motionMap)) {
        const val = (motion as unknown as Record<string, string | null>)[key];
        const def = (DEFAULT_MOTION as Record<string, string>)[key];
        if (val && val !== def) {
            lines.push(`  --${cssName}: ${val};`);
        }
    }

    const darkness = config.effects?.keyholeDarkness;
    if (typeof darkness === 'number' && Number.isFinite(darkness) && darkness !== DEFAULT_EFFECTS.keyholeDarkness) {
        lines.push(`  --kh-darkness: ${darkness};`);
    }

    const birdColor = config.effects?.birdCircleColor;
    if (birdColor && birdColor !== DEFAULT_EFFECTS.birdCircleColor) {
        lines.push(`  --bird-circle-color: ${birdColor};`);
    }

    lines.push('}');
    return lines.length > 2 ? lines.join('\n') : '';
}

export const themeConfig = writable<ThemeConfig>(makeDefaultConfig());
export const themeCSS = derived(themeConfig, ($config) => generateThemeCSS($config));

// ── Hex bridge: maps hard-coded Tailwind arbitrary classes → CSS vars ────────
// Components use bg-[#f8f1e7] / text-[#34251c] etc. (554 places).
// This bridge injects a <style> block that redirects them all to CSS variables,
// so live preview works without migrating every component.

const COLOR_HEX_ALIASES: Array<{ varName: string; hexes: string[] }> = [
    { varName: '--color-canvas-base',    hexes: ['#f8f1e7', '#faf6ee'] },
    { varName: '--color-canvas-raised',  hexes: ['#fff9f0', '#fdfaf5'] },
    { varName: '--color-canvas-sunken',  hexes: ['#f0e9da'] },
    { varName: '--color-canvas-deep',    hexes: ['#e8d9c4'] },
    { varName: '--color-ink-primary',    hexes: ['#34251c', '#2c1710'] },
    { varName: '--color-ink-secondary',  hexes: ['#5f4636', '#5a3420'] },
    { varName: '--color-ink-tertiary',   hexes: ['#7a5035'] },
    { varName: '--color-ink-muted',      hexes: ['#a0745a'] },
    { varName: '--color-ink-disabled',   hexes: ['#c4a088'] },
    { varName: '--color-ember',          hexes: ['#c65f3c', '#c0582c'] },
    { varName: '--color-ember-deep',     hexes: ['#9a4120', '#6f3b24', '#72300e'] },
    { varName: '--color-ember-mid',      hexes: ['#d97b52'] },
    { varName: '--color-ember-light',    hexes: ['#f5c5ad'] },
    { varName: '--color-ember-subtle',   hexes: ['#fce8df'] },
    { varName: '--color-ochre',          hexes: ['#b08820'] },
    { varName: '--color-ochre-deep',     hexes: ['#856615'] },
    { varName: '--color-ochre-light',    hexes: ['#f5d98a'] },
    { varName: '--color-ochre-subtle',   hexes: ['#fbf0d4'] },
    { varName: '--color-sage',           hexes: ['#6b8a56'] },
    { varName: '--color-sage-ink',       hexes: ['#344529'] },
    { varName: '--color-cabinet-wood',   hexes: ['#ceaf86', '#d8c6b1'] },
];

export function generateHexBridgeCSS(): string {
    const lines: string[] = ['/* gotiga-theme-bridge: auto-generated, do not edit */'];
    for (const { varName, hexes } of COLOR_HEX_ALIASES) {
        for (const hex of hexes) {
            const esc = `\\[${hex.replace('#', '\\#')}\\]`;
            lines.push(`.bg-${esc}{background-color:var(${varName})!important}`);
            lines.push(`.text-${esc}{color:var(${varName})!important}`);
            lines.push(`.border-${esc}{border-color:var(${varName})!important}`);
            lines.push(`.ring-${esc}{--tw-ring-color:var(${varName})!important}`);
            lines.push(`.fill-${esc}{fill:var(${varName})!important}`);
            lines.push(`.stroke-${esc}{stroke:var(${varName})!important}`);
            lines.push(`.from-${esc}{--tw-gradient-from:var(${varName})!important}`);
            lines.push(`.to-${esc}{--tw-gradient-to:var(${varName})!important}`);
            // Slash-opacity variants e.g. bg-[#34251c]/10 → class `.bg-\[#34251c\]\/10`
            for (const op of ['5','8','10','12','15','20','25','30','35','40','50','60','70','75','80','85','90']) {
                lines.push(`.bg-${esc}\\/${op}{background-color:color-mix(in srgb,var(${varName}) ${op}%,transparent)!important}`);
            }
        }
    }
    return lines.join('\n');
}

// ── CSS root manipulation ────────────────────────────────────────────────────

export function applyConfigToElement(config: ThemeConfig, root: HTMLElement): void {
    for (const [name, value] of Object.entries(config.colors ?? {})) {
        if (value) root.style.setProperty(`--color-${name}`, value);
    }
    root.style.setProperty('--cream', 'var(--color-canvas-base)');
    root.style.setProperty('--cream2', 'var(--color-canvas-raised)');
    root.style.setProperty('--ink', 'var(--color-ink-primary)');
    root.style.setProperty('--brown', 'var(--color-ink-primary)');
    root.style.setProperty('--mid', 'var(--color-ember-deep)');
    root.style.setProperty('--tan', 'var(--color-ember-ink)');
    root.style.setProperty('--copper', 'var(--color-ember)');
    root.style.setProperty('--gold', 'var(--color-ochre)');
    root.style.setProperty('--muted', 'color-mix(in srgb, var(--color-ink-secondary) 68%, transparent)');
    root.style.setProperty('--muted2', 'color-mix(in srgb, var(--color-ink-secondary) 40%, transparent)');
    root.style.setProperty('--border', 'color-mix(in srgb, var(--color-ink-primary) 10%, transparent)');
    root.style.setProperty('--border2', 'color-mix(in srgb, var(--color-ink-primary) 18%, transparent)');

    const fonts = config.fonts ?? {};
    for (const role of ['display', 'body', 'serif', 'mono'] as const) {
        const family = fonts[role];
        if (family) root.style.setProperty(`--font-${role}`, `'${family}', ${FONT_FALLBACKS[role]}`);
    }
    const motion = config.motion ?? {};
    const motionMap: Record<string, string> = {
        durationFast: 'duration-fast', durationDefault: 'duration-default',
        durationSlow: 'duration-slow', durationGlacial: 'duration-glacial',
    };
    for (const [key, cssName] of Object.entries(motionMap)) {
        const val = (motion as unknown as Record<string, string | null>)[key];
        if (val) root.style.setProperty(`--${cssName}`, val);
    }

    const darkness = config.effects?.keyholeDarkness;
    if (typeof darkness === 'number' && Number.isFinite(darkness)) {
        root.style.setProperty('--kh-darkness', String(darkness));
    }

    const birdColor = config.effects?.birdCircleColor;
    if (birdColor) root.style.setProperty('--bird-circle-color', birdColor);
}

function applyConfigToRoot(config: ThemeConfig): void {
    if (typeof document === 'undefined') return;
    applyConfigToElement(config, document.documentElement);
}

function clearRootInlineStyles(): void {
    if (typeof document === 'undefined') return;
    const root = document.documentElement;
    for (const name of Object.keys(DEFAULT_COLORS)) root.style.removeProperty(`--color-${name}`);
    for (const name of ['cream', 'cream2', 'ink', 'brown', 'mid', 'tan', 'copper', 'gold', 'muted', 'muted2', 'border', 'border2']) {
        root.style.removeProperty(`--${name}`);
    }
    for (const role of ['display', 'body', 'serif', 'mono']) root.style.removeProperty(`--font-${role}`);
    for (const n of ['duration-fast', 'duration-default', 'duration-slow', 'duration-glacial']) {
        root.style.removeProperty(`--${n}`);
    }
    root.style.removeProperty('--bird-circle-color');
}

// ── BroadcastChannel — single shared instance ────────────────────────────────

const CHANNEL_NAME = 'gotiga_theme_preview';
type PreviewMessage = { type: 'apply'; config: ThemeConfig } | { type: 'clear' };

let _senderChannel: BroadcastChannel | null = null;

function getSenderChannel(): BroadcastChannel | null {
    if (typeof BroadcastChannel === 'undefined') return null;
    if (!_senderChannel) _senderChannel = new BroadcastChannel(CHANNEL_NAME);
    return _senderChannel;
}

export function applyLivePreview(config: ThemeConfig): void {
    const plain = toPlainThemeConfig(config);
    applyConfigToRoot(plain);
    try {
        getSenderChannel()?.postMessage({ type: 'apply', config: plain } satisfies PreviewMessage);
    } catch {
        // Some callers pass Svelte state proxies; keep local preview working even
        // if cross-context delivery fails in an older browser/runtime.
    }
}

export function clearLivePreview(): void {
    clearRootInlineStyles();
    getSenderChannel()?.postMessage({ type: 'clear' } satisfies PreviewMessage);
}

// Apply a preview payload: inject the hex bridge (once) then set CSS variables.
// Called from both BroadcastChannel and window.postMessage paths.
export function applyPreviewPayload(config: ThemeConfig, bridgeCSSOverride?: string): void {
    if (typeof document === 'undefined') return;
    const plain = toPlainThemeConfig(config);
    if (!document.getElementById('gotiga-theme-bridge')) {
        const style = document.createElement('style');
        style.id = 'gotiga-theme-bridge';
        style.textContent = bridgeCSSOverride ?? generateHexBridgeCSS();
        document.head.appendChild(style);
    }
    applyConfigToRoot(plain);
}

export function startListeningForPreview(): () => void {
    if (typeof BroadcastChannel === 'undefined') return () => {};
    const ch = new BroadcastChannel(CHANNEL_NAME);
    ch.onmessage = (e: MessageEvent<PreviewMessage>) => {
        if (e.data.type === 'apply') applyPreviewPayload(e.data.config);
        else if (e.data.type === 'clear') clearRootInlineStyles();
    };
    return () => ch.close();
}
