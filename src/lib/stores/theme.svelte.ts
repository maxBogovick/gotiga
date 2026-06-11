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
    mono: 'JetBrains Mono',
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

export function makeDefaultConfig(): ThemeConfig {
    return {
        colors: { ...DEFAULT_COLORS },
        fonts: { ...DEFAULT_FONTS } as ThemeConfig['fonts'],
        motion: { ...DEFAULT_MOTION } as ThemeConfig['motion'],
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

    const fonts = config.fonts ?? {};
    for (const role of ['display', 'body', 'serif', 'mono'] as const) {
        const family = fonts[role];
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

    lines.push('}');
    return lines.length > 2 ? lines.join('\n') : '';
}

export const themeConfig = writable<ThemeConfig>(makeDefaultConfig());
export const themeCSS = derived(themeConfig, ($config) => generateThemeCSS($config));

// ── CSS root manipulation ────────────────────────────────────────────────────

export function applyConfigToElement(config: ThemeConfig, root: HTMLElement): void {
    for (const [name, value] of Object.entries(config.colors ?? {})) {
        if (value) root.style.setProperty(`--color-${name}`, value);
    }
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
}

function applyConfigToRoot(config: ThemeConfig): void {
    if (typeof document === 'undefined') return;
    applyConfigToElement(config, document.documentElement);
}

function clearRootInlineStyles(): void {
    if (typeof document === 'undefined') return;
    const root = document.documentElement;
    for (const name of Object.keys(DEFAULT_COLORS)) root.style.removeProperty(`--color-${name}`);
    for (const role of ['display', 'body', 'serif', 'mono']) root.style.removeProperty(`--font-${role}`);
    for (const n of ['duration-fast', 'duration-default', 'duration-slow', 'duration-glacial']) {
        root.style.removeProperty(`--${n}`);
    }
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
    applyConfigToRoot(config);
    getSenderChannel()?.postMessage({ type: 'apply', config } satisfies PreviewMessage);
}

export function clearLivePreview(): void {
    clearRootInlineStyles();
    getSenderChannel()?.postMessage({ type: 'clear' } satisfies PreviewMessage);
}

export function startListeningForPreview(): () => void {
    if (typeof BroadcastChannel === 'undefined') return () => {};
    const ch = new BroadcastChannel(CHANNEL_NAME);
    ch.onmessage = (e: MessageEvent<PreviewMessage>) => {
        if (e.data.type === 'apply') applyConfigToRoot(e.data.config);
        else if (e.data.type === 'clear') clearRootInlineStyles();
    };
    return () => ch.close();
}
