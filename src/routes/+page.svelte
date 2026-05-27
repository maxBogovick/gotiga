<script lang="ts">
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { fade, fly } from 'svelte/transition';
    import { cubicOut } from 'svelte/easing';
    import { spring } from 'svelte/motion';
    import { api } from '$lib/api';
    import type { CabinetZone, FigurineListItem } from '$lib/types/api';
    import { t } from '$lib/i18n';
    import LangSwitcher from '$lib/components/LangSwitcher.svelte';
    import FeaturedFigurineCard from '$lib/components/FeaturedFigurineCard.svelte';

    let zones = $state<CabinetZone[]>([]);
    let isLoaded = $state(false);
    let imageLoaded = $state(false);
    let hoveredZone = $state<CabinetZone | null>(null);
    let isNavigating = $state(false);
    let ambientIntensity = $state(1);
    let featuredFigurines = $state<FigurineListItem[]>([]);
    let collectionTotal = $state(0);
    let availableTotal = $state(0);
    let mouseX = $state(0.5);
    let mouseY = $state(0.5);

    const parallaxSpring = spring({ x: 0, y: 0 }, { stiffness: 0.04, damping: 0.45 });

    let imageUrl = $state('/images/cabinet-room.jpg');

    const DEFAULT_ZONES: CabinetZone[] = [
        { id: 'curator', zoneType: 'curator', x: 38, y: 15, width: 24, height: 75, targetRoute: '/author' },
        { id: 'cabinet', zoneType: 'cabinet', x: 76, y: 42, width: 20, height: 45, targetRoute: '/figurines' },
        { id: 'portrait', zoneType: 'portrait', x: 8, y: 28, width: 15, height: 30, targetRoute: '/author' },
        { id: 'windows', zoneType: 'windows', x: 25, y: 2, width: 50, height: 20, targetRoute: '/workshop' },
    ];

    const ZONE_STATIC: Record<string, { icon: string; accent: string; labelKey: string; descKey: string }> = {
        showcase: { icon: '🕯', accent: '#c65f3c', labelKey: 'zoneShowcase', descKey: 'zoneShowcaseDesc' },
        desk:     { icon: '🌙', accent: '#6f7d45', labelKey: 'zoneDesk',     descKey: 'zoneDeskDesc'     },
        shelf:    { icon: '🗝', accent: '#c9a875', labelKey: 'zoneShelf',    descKey: 'zoneShelfDesc'    },
        note:     { icon: '⚰', accent: '#a86124', labelKey: 'zoneNote',     descKey: 'zoneNoteDesc'     },
        curator:  { icon: '⚰', accent: '#a86124', labelKey: 'zoneCurator',  descKey: 'zoneCuratorDesc'  },
        cabinet:  { icon: '🕯', accent: '#c65f3c', labelKey: 'zoneCabinet',  descKey: 'zoneCabinetDesc'  },
        portrait: { icon: '🗝', accent: '#c9a875', labelKey: 'zonePortrait', descKey: 'zonePortraitDesc' },
        windows:  { icon: '🌙', accent: '#6f7d45', labelKey: 'zoneWindows',  descKey: 'zoneWindowsDesc'  },
    };

    let ZONE_DATA = $derived(
        Object.fromEntries(
            Object.entries(ZONE_STATIC).map(([k, v]) => [k, {
                label: $t(v.labelKey as import('$lib/i18n').TranslationKey),
                description: $t(v.descKey as import('$lib/i18n').TranslationKey),
                icon: v.icon,
                accent: v.accent,
            }])
        ) as Record<string, { label: string; description: string; icon: string; accent: string }>
    );

    function sortFeaturedFigurines(items: FigurineListItem[]) {
        const statusRank = { available: 0, reserved: 1, sold: 2 };
        return items.slice().sort((a, b) => {
            const byStatus = statusRank[a.status] - statusRank[b.status];
            const byYear = (b.year ?? -Infinity) - (a.year ?? -Infinity);
            const byOrder = (a.sortOrder ?? 0) - (b.sortOrder ?? 0);
            return byStatus || byYear || byOrder || a.name.localeCompare(b.name);
        });
    }

    async function init() {
        try {
            const [dbZones, bgPath, figurines] = await Promise.all([
                api.getCabinetZones().catch(() => DEFAULT_ZONES),
                api.getMainBackground().catch(() => null),
                api.getAllFigurines().catch(() => [] as FigurineListItem[])
            ]);
            if (bgPath) imageUrl = bgPath;
            await preloadImage(imageUrl);
            zones = dbZones && dbZones.length > 0 ? dbZones : DEFAULT_ZONES;
            collectionTotal = figurines.length;
            availableTotal = figurines.filter((item) => item.status === 'available').length;
            featuredFigurines = sortFeaturedFigurines(figurines).slice(0, 4);
            isLoaded = true;
        } catch (e) {
            zones = DEFAULT_ZONES;
            isLoaded = true;
        }
    }

    function preloadImage(url: string): Promise<void> {
        return new Promise((resolve) => {
            const img = new Image();
            img.onload = () => { imageLoaded = true; resolve(); };
            img.onerror = () => { imageLoaded = true; resolve(); };
            img.src = url;
        });
    }

    function handleMouseMove(e: MouseEvent) {
        const { innerWidth, innerHeight } = window;
        mouseX = e.clientX / innerWidth;
        mouseY = e.clientY / innerHeight;
        parallaxSpring.set({ x: (mouseX - 0.5) * 2, y: (mouseY - 0.5) * 2 });
    }

    function handleTouchMove(e: TouchEvent) {
        if (e.touches.length === 0) return;
        const { innerWidth, innerHeight } = window;
        const touch = e.touches[0];
        parallaxSpring.set({
            x: (touch.clientX / innerWidth - 0.5) * 2,
            y: (touch.clientY / innerHeight - 0.5) * 2
        });
    }

    async function handleZoneInteraction(zone: CabinetZone) {
        if (isNavigating) return;
        isNavigating = true;
        await goto(zone.targetRoute);
    }

    let showHint = $state(false);
    let hintDismissed = $state(false);

    onMount(() => {
        init();
        const hintTimer = setTimeout(() => { if (!hoveredZone) showHint = true; }, 3000);
        const flickerInterval = setInterval(() => {
            ambientIntensity = 0.94 + Math.random() * 0.06;
        }, 200);
        return () => { clearInterval(flickerInterval); clearTimeout(hintTimer); };
    });
</script>

<svelte:head>
    <title>Gotiga — кабинет авторских готических фигурок</title>
    <meta name="description" content="Авторский кабинет готических фигурок и миниатюр ручной работы." />
    <meta property="og:title" content="Gotiga — кабинет готических фигурок" />
    <meta property="og:image" content="/images/cabinet-room.jpg" />
    <meta property="og:type" content="website" />
    <meta name="theme-color" content="#f8f1e7" />
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
    <link href="https://fonts.googleapis.com/css2?family=Cormorant+Garamond:ital,wght@0,300;0,400;0,500;1,300;1,400;1,500&family=Instrument+Sans:wght@400;500&display=swap" rel="stylesheet">
</svelte:head>

<svelte:window onmousemove={handleMouseMove} ontouchmove={handleTouchMove} />

<div class="root">
    <div class="cursor-glow" style="left:{mouseX*100}%;top:{mouseY*100}%"></div>
    <div class="grain" aria-hidden="true"></div>

    {#if imageLoaded}
    <main in:fade={{ duration: 1600, delay: 100 }}>

        <!-- HEADER -->
        <header class="header">
            <a href="/" class="brand" aria-label="Gotiga">
                <span class="brand-name">Gotiga</span>
                <span class="brand-sub">Cabinet of Gothic Miniatures</span>
            </a>

            <nav class="nav" aria-label="Primary">
                <a href="/figurines" class="nav-link">Archive</a>
                <a href="/workshop" class="nav-link">Workshop</a>
                <a href="/author" class="nav-link">Author</a>
            </nav>

            <div class="header-end">
                <LangSwitcher variant="dark" />
                <a href="/admin" class="key-link" aria-label="Admin">
                    <svg width="13" height="13" viewBox="0 0 13 13" fill="none">
                        <circle cx="4.5" cy="4.5" r="3" stroke="currentColor" stroke-width="1"/>
                        <path d="M7 7L11.5 11.5" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
                        <path d="M9.5 9L11 7.5" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
                    </svg>
                </a>
            </div>
        </header>

        <!-- HERO -->
        <section class="hero" aria-labelledby="home-title">

            <!-- Left: text -->
            <div class="hero-text" in:fly={{ x: -20, duration: 900, delay: 350, easing: cubicOut }}>
                <p class="eyebrow">
                    <span class="eyebrow-rule"></span>
                    {$t('homeKicker')}
                </p>

                <h1 id="home-title" class="hero-title">Gotiga</h1>

                <p class="hero-lead">{$t('homeLead')}</p>

                <div class="hero-ctas">
                    <a href="#available-works" class="cta-primary">
                        {$t('homePrimaryCta')}
                        <svg class="cta-arrow" width="18" height="9" viewBox="0 0 18 9" fill="none">
                            <path d="M0 4.5H17M17 4.5L12.5 1M17 4.5L12.5 8" stroke="currentColor" stroke-width="1"/>
                        </svg>
                    </a>
                    <a href="/figurines" class="cta-ghost">{$t('homeSecondaryCta')}</a>
                </div>

                <a href="/workshop" class="workshop-link">
                    Workshop
                    <span class="wl-arrow">↗</span>
                </a>

                <dl class="stats">
                    <div class="stat">
                        <dt class="stat-num">{availableTotal}</dt>
                        <dd class="stat-label">{$t('homeAvailableStat')}</dd>
                    </div>
                    <div class="stat-sep"></div>
                    <div class="stat">
                        <dt class="stat-num">{collectionTotal}</dt>
                        <dd class="stat-label">{$t('homeArchiveStat')}</dd>
                    </div>
                </dl>
            </div>

            <!-- Right: image -->
            <div class="hero-visual">
                <div class="img-meta">
                    <span>№ 001</span>
                    <span>Cabinet View</span>
                </div>

                <div
                    class="img-frame"
                    style="
                        transform:
                            perspective(2200px)
                            rotateY({$parallaxSpring.x * -1.2}deg)
                            rotateX({$parallaxSpring.y * 1.2}deg)
                            scale(1.02);
                        filter: brightness({ambientIntensity});
                    "
                >
                    <img src={imageUrl} alt="Gothic Cabinet Interior" class="hero-img" draggable="false" />
                    <div class="img-vignette"></div>
                    <div class="img-grade"></div>
                    <div class="img-noise"></div>
                    <div class="fog fog-a"></div>
                    <div class="fog fog-b"></div>

                    {#if isLoaded}
                        <div class="zones-layer">
                            {#each zones as zone, i (zone.id)}
                                {@render zoneBtn(zone, i)}
                            {/each}
                        </div>
                    {/if}
                </div>

                <!-- Decorative frame corners -->
                <span class="fc fc-tl"></span>
                <span class="fc fc-tr"></span>
                <span class="fc fc-bl"></span>
                <span class="fc fc-br"></span>

                {#if showHint && !hintDismissed}
                    <a href="#available-works" class="scroll-cue" in:fade={{ duration: 400 }}>
                        <span class="sc-line"></span>
                        <span>{$t('homeScrollCue')}</span>
                    </a>
                {/if}
            </div>

        </section>

        <!-- FEATURED -->
        {#if featuredFigurines.length > 0}
<section id="available-works" class="featured" aria-labelledby="featured-title">
    <div class="featured-hd">
        <div class="featured-hd-left">
            <p class="eyebrow">
                <span class="eyebrow-rule"></span>
                Archive selection
            </p>
            <h2 id="featured-title" class="featured-title">{$t('homeFeaturedTitle')}</h2>
        </div>
        <div class="featured-hd-right">
            <p class="featured-desc">{$t('homeFeaturedText')}</p>
            <a href="/figurines" class="all-link">
                {$t('homeAllWorks')}
                <svg width="16" height="8" viewBox="0 0 16 8" fill="none">
                    <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                </svg>
            </a>
        </div>
    </div>

    <div class="cards">
        {#each featuredFigurines as fig, i}
            <FeaturedFigurineCard {fig} index={i} />
        {/each}
    </div>
</section>
{/if}

    </main>
    {/if}
</div>

{#snippet zoneBtn(zone: CabinetZone, index: number)}
    {@const zd = ZONE_DATA[zone.zoneType]}
    <button
        class="zone"
        style="left:{zone.x}%;top:{zone.y}%;width:{zone.width}%;height:{zone.height}%;--di:{index * 0.7}s;"
        onclick={() => handleZoneInteraction(zone)}
        onmouseenter={() => { hoveredZone = zone; hintDismissed = true; }}
        onmouseleave={() => hoveredZone = null}
        ontouchstart={() => { hintDismissed = true; }}
        disabled={isNavigating}
        aria-label={zd?.label}
    >
        <div class="zone-pulse"></div>
        <span class="zc zc-tl"></span>
        <span class="zc zc-tr"></span>
        <span class="zc zc-bl"></span>
        <span class="zc zc-br"></span>
        <div class="zone-scan"></div>
        <div class="zone-halo"></div>

        {#if zd}
            <div class="zone-tip">
                <span class="zt-icon">{zd.icon}</span>
                <span class="zt-name">{zd.label}</span>
                <span class="zt-desc">{zd.description}</span>
            </div>
        {/if}
    </button>
{/snippet}

<style>
    /* ── TOKENS ──────────────────────────────────── */
    :root {
        --cream:   #f8f1e7;
        --cream2:  #fff9f0;
        --ink:     #2c1710;
        --brown:   #34251c;
        --mid:     #6f3b24;
        --tan:     #a86124;
        --copper:  #c65f3c;
        --gold:    #c9a875;
        --muted:   rgba(95,70,54,0.68);
        --muted2:  rgba(95,70,54,0.40);
        --border:  rgba(52,37,28,0.10);
        --border2: rgba(52,37,28,0.18);
        --ease:    cubic-bezier(0.16,1,0.3,1);
    }

    * { margin: 0; padding: 0; box-sizing: border-box; }

    :global(body) {
        background: var(--cream);
        color: var(--brown);
        font-family: 'Instrument Sans', sans-serif;
        -webkit-font-smoothing: antialiased;
    }

    /* ── ROOT ────────────────────────────────────── */
    .root {
        width: 100vw;
        min-height: 100svh;
        overflow-x: hidden;
        position: relative;
        background:
            radial-gradient(ellipse 70% 55% at 72% 38%, rgba(198,95,60,0.07) 0%, transparent 65%),
            radial-gradient(ellipse 50% 70% at 18% 72%, rgba(201,168,117,0.06) 0%, transparent 60%),
            var(--cream);
    }

    /* ── CURSOR GLOW ─────────────────────────────── */
    .cursor-glow {
        position: fixed;
        width: 500px;
        height: 500px;
        border-radius: 50%;
        background: radial-gradient(circle, rgba(198,95,60,0.07) 0%, transparent 70%);
        transform: translate(-50%, -50%);
        pointer-events: none;
        z-index: 0;
        transition: left 0.8s ease, top 0.8s ease;
        will-change: left, top;
    }

    /* ── GRAIN ───────────────────────────────────── */
    .grain {
        position: fixed;
        inset: -50%;
        width: 200%;
        height: 200%;
        opacity: 0.028;
        pointer-events: none;
        z-index: 500;
        background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
        animation: grain-anim 6s steps(1) infinite;
    }
    @keyframes grain-anim {
        0%   { transform: translate(0,0); }
        16%  { transform: translate(-5%,-8%); }
        33%  { transform: translate(8%,4%); }
        50%  { transform: translate(-3%,10%); }
        66%  { transform: translate(10%,-4%); }
        83%  { transform: translate(-8%,6%); }
        100% { transform: translate(0,0); }
    }

    main {
        width: 100%;
        min-height: 100svh;
        position: relative;
        z-index: 1;
    }

    /* ── HEADER ──────────────────────────────────── */
    .header {
        position: fixed;
        top: 0; left: 0;
        width: 100%;
        height: 68px;
        display: flex;
        align-items: center;
        padding: 0 clamp(20px, 4.5vw, 72px);
        background: rgba(248,241,231,0.85);
        backdrop-filter: blur(20px) saturate(1.3);
        -webkit-backdrop-filter: blur(20px) saturate(1.3);
        border-bottom: 1px solid var(--border);
        z-index: 200;
    }

    .brand {
        display: flex;
        flex-direction: column;
        gap: 3px;
        text-decoration: none;
        color: inherit;
        flex-shrink: 0;
    }

    .brand-name {
        font-family: 'Cormorant Garamond', serif;
        font-size: 20px;
        font-weight: 400;
        letter-spacing: 0.3em;
        text-transform: uppercase;
        color: var(--ink);
        line-height: 1;
    }

    .brand-sub {
        font-size: 8.5px;
        letter-spacing: 0.22em;
        text-transform: uppercase;
        color: var(--muted2);
        line-height: 1;
    }

    .nav {
        display: flex;
        align-items: center;
        margin-left: auto;
    }

    .nav-link {
        position: relative;
        display: flex;
        align-items: center;
        height: 68px;
        padding: 0 22px;
        font-size: 9.5px;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: var(--muted);
        text-decoration: none;
        transition: color 0.25s;
        overflow: hidden;
    }

    .nav-link::after {
        content: '';
        position: absolute;
        bottom: 0; left: 22px; right: 22px;
        height: 1px;
        background: var(--copper);
        transform: scaleX(0);
        transform-origin: left;
        transition: transform 0.35s var(--ease);
    }

    .nav-link:hover { color: var(--ink); }
    .nav-link:hover::after { transform: scaleX(1); }

    .header-end {
        display: flex;
        align-items: center;
        gap: 14px;
        margin-left: 20px;
        padding-left: 20px;
        border-left: 1px solid var(--border);
    }

    .key-link {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 28px; height: 28px;
        color: var(--muted2);
        text-decoration: none;
        transition: color 0.25s;
    }
    .key-link:hover { color: var(--mid); }

    /* ── HERO LAYOUT ─────────────────────────────── */
    .hero {
        display: grid;
        grid-template-columns: minmax(280px, 0.68fr) minmax(480px, 1.42fr);
        gap: clamp(32px, 5vw, 88px);
        align-items: center;
        min-height: 100svh;
        padding: 68px clamp(20px, 4.5vw, 72px) 40px;
        max-width: 1680px;
        margin: 0 auto;
    }

    /* ── HERO TEXT ───────────────────────────────── */
    .hero-text {
        position: relative;
        z-index: 10;
    }

    .eyebrow {
        display: flex;
        align-items: center;
        gap: 12px;
        font-size: 9px;
        letter-spacing: 0.22em;
        text-transform: uppercase;
        color: var(--muted2);
        margin-bottom: 18px;
    }

    .eyebrow-rule {
        display: inline-block;
        width: 26px;
        height: 1px;
        background: var(--copper);
        opacity: 0.65;
        flex-shrink: 0;
    }

    /* ── H1: single word, large, on one line ─────── */
    .hero-title {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(80px, 10vw, 152px);
        font-weight: 300;
        line-height: 0.9;
        letter-spacing: -0.01em;
        color: var(--ink);
        margin: 0 0 26px;
        /* Subtle italic accent on the 'ga' via background-clip text */
        background: linear-gradient(
            135deg,
            var(--ink) 0%,
            var(--ink) 60%,
            var(--mid) 100%
        );
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
        background-clip: text;
        animation: title-in 1.1s var(--ease) both;
        animation-delay: 0.4s;
    }

    @keyframes title-in {
        from { opacity: 0; transform: translateY(30px); }
        to   { opacity: 1; transform: none; }
    }

    .hero-lead {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(17px, 2vw, 22px);
        font-weight: 300;
        font-style: italic;
        line-height: 1.52;
        color: rgba(52,37,28,0.76);
        max-width: 360px;
        margin-bottom: 34px;
    }

    /* ── CTAs ────────────────────────────────────── */
    .hero-ctas {
        display: flex;
        align-items: center;
        gap: 14px;
        flex-wrap: wrap;
        margin-bottom: 18px;
    }

    .cta-primary {
        display: inline-flex;
        align-items: center;
        gap: 12px;
        height: 44px;
        padding: 0 22px;
        background: var(--ink);
        color: var(--cream2);
        font-size: 9.5px;
        letter-spacing: 0.16em;
        text-transform: uppercase;
        text-decoration: none;
        transition: background 0.28s, gap 0.28s;
        clip-path: polygon(0 0, calc(100% - 7px) 0, 100% 7px, 100% 100%, 7px 100%, 0 calc(100% - 7px));
    }

    .cta-arrow {
        flex-shrink: 0;
        transition: transform 0.28s;
    }

    .cta-primary:hover {
        background: var(--mid);
        gap: 18px;
    }

    .cta-primary:hover .cta-arrow {
        transform: translateX(3px);
    }

    .cta-ghost {
        display: inline-flex;
        align-items: center;
        height: 44px;
        padding: 0 22px;
        border: 1px solid var(--border2);
        color: var(--brown);
        font-size: 9.5px;
        letter-spacing: 0.16em;
        text-transform: uppercase;
        text-decoration: none;
        transition: border-color 0.28s, background 0.28s;
    }

    .cta-ghost:hover {
        border-color: rgba(198,95,60,0.5);
        background: rgba(198,95,60,0.04);
    }

    .workshop-link {
        display: inline-flex;
        align-items: center;
        gap: 5px;
        font-size: 9.5px;
        letter-spacing: 0.16em;
        text-transform: uppercase;
        color: var(--muted2);
        text-decoration: none;
        padding: 10px 0;
        transition: color 0.25s;
        margin-bottom: 34px;
    }

    .workshop-link:hover { color: var(--copper); }

    .wl-arrow {
        display: inline-block;
        transition: transform 0.25s;
    }

    .workshop-link:hover .wl-arrow {
        transform: translate(2px, -2px);
    }

    /* ── STATS ───────────────────────────────────── */
    .stats {
        display: flex;
        align-items: stretch;
    }

    .stat {
        padding-right: 24px;
    }

    .stat-sep {
        width: 1px;
        background: var(--border2);
        margin: 6px 24px 6px 0;
        flex-shrink: 0;
    }

    .stat-num {
        font-family: 'Cormorant Garamond', serif;
        font-size: 38px;
        font-weight: 300;
        line-height: 1;
        color: var(--mid);
    }

    .stat-label {
        margin-top: 5px;
        font-size: 8.5px;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: var(--muted2);
    }

    /* ── HERO VISUAL ─────────────────────────────── */
    .hero-visual {
        position: relative;
    }

    .img-meta {
        display: flex;
        justify-content: space-between;
        font-size: 8.5px;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: var(--muted2);
        padding-bottom: 10px;
        pointer-events: none;
    }

    .img-frame {
        position: relative;
        width: 100%;
        height: clamp(480px, 69svh, 850px);
        overflow: hidden;
        transform-style: preserve-3d;
        transition: filter 0.3s;
        will-change: transform;
    }

    .hero-img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        display: block;
        position: relative;
        z-index: 1;
        filter: saturate(0.78) contrast(1.06);
    }

    .img-vignette {
        position: absolute;
        inset: 0; z-index: 2;
        background: radial-gradient(ellipse at center, transparent 25%, rgba(44,23,16,0.52) 100%);
        pointer-events: none;
    }

    .img-grade {
        position: absolute;
        inset: 0; z-index: 3;
        background: linear-gradient(180deg, rgba(198,95,60,0.05) 0%, transparent 45%, rgba(44,23,16,0.14) 100%);
        mix-blend-mode: multiply;
        pointer-events: none;
    }

    .img-noise {
        position: absolute;
        inset: 0; z-index: 4;
        opacity: 0.06;
        mix-blend-mode: overlay;
        pointer-events: none;
        background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
    }

    .fog {
        position: absolute;
        inset: 0; z-index: 5;
        pointer-events: none;
    }

    .fog-a {
        background: radial-gradient(ellipse 55% 40% at 22% 58%, rgba(155,135,120,0.07) 0%, transparent 70%);
        animation: fog-a 30s ease-in-out infinite;
    }

    .fog-b {
        background: radial-gradient(ellipse 48% 48% at 78% 30%, rgba(135,115,105,0.05) 0%, transparent 65%);
        animation: fog-b 38s ease-in-out infinite;
    }

    @keyframes fog-a {
        0%,100% { transform: translate(0,0) scale(1); opacity: 0.6; }
        50% { transform: translate(18px,-10px) scale(1.09); opacity: 1; }
    }
    @keyframes fog-b {
        0%,100% { transform: translate(0,0) scale(1); opacity: 0.5; }
        50% { transform: translate(-22px,16px) scale(1.11); opacity: 0.85; }
    }

    /* Frame deco corners */
    .fc {
        position: absolute;
        width: 16px; height: 16px;
        pointer-events: none;
        z-index: 10;
        opacity: 0.45;
    }

    .fc-tl { top: 32px; left: 0; border-top: 1px solid var(--copper); border-left: 1px solid var(--copper); }
    .fc-tr { top: 32px; right: 0; border-top: 1px solid var(--copper); border-right: 1px solid var(--copper); }
    .fc-bl { bottom: 0; left: 0; border-bottom: 1px solid var(--copper); border-left: 1px solid var(--copper); }
    .fc-br { bottom: 0; right: 0; border-bottom: 1px solid var(--copper); border-right: 1px solid var(--copper); }

    /* Scroll cue */
    .scroll-cue {
        position: absolute;
        bottom: -32px; right: 0;
        display: flex;
        align-items: center;
        gap: 10px;
        font-size: 8.5px;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: var(--muted2);
        text-decoration: none;
        transition: color 0.25s;
    }

    .scroll-cue:hover { color: var(--brown); }

    .sc-line {
        display: block;
        width: 22px;
        height: 1px;
        background: currentColor;
        animation: sc-pulse 2.2s ease-in-out infinite;
    }

    @keyframes sc-pulse {
        0%,100% { width: 22px; }
        50% { width: 38px; }
    }

    /* ── ZONES ───────────────────────────────────── */
    .zones-layer {
        position: absolute;
        inset: 0;
        z-index: 20;
    }

    .zone {
        position: absolute;
        background: transparent;
        border: none;
        cursor: crosshair;
        outline: none;
    }

    .zone-pulse {
        position: absolute;
        inset: -8%;
        border-radius: 50%;
        background: radial-gradient(ellipse at center, rgba(198,95,60,0.09) 0%, transparent 70%);
        animation: pulse 3.8s ease-in-out infinite;
        animation-delay: var(--di);
        opacity: 0.45;
        pointer-events: none;
    }

    .zone:hover .zone-pulse { opacity: 0; }

    @keyframes pulse {
        0%,100% { transform: scale(0.82); opacity: 0.3; }
        50% { transform: scale(1.14); opacity: 0.55; }
    }

    .zc {
        position: absolute;
        width: 12px; height: 12px;
        transition: all 0.4s var(--ease);
        pointer-events: none;
    }

    .zc-tl { top: 0; left: 0; border-top: 1.5px solid rgba(198,95,60,0.45); border-left: 1.5px solid rgba(198,95,60,0.45); }
    .zc-tr { top: 0; right: 0; border-top: 1.5px solid rgba(198,95,60,0.45); border-right: 1.5px solid rgba(198,95,60,0.45); }
    .zc-bl { bottom: 0; left: 0; border-bottom: 1.5px solid rgba(198,95,60,0.45); border-left: 1.5px solid rgba(198,95,60,0.45); }
    .zc-br { bottom: 0; right: 0; border-bottom: 1.5px solid rgba(198,95,60,0.45); border-right: 1.5px solid rgba(198,95,60,0.45); }

    .zone:hover .zc {
        width: 22px; height: 22px;
        border-color: rgba(198,95,60,1);
    }

    .zone-scan {
        position: absolute;
        left: 0; right: 0; top: 0;
        height: 1px;
        background: linear-gradient(90deg, transparent, rgba(198,95,60,0.55), transparent);
        opacity: 0;
        pointer-events: none;
    }

    .zone:hover .zone-scan {
        opacity: 1;
        animation: scan 1.8s ease-in-out infinite;
    }

    @keyframes scan {
        0% { top: 0; }
        100% { top: 100%; }
    }

    .zone-halo {
        position: absolute;
        inset: -18%;
        background: radial-gradient(ellipse at center, rgba(198,95,60,0.18) 0%, transparent 65%);
        opacity: 0;
        filter: blur(14px);
        transition: opacity 0.45s;
        pointer-events: none;
    }

    .zone:hover .zone-halo { opacity: 1; }

    .zone-tip {
        position: absolute;
        bottom: 8%;
        left: 50%;
        transform: translateX(-50%) translateY(8px);
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 5px;
        opacity: 0;
        pointer-events: none;
        white-space: nowrap;
        transition: opacity 0.32s, transform 0.32s var(--ease);
    }

    .zone:hover .zone-tip {
        opacity: 1;
        transform: translateX(-50%) translateY(0);
    }

    .zt-icon { font-size: 16px; filter: drop-shadow(0 0 6px rgba(198,95,60,0.7)); }

    .zt-name {
        font-family: 'Instrument Sans', sans-serif;
        font-size: 8.5px;
        letter-spacing: 0.28em;
        text-transform: uppercase;
        color: var(--cream2);
        background: rgba(44,23,16,0.80);
        backdrop-filter: blur(8px);
        padding: 4px 10px;
        border: 1px solid rgba(198,95,60,0.28);
    }

    .zt-desc {
        font-family: 'Cormorant Garamond', serif;
        font-size: 12px;
        font-style: italic;
        color: rgba(255,240,218,0.88);
        background: rgba(44,23,16,0.60);
        backdrop-filter: blur(8px);
        padding: 3px 10px;
        opacity: 0;
        transform: translateY(4px);
        transition: opacity 0.28s 0.06s, transform 0.28s 0.06s;
    }

    .zone:hover .zt-desc {
        opacity: 1;
        transform: translateY(0);
    }

    /* ── FEATURED ────────────────────────────────── */
    .featured {
        padding: clamp(72px, 9vw, 128px) clamp(20px, 4.5vw, 72px) clamp(80px, 10vw, 144px);
        max-width: 1680px;
        margin: 0 auto;
    }

    .featured-hd {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: clamp(24px, 4vw, 64px);
        align-items: end;
        margin-bottom: clamp(36px, 5vw, 64px);
        padding-bottom: clamp(28px, 4vw, 48px);
        border-bottom: 1px solid var(--border);
    }

    .featured-title {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(36px, 5.5vw, 76px);
        font-weight: 300;
        line-height: 0.92;
        color: var(--ink);
        margin-top: 10px;
    }

    .featured-hd-right {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 18px;
        padding-bottom: 4px;
    }

    .featured-desc {
        font-family: 'Cormorant Garamond', serif;
        font-size: 18px;
        font-weight: 300;
        font-style: italic;
        line-height: 1.5;
        color: var(--muted);
        max-width: 440px;
    }

    .all-link {
        display: inline-flex;
        align-items: center;
        gap: 10px;
        font-size: 9.5px;
        letter-spacing: 0.16em;
        text-transform: uppercase;
        color: var(--mid);
        text-decoration: none;
        padding-bottom: 4px;
        border-bottom: 1px solid rgba(111,59,36,0.22);
        transition: gap 0.28s, color 0.28s;
    }

    .all-link:hover { color: var(--copper); gap: 16px; }

    /* ── CARDS ───────────────────────────────────── */
    .cards {
        display: grid;
        grid-template-columns: repeat(4, 1fr);
        gap: clamp(20px, 2.4vw, 34px);
    }

    /* ── RESPONSIVE ──────────────────────────────── */
    @media (max-width: 1080px) {
        .hero {
            grid-template-columns: 1fr;
            min-height: auto;
            padding-top: 88px;
            gap: 32px;
        }

        .hero-visual { order: 2; }
        .hero-text { order: 1; max-width: 580px; }

        .img-frame { height: min(58svh, 620px); }

        .featured-hd { grid-template-columns: 1fr; }

        .cards { grid-template-columns: repeat(2, 1fr); }
    }

    @media (max-width: 680px) {
        .header { height: 58px; padding: 0 16px; }
        .brand-name { font-size: 17px; }
        .brand-sub { display: none; }
        .nav { display: none; }
        .header-end { margin-left: auto; }

        .hero {
            padding: 70px 16px 36px;
            gap: 24px;
        }

        .hero-title { font-size: clamp(64px, 20vw, 108px); }
        .hero-lead { font-size: 16px; max-width: 300px; }

        .cta-primary, .cta-ghost { height: 40px; padding: 0 16px; font-size: 9px; }

        .img-frame { height: 42svh; min-height: 260px; }

        .scroll-cue { display: none; }

        .featured {
            padding-inline: 16px;
        }

        .featured-title { font-size: clamp(32px, 9vw, 52px); }
        .featured-desc { font-size: 16px; }

        .cards { grid-template-columns: 1fr; gap: 22px; }
    }

    @media (hover: none) {
        .zone-tip { display: none; }
    }
</style>
