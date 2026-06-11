<script lang="ts">
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { fade, fly } from 'svelte/transition';
    import { cubicOut } from 'svelte/easing';
    import { spring, tweened } from 'svelte/motion';
    import { api } from '$lib/api';
    import type { CabinetZone, FigurineListItem, HomeContent } from '$lib/types/api';
    import { t } from '$lib/i18n';
    import FeaturedFigurineCard from '$lib/components/FeaturedFigurineCard.svelte';

    let zones = $state<CabinetZone[]>([]);
    let isLoaded = $state(false);
    let imageLoaded = $state(false);
    let hoveredZone = $state<CabinetZone | null>(null);
    let isNavigating = $state(false);
    let ambientIntensity = $state(1);
    let featuredFigurines = $state<FigurineListItem[]>([]);
    let heroFigurine = $state<FigurineListItem | null>(null);
    let collectionTotal = $state(0);
    let availableTotal = $state(0);
    let homeContent = $state<HomeContent>({
        title: null,
        kicker: null,
        lead: null,
        heroFigurineId: null,
        heroCaptionTitle: null,
        heroCaptionMeta: null,
        heroCaptionCta: null,
        heroMode: null,
    });
    let mouseX = $state(0.5);
    let mouseY = $state(0.5);

    const parallaxSpring = spring({ x: 0, y: 0 }, { stiffness: 0.04, damping: 0.45 });

    type HeroMode = 'showcase' | 'release';

    let configuredHeroMode = $derived(homeContent.heroMode ?? 'auto');
    let activeHeroMode = $derived<HeroMode>(
        configuredHeroMode === 'release' || (configuredHeroMode === 'auto' && availableTotal === 0)
            ? 'release'
            : 'showcase'
    );
    let isReleaseMode = $derived(activeHeroMode === 'release');

    // Kinetic title: keep the homepage promise stable; custom title is used as artwork caption.
    let titleText = $derived(isReleaseMode ? $t('homeReleaseTitle') : $t('homeTitle'));
    let leadText = $derived(homeContent.lead?.trim() || (isReleaseMode ? $t('homeReleaseLead') : $t('homeLead')));
    let primaryCtaHref = $derived(isReleaseMode ? '/figurines' : '#available-works');
    let primaryCtaText = $derived(isReleaseMode ? $t('homeReleasePrimaryCta') : $t('homePrimaryCta'));
    let secondaryCtaHref = $derived(isReleaseMode ? '/upcoming' : '#request-path');
    let secondaryCtaText = $derived(isReleaseMode ? $t('homeReleaseSecondaryCta') : $t('homeOrderCta'));
    let heroObjectName = $derived(homeContent.heroCaptionTitle?.trim() || heroFigurine?.name || homeContent.title?.trim() || '');
    let heroObjectMeta = $derived(homeContent.heroCaptionMeta?.trim() || $t('homeHeroObjectMeta'));
    let heroObjectCta = $derived(homeContent.heroCaptionCta?.trim() || (heroFigurine ? $t('homeHeroObjectOpen') : $t('homeSecondaryCta')));
    let titleWords = $derived(titleText.split(/\s+/).filter(Boolean));
    let showStats = $derived(availableTotal > 0 || collectionTotal >= 3);
    let heroObjectHref = $derived(heroFigurine ? `/figurines/${heroFigurine.id}` : '/figurines');
    let showHeroCaption = $derived(Boolean(heroObjectName));

    // Count-up stats
    const availDisplay = tweened(0, { duration: 1100, easing: cubicOut });
    const collDisplay = tweened(0, { duration: 1100, easing: cubicOut });

    // Magnetic hover action for CTAs
    function magnetic(node: HTMLElement, strength = 0.3) {
        function move(e: MouseEvent) {
            const r = node.getBoundingClientRect();
            const x = (e.clientX - r.left - r.width / 2) * strength;
            const y = (e.clientY - r.top - r.height / 2) * (strength + 0.1);
            node.style.transform = `translate(${x}px, ${y}px)`;
        }
        function leave() { node.style.transform = ''; }
        node.addEventListener('mousemove', move);
        node.addEventListener('mouseleave', leave);
        return {
            destroy() {
                node.removeEventListener('mousemove', move);
                node.removeEventListener('mouseleave', leave);
            }
        };
    }

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
        const statusRank: Record<import('$lib/types/api').FigurineStatus, number> =
            { available: 0, reserved: 1, sold: 2, in_progress: 3 };
        return items.slice().sort((a, b) => {
            const byStatus = statusRank[a.status] - statusRank[b.status];
            const byYear = (b.year ?? -Infinity) - (a.year ?? -Infinity);
            const byOrder = (a.sortOrder ?? 0) - (b.sortOrder ?? 0);
            return byStatus || byYear || byOrder || a.name.localeCompare(b.name);
        });
    }

    async function init() {
        try {
            const [dbZones, bgPath, figurines, content] = await Promise.all([
                api.getCabinetZones().catch(() => DEFAULT_ZONES),
                api.getMainBackground().catch(() => null),
                api.getAllFigurines().catch(() => [] as FigurineListItem[]),
                api.getHomeContent().catch(() => ({
                    title: null,
                    kicker: null,
                    lead: null,
                    heroFigurineId: null,
                    heroCaptionTitle: null,
                    heroCaptionMeta: null,
                    heroCaptionCta: null,
                    heroMode: null,
                } satisfies HomeContent))
            ]);
            if (bgPath) imageUrl = bgPath;
            homeContent = content;
            await preloadImage(imageUrl);
            zones = dbZones && dbZones.length > 0 ? dbZones : DEFAULT_ZONES;
            const visibleFigurines = figurines.filter(f => f.status !== 'in_progress');
            collectionTotal = visibleFigurines.length;
            availableTotal = figurines.filter((item) => item.status === 'available').length;
            heroFigurine = content.heroFigurineId
                ? visibleFigurines.find((item) => item.id === content.heroFigurineId) ?? null
                : null;
            const pinned = visibleFigurines.filter(f => f.isFeatured);
            featuredFigurines = pinned.length > 0
                ? sortFeaturedFigurines(pinned).slice(0, 4)
                : sortFeaturedFigurines(visibleFigurines).slice(0, 4);
            availDisplay.set(availableTotal);
            collDisplay.set(collectionTotal);
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

        <!-- HERO -->
        <section class="hero" aria-labelledby="home-title">

            <!-- Left: text -->
            <div class="hero-text" in:fly={{ x: -20, duration: 900, delay: 350, easing: cubicOut }}>
                <p class="eyebrow">
                    <span class="eyebrow-rule"></span>
                    {homeContent.kicker?.trim() || $t('homeKicker')}
                </p>

                <h1 id="home-title" class="hero-title" aria-label={titleText}>
                    {#each titleWords as word, i}
                        <span
                            class="title-word"
                            class:accent={i === titleWords.length - 1}
                            style="animation-delay:{0.12 + i * 0.08}s"
                            aria-hidden="true"
                        >{word}</span>
                    {/each}
                </h1>

                <p class="hero-lead">{leadText}</p>

                <div class="hero-ctas">
                    <a href={primaryCtaHref} class="cta-primary" use:magnetic={0.28}>
                        {primaryCtaText}
                        <svg class="cta-arrow" width="18" height="9" viewBox="0 0 18 9" fill="none">
                            <path d="M0 4.5H17M17 4.5L12.5 1M17 4.5L12.5 8" stroke="currentColor" stroke-width="1"/>
                        </svg>
                    </a>
                    <a href={secondaryCtaHref} class="cta-ghost" use:magnetic={0.22}>{secondaryCtaText}</a>
                </div>

                <div class="trust-line" aria-label="Gotiga">
                    <span>{$t('homeTrustUnique')}</span>
                    <span>{$t('homeTrustHandmade')}</span>
                    <span>{$t('homeTrustAuthorReply')}</span>
                </div>

                {#if showStats}
                    <dl class="stats">
                        <div class="stat">
                            <dt class="stat-num">{Math.round($availDisplay)}</dt>
                            <dd class="stat-label">{$t('homeAvailableStat')}</dd>
                        </div>
                        <div class="stat-sep"></div>
                        <div class="stat">
                            <dt class="stat-num">{Math.round($collDisplay)}</dt>
                            <dd class="stat-label">{$t('homeArchiveStat')}</dd>
                        </div>
                    </dl>
                {:else}
                    <p class="release-note">{$t('homeReleaseNote')}</p>
                {/if}
            </div>

            <!-- Right: image -->
            <div class="hero-visual">
                <div class="img-meta">
                    <span>№ 001</span>
                    <span>{$t('homeHeroObjectLabel')}</span>
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

                    {#if showHeroCaption}
                    <a class="art-caption" href={heroObjectHref} aria-label="{heroObjectCta}: {heroObjectName}">
                        <span class="art-caption-kicker">{$t('homeHeroObjectLabel')}</span>
                        <span class="art-caption-name">{heroObjectName}</span>
                        <span class="art-caption-meta">{heroObjectMeta}</span>
                        <span class="art-caption-open">{heroObjectCta} →</span>
                    </a>
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

        <nav class="route-strip" aria-label={$t('homePathTitle')}>
            <a href="#available-works">
                <span>{$t('homePathAvailable')}</span>
                <small>{$t('homePathAvailableDesc')}</small>
            </a>
            <a href="/upcoming">
                <span>{$t('homePathUpcoming')}</span>
                <small>{$t('homePathUpcomingDesc')}</small>
            </a>
            <a href="/figurines">
                <span>{$t('homePathArchive')}</span>
                <small>{$t('homePathArchiveDesc')}</small>
            </a>
            <a href="/author">
                <span>{$t('homePathAuthor')}</span>
                <small>{$t('homePathAuthorDesc')}</small>
            </a>
        </nav>

        <!-- FEATURED -->
        {#if featuredFigurines.length > 0}
<section id="available-works" class="featured" aria-labelledby="featured-title">
    <div class="featured-hd">
        <div class="featured-hd-left">
            <p class="eyebrow">
                <span class="eyebrow-rule"></span>
                {$t('homeFeaturedEyebrow')}
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

        <section id="request-path" class="request-path" aria-labelledby="request-path-title">
            <div class="request-copy">
                <p class="eyebrow">
                    <span class="eyebrow-rule"></span>
                    {$t('homeHowEyebrow')}
                </p>
                <h2 id="request-path-title" class="request-title">{$t('homeHowTitle')}</h2>
                <p class="request-desc">{$t('homeHowText')}</p>
            </div>

            <div class="request-steps">
                <article class="request-step">
                    <span class="step-num">01</span>
                    <h3>{$t('homeHowStep1Title')}</h3>
                    <p>{$t('homeHowStep1Text')}</p>
                </article>
                <article class="request-step">
                    <span class="step-num">02</span>
                    <h3>{$t('homeHowStep2Title')}</h3>
                    <p>{$t('homeHowStep2Text')}</p>
                </article>
                <article class="request-step">
                    <span class="step-num">03</span>
                    <h3>{$t('homeHowStep3Title')}</h3>
                    <p>{$t('homeHowStep3Text')}</p>
                </article>
            </div>
        </section>

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
        --cream:   var(--color-canvas-base);
        --cream2:  var(--color-canvas-raised);
        --ink:     var(--color-ink-primary);
        --brown:   var(--color-ink-primary);
        --mid:     var(--color-ember-deep);
        --tan:     var(--color-ember-ink);
        --copper:  var(--color-ember);
        --gold:    var(--color-ochre);
        --muted:   color-mix(in srgb, var(--color-ink-secondary) 68%, transparent);
        --muted2:  color-mix(in srgb, var(--color-ink-secondary) 40%, transparent);
        --border:  color-mix(in srgb, var(--color-ink-primary) 10%, transparent);
        --border2: color-mix(in srgb, var(--color-ink-primary) 18%, transparent);
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

    /* ── HERO LAYOUT ─────────────────────────────── */
    .hero {
        display: grid;
        grid-template-columns: minmax(320px, 0.62fr) minmax(560px, 1.38fr);
        gap: clamp(44px, 5.8vw, 104px);
        align-items: center;
        min-height: 100svh;
        padding: 76px clamp(20px, 4.5vw, 72px) 54px;
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

    /* ── H1: word-based reveal, so Russian titles wrap like typography ─────── */
    .hero-title {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(54px, 5.8vw, 104px);
        font-weight: 300;
        line-height: 0.94;
        letter-spacing: 0;
        color: var(--ink);
        margin: 0 0 26px;
        word-break: keep-all;
        overflow-wrap: normal;
        hyphens: none;
        display: flex;
        flex-wrap: wrap;
        column-gap: 0.18em;
        row-gap: 0.04em;
        overflow: hidden;
        padding-bottom: 0.12em;
    }

    .hero-title:lang(ru) {
        font-size: clamp(48px, 5.35vw, 96px);
    }

    .title-word {
        display: inline-block;
        transform: translateY(112%) rotate(7deg);
        opacity: 0;
        will-change: transform, opacity;
        animation: ht-rise 0.92s var(--ease-spring, cubic-bezier(0.34, 1.4, 0.64, 1)) both;
    }

    .title-word.accent {
        color: var(--copper);
        font-style: italic;
    }

    @keyframes ht-rise {
        to { transform: none; opacity: 1; }
    }

    @media (prefers-reduced-motion: reduce) {
        .title-word { animation: none; transform: none; opacity: 1; }
    }

    .hero-lead {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(17px, 2vw, 22px);
        font-weight: 300;
        font-style: italic;
        line-height: 1.52;
        color: rgba(52,37,28,0.76);
        max-width: 430px;
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
        transition: background 0.28s, gap 0.28s, transform 0.35s var(--ease);
        will-change: transform;
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
        min-height: 44px;
        padding: 0 4px;
        color: color-mix(in srgb, var(--brown) 72%, transparent);
        font-size: 9.5px;
        letter-spacing: 0.16em;
        text-transform: uppercase;
        text-decoration: none;
        border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 18%, transparent);
        transition: color 0.28s, border-color 0.28s, transform 0.35s var(--ease);
        will-change: transform;
    }

    .cta-ghost:hover {
        color: var(--copper);
        border-color: rgba(198,95,60,0.5);
    }

    .trust-line {
        display: grid;
        gap: 7px;
        max-width: 420px;
        color: rgba(52,37,28,0.60);
        font-size: 9.5px;
        letter-spacing: 0.14em;
        line-height: 1.45;
        text-transform: uppercase;
    }

    .trust-line span {
        position: relative;
        padding-left: 16px;
    }

    .trust-line span::before {
        content: "";
        position: absolute;
        left: 0;
        top: 0.62em;
        width: 7px;
        height: 1px;
        background: rgba(198,95,60,0.58);
    }

    .release-note {
        max-width: 360px;
        margin-top: 34px;
        padding-left: 18px;
        border-left: 1px solid rgba(198,95,60,0.36);
        font-family: 'Cormorant Garamond', serif;
        font-size: 17px;
        font-style: italic;
        font-weight: 300;
        line-height: 1.42;
        color: rgba(52,37,28,0.58);
    }

    /* ── STATS ───────────────────────────────────── */
    .stats {
        display: flex;
        align-items: stretch;
        margin-top: 34px;
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
        height: clamp(520px, 70svh, 860px);
        overflow: hidden;
        transform-style: preserve-3d;
        transition: filter 0.3s;
        will-change: transform;
    }

    .hero-img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        object-position: 58% 45%;
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

    .art-caption {
        position: absolute;
        left: clamp(18px, 2.3vw, 34px);
        bottom: clamp(18px, 2.3vw, 34px);
        z-index: 28;
        display: flex;
        flex-direction: column;
        gap: 5px;
        max-width: min(330px, calc(100% - 36px));
        padding: 0 0 0 14px;
        color: rgba(255,240,218,0.88);
        text-decoration: none;
        border-left: 1px solid rgba(198,95,60,0.62);
        filter: drop-shadow(0 10px 22px rgba(25,12,7,0.34));
        transition: color 0.25s, border-color 0.25s, transform 0.25s var(--ease);
    }

    .art-caption:hover {
        color: var(--cream2);
        border-color: rgba(255,240,218,0.78);
        transform: translateX(3px);
    }

    .art-caption-kicker {
        font-size: 8px;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: rgba(255,240,218,0.58);
    }

    .art-caption-name {
        font-family: 'Cormorant Garamond', serif;
        font-size: 26px;
        font-style: italic;
        line-height: 1;
    }

    .art-caption-meta,
    .art-caption-open {
        font-size: 8px;
        letter-spacing: 0.12em;
        line-height: 1.3;
        text-transform: uppercase;
        color: rgba(255,240,218,0.68);
    }

    .art-caption-open {
        margin-top: 2px;
        color: rgba(255,240,218,0.88);
    }

    /* ── ROUTE STRIP ─────────────────────────────── */
    .route-strip {
        display: grid;
        grid-template-columns: repeat(4, minmax(0, 1fr));
        gap: 0;
        max-width: 1680px;
        margin: -12px auto 0;
        padding: 0 clamp(20px, 4.5vw, 72px);
    }

    .route-strip a {
        display: flex;
        min-height: 102px;
        flex-direction: column;
        justify-content: center;
        gap: 8px;
        padding: 22px clamp(16px, 2vw, 28px);
        border-top: 1px solid var(--border);
        border-bottom: 1px solid var(--border);
        color: var(--brown);
        text-decoration: none;
        transition: background 0.25s, border-color 0.25s;
    }

    .route-strip a + a {
        border-left: 1px solid var(--border);
    }

    .route-strip a:hover {
        background: rgba(198,95,60,0.035);
        border-color: color-mix(in srgb, var(--color-ember) 28%, transparent);
    }

    .route-strip span {
        font-size: 9px;
        letter-spacing: 0.17em;
        text-transform: uppercase;
        color: var(--ink);
    }

    .route-strip small {
        max-width: 220px;
        font-family: 'Cormorant Garamond', serif;
        font-size: 15px;
        font-style: italic;
        font-weight: 300;
        line-height: 1.28;
        color: var(--muted);
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

    /* ── REQUEST PATH ───────────────────────────── */
    .request-path {
        display: grid;
        grid-template-columns: minmax(280px, 0.76fr) minmax(420px, 1.24fr);
        gap: clamp(28px, 5vw, 82px);
        align-items: start;
        max-width: 1680px;
        margin: 0 auto;
        padding: 0 clamp(20px, 4.5vw, 72px) clamp(88px, 10vw, 148px);
    }

    .request-title {
        margin: 10px 0 18px;
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(34px, 4.8vw, 72px);
        font-weight: 300;
        line-height: 0.96;
        color: var(--ink);
    }

    .request-desc {
        max-width: 460px;
        font-family: 'Cormorant Garamond', serif;
        font-size: 18px;
        font-style: italic;
        font-weight: 300;
        line-height: 1.5;
        color: var(--muted);
    }

    .request-steps {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: 14px;
    }

    .request-step {
        min-height: 220px;
        padding: 22px 20px;
        border: 1px solid var(--border);
        background: color-mix(in srgb, var(--color-canvas-raised) 72%, transparent);
    }

    .step-num {
        display: block;
        margin-bottom: 42px;
        font-family: 'Cormorant Garamond', serif;
        font-size: 34px;
        line-height: 1;
        color: color-mix(in srgb, var(--color-ember) 74%, transparent);
    }

    .request-step h3 {
        margin-bottom: 10px;
        font-size: 10px;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: var(--ink);
    }

    .request-step p {
        font-family: 'Cormorant Garamond', serif;
        font-size: 16px;
        font-style: italic;
        font-weight: 300;
        line-height: 1.42;
        color: var(--muted);
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

        .route-strip {
            grid-template-columns: repeat(2, minmax(0, 1fr));
            margin-top: 0;
        }

        .route-strip a:nth-child(odd) {
            border-left: 0;
        }

        .featured-hd { grid-template-columns: 1fr; }

        .cards { grid-template-columns: repeat(2, 1fr); }

        .request-path {
            grid-template-columns: 1fr;
            padding-top: 16px;
        }
    }

    @media (max-width: 680px) {
        .hero {
            padding: 70px 16px 36px;
            gap: 24px;
        }

        .hero-title,
        .hero-title:lang(ru) {
            font-size: clamp(42px, 13.2vw, 68px);
            line-height: 0.98;
        }

        .hero-lead { font-size: 16px; max-width: 330px; }

        .cta-primary { height: 40px; padding: 0 16px; font-size: 9px; }

        .cta-ghost {
            min-height: 40px;
            padding: 0 2px;
            font-size: 9px;
        }

        .trust-line {
            font-size: 8.5px;
            line-height: 1.45;
        }

        .release-note {
            margin-top: 26px;
            font-size: 16px;
        }

        .img-frame { height: 42svh; min-height: 300px; }

        .scroll-cue { display: none; }

        .art-caption {
            left: 14px;
            bottom: 14px;
            max-width: calc(100% - 28px);
            padding-left: 12px;
        }

        .art-caption-name {
            font-size: 22px;
        }

        .art-caption-meta {
            display: none;
        }

        .route-strip {
            grid-template-columns: 1fr;
            padding: 0 16px 18px;
        }

        .route-strip a,
        .route-strip a + a,
        .route-strip a:nth-child(odd) {
            min-height: 78px;
            border-left: 0;
            border-top: 1px solid var(--border);
            border-bottom: 0;
        }

        .route-strip a:last-child {
            border-bottom: 1px solid var(--border);
        }

        .featured {
            padding-inline: 16px;
        }

        .featured-title { font-size: clamp(32px, 9vw, 52px); }
        .featured-desc { font-size: 16px; }

        .cards { grid-template-columns: 1fr; gap: 22px; }

        .request-path {
            padding: 0 16px 88px;
        }

        .request-steps {
            grid-template-columns: 1fr;
        }

        .request-step {
            min-height: 0;
        }

        .step-num {
            margin-bottom: 24px;
        }
    }

    @media (hover: none) {
        .zone-tip { display: none; }
    }
</style>
