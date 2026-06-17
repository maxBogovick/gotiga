<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { api } from '$lib/api';
    import { t } from '$lib/i18n';
    import {
        themeConfig,
        DEFAULT_COLORS,
        DEFAULT_FONTS,
        DEFAULT_MOTION,
        FONT_FALLBACKS,
        makeDefaultConfig,
        applyLivePreview,
        clearLivePreview,
        generateHexBridgeCSS,
        toPlainThemeConfig,
    } from '$lib/stores/theme.svelte';
    import type { ThemeConfig } from '$lib/types/api';

    // ── State ───────────────────────────────────────────────────────────────

    let draft = $state<ThemeConfig>(makeDefaultConfig());
    let saving = $state(false);
    let savedOk = $state(false);
    let errorMsg = $state('');
    let activeSection = $state<'colors' | 'fonts' | 'motion'>('colors');
    let colorEditMode = $state<'simple' | 'advanced'>('simple');
    let highlightedToken = $state<string | null>(null);

    // Preview iframe
    let iframeEl = $state<HTMLIFrameElement | null>(null);
    let previewUrl = $state('/');
    let previewLoading = $state(true);
    let previewContainerEl = $state<HTMLDivElement | null>(null);
    let previewContainerWidth = $state(0);

    const PREVIEW_DESIGN_WIDTH = 1280;
    let previewScale = $derived(previewContainerWidth > 0
        ? previewContainerWidth / PREVIEW_DESIGN_WIDTH
        : 0.5);
    let previewHeight = $derived(Math.round(previewContainerWidth / previewScale * 0.78));

    const PREVIEW_PAGES = [
        { path: '/',          label: 'Home' },
        { path: '/figurines', label: 'Archive' },
        { path: '/author',    label: 'Author' },
        { path: '/workshop',  label: 'Workshop' },
    ];

    // ── Font catalog ────────────────────────────────────────────────────────

    type FontEntry = { name: string; preview: string };
    const FONT_CATALOG: Record<'display' | 'body' | 'serif' | 'mono', FontEntry[]> = {
        display: [
            { name: 'Fraunces',           preview: 'Whispers of the Archive' },
            { name: 'Cinzel',             preview: 'Whispers of the Archive · Annals' },
            { name: 'Playfair Display',   preview: 'Whispers of the Archive' },
            { name: 'Cormorant Garamond', preview: 'Whispers of the Archive' },
            { name: 'Uncial Antiqua',     preview: 'Whispers of the Archive' },
            { name: 'Pirata One',         preview: 'Whispers of the Archive' },
            { name: 'Cardo',              preview: 'Whispers of the Archive' },
            { name: 'IM Fell English',    preview: 'Whispers of the Archive' },
        ],
        body: [
            { name: 'DM Sans',        preview: 'Gothic miniatures by hand' },
            { name: 'Inter',          preview: 'Gothic miniatures by hand' },
            { name: 'Raleway',        preview: 'Gothic miniatures by hand' },
            { name: 'Nunito',         preview: 'Gothic miniatures by hand' },
            { name: 'Lato',           preview: 'Gothic miniatures by hand' },
            { name: 'Mulish',         preview: 'Gothic miniatures by hand' },
            { name: 'Source Sans 3',  preview: 'Gothic miniatures by hand' },
        ],
        serif: [
            { name: 'EB Garamond',       preview: 'Figures with their own faces' },
            { name: 'Libre Baskerville', preview: 'Figures with their own faces' },
            { name: 'Lora',              preview: 'Figures with their own faces' },
            { name: 'Merriweather',      preview: 'Figures with their own faces' },
            { name: 'Spectral',          preview: 'Figures with their own faces' },
            { name: 'Crimson Pro',       preview: 'Figures with their own faces' },
        ],
        mono: [
            { name: 'JetBrains Mono',  preview: 'const shadow = true;' },
            { name: 'Fira Code',       preview: 'const shadow = true;' },
            { name: 'Source Code Pro', preview: 'const shadow = true;' },
            { name: 'Courier Prime',   preview: 'const shadow = true;' },
            { name: 'IBM Plex Mono',   preview: 'const shadow = true;' },
        ],
    };

    let fontCustomMode = $state<Record<string, boolean>>({ display: false, body: false, serif: false, mono: false });
    let fontCustomValues = $state<Record<string, string>>({ display: '', body: '', serif: '', mono: '' });

    // ── Color groups ────────────────────────────────────────────────────────

    const COLOR_GROUPS = [
        { key: 'canvas', labelKey: 'adminDesignColorCanvas', tokens: ['canvas-base','canvas-raised','canvas-sunken','canvas-deep'] },
        { key: 'ink',    labelKey: 'adminDesignColorInk',    tokens: ['ink-primary','ink-secondary','ink-tertiary','ink-muted','ink-disabled'] },
        { key: 'ember',  labelKey: 'adminDesignColorEmber',  tokens: ['ember-subtle','ember-light','ember-mid','ember','ember-deep','ember-ink'] },
        { key: 'ochre',  labelKey: 'adminDesignColorOchre',  tokens: ['ochre-subtle','ochre-light','ochre','ochre-deep','ochre-ink'] },
        { key: 'sage',   labelKey: 'adminDesignColorSage',   tokens: ['sage-subtle','sage','sage-ink'] },
    ];

    type TokenImpact = 'High' | 'Medium' | 'Low';
    type TokenMeta = {
        title: string;
        role: string;
        usage: string[];
        impact: TokenImpact;
        selectors: string[];
    };

    const COLOR_TOKEN_META: Record<string, TokenMeta> = {
        'canvas-base': {
            title: 'Page background',
            role: 'Main page canvas behind all content.',
            usage: ['home background', 'archive/detail page shell', 'empty space around content'],
            impact: 'High',
            selectors: ['main', '.page-root', '.detail-backdrop'],
        },
        'canvas-raised': {
            title: 'Raised surfaces',
            role: 'Cards, panels, inputs, and modal surfaces.',
            usage: ['cards and panels', 'image frames', 'modal backgrounds', 'form fields'],
            impact: 'High',
            selectors: ['.image-frame', '.story-modal', '.action-card', '.claim-lookup-input', '.reserved-notice', '.sold-notice'],
        },
        'canvas-sunken': {
            title: 'Inset surfaces',
            role: 'Quiet recessed areas and loading/skeleton surfaces.',
            usage: ['image stages', 'thumb placeholders', 'subtle inactive areas'],
            impact: 'Medium',
            selectors: ['.image-stage', '.topnav-mini-img'],
        },
        'canvas-deep': {
            title: 'Deep canvas',
            role: 'Darker warm surface used for depth and separation.',
            usage: ['deeper decorative backgrounds', 'strong surface contrast'],
            impact: 'Low',
            selectors: ['.page-root', '.detail-backdrop'],
        },
        'ink-primary': {
            title: 'Primary text',
            role: 'Main readable text and dark primary buttons.',
            usage: ['headings', 'body emphasis', 'main CTA buttons', 'strong icons'],
            impact: 'High',
            selectors: ['h1', 'h2', 'h3', 'p', '.figurine-title', '.cta-btn', '.control-btn--cta', '.mobile-cta-btn', '.related-name'],
        },
        'ink-secondary': {
            title: 'Secondary text',
            role: 'Support copy, labels, and metadata.',
            usage: ['eyebrows', 'helper text', 'notes under buttons', 'dates and captions'],
            impact: 'High',
            selectors: ['.eyebrow-year', '.eyebrow-series', '.spec-label', '.cta-note', '.price-on-request', '.claim-head', '.action-card-sub'],
        },
        'ink-tertiary': {
            title: 'Tertiary text',
            role: 'Quiet navigation, inactive controls, and minor UI labels.',
            usage: ['back links', 'inactive nav', 'secondary icons'],
            impact: 'Medium',
            selectors: ['.back-link', '.fig-nav-pill', '.cta-heart'],
        },
        'ink-muted': {
            title: 'Muted text',
            role: 'Very soft text and background-adjacent details.',
            usage: ['low priority notes', 'subtle labels', 'decorative captions'],
            impact: 'Medium',
            selectors: ['small', '.claim-code-small', '.cp-token'],
        },
        'ink-disabled': {
            title: 'Disabled text',
            role: 'Unavailable and disabled states.',
            usage: ['disabled controls', 'inactive text', 'unavailable UI'],
            impact: 'Low',
            selectors: ['button:disabled', '.fig-nav-pill--off'],
        },
        'ember-subtle': {
            title: 'Accent background',
            role: 'Soft terracotta background behind accent content.',
            usage: ['sold badge background', 'grimoire panels', 'saved/hover accent fills'],
            impact: 'Medium',
            selectors: ['.status-pill--sold', '.grimoire-trigger', '.cta-heart:hover', '.cta-heart--saved'],
        },
        'ember-light': {
            title: 'Light accent',
            role: 'Pale terracotta decoration and gentle gradients.',
            usage: ['soft decorative fills', 'light accents', 'transition areas'],
            impact: 'Low',
            selectors: ['.grimoire-trigger', '.page-root', '.detail-backdrop'],
        },
        'ember-mid': {
            title: 'Mid accent',
            role: 'Intermediate accent used when the main accent needs depth.',
            usage: ['secondary accent states', 'hover blends', 'warm transitions'],
            impact: 'Low',
            selectors: ['.grimoire-icon', '.action-card:hover .action-card-icon'],
        },
        ember: {
            title: 'Primary accent',
            role: 'Main terracotta brand accent.',
            usage: ['active tabs', 'links', 'icons', 'decorative lines', 'hover borders'],
            impact: 'High',
            selectors: ['a', '.sec-label', '.topnav-dot--on', '.spec-icon', '.related-card:hover .related-name', '.control-btn--eye-pulse', '.cp-link--reschedule'],
        },
        'ember-deep': {
            title: 'Dark accent',
            role: 'Strong terracotta for hover and emphasis.',
            usage: ['primary button hover', 'short lore text', 'deep accent states'],
            impact: 'High',
            selectors: ['.cta-btn:hover', '.mobile-cta-btn:hover', '.control-btn--cta:hover', '.lore-short'],
        },
        'ember-ink': {
            title: 'Text on accent',
            role: 'Readable text color placed on terracotta backgrounds.',
            usage: ['sold labels', 'accent badge text', 'warm notice text'],
            impact: 'Medium',
            selectors: ['.status-pill--sold', '.reserved-sub'],
        },
        'ochre-subtle': {
            title: 'Gold background',
            role: 'Soft gold background for reservations and warm notices.',
            usage: ['reserved notice background', 'gold accent panels'],
            impact: 'Medium',
            selectors: ['.reserved-notice', '.status-pill--reserved'],
        },
        'ochre-light': {
            title: 'Light gold',
            role: 'Pale gold decoration and warm secondary gradients.',
            usage: ['soft gold accents', 'decorative highlights'],
            impact: 'Low',
            selectors: ['.reserved-notice', '.page-root'],
        },
        ochre: {
            title: 'Gold accent',
            role: 'Secondary warm accent for reserved/exhibition language.',
            usage: ['reserved icons', 'gold badges', 'secondary decorative accents'],
            impact: 'Medium',
            selectors: ['.reserved-icon', '.status-pill--reserved', '.price-on-request'],
        },
        'ochre-deep': {
            title: 'Dark gold',
            role: 'Darker gold for hover, borders, and contrast.',
            usage: ['gold hover states', 'deep reserved accents'],
            impact: 'Low',
            selectors: ['.reserved-title', '.status-pill--reserved'],
        },
        'ochre-ink': {
            title: 'Text on gold',
            role: 'Readable text on gold backgrounds.',
            usage: ['reserved label text', 'gold notice titles'],
            impact: 'Medium',
            selectors: ['.reserved-title', '.status-pill--reserved'],
        },
        'sage-subtle': {
            title: 'Nature background',
            role: 'Soft green background for available/success states.',
            usage: ['available badge background', 'success/available surfaces'],
            impact: 'Medium',
            selectors: ['.status-pill--available'],
        },
        sage: {
            title: 'Nature accent',
            role: 'Green accent for availability and positive states.',
            usage: ['available status', 'mobile availability label', 'positive markers'],
            impact: 'Medium',
            selectors: ['.status-pill--available', '.mobile-cta-status'],
        },
        'sage-ink': {
            title: 'Text on nature',
            role: 'Readable text on green backgrounds.',
            usage: ['available badge text', 'positive status labels'],
            impact: 'Medium',
            selectors: ['.status-pill--available'],
        },
    };

    const COLOR_ZONES = [
        {
            key: 'backgrounds',
            title: 'Backgrounds & surfaces',
            description: 'Controls the warm paper base, cards, panels, modals, and recessed image areas.',
            previewPath: '/',
            tokens: ['canvas-base', 'canvas-raised', 'canvas-sunken', 'canvas-deep'],
        },
        {
            key: 'text',
            title: 'Text hierarchy',
            description: 'Controls headings, body copy, labels, quiet captions, and disabled states.',
            previewPath: '/',
            tokens: ['ink-primary', 'ink-secondary', 'ink-tertiary', 'ink-muted', 'ink-disabled'],
        },
        {
            key: 'primary-accent',
            title: 'Primary accent',
            description: 'Terracotta brand accent used for active UI, links, icons, CTA hover states, and decorative lines.',
            previewPath: '/',
            tokens: ['ember-subtle', 'ember-light', 'ember-mid', 'ember', 'ember-deep', 'ember-ink'],
        },
        {
            key: 'gold-accent',
            title: 'Gold / reserved accent',
            description: 'Secondary gold accent for reserved states, exhibition language, and warm supporting emphasis.',
            previewPath: '/figurines',
            tokens: ['ochre-subtle', 'ochre-light', 'ochre', 'ochre-deep', 'ochre-ink'],
        },
        {
            key: 'nature-status',
            title: 'Available / nature status',
            description: 'Green system color used for available, success, and positive status states.',
            previewPath: '/figurines',
            tokens: ['sage-subtle', 'sage', 'sage-ink'],
        },
    ];

    const MOTION_FIELDS: Array<{ key: keyof ThemeConfig['motion']; labelKey: string }> = [
        { key: 'durationFast',    labelKey: 'adminDesignDurationFast' },
        { key: 'durationDefault', labelKey: 'adminDesignDurationDefault' },
        { key: 'durationSlow',    labelKey: 'adminDesignDurationSlow' },
        { key: 'durationGlacial', labelKey: 'adminDesignDurationGlacial' },
    ];

    // ── Lifecycle ───────────────────────────────────────────────────────────

    onMount(() => {
        api.getThemeConfig()
            .then(remote => {
                draft = mergeWithDefaults(remote);
                for (const role of ['display', 'body', 'serif', 'mono'] as const) {
                    const current = draft.fonts[role] ?? DEFAULT_FONTS[role];
                    if (!FONT_CATALOG[role].some(f => f.name === current)) {
                        fontCustomMode[role] = true;
                        fontCustomValues[role] = current;
                    }
                }
            })
            .catch(() => { draft = makeDefaultConfig(); });
    });

    // ResizeObserver to track preview container width reactively
    $effect(() => {
        const el = previewContainerEl;
        if (!el) return;
        previewContainerWidth = el.getBoundingClientRect().width;
        const ro = new ResizeObserver(entries => {
            previewContainerWidth = entries[0].contentRect.width;
        });
        ro.observe(el);
        return () => ro.disconnect();
    });

    onDestroy(() => clearLivePreview());

    // ── Preview iframe ──────────────────────────────────────────────────────

    let hexBridgeCSS: string | null = null;

    function getBridgeCSS(): string {
        if (!hexBridgeCSS) hexBridgeCSS = generateHexBridgeCSS();
        return hexBridgeCSS;
    }

    // Send theme to the iframe via postMessage (works regardless of hydration timing)
    function postThemeToIframe(config: ThemeConfig) {
        const plain = toPlainThemeConfig(config);
        iframeEl?.contentWindow?.postMessage(
            { type: 'gotiga-preview', config: plain, bridgeCSS: getBridgeCSS() },
            '*'
        );
    }

    function onIframeLoad() {
        previewLoading = false;
        // Wait for SvelteKit to hydrate inside the iframe before sending the initial state
        setTimeout(() => {
            postThemeToIframe(draft);
            if (highlightedToken) previewToken(highlightedToken);
        }, 350);
    }

    function navigatePreview(path: string) {
        previewUrl = path;
        previewLoading = true;
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    function mergeWithDefaults(remote: ThemeConfig): ThemeConfig {
        return {
            colors: { ...DEFAULT_COLORS, ...remote.colors },
            fonts: {
                display: remote.fonts?.display ?? DEFAULT_FONTS.display,
                body:    remote.fonts?.body    ?? DEFAULT_FONTS.body,
                serif:   remote.fonts?.serif   ?? DEFAULT_FONTS.serif,
                mono:    remote.fonts?.mono    ?? DEFAULT_FONTS.mono,
            },
            motion: {
                durationFast:    remote.motion?.durationFast    ?? DEFAULT_MOTION.durationFast,
                durationDefault: remote.motion?.durationDefault ?? DEFAULT_MOTION.durationDefault,
                durationSlow:    remote.motion?.durationSlow    ?? DEFAULT_MOTION.durationSlow,
                durationGlacial: remote.motion?.durationGlacial ?? DEFAULT_MOTION.durationGlacial,
            },
        };
    }

    function tokenLabel(token: string): string {
        const fallback = token.split('-').slice(1).join(' ') || token;
        return COLOR_TOKEN_META[token]?.title ?? fallback;
    }

    function tokenShortName(token: string): string {
        return token.split('-').slice(1).join(' ') || token;
    }

    function tokenMeta(token: string): TokenMeta {
        return COLOR_TOKEN_META[token] ?? {
            title: tokenLabel(token),
            role: token,
            usage: [token],
            impact: 'Low',
            selectors: [],
        };
    }

    function impactClass(impact: TokenImpact): string {
        if (impact === 'High') return 'bg-[#c65f3c]/10 text-[#9a4120] border-[#c65f3c]/25';
        if (impact === 'Medium') return 'bg-[#b08820]/10 text-[#856615] border-[#b08820]/25';
        return 'bg-[#34251c]/5 text-[#5f4636]/55 border-[#34251c]/10';
    }

    function highlightCSS(token: string): string {
        const meta = tokenMeta(token);
        const selectors = meta.selectors.length ? meta.selectors : ['main'];
        const label = `${meta.title}: ${meta.usage.join(', ')}`.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
        return `
            ${selectors.join(',\n')} {
                outline: 2px solid #00dffd !important;
                outline-offset: 3px !important;
                box-shadow: 0 0 0 5px rgba(0, 223, 253, 0.20), 0 0 22px rgba(0, 223, 253, 0.28) !important;
                transition: outline-color 120ms ease, box-shadow 120ms ease !important;
            }
            body::after {
                content: "${label}";
                position: fixed;
                right: 18px;
                bottom: 18px;
                z-index: 2147483647;
                max-width: min(420px, calc(100vw - 36px));
                padding: 10px 12px;
                border: 1px solid rgba(0, 223, 253, 0.42);
                background: rgba(44, 23, 16, 0.92);
                color: #f8f1e7;
                font: 600 11px/1.45 system-ui, sans-serif;
                letter-spacing: 0.02em;
                box-shadow: 0 12px 34px rgba(44, 23, 16, 0.28);
                pointer-events: none;
            }
        `;
    }

    function previewToken(token: string | null) {
        highlightedToken = token;
        iframeEl?.contentWindow?.postMessage(
            token
                ? { type: 'gotiga-highlight', token, css: highlightCSS(token) }
                : { type: 'gotiga-highlight-clear' },
            '*'
        );
    }

    function showZoneUsage(zone: { previewPath: string; tokens: string[] }) {
        if (previewUrl !== zone.previewPath) navigatePreview(zone.previewPath);
        previewToken(zone.tokens[0]);
    }

    function applyAll(config: ThemeConfig) {
        const plain = toPlainThemeConfig(config);
        applyLivePreview(plain);        // BroadcastChannel → iframe + other tabs (bridge injected by listener)
        postThemeToIframe(plain);       // postMessage → iframe directly (guarantees delivery)
    }

    function setColor(token: string, value: string) {
        draft = {
            ...draft,
            colors: { ...draft.colors, [token]: value },
        };
        applyAll(draft);
    }

    function resetColor(token: string) {
        draft = {
            ...draft,
            colors: { ...draft.colors, [token]: DEFAULT_COLORS[token] },
        };
        applyAll(draft);
    }

    function isColorModified(token: string): boolean {
        return (draft.colors[token] ?? DEFAULT_COLORS[token]) !== DEFAULT_COLORS[token];
    }

    function getSelectedFont(role: string): string {
        return draft.fonts[role as keyof typeof draft.fonts] ?? DEFAULT_FONTS[role as keyof typeof DEFAULT_FONTS];
    }

    const loadedFonts = new Set<string>();
    function loadGoogleFont(family: string, role: string) {
        if (typeof document === 'undefined') return;
        const key = family.toLowerCase();
        if (loadedFonts.has(key)) return;
        loadedFonts.add(key);
        const weights = role === 'mono' ? '400' : '300;400;500';
        const url = `https://fonts.googleapis.com/css2?family=${encodeURIComponent(family)}:wght@${weights}&display=swap`;
        const link = document.createElement('link');
        link.rel = 'stylesheet';
        link.href = url;
        document.head.appendChild(link);
        // Also inject into iframe via postMessage so font loads inside it
        iframeEl?.contentWindow?.postMessage({ type: 'gotiga-font', href: url }, '*');
    }

    function loadAllCatalogFonts() {
        for (const [role, fonts] of Object.entries(FONT_CATALOG)) {
            for (const f of fonts) loadGoogleFont(f.name, role);
        }
    }

    function onFontSelect(role: keyof ThemeConfig['fonts'], value: string) {
        if (value === '__custom__') {
            fontCustomMode[role] = true;
            fontCustomValues[role] = draft.fonts[role] ?? '';
        } else {
            fontCustomMode[role] = false;
            draft = {
                ...draft,
                fonts: { ...draft.fonts, [role]: value },
            };
            loadGoogleFont(value, role);
            applyAll(draft);
        }
    }

    function onCustomFontInput(role: keyof ThemeConfig['fonts'], value: string) {
        fontCustomValues[role] = value;
        draft = {
            ...draft,
            fonts: { ...draft.fonts, [role]: value },
        };
        if (value.length > 2) loadGoogleFont(value, role);
        applyAll(draft);
    }

    function getDurationMs(key: keyof ThemeConfig['motion']): number {
        return parseInt(draft.motion[key] ?? '0', 10) || 0;
    }

    function setMotion(key: keyof ThemeConfig['motion'], ms: number) {
        draft = {
            ...draft,
            motion: { ...draft.motion, [key]: `${ms}ms` },
        };
        applyAll(draft);
    }

    async function handleSave() {
        saving = true; errorMsg = ''; savedOk = false;
        try {
            const result = await api.saveThemeConfig(draft);
            themeConfig.set(result);
            clearLivePreview();
            savedOk = true;
            setTimeout(() => savedOk = false, 2500);
        } catch {
            errorMsg = $t('adminDesignError');
        } finally {
            saving = false;
        }
    }

    function handleReset() {
        draft = makeDefaultConfig();
        for (const role of ['display', 'body', 'serif', 'mono']) {
            fontCustomMode[role] = false;
            fontCustomValues[role] = '';
        }
        applyAll(draft);
    }

    let hasChanges = $derived(
        JSON.stringify(draft) !== JSON.stringify(makeDefaultConfig())
    );
</script>

<!-- Split layout: controls left, preview right -->
<div class="h-full flex gap-0 overflow-hidden">

    <!-- ══ LEFT: Controls panel ════════════════════════════════════════════ -->
    <div class="w-[340px] shrink-0 flex flex-col overflow-hidden border-r border-[#34251c]/10">

        <!-- Header -->
        <div class="flex items-center justify-between shrink-0 px-4 py-3 border-b border-[#34251c]/10">
            <div>
                <h2 class="text-[11px] uppercase tracking-widest text-[#5f4636] font-semibold">{$t('adminDesignTitle')}</h2>
                <p class="text-[9px] text-[#5f4636]/40 mt-0.5">{$t('adminDesignPreviewNote')}</p>
            </div>
            <div class="flex items-center gap-1.5">
                {#if hasChanges}
                    <button onclick={handleReset}
                        class="btn-gothic text-[9px] opacity-60 hover:opacity-100 py-1 px-2">
                        {$t('adminDesignReset')}
                    </button>
                {/if}
                <button onclick={handleSave} disabled={saving}
                    class="btn-gothic text-[9px] py-1 px-2 min-w-[60px]
                        {savedOk ? 'text-green-700 border-green-700/40' : ''}
                        {saving ? 'opacity-50' : ''}">
                    {#if saving}…{:else if savedOk}✓{:else}{$t('adminDesignSave')}{/if}
                </button>
            </div>
        </div>

        {#if errorMsg}
            <p class="shrink-0 text-red-700 text-[9px] mx-3 my-1.5 px-2 py-1 bg-red-50 border border-red-200">{errorMsg}</p>
        {/if}

        <!-- Section tabs -->
        <div class="flex shrink-0 border-b border-[#34251c]/10">
            {#each [
                ['colors', $t('adminDesignColorsSection')],
                ['fonts',  $t('adminDesignFontsSection')],
                ['motion', $t('adminDesignMotionSection')],
            ] as [s, label]}
                <button
                    onclick={() => {
                        activeSection = s as typeof activeSection;
                        if (s === 'fonts') loadAllCatalogFonts();
                    }}
                    class="flex-1 py-2 text-[9px] uppercase tracking-wider border-b-2 -mb-px transition-colors
                        {activeSection === s
                            ? 'border-[#c65f3c] text-[#34251c] font-semibold'
                            : 'border-transparent text-[#5f4636]/50 hover:text-[#5f4636]'}"
                >{label}</button>
            {/each}
        </div>

        <!-- Scrollable form area -->
        <div class="flex-1 overflow-y-auto px-4 py-4">

            <!-- ── COLORS ─────────────────────────────────────────────── -->
            {#if activeSection === 'colors'}
                <div class="flex flex-col gap-4">
                    <div class="flex items-center gap-1 rounded-sm border border-[#d8c6b1]/60 bg-[#fff9f0]/55 p-1">
                        <button
                            onclick={() => colorEditMode = 'simple'}
                            class="flex-1 px-2 py-1 text-[9px] uppercase tracking-wider transition-colors
                                {colorEditMode === 'simple'
                                    ? 'bg-[#34251c] text-[#f8f1e7]'
                                    : 'text-[#5f4636]/55 hover:text-[#34251c]'}"
                        >Simple</button>
                        <button
                            onclick={() => colorEditMode = 'advanced'}
                            class="flex-1 px-2 py-1 text-[9px] uppercase tracking-wider transition-colors
                                {colorEditMode === 'advanced'
                                    ? 'bg-[#34251c] text-[#f8f1e7]'
                                    : 'text-[#5f4636]/55 hover:text-[#34251c]'}"
                        >Advanced</button>
                    </div>

                    <div class="min-h-[58px] border px-2.5 py-2 transition-colors
                        {highlightedToken
                            ? 'border-[#00dffd]/35 bg-[#00dffd]/8'
                            : 'border-[#d8c6b1]/45 bg-[#fff9f0]/35'}"
                    >
                        {#if highlightedToken}
                            {@const meta = tokenMeta(highlightedToken)}
                            <div class="flex items-start justify-between gap-2">
                                <div class="min-w-0">
                                    <p class="m-0 truncate text-[9px] font-bold uppercase tracking-wider text-[#34251c]">{meta.title}</p>
                                    <p class="m-0 mt-0.5 line-clamp-2 text-[8.5px] leading-snug text-[#5f4636]/70">{meta.usage.join(' · ')}</p>
                                </div>
                                <button onclick={() => previewToken(null)}
                                    class="shrink-0 text-[10px] leading-none text-[#5f4636]/45 hover:text-[#c65f3c]"
                                    title="Clear preview highlight">×</button>
                            </div>
                        {:else}
                            <p class="m-0 text-[9px] font-bold uppercase tracking-wider text-[#34251c]/55">Preview usage</p>
                            <p class="m-0 mt-0.5 text-[8.5px] leading-snug text-[#5f4636]/45">Hover a color or press Show to highlight where it is used.</p>
                        {/if}
                    </div>

                    {#if colorEditMode === 'simple'}
                        {#each COLOR_ZONES as zone}
                            <section class="border border-[#d8c6b1]/55 bg-[#fff9f0]/45 p-3">
                                <div class="mb-2.5 flex items-start justify-between gap-3">
                                    <div>
                                        <h3 class="m-0 text-[10px] font-bold uppercase tracking-widest text-[#34251c]">
                                            {zone.title}
                                        </h3>
                                        <p class="m-0 mt-1 text-[9px] leading-snug text-[#5f4636]/60">
                                            {zone.description}
                                        </p>
                                    </div>
                                    <button
                                        onclick={() => showZoneUsage(zone)}
                                        class="shrink-0 border border-[#34251c]/15 px-2 py-1 text-[8px] uppercase tracking-wider text-[#34251c]/65 hover:border-[#c65f3c]/40 hover:text-[#c65f3c]"
                                    >Show</button>
                                </div>

                                <div class="mb-3 grid grid-cols-[1fr_auto] overflow-hidden border border-[#d8c6b1]/40">
                                    <div class="min-w-0 p-2" style="background: {draft.colors[zone.tokens[0]] ?? DEFAULT_COLORS[zone.tokens[0]]}; color: {draft.colors['ink-primary'] ?? DEFAULT_COLORS['ink-primary']}">
                                        <div class="text-[13px] leading-none" style="font-family: var(--font-display)">Preview sample</div>
                                        <div class="mt-1 text-[8px] uppercase tracking-widest opacity-60">surfaces · text · accent</div>
                                    </div>
                                    <div class="flex items-center gap-1 px-2" style="background: {draft.colors['canvas-raised'] ?? DEFAULT_COLORS['canvas-raised']}">
                                        <span class="h-5 w-5 rounded-full border border-black/10" style="background: {draft.colors[zone.tokens[Math.min(1, zone.tokens.length - 1)]] ?? DEFAULT_COLORS[zone.tokens[Math.min(1, zone.tokens.length - 1)]]}"></span>
                                        <span class="h-5 w-5 rounded-full border border-black/10" style="background: {draft.colors[zone.tokens[Math.min(2, zone.tokens.length - 1)]] ?? DEFAULT_COLORS[zone.tokens[Math.min(2, zone.tokens.length - 1)]]}"></span>
                                    </div>
                                </div>

                                <div class="grid grid-cols-2 gap-2">
                                    {#each zone.tokens as token}
                                        {@const val = draft.colors[token] ?? DEFAULT_COLORS[token]}
                                        {@const modified = isColorModified(token)}
                                        {@const meta = tokenMeta(token)}
                                        <div
                                            role="presentation"
                                            class="border p-2 transition-colors
                                                {highlightedToken === token
                                                    ? 'border-[#00dffd]/65 bg-[#00dffd]/8'
                                                    : 'border-[#d8c6b1]/45 bg-[#f8f1e7]/35 hover:border-[#c65f3c]/35'}"
                                            onmouseenter={() => previewToken(token)}
                                        >
                                            <div class="mb-1.5 flex items-start justify-between gap-1">
                                                <div class="min-w-0">
                                                    <p class="m-0 truncate text-[9px] font-semibold text-[#34251c]">{meta.title}</p>
                                                    <p class="m-0 mt-0.5 text-[7.5px] uppercase tracking-wide text-[#5f4636]/40">{token}</p>
                                                </div>
                                                <span class="shrink-0 border px-1 py-0.5 text-[7px] uppercase tracking-wide {impactClass(meta.impact)}">{meta.impact}</span>
                                            </div>
                                            <label class="relative block cursor-pointer" title={meta.role}>
                                                <div
                                                    class="h-8 w-full border transition-all duration-150
                                                        {modified
                                                            ? 'border-[#c65f3c]/70 ring-1 ring-[#c65f3c]/30'
                                                            : 'border-[#d8c6b1]/60'}"
                                                    style="background: {val}"
                                                ></div>
                                                <input
                                                    type="color"
                                                    value={val}
                                                    oninput={(e) => setColor(token, (e.target as HTMLInputElement).value)}
                                                    class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                                                />
                                            </label>
                                            <div class="mt-1 flex items-center gap-0.5">
                                                <input
                                                    type="text"
                                                    value={val.toUpperCase()}
                                                    oninput={(e) => {
                                                        const v = (e.target as HTMLInputElement).value;
                                                        if (/^#[0-9a-fA-F]{6}$/.test(v)) setColor(token, v);
                                                    }}
                                                    maxlength={7}
                                                    class="w-full border-b border-[#d8c6b1]/30 bg-transparent py-0.5 font-mono text-[8px] uppercase leading-none text-[#34251c]/70
                                                        focus:border-[#c65f3c] focus:outline-none"
                                                />
                                                {#if modified}
                                                    <button
                                                        onclick={() => resetColor(token)}
                                                        class="shrink-0 pb-0.5 text-[9px] leading-none text-[#5f4636]/30 hover:text-[#c65f3c]"
                                                        title="Reset">↩</button>
                                                {/if}
                                            </div>
                                            <p class="m-0 mt-1.5 line-clamp-2 text-[8px] leading-snug text-[#5f4636]/55">
                                                {meta.usage.join(' · ')}
                                            </p>
                                        </div>
                                    {/each}
                                </div>
                            </section>
                        {/each}
                    {:else}
                        <div class="flex flex-col gap-6">
                            {#each COLOR_GROUPS as group}
                                <div>
                                    <h3 class="mb-2 text-[8px] font-bold uppercase tracking-widest text-[#5f4636]/40">
                                        {$t(group.labelKey as any)}
                                    </h3>
                                    <div class="grid grid-cols-2 gap-2">
                                        {#each group.tokens as token}
                                            {@const val = draft.colors[token] ?? DEFAULT_COLORS[token]}
                                            {@const modified = isColorModified(token)}
                                            {@const meta = tokenMeta(token)}
                                            <div
                                                role="presentation"
                                                class="border p-2 transition-colors
                                                    {highlightedToken === token
                                                        ? 'border-[#00dffd]/65 bg-[#00dffd]/8'
                                                        : 'border-[#d8c6b1]/45 hover:border-[#c65f3c]/35'}"
                                                onmouseenter={() => previewToken(token)}
                                            >
                                                <div class="mb-1 flex items-center justify-between gap-1">
                                                    <span class="truncate text-[8.5px] font-semibold text-[#34251c]">{meta.title}</span>
                                                    <span class="shrink-0 text-[7px] uppercase text-[#5f4636]/35">{tokenShortName(token)}</span>
                                                </div>
                                                <label class="relative block cursor-pointer" title={token}>
                                                    <div
                                                        class="h-7 w-full border transition-all duration-150
                                                            {modified
                                                                ? 'border-[#c65f3c]/70 ring-1 ring-[#c65f3c]/30'
                                                                : 'border-[#d8c6b1]/50'}"
                                                        style="background: {val}"
                                                    ></div>
                                                    <input
                                                        type="color"
                                                        value={val}
                                                        oninput={(e) => setColor(token, (e.target as HTMLInputElement).value)}
                                                        class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                                                    />
                                                </label>
                                                <div class="mt-1 flex items-center gap-0.5">
                                                    <input
                                                        type="text"
                                                        value={val.toUpperCase()}
                                                        oninput={(e) => {
                                                            const v = (e.target as HTMLInputElement).value;
                                                            if (/^#[0-9a-fA-F]{6}$/.test(v)) setColor(token, v);
                                                        }}
                                                        maxlength={7}
                                                        class="w-full border-b border-[#d8c6b1]/30 bg-transparent py-0.5 font-mono text-[8px] uppercase leading-none text-[#34251c]/70
                                                            focus:border-[#c65f3c] focus:outline-none"
                                                    />
                                                    {#if modified}
                                                        <button
                                                            onclick={() => resetColor(token)}
                                                            class="shrink-0 pb-0.5 text-[9px] leading-none text-[#5f4636]/30 hover:text-[#c65f3c]"
                                                            title="Reset">↩</button>
                                                    {/if}
                                                </div>
                                                <p class="m-0 mt-1 text-[7.5px] leading-snug text-[#5f4636]/50">
                                                    {meta.usage.join(' · ')}
                                                </p>
                                            </div>
                                        {/each}
                                    </div>
                                </div>
                            {/each}
                        </div>
                    {/if}
                </div>

            <!-- ── FONTS ──────────────────────────────────────────────── -->
            {:else if activeSection === 'fonts'}
                <div class="flex flex-col gap-5">
                    {#each (['display','body','serif','mono'] as const) as role}
                        {@const labelKey = role === 'display' ? 'adminDesignFontDisplay'
                                         : role === 'body'    ? 'adminDesignFontBody'
                                         : role === 'serif'   ? 'adminDesignFontSerif'
                                         :                      'adminDesignFontMono'}
                        {@const currentFont = getSelectedFont(role)}
                        {@const isCustom = fontCustomMode[role]}
                        <div class="border border-[#d8c6b1]/50 p-2.5 bg-[#fff9f0]/40">
                            <label class="label mb-1.5" for="font-{role}">{$t(labelKey as any)}</label>

                            <select
                                id="font-{role}"
                                value={isCustom ? '__custom__' : currentFont}
                                onchange={(e) => onFontSelect(role, (e.target as HTMLSelectElement).value)}
                                class="w-full border border-[#d8c6b1] bg-white text-[#34251c] text-[11px] px-2 py-1
                                    focus:outline-none focus:border-[#c65f3c] cursor-pointer mb-1.5"
                            >
                                {#each FONT_CATALOG[role] as { name }}
                                    <option value={name}>{name}</option>
                                {/each}
                                <option value="__custom__">— Custom…</option>
                            </select>

                            {#if isCustom}
                                <input
                                    type="text"
                                    value={fontCustomValues[role]}
                                    oninput={(e) => onCustomFontInput(role, (e.target as HTMLInputElement).value)}
                                    placeholder="Any Google Font name…"
                                    class="w-full border border-[#d8c6b1] bg-white text-[#34251c] text-[11px] px-2 py-1
                                        focus:outline-none focus:border-[#c65f3c] mb-1.5"
                                />
                            {/if}

                            <!-- Font preview line -->
                            <div
                                class="px-2 py-1.5 bg-[#f8f1e7] border border-[#d8c6b1]/30 text-[#34251c] overflow-hidden whitespace-nowrap text-ellipsis
                                    {role === 'mono' ? 'text-[11px]' : role === 'display' ? 'text-base' : 'text-[12px]'}"
                                style="font-family: '{currentFont}', {FONT_FALLBACKS[role]}"
                            >
                                {FONT_CATALOG[role]?.find(f => f.name === currentFont)?.preview
                                    ?? (role === 'mono' ? 'const shadow = true;' : 'Whispers of the Archive')}
                            </div>

                            {#if currentFont !== DEFAULT_FONTS[role]}
                                <p class="text-[8px] text-[#5f4636]/40 mt-1">
                                    default: {DEFAULT_FONTS[role]}
                                    <button
                                        onclick={() => {
                                            draft = {
                                                ...draft,
                                                fonts: { ...draft.fonts, [role]: DEFAULT_FONTS[role] },
                                            };
                                            fontCustomMode[role] = false;
                                            applyAll(draft);
                                        }}
                                        class="ml-1 text-[#c65f3c]/60 hover:text-[#c65f3c] underline">reset</button>
                                </p>
                            {/if}
                        </div>
                    {/each}
                </div>

            <!-- ── MOTION ─────────────────────────────────────────────── -->
            {:else if activeSection === 'motion'}
                <div class="flex flex-col gap-5">
                    {#each MOTION_FIELDS as { key, labelKey }}
                        {@const ms = getDurationMs(key)}
                        {@const defMs = parseInt(DEFAULT_MOTION[key], 10)}
                        {@const modified = ms !== defMs}
                        <div>
                            <div class="flex items-center justify-between mb-1.5">
                                <label class="label !mb-0" for="motion-{key}">{$t(labelKey as any)}</label>
                                <div class="flex items-center gap-1.5">
                                    <span class="text-[10px] font-mono text-[#34251c] tabular-nums">{ms}ms</span>
                                    {#if modified}
                                        <button onclick={() => setMotion(key, defMs)}
                                            class="text-[#5f4636]/40 hover:text-[#c65f3c] text-[10px]"
                                            title="Reset to {defMs}ms">↩</button>
                                    {/if}
                                </div>
                            </div>
                            <input id="motion-{key}" type="range" min="50" max="3000" step="50"
                                value={ms}
                                oninput={(e) => setMotion(key, parseInt((e.target as HTMLInputElement).value, 10))}
                                class="w-full accent-[#c65f3c]"
                            />
                            <div class="flex justify-between text-[7.5px] text-[#5f4636]/30 mt-0.5">
                                <span>50ms</span>
                                <span>default: {defMs}ms</span>
                                <span>3000ms</span>
                            </div>
                            <!-- Motion demo square -->
                            <div class="mt-2 h-5 relative overflow-hidden">
                                <div
                                    class="w-4 h-4 rounded-sm absolute left-0 bg-[#c65f3c]/25 hover:left-[calc(100%-1rem)] hover:bg-[#c65f3c]/60 cursor-pointer"
                                    style="transition: left {ms}ms ease, background-color {ms}ms ease"
                                    title="Hover to preview speed"
                                ></div>
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}

        </div>
    </div>

    <!-- ══ RIGHT: Live preview ══════════════════════════════════════════════ -->
    <div class="flex-1 flex flex-col overflow-hidden bg-[#e8ddd4]">

        <!-- Preview toolbar -->
        <div class="shrink-0 flex items-center gap-1 px-3 py-2 bg-[#34251c]/90 border-b border-[#34251c]">
            <!-- Fake browser chrome dots -->
            <div class="flex gap-1 mr-3">
                <span class="w-2.5 h-2.5 rounded-full bg-[#ff5f57] opacity-80"></span>
                <span class="w-2.5 h-2.5 rounded-full bg-[#febc2e] opacity-80"></span>
                <span class="w-2.5 h-2.5 rounded-full bg-[#28c840] opacity-80"></span>
            </div>
            <!-- Page selector -->
            {#each PREVIEW_PAGES as { path, label }}
                <button
                    onclick={() => navigatePreview(path)}
                    class="px-2.5 py-1 text-[9px] uppercase tracking-wider rounded-sm transition-colors
                        {previewUrl === path
                            ? 'bg-[#c65f3c]/80 text-[#f8f1e7]'
                            : 'text-[#f8f1e7]/50 hover:text-[#f8f1e7]/90 hover:bg-white/10'}"
                >{label}</button>
            {/each}
            <!-- Loading indicator -->
            {#if previewLoading}
                <span class="ml-auto text-[8px] text-[#f8f1e7]/40 animate-pulse">loading…</span>
            {:else}
                <span class="ml-auto text-[8px] text-[#f8f1e7]/20">{previewUrl}</span>
            {/if}
        </div>

        <!-- Scaled iframe container -->
        <div class="flex-1 overflow-hidden relative" bind:this={previewContainerEl}>
            {#if previewContainerWidth > 0}
                <!-- Wrapper that clips overflow from scaled iframe -->
                <div
                    class="absolute inset-0 overflow-hidden"
                    style="pointer-events: none"
                >
                    <!-- Re-enable pointer events for iframe itself so links/scroll work -->
                    <div style="
                        width: {PREVIEW_DESIGN_WIDTH}px;
                        transform: scale({previewScale});
                        transform-origin: top left;
                        pointer-events: auto;
                        height: {Math.ceil(previewContainerEl?.getBoundingClientRect().height ?? 800 / previewScale)}px;
                    ">
                        <iframe
                            bind:this={iframeEl}
                            src={previewUrl}
                            title="Site preview"
                            onload={onIframeLoad}
                            class="w-full h-full border-0 bg-[#f8f1e7]"
                            style="height: {Math.ceil((previewContainerEl?.getBoundingClientRect().height ?? 800) / previewScale)}px"
                        ></iframe>
                    </div>
                </div>
            {:else}
                <div class="flex items-center justify-center h-full text-[11px] text-[#5f4636]/30">
                    Loading preview…
                </div>
            {/if}
        </div>

    </div>
</div>

<style>
    .label {
        font-size: 9px;
        text-transform: uppercase;
        letter-spacing: 0.07em;
        color: #5f4636;
        display: block;
        font-weight: 700;
    }
</style>
