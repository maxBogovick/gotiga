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
        applyConfigToElement,
    } from '$lib/stores/theme.svelte';
    import type { ThemeConfig } from '$lib/types/api';

    // ── State ───────────────────────────────────────────────────────────────

    let draft = $state<ThemeConfig>(makeDefaultConfig());
    let saving = $state(false);
    let savedOk = $state(false);
    let errorMsg = $state('');
    let activeSection = $state<'colors' | 'fonts' | 'motion'>('colors');

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
            { name: 'Fraunces',           preview: 'Шёпоты архива · Whispers of the Archive' },
            { name: 'Cinzel',             preview: 'Whispers of the Archive · Annals' },
            { name: 'Playfair Display',   preview: 'Шёпоты архива · Whispers of the Archive' },
            { name: 'Cormorant Garamond', preview: 'Шёпоты архива · Whispers of the Archive' },
            { name: 'Uncial Antiqua',     preview: 'Whispers of the Archive' },
            { name: 'Pirata One',         preview: 'Whispers of the Archive' },
            { name: 'Cardo',              preview: 'Шёпоты архива · Whispers of the Archive' },
            { name: 'IM Fell English',    preview: 'Шёпоты архива · Whispers of the Archive' },
        ],
        body: [
            { name: 'DM Sans',        preview: 'Авторские фигурки ручной работы · Gothic miniatures by hand' },
            { name: 'Inter',          preview: 'Авторские фигурки ручной работы · Gothic miniatures by hand' },
            { name: 'Raleway',        preview: 'Авторские фигурки ручной работы · Gothic miniatures by hand' },
            { name: 'Nunito',         preview: 'Авторские фигурки ручной работы · Gothic miniatures by hand' },
            { name: 'Lato',           preview: 'Авторские фигурки ручной работы · Gothic miniatures by hand' },
            { name: 'Mulish',         preview: 'Авторские фигурки ручной работы · Gothic miniatures by hand' },
            { name: 'Source Sans 3',  preview: 'Авторские фигурки ручной работы · Gothic miniatures by hand' },
        ],
        serif: [
            { name: 'EB Garamond',       preview: 'Фигурки с собственными лицами · Figures with their own faces' },
            { name: 'Libre Baskerville', preview: 'Фигурки с собственными лицами · Figures with their own faces' },
            { name: 'Lora',              preview: 'Фигурки с собственными лицами · Figures with their own faces' },
            { name: 'Merriweather',      preview: 'Фигурки с собственными лицами · Figures with their own faces' },
            { name: 'Spectral',          preview: 'Фигурки с собственными лицами · Figures with their own faces' },
            { name: 'Crimson Pro',       preview: 'Фигурки с собственными лицами · Figures with their own faces' },
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

    function onIframeLoad() {
        previewLoading = false;
        applyThemeToIframe();
    }

    function applyThemeToIframe() {
        const doc = iframeEl?.contentDocument;
        if (!doc) return;
        applyConfigToElement(draft, doc.documentElement);
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
        return token.split('-').slice(1).join(' ') || token;
    }

    function applyAll(config: ThemeConfig) {
        applyLivePreview(config);
        applyThemeToIframe();
    }

    function setColor(token: string, value: string) {
        draft.colors[token] = value;
        applyAll(draft);
    }

    function resetColor(token: string) {
        draft.colors[token] = DEFAULT_COLORS[token];
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
        // Also inject into iframe
        const iframeDoc = iframeEl?.contentDocument;
        if (iframeDoc) {
            const iLink = iframeDoc.createElement('link');
            iLink.rel = 'stylesheet';
            iLink.href = url;
            iframeDoc.head.appendChild(iLink);
        }
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
            draft.fonts[role] = value;
            loadGoogleFont(value, role);
            applyAll(draft);
        }
    }

    function onCustomFontInput(role: keyof ThemeConfig['fonts'], value: string) {
        fontCustomValues[role] = value;
        draft.fonts[role] = value;
        if (value.length > 2) loadGoogleFont(value, role);
        applyAll(draft);
    }

    function getDurationMs(key: keyof ThemeConfig['motion']): number {
        return parseInt(draft.motion[key] ?? '0', 10) || 0;
    }

    function setMotion(key: keyof ThemeConfig['motion'], ms: number) {
        draft.motion[key] = `${ms}ms`;
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
                <div class="flex flex-col gap-6">
                    {#each COLOR_GROUPS as group}
                        <div>
                            <h3 class="text-[8px] uppercase tracking-widest text-[#5f4636]/40 mb-2.5 font-bold">
                                {$t(group.labelKey as any)}
                            </h3>
                            <div class="grid grid-cols-4 gap-2">
                                {#each group.tokens as token}
                                    {@const val = draft.colors[token] ?? DEFAULT_COLORS[token]}
                                    {@const modified = isColorModified(token)}
                                    <div class="flex flex-col gap-1">
                                        <label class="relative cursor-pointer group block" title={token}>
                                            <div
                                                class="h-7 w-full border transition-all duration-150
                                                    {modified
                                                        ? 'border-[#c65f3c]/70 ring-1 ring-[#c65f3c]/30'
                                                        : 'border-[#d8c6b1]/50 group-hover:border-[#d8c6b1]'}"
                                                style="background: {val}"
                                            ></div>
                                            <input
                                                type="color"
                                                value={val}
                                                oninput={(e) => setColor(token, (e.target as HTMLInputElement).value)}
                                                class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                                            />
                                        </label>
                                        <div class="flex items-center gap-0.5">
                                            <input
                                                type="text"
                                                value={val.toUpperCase()}
                                                oninput={(e) => {
                                                    const v = (e.target as HTMLInputElement).value;
                                                    if (/^#[0-9a-fA-F]{6}$/.test(v)) setColor(token, v);
                                                }}
                                                maxlength={7}
                                                class="w-full text-[8px] font-mono text-[#34251c]/70 bg-transparent border-b border-[#d8c6b1]/30
                                                    focus:outline-none focus:border-[#c65f3c] uppercase leading-none py-0.5"
                                            />
                                            {#if modified}
                                                <button
                                                    onclick={() => resetColor(token)}
                                                    class="shrink-0 text-[#5f4636]/30 hover:text-[#c65f3c] text-[9px] leading-none pb-0.5"
                                                    title="Reset">↩</button>
                                            {/if}
                                        </div>
                                        <span class="text-[7.5px] text-[#5f4636]/40 leading-none">{tokenLabel(token)}</span>
                                    </div>
                                {/each}
                            </div>
                        </div>
                    {/each}
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
                                            draft.fonts[role] = DEFAULT_FONTS[role];
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
