<script lang="ts">
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { page } from '$app/state';
    import { fade, fly } from 'svelte/transition';
    import { cubicOut } from 'svelte/easing';
    import { spring, tweened } from 'svelte/motion';
    import { api } from '$lib/api';
    import type { CabinetZone, FigurineListItem, HomeContent } from '$lib/types/api';
    import { t } from '$lib/i18n';
    import AppImage from '$lib/components/AppImage.svelte';
    import HomeFigurineTile from '$lib/components/HomeFigurineTile.svelte';
    import { savedFigurines } from '$lib/stores/saved-figurines.svelte';

    let zones = $state<CabinetZone[]>([]);
    let isLoaded = $state(false);
    let imageLoaded = $state(false);
    let hoveredZone = $state<CabinetZone | null>(null);
    let isNavigating = $state(false);
    let ambientIntensity = $state(1);
    let availableFigurines = $state<FigurineListItem[]>([]);
    let inProgressFigurines = $state<FigurineListItem[]>([]);
    let archivePreviewFigurines = $state<FigurineListItem[]>([]);
    let collectionFigurines = $state<FigurineListItem[]>([]);
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
    type WorkFilter = 'available' | 'saved' | 'upcoming' | 'archive';
    const WORK_FILTERS: WorkFilter[] = ['available', 'saved', 'upcoming', 'archive'];

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
    let activeWorkFilter = $state<WorkFilter>('available');
    let heroObjectName = $derived(homeContent.heroCaptionTitle?.trim() || heroFigurine?.name || homeContent.title?.trim() || '');
    let heroObjectMeta = $derived(homeContent.heroCaptionMeta?.trim() || $t('homeHeroObjectMeta'));
    let heroObjectCta = $derived(homeContent.heroCaptionCta?.trim() || (heroFigurine ? $t('homeHeroObjectOpen') : $t('homeSecondaryCta')));
    let titleWords = $derived(titleText.split(/\s+/).filter(Boolean));
    let showStats = $derived(availableTotal > 0 || collectionTotal >= 3);
    let heroObjectHref = $derived(heroFigurine ? `/figurines/${heroFigurine.id}` : '/figurines');
    let showHeroCaption = $derived(Boolean(heroObjectName));
    let savedPreviewFigurines = $derived(
        savedFigurines.ids
            .map((id) => collectionFigurines.find((item) => item.id === id))
            .filter((item): item is FigurineListItem => Boolean(item))
            .slice(0, 5)
    );
    let savedWorkFigurines = $derived(
        savedFigurines.ids
            .map((id) => collectionFigurines.find((item) => item.id === id))
            .filter((item): item is FigurineListItem => Boolean(item))
    );
    let heroQuickFigurines = $derived(
        (availableFigurines.length > 0 ? availableFigurines : archivePreviewFigurines).slice(0, 3)
    );
    let heroQuickTitle = $derived(
        availableFigurines.length > 0 ? $t('homeHeroQuickAvailable') : $t('homeHeroQuickArchive')
    );
    let activeWorkFigurines = $derived(
        activeWorkFilter === 'saved'
            ? savedWorkFigurines
            : activeWorkFilter === 'upcoming'
                ? inProgressFigurines
                : activeWorkFilter === 'archive'
                    ? archivePreviewFigurines
                    : availableFigurines
    );
    let visibleWorkFigurines = $derived(activeWorkFigurines.slice(0, 8));
    let activeWorkHref = $derived(
        activeWorkFilter === 'saved'
            ? '/profile'
            : activeWorkFilter === 'upcoming'
                ? '/upcoming'
                : '/figurines'
    );
    let activeWorkCta = $derived(
        activeWorkFilter === 'saved'
            ? $t('homeSavedProfile')
            : activeWorkFilter === 'upcoming'
                ? $t('homeViewUpcoming')
                : activeWorkFilter === 'archive'
                    ? $t('homeOpenArchive')
                    : $t('homeAllWorks')
    );
    let activeWorkText = $derived(
        activeWorkFilter === 'saved'
            ? $t('homeWorksSavedText')
            : activeWorkFilter === 'upcoming'
                ? $t('homeContextUpcomingText')
                : activeWorkFilter === 'archive'
                    ? $t('homeContextArchiveText')
                    : $t('homeContextAvailableText')
    );
    let activeWorkEmptyText = $derived(
        activeWorkFilter === 'saved'
            ? $t('homeWorksEmptySaved')
            : activeWorkFilter === 'available'
                ? $t('homeWorksEmptyAvailable')
                : activeWorkFilter === 'upcoming'
                    ? $t('homeWorksEmptyUpcoming')
                    : $t('homeWorksEmptyArchive')
    );
    let activeWorkEmptyHref = $derived(activeWorkFilter === 'saved' ? '/figurines' : '/commission');
    let activeWorkEmptyCta = $derived(activeWorkFilter === 'saved' ? $t('homeOpenArchive') : $t('commissionInvite'));

    // Count-up stats
    const availDisplay = tweened(0, { duration: 1100, easing: cubicOut });
    const collDisplay = tweened(0, { duration: 1100, easing: cubicOut });

    // Magnetic hover action for CTAs
    function magnetic(node: HTMLElement, strength = 0.3) {
        if (
            typeof window !== 'undefined' &&
            (window.matchMedia('(hover: none)').matches || window.matchMedia('(prefers-reduced-motion: reduce)').matches)
        ) {
            return { destroy() {} };
        }

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

    function parseWorkFilter(value: string | null): WorkFilter | null {
        return WORK_FILTERS.includes(value as WorkFilter) ? value as WorkFilter : null;
    }

    function setWorkFilter(filter: WorkFilter) {
        activeWorkFilter = filter;
        const params = new URLSearchParams(page.url.searchParams);
        params.set('view', filter);
        const query = params.toString();
        goto(`${page.url.pathname}${query ? `?${query}` : ''}#available-works`, {
            replaceState: true,
            noScroll: true,
            keepFocus: true,
        });
    }

    $effect(() => {
        const urlFilter = parseWorkFilter(page.url.searchParams.get('view'));
        if (urlFilter && urlFilter !== activeWorkFilter) {
            activeWorkFilter = urlFilter;
        }
    });

    async function init() {
        try {
            const [dbZones, bgPath, figurines, inProgress, content] = await Promise.all([
                api.getCabinetZones().catch(() => DEFAULT_ZONES),
                api.getMainBackground().catch(() => null),
                api.getAllFigurines().catch(() => [] as FigurineListItem[]),
                api.getInProgressFigurines().catch(() => [] as FigurineListItem[]),
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
            collectionFigurines = sortFeaturedFigurines(visibleFigurines);
            collectionTotal = visibleFigurines.length;
            availableTotal = figurines.filter((item) => item.status === 'available').length;
            availableFigurines = sortFeaturedFigurines(visibleFigurines.filter((item) => item.status === 'available'));
            inProgressFigurines = sortFeaturedFigurines(inProgress);
            archivePreviewFigurines = sortFeaturedFigurines(visibleFigurines);
            heroFigurine = content.heroFigurineId
                ? visibleFigurines.find((item) => item.id === content.heroFigurineId) ?? null
                : null;
            activeWorkFilter = parseWorkFilter(page.url.searchParams.get('view')) ?? (!isReleaseMode && availableFigurines.length > 0
                ? 'available'
                : inProgressFigurines.length > 0
                    ? 'upcoming'
                    : 'archive');
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
        savedFigurines.load();
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

                <div class="hero-proof" aria-label="Gotiga">
                    <span>{$t('homeTrustUnique')}</span>
                    <span>{$t('homeTrustHandmade')}</span>
                    <span>{$t('homeTrustAuthorReply')}</span>
                </div>

                {#if heroQuickFigurines.length > 0}
                    <div class="hero-work-strip" aria-label={heroQuickTitle}>
                        <div class="hero-work-strip-head">
                            <span>{heroQuickTitle}</span>
                            {#if showStats}
                                <em>{Math.round($availDisplay)} {$t('homeAvailableStat')} · {Math.round($collDisplay)} {$t('homeArchiveStat')}</em>
                            {/if}
                        </div>
                        <div class="hero-work-links">
                            {#each heroQuickFigurines as fig}
                                <a href="/figurines/{fig.id}" class="hero-work-link" aria-label="{$t('homeViewFigurine')}: {fig.name}">
                                    {#if fig.faceImageUrl}
                                        <AppImage src={fig.faceImageUrl} thumbUrl={fig.thumbUrl} alt={fig.name} class="hero-work-img" loading="lazy" />
                                    {:else}
                                        <span class="hero-work-placeholder">?</span>
                                    {/if}
                                    <span>
                                        <strong>{fig.name}</strong>
                                        <em>{fig.year ?? $t('homeHeroQuickOpen')}</em>
                                    </span>
                                </a>
                            {/each}
                        </div>
                    </div>
                {:else}
                    <p class="release-note">{$t('homeReleaseNote')}</p>
                {/if}
            </div>

            <!-- Right: image -->
            <div class="hero-visual">
                <div class="img-meta">
                    <span>№ 001</span>
                    <span>{$t('homeHeroMapLabel')}</span>
                </div>

                <div
                    class="img-frame"
                    role="group"
                    aria-label={$t('homeHeroMapLabel')}
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

        <section id="available-works" class="context-section work-hub" aria-labelledby="context-title">
            <div class="context-hd work-hd">
                <div>
                    <p class="eyebrow">
                        <span class="eyebrow-rule"></span>
                        {$t('homeWorksEyebrow')}
                    </p>
                    <h2 id="context-title" class="context-title">{$t('homeWorksTitle')}</h2>
                </div>
                <div class="context-side">
                    <p class="context-desc">{activeWorkText}</p>
                    <a href={activeWorkHref} class="all-link">
                        {activeWorkCta}
                        <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                            <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                        </svg>
                    </a>
                </div>
            </div>

            {#if savedPreviewFigurines.length > 0}
                <aside class="saved-rail" aria-label={$t('homeSavedTitle')}>
                    <div class="saved-rail-copy">
                        <span>{$t('homeSavedEyebrow')}</span>
                        <strong>{$t('homeSavedTitle')} · {savedWorkFigurines.length}</strong>
                    </div>
                    <div class="saved-rail-items">
                        {#each savedPreviewFigurines as fig}
                            <a class="saved-rail-item" href="/figurines/{fig.id}" aria-label="{$t('homeViewFigurine')}: {fig.name}">
                                {#if fig.faceImageUrl}
                                    <AppImage src={fig.faceImageUrl} thumbUrl={fig.thumbUrl} alt={fig.name} class="saved-rail-img" loading="lazy" />
                                {:else}
                                    <span>?</span>
                                {/if}
                                <span>{fig.name}</span>
                            </a>
                        {/each}
                    </div>
                    <a href="/profile" class="saved-rail-link">{$t('homeSavedProfile')}</a>
                </aside>
            {/if}

            <div class="work-tabs" role="group" aria-label={$t('homeWorksTitle')}>
                <button
                    class:active={activeWorkFilter === 'available'}
                    onclick={() => setWorkFilter('available')}
                    aria-pressed={activeWorkFilter === 'available'}
                    type="button"
                >
                    {$t('homeWorksAvailableTab')}
                    <span>{availableFigurines.length}</span>
                </button>
                <button
                    class:active={activeWorkFilter === 'saved'}
                    onclick={() => setWorkFilter('saved')}
                    aria-pressed={activeWorkFilter === 'saved'}
                    type="button"
                >
                    {$t('homeWorksSavedTab')}
                    <span>{savedWorkFigurines.length}</span>
                </button>
                <button
                    class:active={activeWorkFilter === 'upcoming'}
                    onclick={() => setWorkFilter('upcoming')}
                    aria-pressed={activeWorkFilter === 'upcoming'}
                    type="button"
                >
                    {$t('homeWorksUpcomingTab')}
                    <span>{inProgressFigurines.length}</span>
                </button>
                <button
                    class:active={activeWorkFilter === 'archive'}
                    onclick={() => setWorkFilter('archive')}
                    aria-pressed={activeWorkFilter === 'archive'}
                    type="button"
                >
                    {$t('homeWorksArchiveTab')}
                    <span>{archivePreviewFigurines.length}</span>
                </button>
            </div>

            {#if visibleWorkFigurines.length > 0}
                <div class="work-grid">
                    {#each visibleWorkFigurines as fig, i}
                        <HomeFigurineTile {fig} index={i} />
                    {/each}
                </div>
            {:else}
                <div class="work-empty">
                    <p>{activeWorkEmptyText}</p>
                    <a href={activeWorkEmptyHref} class="all-link">
                        {activeWorkEmptyCta}
                        <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                            <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                        </svg>
                    </a>
                </div>
            {/if}

            {#if activeWorkFigurines.length > visibleWorkFigurines.length}
                <div class="work-more">
                    <a href={activeWorkHref} class="all-link">
                        {$t('homeWorksShowAll')}
                        <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                            <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                        </svg>
                    </a>
                </div>
            {/if}
        </section>

        <section id="request-path" class="request-path compact-request" aria-labelledby="request-path-title">
            <div class="request-copy">
                <p class="eyebrow">
                    <span class="eyebrow-rule"></span>
                    {$t('homeHowEyebrow')}
                </p>
                <h2 id="request-path-title" class="request-title">{$t('homeHowTitle')}</h2>
            </div>

            <div class="request-flow">
                <span><b>01</b><strong>{$t('homeHowStep1Title')}</strong><em>{$t('homeHowStep1Text')}</em></span>
                <span><b>02</b><strong>{$t('homeHowStep2Title')}</strong><em>{$t('homeHowStep2Text')}</em></span>
                <span><b>03</b><strong>{$t('homeHowStep3Title')}</strong><em>{$t('homeHowStep3Text')}</em></span>
                <a href="/commission">{$t('commissionInvite')}</a>
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
        --site-header-height: 68px;
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
        grid-template-columns: minmax(360px, 0.84fr) minmax(520px, 1.16fr);
        gap: clamp(30px, 4.2vw, 72px);
        align-items: center;
        min-height: min(600px, calc(100svh - var(--site-header-height, 68px)));
        padding: clamp(28px, 4.2vw, 52px) clamp(20px, 4.5vw, 64px) clamp(18px, 2.5vw, 32px);
        max-width: 1520px;
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
        font-size: clamp(48px, 4.65vw, 78px);
        font-weight: 300;
        line-height: 0.96;
        letter-spacing: 0;
        color: var(--ink);
        max-width: 10.8ch;
        margin: 0 0 18px;
        word-break: keep-all;
        overflow-wrap: anywhere;
        hyphens: none;
        display: flex;
        flex-wrap: wrap;
        column-gap: 0.18em;
        row-gap: 0.04em;
        overflow: visible;
        padding-bottom: 0.12em;
    }

    .hero-title:lang(ru) {
        font-size: clamp(44px, 4.3vw, 72px);
    }

    .title-word {
        display: inline-block;
        max-width: 100%;
        overflow-wrap: anywhere;
        white-space: normal;
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
        .title-word,
        .fog-a,
        .fog-b,
        .grain,
        .sc-line,
        .zone-pulse {
            animation: none;
            transform: none;
            opacity: 1;
        }

        .cursor-glow {
            display: none;
        }
    }

    .hero-lead {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(16px, 1.55vw, 20px);
        font-weight: 300;
        font-style: italic;
        line-height: 1.46;
        color: rgba(52,37,28,0.76);
        max-width: 390px;
        margin-bottom: 18px;
    }

    /* ── CTAs ────────────────────────────────────── */
    .hero-ctas {
        display: flex;
        align-items: center;
        gap: 14px;
        flex-wrap: wrap;
        margin-bottom: 14px;
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

    .hero-proof {
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 8px 14px;
        max-width: 520px;
        color: rgba(52,37,28,0.60);
        font-size: 8.5px;
        letter-spacing: 0.14em;
        line-height: 1.45;
        text-transform: uppercase;
    }

    .hero-proof span {
        position: relative;
        padding-left: 13px;
    }

    .hero-proof span::before {
        content: "";
        position: absolute;
        left: 0;
        top: 0.62em;
        width: 6px;
        height: 1px;
        background: rgba(198,95,60,0.58);
    }

    .hero-work-strip {
        max-width: 540px;
        margin-top: 16px;
        padding-top: 14px;
        border-top: 1px solid rgba(52,37,28,0.10);
    }

    .hero-work-strip-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 14px;
        margin-bottom: 10px;
        font-size: 8.5px;
        letter-spacing: 0.16em;
        line-height: 1.3;
        text-transform: uppercase;
        color: rgba(95,70,54,0.64);
    }

    .hero-work-strip-head em {
        font-style: normal;
        color: rgba(95,70,54,0.42);
        text-align: right;
    }

    .hero-work-links {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: 8px;
    }

    .hero-work-link {
        min-width: 0;
        display: grid;
        grid-template-columns: 42px minmax(0, 1fr);
        align-items: center;
        gap: 9px;
        min-height: 54px;
        padding: 5px;
        border: 1px solid rgba(52,37,28,0.12);
        background: rgba(255,249,240,0.58);
        color: rgba(52,37,28,0.78);
        text-decoration: none;
        transition: border-color 0.2s, background 0.2s, transform 0.2s;
    }

    .hero-work-link:hover {
        border-color: rgba(198,95,60,0.34);
        background: rgba(255,249,240,0.88);
        transform: translateY(-1px);
    }

    .hero-work-link :global(.hero-work-img),
    .hero-work-link :global(.hero-work-img .app-image-main),
    .hero-work-link :global(.hero-work-img .app-image-thumb),
    .hero-work-placeholder {
        width: 42px;
        height: 42px;
        display: block;
        object-fit: cover;
        object-position: center 42%;
        background: rgba(201,168,117,0.12);
    }

    .hero-work-placeholder {
        display: grid;
        place-items: center;
        font-family: 'Cormorant Garamond', serif;
        color: rgba(95,70,54,0.42);
    }

    .hero-work-link span:last-child {
        min-width: 0;
        display: grid;
        gap: 3px;
    }

    .hero-work-link strong {
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
        font-family: 'Cormorant Garamond', serif;
        font-size: 18px;
        font-weight: 400;
        line-height: 1;
    }

    .hero-work-link em {
        font-size: 8px;
        font-style: normal;
        letter-spacing: 0.14em;
        text-transform: uppercase;
        color: rgba(95,70,54,0.46);
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
        height: clamp(300px, 38svh, 460px);
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
        background: radial-gradient(ellipse at center, transparent 32%, rgba(44,23,16,0.38) 100%);
        pointer-events: none;
    }

    .img-grade {
        position: absolute;
        inset: 0; z-index: 3;
        background: linear-gradient(180deg, rgba(198,95,60,0.04) 0%, transparent 46%, rgba(44,23,16,0.12) 100%);
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

    /* ── WORK HUB / SAVED RAIL ───────────────────── */
    .hero-work-link:focus-visible,
    .saved-rail-link:focus-visible,
    .all-link:focus-visible,
    .request-flow a:focus-visible {
        outline: 2px solid rgba(198,95,60,0.56);
        outline-offset: 3px;
    }

    .saved-rail {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr) auto;
        align-items: center;
        gap: clamp(14px, 2vw, 24px);
        margin: 0 0 clamp(16px, 2vw, 24px);
        padding: 10px 12px;
        border: 1px solid rgba(52,37,28,0.10);
        background: rgba(255,249,240,0.42);
    }

    .saved-rail-copy {
        display: grid;
        gap: 4px;
        min-width: 142px;
    }

    .saved-rail-copy span,
    .saved-rail-link {
        font-size: 8.5px;
        letter-spacing: 0.16em;
        text-transform: uppercase;
        color: var(--muted2);
    }

    .saved-rail-copy strong {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(20px, 1.8vw, 28px);
        font-weight: 300;
        line-height: 1;
        color: var(--ink);
    }

    .saved-rail-items {
        display: flex;
        align-items: center;
        gap: 8px;
        min-width: 0;
        overflow-x: auto;
        scrollbar-width: none;
    }

    .saved-rail-items::-webkit-scrollbar {
        display: none;
    }

    .saved-rail-item {
        position: relative;
        width: clamp(112px, 11vw, 154px);
        min-height: 54px;
        display: grid;
        grid-template-columns: 42px minmax(0, 1fr);
        align-items: center;
        gap: 9px;
        padding: 5px;
        flex: 0 0 auto;
        overflow: hidden;
        border: 1px solid rgba(52,37,28,0.14);
        background: rgba(255,249,240,0.74);
        color: rgba(95,70,54,0.42);
        text-decoration: none;
        transition: transform 0.22s, border-color 0.22s;
    }

    .saved-rail-item:hover {
        transform: translateY(-2px);
        border-color: rgba(198,95,60,0.34);
    }

    .saved-rail-item:focus-visible {
        outline: 2px solid rgba(198,95,60,0.56);
        outline-offset: 3px;
    }

    .saved-rail-item :global(.saved-rail-img),
    .saved-rail-item :global(.saved-rail-img .app-image-main),
    .saved-rail-item :global(.saved-rail-img .app-image-thumb) {
        width: 42px;
        height: 42px;
        object-fit: cover;
        object-position: center 42%;
        display: block;
    }

    .saved-rail-item > span {
        min-width: 0;
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
        font-family: 'Cormorant Garamond', serif;
        font-size: 17px;
        line-height: 1;
        color: rgba(52,37,28,0.74);
    }

    .saved-rail-link {
        color: var(--mid);
        text-decoration: none;
        border-bottom: 1px solid rgba(111,59,36,0.22);
        padding-bottom: 3px;
        white-space: nowrap;
    }

    .saved-rail-link:hover {
        color: var(--copper);
    }

    /* ── CONTEXT SECTION ─────────────────────────── */
    .context-section {
        padding: clamp(22px, 3.2vw, 44px) clamp(20px, 4.5vw, 64px) clamp(46px, 6vw, 82px);
        max-width: 1520px;
        margin: 0 auto;
    }

    .context-hd {
        display: grid;
        grid-template-columns: minmax(280px, 0.82fr) minmax(320px, 1.18fr);
        gap: clamp(20px, 3vw, 48px);
        align-items: end;
        margin-bottom: clamp(16px, 2vw, 24px);
        padding-bottom: clamp(14px, 2vw, 22px);
        border-bottom: 1px solid var(--border);
    }

    .context-title {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(32px, 3.4vw, 52px);
        font-weight: 300;
        line-height: 0.92;
        color: var(--ink);
        margin-top: 10px;
    }

    .context-side {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 18px;
        padding-bottom: 4px;
    }

    .context-desc {
        font-family: 'Cormorant Garamond', serif;
        font-size: 17px;
        font-weight: 300;
        font-style: italic;
        line-height: 1.5;
        color: var(--muted);
        max-width: 440px;
    }

    .work-tabs {
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 8px;
        margin-bottom: clamp(18px, 2.4vw, 30px);
    }

    .work-tabs button {
        min-height: 34px;
        display: inline-flex;
        align-items: center;
        gap: 8px;
        padding: 0 13px;
        border: 1px solid rgba(52,37,28,0.12);
        background: rgba(255,249,240,0.48);
        color: rgba(95,70,54,0.66);
        font-size: 8.5px;
        letter-spacing: 0.14em;
        text-transform: uppercase;
        cursor: pointer;
        transition: color 0.2s, border-color 0.2s, background 0.2s;
    }

    .work-tabs button span {
        color: rgba(95,70,54,0.36);
    }

    .work-tabs button.active {
        color: var(--ink);
        border-color: rgba(198,95,60,0.36);
        background: rgba(255,249,240,0.86);
    }

    .work-tabs button:disabled {
        opacity: 0.36;
        cursor: not-allowed;
    }

    .work-tabs button:focus-visible {
        outline: 2px solid rgba(198,95,60,0.52);
        outline-offset: 2px;
    }

    .work-grid {
        display: grid;
        grid-template-columns: repeat(5, minmax(0, 1fr));
        gap: clamp(10px, 1.2vw, 16px);
    }

    .work-empty {
        display: grid;
        justify-items: start;
        gap: 14px;
        padding: 30px 0;
    }

    .work-empty p {
        margin: 0;
        font-family: 'Cormorant Garamond', serif;
        font-size: 19px;
        font-style: italic;
        color: var(--muted);
    }

    .work-more {
        display: flex;
        justify-content: center;
        margin-top: clamp(22px, 3vw, 38px);
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

    /* ── REQUEST PATH ───────────────────────────── */
    .request-path {
        display: grid;
        grid-template-columns: minmax(220px, 0.62fr) minmax(360px, 1.38fr);
        gap: clamp(18px, 3vw, 46px);
        align-items: center;
        max-width: 1680px;
        margin: 0 auto;
        padding: 0 clamp(20px, 4.5vw, 72px) clamp(46px, 6vw, 82px);
    }

    .compact-request {
        border-top: 1px solid rgba(52,37,28,0.10);
        padding-top: clamp(22px, 3.2vw, 42px);
    }

    .request-title {
        margin: 8px 0 0;
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(28px, 3.4vw, 48px);
        font-weight: 300;
        line-height: 0.96;
        color: var(--ink);
    }

    .request-flow {
        display: grid;
        grid-template-columns: repeat(4, minmax(0, 1fr));
        gap: 10px;
    }

    .request-flow span,
    .request-flow a {
        min-height: 64px;
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 10px 12px;
        border: 1px solid rgba(52,37,28,0.12);
        background: rgba(255,249,240,0.46);
        color: rgba(52,37,28,0.72);
        font-size: 9px;
        letter-spacing: 0.12em;
        line-height: 1.25;
        text-transform: uppercase;
        text-decoration: none;
    }

    .request-flow span {
        display: grid;
        grid-template-columns: auto 1fr;
        align-items: start;
        column-gap: 10px;
        row-gap: 3px;
    }

    .request-flow b {
        grid-row: span 2;
        font-family: 'Cormorant Garamond', serif;
        font-size: 20px;
        font-weight: 300;
        line-height: 1;
        color: rgba(198,95,60,0.78);
    }

    .request-flow strong {
        font-weight: 500;
        color: rgba(52,37,28,0.78);
    }

    .request-flow em {
        display: block;
        max-width: 26ch;
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
        font-family: 'Cormorant Garamond', serif;
        font-size: 13px;
        font-style: italic;
        letter-spacing: 0;
        text-transform: none;
        color: var(--muted);
    }

    .request-flow a {
        justify-content: center;
        color: var(--mid);
        border-color: rgba(198,95,60,0.24);
    }

    .request-flow a:hover {
        color: var(--copper);
        border-color: rgba(198,95,60,0.44);
        background: rgba(255,246,239,0.86);
    }

    /* ── RESPONSIVE ──────────────────────────────── */
    @media (max-width: 1080px) {
        .hero {
            grid-template-columns: 1fr;
            min-height: auto;
            padding-top: 50px;
            gap: 28px;
        }

        .hero-visual { order: 2; }
        .hero-text { order: 1; max-width: 580px; }

        .img-frame { height: min(42svh, 430px); }

        .context-hd { grid-template-columns: 1fr; }

        .saved-rail {
            grid-template-columns: 1fr auto;
        }

        .saved-rail-items {
            grid-column: 1 / -1;
            order: 3;
        }

        .work-grid { grid-template-columns: repeat(3, minmax(0, 1fr)); }

        .request-path {
            grid-template-columns: 1fr;
            padding-top: 16px;
        }

        .request-flow {
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }
    }

    @media (max-width: 680px) {
        :root {
            --site-header-height: 58px;
        }

        .hero {
            padding: 38px 16px 22px;
            gap: 24px;
        }

        .hero-title,
        .hero-title:lang(ru) {
            font-size: clamp(40px, 12vw, 60px);
            line-height: 0.98;
        }

        .hero-lead { font-size: 16px; max-width: 330px; }

        .cta-primary { height: 40px; padding: 0 16px; font-size: 9px; }

        .cta-ghost {
            min-height: 40px;
            padding: 0 2px;
            font-size: 9px;
        }

        .hero-proof {
            font-size: 8.5px;
            line-height: 1.45;
        }

        .hero-work-strip-head {
            align-items: flex-start;
            flex-direction: column;
            gap: 4px;
        }

        .hero-work-strip-head em {
            text-align: left;
        }

        .hero-work-links {
            display: flex;
            overflow-x: auto;
            scrollbar-width: none;
        }

        .hero-work-links::-webkit-scrollbar {
            display: none;
        }

        .hero-work-link {
            width: 190px;
            flex: 0 0 auto;
        }

        .release-note {
            margin-top: 26px;
            font-size: 16px;
        }

        .img-frame { height: 34svh; min-height: 260px; }

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

        .saved-rail {
            grid-template-columns: 1fr;
            gap: 12px;
        }

        .saved-rail-copy {
            min-width: 0;
        }

        .saved-rail-link {
            justify-self: start;
        }

        .context-section {
            padding-inline: 16px;
            padding-top: 28px;
            padding-bottom: 54px;
        }

        .context-title { font-size: clamp(32px, 9vw, 52px); }
        .context-desc { font-size: 16px; }

        .work-tabs {
            overflow-x: auto;
            flex-wrap: nowrap;
            scrollbar-width: none;
        }

        .work-tabs::-webkit-scrollbar {
            display: none;
        }

        .work-tabs button {
            flex: 0 0 auto;
        }

        .work-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }

        .request-path {
            padding: 24px 16px 70px;
        }

        .request-flow {
            grid-template-columns: 1fr;
        }

    }

    @media (hover: none) {
        .zone-tip { display: none; }
        .cursor-glow { display: none; }
        .cta-primary,
        .cta-ghost {
            transform: none !important;
        }
    }

    @media (max-width: 460px) {
        .work-grid {
            grid-template-columns: 1fr;
        }
    }
</style>
