<script lang="ts">
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { page } from '$app/state';
    import { fade, fly } from 'svelte/transition';
    import { cubicOut } from 'svelte/easing';
    import { spring } from 'svelte/motion';
    import { api } from '$lib/api';
    import type { CabinetZone, FigurineListItem, HomeContent, WorkshopFeature } from '$lib/types/api';
    import { t, brandName, lang } from '$lib/i18n';
    import AppImage from '$lib/components/AppImage.svelte';
    import HomeFigurineTile from '$lib/components/HomeFigurineTile.svelte';
    import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
    import { SITE_URL } from '$lib/site';

    let { data } = $props();

    // WebSite + Organization graph — anchors the brand for search engines and LLMs and
    // ties every other JSON-LD node (figurines, the author) back to one named entity.
    // The Organization carries a logo so Google can show it in brand/knowledge panels.
    let websiteJsonLd = $derived(JSON.stringify({
        '@context': 'https://schema.org',
        '@graph': [
            {
                '@type': 'WebSite',
                '@id': `${SITE_URL}/#website`,
                name: $brandName,
                url: SITE_URL,
                description: 'An author\'s cabinet of gothic figures and handmade miniatures.',
                publisher: { '@id': `${SITE_URL}/#org` },
            },
            {
                '@type': 'Organization',
                '@id': `${SITE_URL}/#org`,
                name: $brandName,
                url: SITE_URL,
                logo: {
                    '@type': 'ImageObject',
                    url: `${SITE_URL}/favicon.png`,
                },
            },
        ],
    }));

    let zones = $state<CabinetZone[]>([]);
    let isLoaded = $state(false);
    let imageLoaded = $state(false);
    let hoveredZone = $state<CabinetZone | null>(null);
    let isNavigating = $state(false);
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
    let workshopFeature = $state<WorkshopFeature>({
        visible: true,
        photoBack: null,
        photoFront: null,
        eyebrowEn: null,
        eyebrowRu: null,
        titleEn: null,
        titleRu: null,
        textEn: null,
        textRu: null,
        link1LabelEn: null,
        link1LabelRu: null,
        link1Href: null,
        link2LabelEn: null,
        link2LabelRu: null,
        link2Href: null,
    });
    let mouseX = $state(0.5);
    let mouseY = $state(0.5);
    let canUseHeroTilt = $state(false);

    // Pick the field for the current language, falling back to the other language.
    const wfLoc = (l: string, en?: string | null, ru?: string | null): string =>
        ((l === 'ru' ? (ru || en) : (en || ru)) ?? '').trim();

    let wfEyebrow = $derived(wfLoc($lang, workshopFeature.eyebrowEn, workshopFeature.eyebrowRu) || $t('homeWorkshopCta'));
    let wfTitle = $derived(wfLoc($lang, workshopFeature.titleEn, workshopFeature.titleRu) || $t('homeStudioTitle'));
    let wfText = $derived(wfLoc($lang, workshopFeature.textEn, workshopFeature.textRu) || $t('homeStudioText'));
    let wfPhotoBack = $derived(workshopFeature.photoBack?.trim() || '/images/workshop/master-1.jpg');
    let wfPhotoFront = $derived(workshopFeature.photoFront?.trim() || '/images/workshop/master-2.jpg');
    let wfLink1Label = $derived(wfLoc($lang, workshopFeature.link1LabelEn, workshopFeature.link1LabelRu) || $t('homeWorkshopCta'));
    let wfLink1Href = $derived(workshopFeature.link1Href?.trim() || '/workshop');
    let wfLink2Label = $derived(wfLoc($lang, workshopFeature.link2LabelEn, workshopFeature.link2LabelRu) || $t('navAuthor'));
    let wfLink2Href = $derived(workshopFeature.link2Href?.trim() || '/author');

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
    let heroObjectCta = $derived(heroObjectName ? $t('homeHeroObjectOpen') : $t('homeSecondaryCta'));
    let titleWords = $derived(titleText.split(/\s+/).filter(Boolean));
    let titleLines = $derived((() => {
        if (titleWords.length <= 2) return [{ words: titleWords, offset: 0 }];
        return [
            { words: titleWords.slice(0, 1), offset: 0 },
            { words: titleWords.slice(1), offset: 1 },
        ];
    })());
    let heroObjectHref = $derived(heroFigurine ? `/figurines/${heroFigurine.id}` : '/figurines');
    let showHeroCaption = $derived(Boolean(heroObjectName));
    let savedWorkFigurines = $derived(
        savedFigurines.ids
            .map((id) => collectionFigurines.find((item) => item.id === id))
            .filter((item): item is FigurineListItem => Boolean(item))
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
            const [dbZones, bgPath, figurines, inProgress, content, workshop] = await Promise.all([
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
                } satisfies HomeContent)),
                api.getWorkshopFeature().catch(() => null)
            ]);
            if (bgPath) imageUrl = bgPath;
            homeContent = content;
            if (workshop) workshopFeature = workshop;
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
        if (!canUseHeroTilt) return;
        const { innerWidth, innerHeight } = window;
        mouseX = e.clientX / innerWidth;
        mouseY = e.clientY / innerHeight;
        parallaxSpring.set({ x: (mouseX - 0.5) * 2, y: (mouseY - 0.5) * 2 });
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
        const reduceMq = window.matchMedia('(prefers-reduced-motion: reduce)');
        const pointerMq = window.matchMedia('(pointer: fine)');
        const syncTiltPreference = () => {
            canUseHeroTilt = pointerMq.matches && !reduceMq.matches;
            if (!canUseHeroTilt) {
                mouseX = 0.5;
                mouseY = 0.5;
                parallaxSpring.set({ x: 0, y: 0 });
            }
        };
        syncTiltPreference();
        reduceMq.addEventListener('change', syncTiltPreference);
        pointerMq.addEventListener('change', syncTiltPreference);
        const hintTimer = setTimeout(() => { if (!hoveredZone) showHint = true; }, 3000);
        return () => {
            clearTimeout(hintTimer);
            reduceMq.removeEventListener('change', syncTiltPreference);
            pointerMq.removeEventListener('change', syncTiltPreference);
        };
    });
</script>

<svelte:head>
    <title>{$brandName} — Cabinet of Gothic Miniatures</title>
    <meta name="description" content="An author's cabinet of gothic figures and handmade miniatures." />
    <meta property="og:site_name" content={$brandName} />
    <meta property="og:locale" content="en_US" />
    <meta property="og:title" content="{$brandName} — Cabinet of Gothic Miniatures" />
    <meta property="og:description" content="An author's cabinet of gothic figures and handmade miniatures." />
    <meta property="og:image" content={data.ogImage} />
    <meta property="og:type" content="website" />
    <meta property="og:url" content={SITE_URL} />
    <meta name="twitter:card" content="summary_large_image" />
    <meta name="twitter:title" content="{$brandName} — Cabinet of Gothic Miniatures" />
    <meta name="twitter:image" content={data.ogImage} />
    <meta name="theme-color" content="#f8f1e7" />
    {@html `<script type="application/ld+json">${websiteJsonLd}<\/script>`}
    <!-- Fonts loaded once globally in app.html -->
</svelte:head>

<svelte:window onmousemove={handleMouseMove} />

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
                    {#each titleLines as line}
                        <span class="title-line" aria-hidden="true">
                            {#each line.words as word, i}
                                <span
                                    class="title-word"
                                    class:accent={line.offset + i === titleWords.length - 1}
                                    style="animation-delay:{0.12 + (line.offset + i) * 0.08}s"
                                >{word}</span>
                            {/each}
                        </span>
                    {/each}
                </h1>

                <p class="hero-lead">{leadText}</p>

                <div class="hero-ctas">
                    <a href={primaryCtaHref} class="cta-primary">
                        {primaryCtaText}
                        <svg class="cta-arrow" width="18" height="9" viewBox="0 0 18 9" fill="none">
                            <path d="M0 4.5H17M17 4.5L12.5 1M17 4.5L12.5 8" stroke="currentColor" stroke-width="1"/>
                        </svg>
                    </a>
                    <a href={secondaryCtaHref} class="cta-ghost">{secondaryCtaText}</a>
                </div>

                <div class="hero-proof" aria-label={$brandName}>
                    <span>{$t('homeTrustUnique')}</span>
                    <span>{$t('homeTrustHandmade')}</span>
                    <span>{$t('homeTrustAuthorReply')}</span>
                </div>

                <!-- Быстрая полоска available убрана: дублировала грид «Works» ниже. -->
                {#if availableFigurines.length === 0}
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
                            rotateY({canUseHeroTilt ? $parallaxSpring.x * -1.2 : 0}deg)
                            rotateX({canUseHeroTilt ? $parallaxSpring.y * 1.2 : 0}deg)
                            scale(1.02);
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

            <!-- Saved-rail убран: дублировал таб SAVED и показывал «битое» имя сохранённой работы. -->

            <!-- Пустые табы (count 0) скрыты; активный таб показываем всегда. -->
            <div class="work-tabs" role="group" aria-label={$t('homeWorksTitle')}>
                {#if availableFigurines.length > 0 || activeWorkFilter === 'available'}
                    <button
                        class:active={activeWorkFilter === 'available'}
                        onclick={() => setWorkFilter('available')}
                        aria-pressed={activeWorkFilter === 'available'}
                        type="button"
                    >
                        {$t('homeWorksAvailableTab')}
                        <span>{availableFigurines.length}</span>
                    </button>
                {/if}
                {#if savedWorkFigurines.length > 0 || activeWorkFilter === 'saved'}
                    <button
                        class:active={activeWorkFilter === 'saved'}
                        onclick={() => setWorkFilter('saved')}
                        aria-pressed={activeWorkFilter === 'saved'}
                        type="button"
                    >
                        {$t('homeWorksSavedTab')}
                        <span>{savedWorkFigurines.length}</span>
                    </button>
                {/if}
                {#if inProgressFigurines.length > 0 || activeWorkFilter === 'upcoming'}
                    <button
                        class:active={activeWorkFilter === 'upcoming'}
                        onclick={() => setWorkFilter('upcoming')}
                        aria-pressed={activeWorkFilter === 'upcoming'}
                        type="button"
                    >
                        {$t('homeWorksUpcomingTab')}
                        <span>{inProgressFigurines.length}</span>
                    </button>
                {/if}
                {#if archivePreviewFigurines.length > 0 || activeWorkFilter === 'archive'}
                    <button
                        class:active={activeWorkFilter === 'archive'}
                        onclick={() => setWorkFilter('archive')}
                        aria-pressed={activeWorkFilter === 'archive'}
                        type="button"
                    >
                        {$t('homeWorksArchiveTab')}
                        <span>{archivePreviewFigurines.length}</span>
                    </button>
                {/if}
            </div>

            <div class="work-content">
                {#if visibleWorkFigurines.length > 0}
                    <div class="work-grid" class:work-grid-short={visibleWorkFigurines.length <= 3}>
                        {#each visibleWorkFigurines as fig, i (`${activeWorkFilter}:${fig.id}`)}
                            <HomeFigurineTile {fig} index={i} selected={heroFigurine?.id === fig.id} />
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

                <aside class="work-guide" aria-labelledby="work-guide-title">
                    <p class="work-guide-kicker">{$t('homeHowEyebrow')}</p>
                    <h3 id="work-guide-title">{$t('homeWorksGuideTitle')}</h3>
                    <p>{$t('homeWorksGuideText')}</p>
                    <div class="work-guide-actions">
                        <a href="/commission" class="guide-primary">{$t('commissionInvite')}</a>
                    </div>
                </aside>
            </div>

            {#if activeWorkFigurines.length > 8}
                <a href={activeWorkHref} class="work-more-ledger">
                    <span class="work-more-ledger__rule"></span>
                    <span class="work-more-ledger__label">
                        <span class="work-more-ledger__count">{activeWorkFigurines.length - 8}</span>
                        {$t('homeMoreInArchive')}
                    </span>
                    <span class="work-more-ledger__rule"></span>
                    <svg class="work-more-ledger__arrow" width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                        <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                    </svg>
                </a>
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

            <div class="request-flow" aria-label={$t('homeHowTitle')}>
                <div class="request-steps">
                    <span><b>01</b><strong>{$t('homeHowStep1Title')}</strong><em>{$t('homeHowStep1Text')}</em></span>
                    <span><b>02</b><strong>{$t('homeHowStep2Title')}</strong><em>{$t('homeHowStep2Text')}</em></span>
                    <span><b>03</b><strong>{$t('homeHowStep3Title')}</strong><em>{$t('homeHowStep3Text')}</em></span>
                </div>

                <a href="/commission" class="request-custom">
                    <p>{$t('homeCustomRequestTitle')}</p>
                    <span>{$t('homeCustomRequestText')}</span>
                    <strong class="request-custom-cta">
                        {$t('homeCustomRequestCta')}
                        <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                            <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                        </svg>
                    </strong>
                </a>
            </div>
        </section>

        {#if workshopFeature.visible}
        <section class="workshop-feature" aria-labelledby="workshop-feature-title">
            <div class="workshop-photos" aria-hidden="true">
                <img src={wfPhotoBack} alt="" class="workshop-photo workshop-photo-back" loading="lazy" />
                <img src={wfPhotoFront} alt="" class="workshop-photo workshop-photo-front" loading="lazy" />
            </div>

            <div class="workshop-copy">
                <p class="eyebrow">
                    <span class="eyebrow-rule"></span>
                    {wfEyebrow}
                </p>
                <h2 id="workshop-feature-title" class="workshop-title">{wfTitle}</h2>
                <p class="workshop-text">{wfText}</p>
                <div class="workshop-actions">
                    {#if wfLink1Label}
                    <a href={wfLink1Href} class="workshop-link">
                        {wfLink1Label}
                        <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                            <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                        </svg>
                    </a>
                    {/if}
                    {#if wfLink2Label}
                    <a href={wfLink2Href} class="workshop-link">
                        {wfLink2Label}
                        <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                            <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                        </svg>
                    </a>
                    {/if}
                </div>
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
        --cream:   var(--color-canvas-base);
        --cream2:  var(--color-canvas-raised);
        --ink:     var(--color-ink-primary);
        --brown:   var(--color-ink-primary);
        --mid:     var(--color-ember-deep);
        --tan:     var(--color-ember-ink);
        --copper:  var(--color-ember);
        --gold:    var(--color-ochre);
        /* Сплошные приглушённые чернила (≥ 4.5:1 над фоном по WCAG AA),
           вместо opacity-over-cream, который давал ~1.7–2.9:1. */
        --muted:   var(--color-ink-tertiary);  /* ~6.45:1 */
        --muted2:  var(--color-ink-tertiary);  /* ~6.45:1 — мелкие лейблы */
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
        grid-template-columns: minmax(420px, 0.72fr) minmax(520px, 1.28fr);
        gap: clamp(28px, 3.8vw, 62px);
        align-items: center;
        min-height: auto;
        /* Главная без header-offset — добавляем верхний воздух, чтобы фото и мета-строка
           не уходили под фиксированную шапку. */
        padding:
            calc(var(--site-header-height) + clamp(18px, 2.4vw, 34px))
            clamp(20px, 4.5vw, 64px)
            clamp(28px, 3.2vw, 44px);
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
        font-size: 12px;
        font-weight: 600;
        letter-spacing: 0.10em;
        text-transform: uppercase;
        color: var(--muted2);
        margin-bottom: 12px;
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
        font-size: clamp(42px, 3.8vw, 64px);
        font-weight: 300;
        line-height: 0.94;
        letter-spacing: 0;
        color: var(--ink);
        max-width: min(620px, 100%);
        margin: 0 0 14px;
        word-break: keep-all;
        overflow-wrap: normal;
        hyphens: none;
        display: grid;
        gap: 0.02em;
        overflow: visible;
        padding-bottom: 0.12em;
    }

    .hero-title:lang(ru) {
        font-size: clamp(40px, 3.6vw, 60px);
    }

    .title-line {
        display: flex;
        flex-wrap: wrap;
        column-gap: 0.18em;
        row-gap: 0.02em;
    }

    .title-word {
        display: inline-block;
        max-width: none;
        overflow-wrap: normal;
        white-space: nowrap;
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
        font-size: clamp(15px, 1.35vw, 18px);
        font-weight: 300;
        font-style: italic;
        line-height: 1.42;
        color: var(--color-ink-secondary);
        max-width: 390px;
        margin-bottom: 14px;
    }

    /* ── CTAs ────────────────────────────────────── */
    .hero-ctas {
        display: flex;
        align-items: center;
        gap: 14px;
        flex-wrap: wrap;
        margin-bottom: 12px;
    }

    .cta-primary {
        display: inline-flex;
        align-items: center;
        gap: 12px;
        height: 40px;
        padding: 0 19px;
        background: var(--ink);
        color: var(--cream2);
        font-size: 12px;
        font-weight: 600;
        letter-spacing: 0.09em;
        text-transform: uppercase;
        text-decoration: none;
        transition:
            background 0.22s ease,
            box-shadow 0.22s ease,
            transform 0.12s ease;
        clip-path: polygon(0 0, calc(100% - 7px) 0, 100% 7px, 100% 100%, 7px 100%, 0 calc(100% - 7px));
    }

    .cta-arrow {
        flex-shrink: 0;
        transition: transform 0.22s ease;
    }

    .cta-primary:hover {
        background: var(--mid);
        box-shadow: 0 10px 24px rgba(68,37,20,0.14);
    }

    .cta-primary:hover .cta-arrow {
        transform: translateX(4px);
    }

    .cta-primary:active {
        transform: translateY(1px);
    }

    .cta-ghost {
        display: inline-flex;
        align-items: center;
        min-height: 40px;
        padding: 0 4px;
        color: var(--color-ink-secondary);
        font-size: 12px;
        font-weight: 600;
        letter-spacing: 0.09em;
        text-transform: uppercase;
        text-decoration: none;
        border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 18%, transparent);
        transition:
            color 0.22s ease,
            border-color 0.22s ease,
            transform 0.12s ease;
    }

    .cta-ghost:hover {
        color: var(--copper);
        border-color: rgba(198,95,60,0.5);
    }

    .cta-ghost:active {
        transform: translateY(1px);
    }

    .hero-proof {
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 6px 12px;
        max-width: 520px;
        color: var(--muted);
        font-size: 12.5px;
        letter-spacing: 0.01em;
        line-height: 1.5;
    }

    .hero-proof span {
        position: relative;
        padding-left: 15px;
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
        color: var(--muted);
    }

    /* ── HERO VISUAL ─────────────────────────────── */
    .hero-visual {
        position: relative;
    }

    .img-meta {
        display: flex;
        justify-content: space-between;
        font-size: 11px;
        letter-spacing: 0.10em;
        text-transform: uppercase;
        color: var(--muted2);
        padding-bottom: 8px;
        pointer-events: none;
    }

    .img-frame {
        position: relative;
        width: 100%;
        height: clamp(340px, 46svh, 520px);
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
        opacity: 0.24;
    }

    .fc-tl { top: 30px; left: 0; border-top: 1px solid var(--copper); border-left: 1px solid var(--copper); }
    .fc-tr { top: 30px; right: 0; border-top: 1px solid var(--copper); border-right: 1px solid var(--copper); }
    .fc-bl { bottom: 0; left: 0; border-bottom: 1px solid var(--copper); border-left: 1px solid var(--copper); }
    .fc-br { bottom: 0; right: 0; border-bottom: 1px solid var(--copper); border-right: 1px solid var(--copper); }

    /* Scroll cue */
    .scroll-cue {
        position: absolute;
        bottom: -32px; right: 0;
        display: flex;
        align-items: center;
        gap: 10px;
        font-size: 11px;
        letter-spacing: 0.10em;
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
        display: grid;
        gap: 6px;
        max-width: min(360px, calc(100% - 36px));
        padding: 13px 15px 12px 16px;
        color: #fff7ea;
        text-decoration: none;
        border: 1px solid rgba(255,247,234,0.16);
        border-left: 2px solid rgba(198,95,60,0.72);
        background:
            linear-gradient(90deg, rgba(28,16,10,0.78), rgba(28,16,10,0.58) 72%, rgba(28,16,10,0.18));
        box-shadow: 0 14px 34px rgba(20,10,6,0.34);
        backdrop-filter: blur(10px) saturate(0.9);
        text-shadow: 0 1px 2px rgba(0,0,0,0.55);
        transition: border-color 0.22s ease, background 0.22s ease, transform 0.18s ease;
    }

    .art-caption:hover {
        border-color: rgba(255,247,234,0.28);
        background:
            linear-gradient(90deg, rgba(28,16,10,0.84), rgba(28,16,10,0.64) 72%, rgba(28,16,10,0.22));
        transform: translateY(-1px);
    }

    .art-caption-kicker {
        font-size: 8px;
        letter-spacing: 0.18em;
        line-height: 1;
        text-transform: uppercase;
        color: rgba(255,247,234,0.68);
    }

    .art-caption-name {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(25px, 2.2vw, 34px);
        font-style: italic;
        line-height: 0.95;
        color: #fff7ea;
    }

    .art-caption-meta {
        max-width: 30ch;
        font-size: 9px;
        letter-spacing: 0.08em;
        line-height: 1.35;
        text-transform: uppercase;
        color: rgba(255,247,234,0.72);
    }

    .art-caption-open {
        width: fit-content;
        margin-top: 2px;
        padding-top: 6px;
        border-top: 1px solid rgba(255,247,234,0.18);
        font-size: 9px;
        letter-spacing: 0.14em;
        line-height: 1;
        text-transform: uppercase;
        color: #fff7ea;
    }

    /* ── ZONES ───────────────────────────────────── */
    .zones-layer {
        position: absolute;
        inset: 0;
        z-index: 20;
        opacity: 0.58;
        transition: opacity 0.25s;
    }

    .img-frame:hover .zones-layer {
        opacity: 0.92;
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
        opacity: 0.22;
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

    .zc-tl { top: 0; left: 0; border-top: 1px solid rgba(198,95,60,0.30); border-left: 1px solid rgba(198,95,60,0.30); }
    .zc-tr { top: 0; right: 0; border-top: 1px solid rgba(198,95,60,0.30); border-right: 1px solid rgba(198,95,60,0.30); }
    .zc-bl { bottom: 0; left: 0; border-bottom: 1px solid rgba(198,95,60,0.30); border-left: 1px solid rgba(198,95,60,0.30); }
    .zc-br { bottom: 0; right: 0; border-bottom: 1px solid rgba(198,95,60,0.30); border-right: 1px solid rgba(198,95,60,0.30); }

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
        font-size: 11px;
        letter-spacing: 0.12em;
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

    /* ── WORK HUB ────────────────────────────────── */
    .all-link:focus-visible,
    .request-flow a:focus-visible {
        outline: 2px solid rgba(198,95,60,0.56);
        outline-offset: 3px;
    }

    /* ── CONTEXT SECTION ─────────────────────────── */
    .context-section {
        padding: clamp(14px, 2vw, 26px) clamp(20px, 4.5vw, 64px) clamp(42px, 5.5vw, 72px);
        max-width: 1520px;
        margin: 0 auto;
    }

    .context-hd {
        display: grid;
        grid-template-columns: minmax(220px, 0.42fr) minmax(420px, 0.58fr);
        gap: clamp(18px, 2.4vw, 36px);
        align-items: center;
        margin-bottom: 12px;
        padding-bottom: 12px;
        border-bottom: 1px solid var(--border);
    }

    .context-title {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(30px, 2.8vw, 42px);
        font-weight: 300;
        line-height: 0.96;
        color: var(--ink);
        margin-top: 6px;
    }

    .context-side {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: end;
        gap: 18px;
        padding-bottom: 0;
    }

    .context-desc {
        font-family: 'Cormorant Garamond', serif;
        font-size: 16px;
        font-weight: 300;
        font-style: italic;
        line-height: 1.42;
        color: var(--muted);
        max-width: 520px;
    }

    .work-tabs {
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 8px;
        margin-bottom: 12px;
    }

    .work-tabs button {
        min-height: 34px;
        display: inline-flex;
        align-items: center;
        gap: 8px;
        padding: 0 13px;
        border: 1px solid rgba(52,37,28,0.12);
        background: rgba(255,249,240,0.48);
        color: var(--muted);
        font-size: 12px;
        font-weight: 500;
        letter-spacing: 0.09em;
        text-transform: uppercase;
        cursor: pointer;
        transition: color 0.2s, border-color 0.2s, background 0.2s;
    }

    .work-tabs button span {
        color: var(--muted);
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

    .work-content {
        display: grid;
        grid-template-columns: minmax(0, 1fr) minmax(260px, 320px);
        gap: clamp(16px, 2vw, 28px);
        align-items: start;
    }

    .work-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
        gap: clamp(10px, 1.2vw, 16px);
    }

    .work-grid-short {
        grid-template-columns: repeat(auto-fit, minmax(250px, 320px));
    }

    .work-guide {
        display: grid;
        gap: 12px;
        padding: 18px;
        border: 1px solid rgba(198,95,60,0.20);
        background:
            linear-gradient(180deg, rgba(255,249,240,0.78), rgba(255,252,246,0.52));
        box-shadow: 0 1px 0 rgba(255,255,255,0.78) inset;
    }

    .work-guide-kicker {
        margin: 0;
        font-size: 10px;
        font-weight: 600;
        letter-spacing: 0.12em;
        line-height: 1.2;
        text-transform: uppercase;
        color: var(--copper);
    }

    .work-guide h3 {
        margin: 0;
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-size: clamp(25px, 2vw, 33px);
        font-weight: 300;
        line-height: 1;
        color: var(--ink);
    }

    .work-guide p:not(.work-guide-kicker) {
        margin: 0;
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-size: 16px;
        font-style: italic;
        line-height: 1.38;
        color: var(--color-ink-secondary);
    }

    .work-guide-actions {
        display: grid;
        gap: 9px;
        margin-top: 2px;
    }

    .guide-primary {
        min-height: 36px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        padding: 0 12px;
        font-size: 10px;
        font-weight: 600;
        letter-spacing: 0.1em;
        line-height: 1.2;
        text-align: center;
        text-transform: uppercase;
        text-decoration: none;
    }

    .guide-primary {
        background: var(--ink);
        color: var(--cream2);
    }

    .guide-primary:hover {
        background: var(--mid);
    }

    .guide-primary:focus-visible {
        outline: 2px solid rgba(198,95,60,0.52);
        outline-offset: 3px;
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

    .all-link {
        display: inline-flex;
        align-items: center;
        gap: 10px;
        font-size: 12px;
        font-weight: 600;
        letter-spacing: 0.09em;
        text-transform: uppercase;
        color: var(--mid);
        text-decoration: none;
        padding-bottom: 4px;
        border-bottom: 1px solid rgba(111,59,36,0.22);
        transition: gap 0.28s, color 0.28s;
    }

    .all-link:hover { color: var(--copper); gap: 16px; }

    /* ── ARCHIVE LEDGER LINE ────────────────────── */
    .work-more-ledger {
        display: flex;
        align-items: center;
        gap: 14px;
        margin-top: 24px;
        text-decoration: none;
        color: var(--mid);
        transition: color 0.32s ease;
    }
    .work-more-ledger:hover { color: var(--copper); }

    .work-more-ledger__rule {
        flex: 1;
        height: 1px;
        background: currentColor;
        opacity: 0.15;
        transition: opacity 0.32s ease;
    }
    .work-more-ledger:hover .work-more-ledger__rule { opacity: 0.30; }

    .work-more-ledger__label {
        display: flex;
        align-items: baseline;
        gap: 6px;
        font-size: 10px;
        font-weight: 600;
        letter-spacing: 0.14em;
        text-transform: uppercase;
        white-space: nowrap;
        flex-shrink: 0;
    }

    .work-more-ledger__count {
        font-family: 'Cormorant Garamond', serif;
        font-size: 18px;
        font-weight: 400;
        letter-spacing: 0;
        line-height: 1;
    }

    .work-more-ledger__arrow {
        flex-shrink: 0;
        opacity: 0.55;
        transition: transform 0.28s ease, opacity 0.28s ease;
    }
    .work-more-ledger:hover .work-more-ledger__arrow {
        transform: translateX(4px);
        opacity: 1;
    }

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
        grid-template-columns: minmax(0, 1fr) minmax(240px, 300px);
        gap: 14px;
    }

    .request-steps {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: 12px;
    }

    .request-steps span,
    .request-custom {
        min-height: 96px;
        display: flex;
        align-items: flex-start;
        gap: 12px;
        padding: 14px 15px;
        border: 1px solid rgba(52,37,28,0.12);
        background: rgba(255,249,240,0.58);
        color: var(--color-ink-secondary);
        font-size: 11px;
        letter-spacing: 0.09em;
        line-height: 1.25;
        text-transform: uppercase;
        text-decoration: none;
    }

    .request-steps span {
        display: grid;
        grid-template-columns: auto 1fr;
        align-items: start;
        column-gap: 12px;
        row-gap: 6px;
    }

    .request-steps b {
        grid-row: span 2;
        font-family: 'Cormorant Garamond', serif;
        font-size: 20px;
        font-weight: 300;
        line-height: 1;
        color: var(--copper);
    }

    .request-steps strong {
        font-weight: 600;
        color: var(--ink);
    }

    .request-steps em {
        display: block;
        max-width: 30ch;
        font-family: 'Cormorant Garamond', serif;
        font-size: 13px;
        font-style: italic;
        letter-spacing: 0;
        line-height: 1.3;
        text-transform: none;
        color: var(--muted);
    }

    .request-custom {
        display: grid;
        align-content: start;
        gap: 10px;
        border-color: rgba(198,95,60,0.34);
        background:
            linear-gradient(135deg, rgba(255,246,239,0.88), rgba(255,249,240,0.68));
        color: var(--color-ink-secondary);
        box-shadow: 0 10px 26px rgba(52,37,28,0.06);
        transition: transform 0.22s ease, border-color 0.22s ease, background 0.22s ease, box-shadow 0.22s ease;
    }

    .request-custom:hover {
        transform: translateY(-2px);
        border-color: rgba(198,95,60,0.58);
        background:
            linear-gradient(135deg, rgba(255,242,233,0.96), rgba(255,249,240,0.78));
        box-shadow: 0 18px 38px rgba(52,37,28,0.10);
    }

    .request-custom:focus-visible {
        outline: 2px solid rgba(198,95,60,0.52);
        outline-offset: 3px;
    }

    .request-custom p {
        margin: 0;
        color: var(--ink);
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.09em;
        line-height: 1.25;
        text-transform: uppercase;
    }

    .request-custom span {
        font-family: 'Cormorant Garamond', serif;
        font-size: 14px;
        font-style: italic;
        letter-spacing: 0;
        line-height: 1.3;
        text-transform: none;
        color: var(--muted);
    }

    .request-custom-cta {
        width: fit-content;
        display: inline-flex;
        align-items: center;
        gap: 10px;
        margin-top: 6px;
        padding: 9px 12px;
        border: 1px solid rgba(198,95,60,0.28);
        background: rgba(255,249,240,0.72);
        color: var(--mid);
        font-size: 10px;
        font-weight: 600;
        letter-spacing: 0.1em;
        line-height: 1.1;
        text-transform: uppercase;
        transition: gap 0.22s ease, color 0.22s ease, border-color 0.22s ease;
    }

    .request-custom:hover .request-custom-cta {
        gap: 15px;
        color: var(--copper);
        border-color: rgba(198,95,60,0.52);
    }

    /* ── WORKSHOP FEATURE ───────────────────────── */
    .workshop-feature {
        display: grid;
        grid-template-columns: minmax(430px, 0.92fr) minmax(420px, 1.08fr);
        gap: clamp(42px, 5.4vw, 92px);
        align-items: center;
        max-width: 1680px;
        margin: 0 auto;
        padding: clamp(56px, 7vw, 112px) clamp(20px, 4.5vw, 72px) clamp(70px, 8vw, 128px);
    }

    .workshop-photos {
        position: relative;
        min-height: clamp(420px, 42vw, 680px);
    }

    .workshop-photo {
        position: absolute;
        display: block;
        object-fit: cover;
        border: 1px solid rgba(52,37,28,0.08);
        box-shadow: 0 28px 76px rgba(52,37,28,0.14);
        filter: saturate(0.9) contrast(1.02);
    }

    .workshop-photo-back {
        left: 0;
        top: 0;
        width: 69%;
        height: 74%;
        object-position: center;
    }

    .workshop-photo-front {
        right: 0;
        bottom: 0;
        width: 69%;
        height: 74%;
        object-position: center;
    }

    .workshop-copy {
        max-width: 760px;
        padding-top: clamp(0px, 3vw, 34px);
    }

    .workshop-title {
        margin: 0;
        color: var(--ink);
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(48px, 5.4vw, 92px);
        font-weight: 300;
        letter-spacing: 0;
        line-height: 0.92;
    }

    .workshop-text {
        max-width: 720px;
        margin: 26px 0 0;
        color: var(--muted);
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(20px, 1.45vw, 27px);
        font-style: italic;
        line-height: 1.42;
    }

    .workshop-actions {
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 22px;
        margin-top: 54px;
    }

    .workshop-link {
        display: inline-flex;
        align-items: center;
        gap: 10px;
        width: fit-content;
        padding-bottom: 5px;
        color: var(--mid);
        font-size: 12px;
        font-weight: 600;
        letter-spacing: 0.1em;
        line-height: 1.2;
        text-transform: uppercase;
        text-decoration: none;
        border-bottom: 1px solid rgba(198,95,60,0.25);
        transition: gap 0.24s ease, color 0.24s ease, border-color 0.24s ease;
    }

    .workshop-link:hover {
        gap: 15px;
        color: var(--copper);
        border-color: rgba(198,95,60,0.52);
    }

    .workshop-link:focus-visible {
        outline: 2px solid rgba(198,95,60,0.52);
        outline-offset: 3px;
    }

    /* ── RESPONSIVE ──────────────────────────────── */
    @media (max-width: 1080px) {
        .hero {
            grid-template-columns: 1fr;
            min-height: auto;
            padding-top: calc(var(--site-header-height) + 24px);
            gap: 22px;
        }

        .hero-visual { order: 2; }
        .hero-text { order: 1; max-width: 580px; }

        .img-frame { height: min(40svh, 420px); }

        .context-hd,
        .context-side {
            grid-template-columns: 1fr;
        }

        .work-content {
            grid-template-columns: 1fr;
        }

        .work-guide {
            order: -1;
        }

        .request-path {
            grid-template-columns: 1fr;
            padding-top: 16px;
        }

        .request-flow {
            grid-template-columns: 1fr;
        }

        .request-steps {
            grid-template-columns: repeat(3, minmax(0, 1fr));
        }

        .workshop-feature {
            grid-template-columns: 1fr;
            gap: 34px;
        }

        .workshop-photos {
            min-height: min(72vw, 560px);
        }
    }

    @media (max-width: 680px) {
        :root {
            --site-header-height: 58px;
        }

        .hero {
            padding: calc(var(--site-header-height) + 18px) 16px 22px;
            gap: 18px;
        }

        .hero-title,
        .hero-title:lang(ru) {
            font-size: clamp(38px, 11vw, 56px);
            line-height: 0.98;
        }

        .hero-lead { font-size: 16px; max-width: 330px; }

        .cta-primary { height: 40px; padding: 0 16px; font-size: 11px; }

        .cta-ghost {
            min-height: 40px;
            padding: 0 2px;
            font-size: 11px;
        }

        .hero-proof {
            font-size: 11px;
            line-height: 1.45;
        }

        .release-note {
            margin-top: 26px;
            font-size: 16px;
        }

        .img-frame { height: 30svh; min-height: 230px; }

        .scroll-cue { display: none; }

        .art-caption {
            left: 14px;
            bottom: 14px;
            max-width: calc(100% - 28px);
            padding: 11px 12px 10px 13px;
        }

        .art-caption-name {
            font-size: 24px;
        }

        .art-caption-meta {
            display: none;
        }

        .context-section {
            padding-inline: 16px;
            padding-top: 20px;
            padding-bottom: 54px;
        }

        .context-title { font-size: clamp(30px, 8vw, 44px); }
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

        .work-grid,
        .work-grid-short {
            grid-template-columns: repeat(2, minmax(0, 1fr));
            gap: 12px;
        }

        .request-path {
            padding: 24px 16px 70px;
        }

        .request-flow {
            grid-template-columns: 1fr;
        }

        .request-steps {
            grid-template-columns: 1fr;
        }

        .workshop-feature {
            padding: 36px 16px 74px;
        }

        .workshop-photos {
            min-height: 360px;
        }

        .workshop-photo-back,
        .workshop-photo-front {
            width: 78%;
            height: 67%;
        }

        .workshop-copy {
            padding-top: 0;
        }

        .workshop-title {
            font-size: clamp(42px, 12vw, 60px);
        }

        .workshop-text {
            margin-top: 18px;
            font-size: 18px;
        }

        .workshop-actions {
            align-items: flex-start;
            flex-direction: column;
            gap: 14px;
            margin-top: 30px;
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

        .work-grid-short {
            grid-template-columns: 1fr;
        }
    }
</style>
