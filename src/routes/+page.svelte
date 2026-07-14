<script lang="ts">
    import { onMount } from 'svelte';
    import { fade, fly } from 'svelte/transition';
    import { cubicOut } from 'svelte/easing';
    import { api, resolveSrcset, resolveWebpUrl } from '$lib/api';
    import AppImage from '$lib/components/AppImage.svelte';
    import type { AuthorProfile, FigurineListItem, HomeContent, WorkshopFeature } from '$lib/types/api';
    import { t, brandName, lang } from '$lib/i18n';
    import ReelWorkCard from '$lib/components/ReelWorkCard.svelte';
    import WorkMarginIndex from '$lib/components/WorkMarginIndex.svelte';
    import HouseNoticeBoard from '$lib/components/HouseNoticeBoard.svelte';
    import VisitLedger from '$lib/components/VisitLedger.svelte';
    import VisitorBook from '$lib/components/VisitorBook.svelte';
    import ImpressionsQuoteStrip from '$lib/components/ImpressionsQuoteStrip.svelte';
    import AuthorStory from '$lib/components/AuthorStory.svelte';
    import FirstLook from '$lib/components/FirstLook.svelte';
    import AtelierReel from '$lib/components/AtelierReel.svelte';
    import HeroWorkshopTeaser from '$lib/components/HeroWorkshopTeaser.svelte';
    import WorkshopReelModal from '$lib/components/WorkshopReelModal.svelte';
    import { visitorBook } from '$lib/stores/visitor-book.svelte';
    import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
    import { visitorMarks } from '$lib/stores/visitor-marks.svelte';
    import MarkedByYou from '$lib/components/MarkedByYou.svelte';
    import NoticedByGuests from '$lib/components/NoticedByGuests.svelte';
    import { houseClock } from '$lib/stores/house-clock.svelte';
    import { showingRooms } from '$lib/stores/showing-rooms.svelte';
    import { isShowingOpen, resolveWindow } from '$lib/showing-window';
    import { SITE_URL } from '$lib/site';
    import {
        HOME_MAIN_BLOCK_IDS,
        HOME_BAND_BLOCK_IDS,
        HOME_SHELF_BLOCK_IDS,
        normalizeHomeOrder,
        isHomeBlockVisible,
        homeBlockWrapperStyle,
        homeBlockClasses,
        homePageStyle,
        generateHomeElementCSS,
    } from '$lib/home-layout';
    import type { HomeLayoutConfig, HomeBlockId, ReelTheme } from '$lib/types/api';
    import { generateReelCSS, startListeningForReelPreview } from '$lib/stores/reel-theme.svelte';

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

    // Advances once per calendar day. Still used to turn the vitrine's single "today"
    // exhibit (see dailyPick); it no longer touches the reel — see sortFeaturedFigurines.
    function dayIndex(d = new Date()): number {
        return Math.floor(d.getTime() / 86_400_000);
    }

    // Pick a single element that advances once per calendar day — the vitrine's
    // "today" exhibit, stable through refreshes, freshly turned tomorrow.
    function dailyPick<T>(items: T[]): T | null {
        if (items.length === 0) return null;
        return items[((dayIndex() % items.length) + items.length) % items.length];
    }

    // Whether a work's showing door is currently open — the vitrine must never
    // spotlight a piece that the rest of the site keeps sealed behind its door.
    function isOpenNow(fig: FigurineListItem): boolean {
        return isShowingOpen(
            resolveWindow({ openFromMin: fig.openFromMin, openUntilMin: fig.openUntilMin, showingRoomId: fig.showingRoomId }, showingRooms.list),
            houseClock.nowDate
        );
    }

    /**
     * The reel's order: the author's own, and nothing else.
     *
     * This used to pin the 2 newest works by createdAt and then rotate the rest by a
     * daily offset. Both overrode `sortOrder`, and the rotation did so destructively:
     * the offset is (day number % list length), so on a day when it came out to 16, the
     * reel started at the sixteenth work and everything the author had deliberately put
     * FIRST — the low sortOrder values — was rotated onto the tail, past the 16 the home
     * page shows. Setting a work to sortOrder 1 was the surest way to hide it. The field
     * looked broken; it was being outvoted by the calendar.
     *
     * The daily turn survives where it belongs: on the vitrine's single exhibit of the
     * day (dailyPick), which is a pick, not an ordering.
     */
    function sortFeaturedFigurines(items: FigurineListItem[]) {
        return items.slice().sort((a, b) => {
            const order = (a.sortOrder ?? 0) - (b.sortOrder ?? 0);
            if (order !== 0) return order;
            // Same sortOrder (the admin form lets that happen freely): newest first, so a
            // fresh work at least leads its own tie instead of landing arbitrarily.
            const da = a.createdAt ? new Date(a.createdAt).getTime() : 0;
            const db = b.createdAt ? new Date(b.createdAt).getTime() : 0;
            return db - da;
        });
    }

    let isLoaded = $state(false);
    // The bundled photo is a *fallback* — only used when the API has no background of
    // its own — so it must never be the initial value while a real one is still in
    // flight (that used to render the bundled 268 KB, fetch it, then swap to the real
    // background a moment later and fetch that too — both downloaded, every visit).
    // Now imageUrl is seeded directly from +page.ts's load(), which has already
    // resolved data.bg (or not) by the time this component exists, so there's no
    // "in flight" moment here to render a placeholder for. It also remains the OG
    // fallback in +page.ts, which costs no download.
    const FALLBACK_HERO = '/images/cabinet-bg.jpeg';
    let imageUrl = $state(data.bg || FALLBACK_HERO);
    let hasCustomHeroPhoto = $state(Boolean(data.bg));
    // The works start EMPTY and are filled in init(), on the client. They must not be
    // seeded from load(): this page is prerendered, so a seeded reel is baked into the
    // static HTML and its photos never refresh on hydration (see the note in +page.ts).
    let availableFigurines = $state<FigurineListItem[]>([]);
    let inProgressFigurines = $state<FigurineListItem[]>([]);
    let collectionFigurines = $state<FigurineListItem[]>([]);
    let heroFigurine = $state<FigurineListItem | null>(null);
    // Author-led hero + story content, reused from the admin-editable profile.
    let authorProfile = $state<AuthorProfile | null>(null);
    // First-time vs returning visitor. The retention hooks (today's exhibit,
    // opening soon, "since your visit") are shown HIGH only to people who already
    // know the house — that is who they are for, and burying them kills them. A
    // first-time stranger gets the clean acquisition arc instead. A prior visit is
    // recorded in localStorage; any engagement (signed book, saved, marked) counts.
    let visitedBefore = $state(false);
    let isReturningVisitor = $derived(
        visitedBefore
        || visitorBook.signed
        || savedFigurines.ids.length > 0
        || Object.keys(visitorMarks.marks).length > 0
    );
    // Admin-arranged layout of this page (order / visibility / width / style per
    // block); null = the hard-coded default. The Home Layout admin tab previews
    // changes here live via postMessage into its iframe (see onMount).
    let homeLayout = $state<HomeLayoutConfig | null>(null);
    // Preview-only override of the returning-visitor split, set by the admin
    // editor's visitor-mode switch. Never set on the real site — the actual
    // isReturningVisitor detection below stays untouched.
    let previewVisitorMode = $state<'returning' | 'new' | null>(null);
    // Set once the admin editor starts driving this page over postMessage —
    // from then on the saved config must not overwrite the live draft.
    let hlPreviewDriven = $state(false);
    let effectiveReturning = $derived(
        previewVisitorMode ? previewVisitorMode === 'returning' : isReturningVisitor
    );
    let hlMainOrder = $derived(normalizeHomeOrder(homeLayout?.blockOrder, HOME_MAIN_BLOCK_IDS));
    let hlBandOrder = $derived(normalizeHomeOrder(homeLayout?.bandOrder, HOME_BAND_BLOCK_IDS));
    let hlShelfOrder = $derived(normalizeHomeOrder(homeLayout?.shelfOrder, HOME_SHELF_BLOCK_IDS));
    const hlVisible = (id: HomeBlockId) => isHomeBlockVisible(homeLayout, id);
    const hlClasses = (id: HomeBlockId) => homeBlockClasses(homeLayout, id);
    const hlStyle = (id: HomeBlockId) =>
        homeBlockWrapperStyle(homeLayout, id, hlMainOrder.indexOf(id as (typeof hlMainOrder)[number]));
    const hlSubStyle = (id: HomeBlockId, order: number) =>
        homeBlockWrapperStyle(homeLayout, id, order);
    // Element-level overrides (colour / free-range size / order inside a block)
    // arrive as generated global CSS — `!important` wins over scoped component
    // styles, so the block components themselves stay untouched.
    let hlElementCSS = $derived(generateHomeElementCSS(homeLayout));

    // "Exhibit of the day": admin-pinned pick, else daily rotation. Rendered
    // as a compact mark inside VisitLedger, not its own section.
    let vitrineFig = $state<FigurineListItem | null>(null);
    let collectionTotal = $state(0);
    let availableTotal = $state(0);
    let homeContent = $state<HomeContent>(data.homeContent);
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
    let wfLink1Label = $derived(wfLoc($lang, workshopFeature.link1LabelEn, workshopFeature.link1LabelRu) || $t('homeWorkshopCta'));
    let wfLink1Href = $derived(workshopFeature.link1Href?.trim() || '/workshop');
    let wfLink2Label = $derived(wfLoc($lang, workshopFeature.link2LabelEn, workshopFeature.link2LabelRu) || $t('navAuthor'));
    let wfLink2Href = $derived(workshopFeature.link2Href?.trim() || '/author');

    // Which workshop reel sits on top of the overlapping pair — whichever the
    // visitor last clicked.
    let frontReel = $state<'a' | 'b'>('a');

    // The workshop reels themselves mount (and start fetching video) only once
    // asked for — a hero locket click, or the section physically scrolling
    // into view. Until then the section shows its static poster frames.
    let workshopActivated = $state(false);
    let workshopSectionEl = $state<HTMLElement>();

    onMount(() => {
        if (typeof IntersectionObserver === 'undefined') { workshopActivated = true; return; }
        if (!workshopSectionEl) return;
        const io = new IntersectionObserver(
            ([entry]) => { if (entry.isIntersecting) { workshopActivated = true; io.disconnect(); } },
            { rootMargin: '600px 0px' },
        );
        io.observe(workshopSectionEl);
        return () => io.disconnect();
    });

    // Clicking a hero locket reveals its clip full-size, growing out of the
    // locket's own screen position rather than a plain fade.
    let reelModalOpen = $state(false);
    let reelModalClip = $state<'a' | 'b'>('a');
    let reelModalOrigin = $state<{ x: number; y: number; width: number; height: number } | null>(null);

    function openReelModal(which: 'a' | 'b', e: MouseEvent) {
        frontReel = which;
        workshopActivated = true;
        const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
        reelModalOrigin = { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
        reelModalClip = which;
        reelModalOpen = true;
    }

    // Works-first hero: the maker's name frames the headline, but the image
    // is a real piece, not a scene to explore — the goal is to show and
    // enchant with the work itself, first thing.
    let authorName = $derived(authorProfile?.name?.trim() || $brandName);
    let titleText = $derived(authorName);
    let leadText = $derived(authorProfile?.tagline?.trim() || homeContent.lead?.trim() || $t('homeAuthorManifesto'));
    let primaryCtaHref = '/commission';
    let primaryCtaText = $derived($t('homeAuthorPrimaryCta'));
    let secondaryCtaHref = '#gallery';
    let secondaryCtaText = $derived($t('homeAuthorSecondaryCta'));
    // The book-holders' "first look" shelf: works genuinely inside their timed
    // early-release window (held out of the public archive by the server until
    // their hour). Rendered only when signed (see template guard).
    let firstLookFigurines = $state<FigurineListItem[]>([]);
    // Hybrid editorial+algorithmic shelf resolved entirely server-side (admin
    // pins + top of the private mark ranking) — see /figurines/noticed.
    let noticedByGuestsFigurines = $state<FigurineListItem[]>([]);
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
    // The hero photo itself: the admin-pinned piece if set, else today's
    // vitrine pick, else simply the first work on hand — always a real
    // figure, never a static room render (that only remains as the very
    // last resort, e.g. an empty catalogue).
    let heroDisplayFigurine = $derived(
        heroFigurine ?? vitrineFig ?? availableFigurines[0] ?? collectionFigurines[0] ?? null
    );
    // An admin-uploaded hero photo (Replace Photo in the admin) always wins over
    // the figurine-driven picks below — it's an explicit override, not a fallback.
    // The hero fills the fold, so it takes the preview-sized photo — the 420px
    // thumbnail behind faceImageUrl is built for the archive's small cards and
    // goes to mush at this size.
    let heroDisplayImage = $derived(
        hasCustomHeroPhoto
            ? imageUrl
            : (heroDisplayFigurine?.faceImageLargeUrl?.trim()
                || heroDisplayFigurine?.faceImageUrl?.trim()
                || imageUrl)
    );
    const HERO_SIZES = '(max-width: 900px) 100vw, 50vw';
    let heroSrcset = $derived(resolveSrcset(heroDisplayImage));
    // The format-only fallback, for a hero that has no size siblings to offer: a custom
    // background (one file by design — see resolveSrcset's comment in api.ts). It is
    // derived from heroDisplayImage, NOT from imageUrl: those are the same string only
    // while no hero figurine is pinned, and reading the background's URL here while the
    // <img> below points at a figurine would hand the browser a WebP <source> from a
    // different photograph entirely. resolveWebpUrl returns null unless a sibling is
    // actually written for that path, so a hero with no WebP simply gets no <source>.
    let heroBackgroundWebp = $derived(heroSrcset ? null : resolveWebpUrl(heroDisplayImage));

    // Today's vitrine pick gets its own pinned specimen card beside the lead text.
    let pinnedSpecimenFig = $derived(vitrineFig);
    // "Отмеченное вами" — the visitor's own private marks, resolved against the
    // same in-memory list as the saved/wishlist tab (see its comment above for
    // the same "first 30 fetched" cap). Never touches the server for counts —
    // this only ever reads the visitor's own localStorage-backed tone map.
    let markedWorkFigurines = $derived(
        Object.keys(visitorMarks.marks)
            .map((id) => collectionFigurines.find((item) => item.id === id))
            .filter((item): item is FigurineListItem => Boolean(item))
    );
    // The returning-only personal shelves (first look, marked-by-you,
    // noticed-by-guests) still dedupe against each other and against the
    // vitrine pick — with a catalog this small, the same card twice reads
    // as broken, not as generous curation.
    let homeShelves = $derived.by(() => {
        const used = new Set<string>();
        const claim = (list: FigurineListItem[]) => {
            const picked = list.filter((f) => !used.has(f.id));
            for (const f of picked) used.add(f.id);
            return picked;
        };

        if (vitrineFig) used.add(vitrineFig.id);

        const marked = claim(markedWorkFigurines);
        const firstLook = claim(firstLookFigurines);
        const noticed = claim(noticedByGuestsFigurines);

        return { marked, firstLook, noticed };
    });

    // The main gallery: the site's actual purpose — a generous, unfiltered
    // wall of the maker's work, not a tabbed shop shelf. Everyone gets the
    // same wide cross-section (status reads as a quiet museum-label mark on
    // each card, not a filter axis); only today's single vitrine pick is
    // excluded so it doesn't appear twice on the same page.
    const GALLERY_LIMIT = 16;
    let galleryFigurines = $derived.by(() => {
        if (!vitrineFig) return collectionFigurines;
        return collectionFigurines.filter((f) => f.id !== vitrineFig!.id);
    });
    let visibleGalleryFigurines = $derived(galleryFigurines.slice(0, GALLERY_LIMIT));
    let galleryRemaining = $derived(Math.max(0, galleryFigurines.length - GALLERY_LIMIT));
    // The first works the reel did NOT reach — shown as edges of plates sticking
    // out of the drawer on the closing card, so the way into the archive is the
    // work itself, not the word "archive".
    let archivePeek = $derived(galleryFigurines.slice(GALLERY_LIMIT, GALLERY_LIMIT + 4));

    // ── The margin index ────────────────────────────────────────────────
    // A long single-column reel gives no sense of place: how far down am I, and
    // how do I get back to the piece I passed two screens ago? So the works that
    // leave the top of the screen are entered, silently, in the left margin —
    // marginalia in a catalogue, not a sticky thumbnail rail. `reelEl` is the
    // observed container; `passedCount` is how many panes are now above the fold.
    let reelEl = $state<HTMLElement | undefined>();
    let passedCount = $state(0);

    $effect(() => {
        // Re-observe whenever the reel's contents change.
        const el = reelEl;
        const n = visibleGalleryFigurines.length;
        if (!el || n === 0) {
            passedCount = 0;
            return;
        }

        const passed = new Set<number>();
        const io = new IntersectionObserver(
            (entries) => {
                for (const entry of entries) {
                    const i = Number((entry.target as HTMLElement).dataset.reelSlot);
                    // Gone, and gone off the TOP — a pane below the fold has not
                    // been read yet and must not be indexed.
                    if (!entry.isIntersecting && entry.boundingClientRect.top < 0) passed.add(i);
                    else passed.delete(i);
                }
                // The high-water mark, so scrolling back up empties the margin again
                // in the same order it filled.
                passedCount = passed.size > 0 ? Math.max(...passed) + 1 : 0;
            },
            // Matches the sticky header's height: a pane still tucked under the
            // header is not yet "passed".
            { rootMargin: '-72px 0px 0px 0px', threshold: 0 }
        );

        for (const slot of el.querySelectorAll<HTMLElement>('[data-reel-slot]')) io.observe(slot);
        return () => io.disconnect();
    });

    // The gallery's works are shown as glass panes (ReelWorkCard). Their whole
    // look — glass, type, buttons — is the "work panes" half of the admin's reel
    // theme, which arrives as one block of CSS variables. The backdrop half of
    // that theme does not apply here: this page keeps its own parchment.
    let reelTheme = $state<ReelTheme>({});
    let reelCSSBlock = $derived(generateReelCSS(reelTheme));

    // Each pane's paragraph is the work's own short text. The list payload now carries
    // it (FigurineListItemDto.short_text), so this is a plain projection of data we
    // already have — no network at all. It used to be a $state map filled by an effect
    // that fired one full `getFigurine(id)` per pane on the wall, i.e. a dozen extra
    // round-trips (each pulling the complete record and its image array) to read one
    // string apiece. The field was already being SELECTed server-side and discarded.
    let galleryStories = $derived(
        Object.fromEntries(
            visibleGalleryFigurines.map((f) => [f.id, f.shortText?.trim() ?? ''])
        ) as Record<string, string>
    );

    // Today's vitrine: the admin-pinned vitrine figure if set (independent of the
    // hero banner's own pin), else a daily-rotating available work (falling back
    // to the wider collection when none are available). A sealed work is skipped
    // in favour of the next candidate — the vitrine never spotlights a piece the
    // rest of the house keeps behind a closed door. Called once showingRooms has
    // loaded (see onMount) so isOpenNow has real door data to gate against.
    function computeVitrineFig(): FigurineListItem | null {
        const pinnedVitrineFigurine = homeContent.vitrineFigurineId
            ? collectionFigurines.find((item) => item.id === homeContent.vitrineFigurineId) ?? null
            : null;
        return (pinnedVitrineFigurine && isOpenNow(pinnedVitrineFigurine) ? pinnedVitrineFigurine : null)
            ?? dailyPick(availableFigurines.filter(isOpenNow))
            ?? dailyPick(collectionFigurines.filter(isOpenNow));
    }

    // The hero image is seeded from +page.ts's load(), so it paints without waiting on
    // this. Everything that varies with the collection — the works reel above all — is
    // fetched HERE, on the client, and never from load(): this page is prerendered, and
    // a work's photo that was rendered at build time is not replaced during hydration
    // (see the note in +page.ts). homeContent is re-fetched too, so an admin's edit shows
    // up without a rebuild.
    async function init() {
        try {
            const [figurines, inProgress, firstLook, noticedByGuests, content, workshop, author, layout, savedReelTheme] = await Promise.all([
                api.getAllFigurines(30).catch(() => [] as FigurineListItem[]),
                api.getInProgressFigurines().catch(() => [] as FigurineListItem[]),
                api.getFirstLookFigurines().catch(() => [] as FigurineListItem[]),
                api.getNoticedByGuests().catch(() => [] as FigurineListItem[]),
                api.getHomeContent().catch(() => null),
                api.getWorkshopFeature().catch(() => null),
                api.getAuthorProfile().catch(() => null),
                api.getHomeLayout().catch(() => null),
                api.getReelTheme().catch(() => null)
            ]);
            if (author) authorProfile = author;
            if (savedReelTheme) reelTheme = savedReelTheme;
            // Editor preview (postMessage) wins over the saved config.
            if (layout && !hlPreviewDriven) homeLayout = layout;
            if (content) homeContent = content;
            if (workshop) workshopFeature = workshop;
            const visibleFigurines = figurines.filter((f) => f.status !== 'in_progress');
            collectionFigurines = sortFeaturedFigurines(visibleFigurines);
            collectionTotal = visibleFigurines.length;
            availableTotal = figurines.filter((item) => item.status === 'available').length;
            availableFigurines = sortFeaturedFigurines(
                visibleFigurines.filter((item) => item.status === 'available')
            );
            inProgressFigurines = sortFeaturedFigurines(inProgress);
            firstLookFigurines = firstLook;
            noticedByGuestsFigurines = noticedByGuests;
            heroFigurine = homeContent.heroFigurineId
                ? visibleFigurines.find((item) => item.id === homeContent.heroFigurineId) ?? null
                : null;
            // The vitrine needs both the works and the doors; onMount also calls this once
            // the rooms are in, whichever of the two lands last.
            vitrineFig = computeVitrineFig();
            isLoaded = true;
        } catch (e) {
            isLoaded = true;
        }
    }

    function handleMouseMove(e: MouseEvent) {
        if (!canUseHeroTilt) return;
        const { innerWidth, innerHeight } = window;
        mouseX = e.clientX / innerWidth;
        mouseY = e.clientY / innerHeight;
    }

    // Hero photo: the frame keeps pushing in as the page scrolls away. It feeds
    // one transform on .hero-lens; the Ken Burns drift stays on the <img>
    // underneath so the two never fight over the same property.
    // The dwell push-in: the frame keeps creeping toward the viewer for as long
    // as it is on screen. It is not a loop — a loop would have to snap back, and
    // any dissolve across that snap shows the photo twice at once. Instead it
    // eases asymptotically toward its cap, so it is always still moving and
    // never resets.
    let heroDwellZoom = $state(0);
    let heroPhotoEl = $state<HTMLElement | null>(null);
    let heroLensScale = $derived((1 + heroDwellZoom).toFixed(4));

    const HERO_DWELL_CAP = 0.22;   // how far the dwell push can ever go
    const HERO_DWELL_TAU = 26000;  // ms to reach ~63% of the cap

    let showHint = $state(false);
    let hintDismissed = $state(false);

    onMount(() => {
        savedFigurines.load();
        visitorMarks.load();
        houseClock.start();
        // vitrineFig needs real door data to gate against — compute it once rooms
        // are in, instead of racing the unrelated calls inside init().
        showingRooms.load().then(() => { vitrineFig = computeVitrineFig(); });
        visitorBook.load();
        // Inside the admin editor's preview iframe: don't pollute the shared
        // localStorage visit flag, and listen for live layout drafts.
        const inHlPreview = new URLSearchParams(window.location.search).has('hlPreview');
        // Record the visit; a flag already present means this is a return.
        try {
            if (localStorage.getItem('gotiga_visited')) visitedBefore = true;
            if (!inHlPreview) localStorage.setItem('gotiga_visited', String(Date.now()));
        } catch { /* storage blocked (private mode) → treat as a new visitor */ }
        function onHlMessage(e: MessageEvent) {
            if (e.data?.type !== 'gotiga-home-layout') return;
            hlPreviewDriven = true;
            if ('config' in e.data) homeLayout = e.data.config ?? null;
            if ('visitorMode' in e.data) previewVisitorMode = e.data.visitorMode ?? null;
            if (typeof e.data.scrollTo === 'string') {
                document.querySelector(`[data-hl="${CSS.escape(e.data.scrollTo)}"]`)
                    ?.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
        }
        window.addEventListener('message', onHlMessage);
        init();
        const reduceMq = window.matchMedia('(prefers-reduced-motion: reduce)');
        const pointerMq = window.matchMedia('(pointer: fine)');
        const syncTiltPreference = () => {
            canUseHeroTilt = pointerMq.matches && !reduceMq.matches;
            if (!canUseHeroTilt) {
                mouseX = 0.5;
                mouseY = 0.5;
            }
        };
        syncTiltPreference();
        reduceMq.addEventListener('change', syncTiltPreference);
        pointerMq.addEventListener('change', syncTiltPreference);

        // Dwell push-in: only accumulates while the photo is actually on screen,
        // and the rAF loop is torn down entirely when it leaves.
        let dwellRaf = 0;
        let dwellMs = 0;
        let lastTick = 0;
        const tickDwell = (now: number) => {
            const dt = lastTick ? Math.min(now - lastTick, 100) : 0;
            lastTick = now;
            dwellMs += dt;
            heroDwellZoom = HERO_DWELL_CAP * (1 - Math.exp(-dwellMs / HERO_DWELL_TAU));
            dwellRaf = requestAnimationFrame(tickDwell);
        };
        const startDwell = () => {
            if (dwellRaf || reduceMq.matches) return;
            lastTick = 0;
            dwellRaf = requestAnimationFrame(tickDwell);
        };
        const stopDwell = () => {
            if (dwellRaf) cancelAnimationFrame(dwellRaf);
            dwellRaf = 0;
        };

        let heroObserver: IntersectionObserver | null = null;
        if (heroPhotoEl) {
            heroObserver = new IntersectionObserver(
                ([entry]) => { entry.isIntersecting ? startDwell() : stopDwell(); },
                { threshold: 0 }
            );
            heroObserver.observe(heroPhotoEl);
        }

        const hintTimer = setTimeout(() => { showHint = true; }, 3000);
        return () => {
            clearTimeout(hintTimer);
            stopDwell();
            heroObserver?.disconnect();
            reduceMq.removeEventListener('change', syncTiltPreference);
            pointerMq.removeEventListener('change', syncTiltPreference);
            window.removeEventListener('message', onHlMessage);
        };
    });

    // The admin's reel-theme panel drives this page live over BroadcastChannel.
    onMount(() => startListeningForReelPreview());

    // The theme block is injected imperatively rather than through <svelte:head>:
    // an {@html} block in the head is rendered once and does not re-run when the
    // theme arrives from the server, so the panes kept their default look.
    $effect(() => {
        const css = reelCSSBlock;
        if (typeof document === 'undefined') return;
        const id = 'reel-theme';
        const existing = document.getElementById(id);
        const style = existing instanceof HTMLStyleElement ? existing : document.createElement('style');
        style.id = id;
        style.textContent = css;
        if (!style.parentNode) document.head.appendChild(style);
        return () => style.remove();
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
    {#if hlElementCSS}
        {@html `<style id="hl-element-overrides">${hlElementCSS}</style>`}
    {/if}
    <!-- Fonts loaded once globally in app.html -->
</svelte:head>

<svelte:window onmousemove={handleMouseMove} />

<div class="root">
    <div class="cursor-glow" style="transform:translate(calc({mouseX*100}vw - 250px),calc({mouseY*100}vh - 250px))"></div>
    <div class="grain" aria-hidden="true"></div>

    <main in:fade={{ duration: 700, delay: 40 }} style={homePageStyle(homeLayout)}>

        <!-- HERO -->
        {#if hlVisible('hero')}
        <div class={hlClasses('hero')} style={hlStyle('hero')} data-hl="hero">
        <section class="hero hero-cine" aria-labelledby="home-title">

            <!-- Text, left column -->
            <div class="hero-text" in:fly={{ y: 20, duration: 900, delay: 350, easing: cubicOut }}>
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

                <div class="hero-body">
                    <div class="hero-main">
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

                        <!-- Process footage, right under the trust line — a quiet second
                             layer of proof, not competing with title/CTA for the eye. -->
                        <div class="hw-teasers">
                            <HeroWorkshopTeaser
                                webm="/images/workshop/atelier-reel-tiny.webm"
                                mp4="/images/workshop/atelier-reel-tiny.mp4"
                                poster="/images/workshop/atelier-reel-tiny-poster.jpg"
                                label={$t('homeWorkshopTeaserLabel')}
                                onSelect={(e) => openReelModal('a', e)}
                            />
                            <HeroWorkshopTeaser
                                webm="/images/workshop/atelier-reel-2-tiny.webm"
                                mp4="/images/workshop/atelier-reel-2-tiny.mp4"
                                poster="/images/workshop/atelier-reel-2-tiny-poster.jpg"
                                label={$t('homeWorkshopTeaserLabel')}
                                delayMs={1700}
                                onSelect={(e) => openReelModal('b', e)}
                            />
                            <span class="hw-teasers-label">{$t('homeWorkshopTeaserLabel')}</span>
                        </div>
                        {#if availableFigurines.length === 0}
                            <p class="release-note">{$t('homeReleaseNote')}</p>
                        {/if}

                        {#if collectionTotal > 0}
                            <div class="hero-stats">
                                <a class="hero-stat" href="#gallery">
                                    <b>{collectionTotal}</b>
                                    <span>{$t('homeHeroCountSuffix')}<svg class="hero-stat-arrow" width="10" height="8" viewBox="0 0 10 8" fill="none" aria-hidden="true"><path d="M0 4H9M9 4L6 1M9 4L6 7" stroke="currentColor" stroke-width="1"/></svg></span>
                                </a>
                                {#if inProgressFigurines.length > 0}
                                    <a class="hero-stat" href="/upcoming">
                                        <b>{inProgressFigurines.length}</b>
                                        <span>{$t('homeLedgerInProgress')}<svg class="hero-stat-arrow" width="10" height="8" viewBox="0 0 10 8" fill="none" aria-hidden="true"><path d="M0 4H9M9 4L6 1M9 4L6 7" stroke="currentColor" stroke-width="1"/></svg></span>
                                    </a>
                                {/if}
                                {#if homeShelves.marked.length > 0}
                                    <a class="hero-stat" href="#marked-by-you">
                                        <b>{homeShelves.marked.length}</b>
                                        <span>{$t('markedByYouEyebrow')}<svg class="hero-stat-arrow" width="10" height="8" viewBox="0 0 10 8" fill="none" aria-hidden="true"><path d="M0 4H9M9 4L6 1M9 4L6 7" stroke="currentColor" stroke-width="1"/></svg></span>
                                    </a>
                                {/if}
                            </div>
                        {/if}
                    </div>

                </div>

            </div>

            <!-- Photo, right column: a slow drift stands in for the pointer-tilt
                 this layout replaced, and a HUD strip carries the "live" read
                 instead of a decorative ornament. -->
            <div class="cine-frame">
                <div class="cine-photo" style="--lens-scale:{heroLensScale}">
                    <div class="hero-lens" bind:this={heroPhotoEl}>
                        <!-- The hero is the LCP element, so what the browser picks here sets
                             the page's headline number. It fills the fold on a phone and about
                             half the width on a wide screen; with the 420/900/1800 renditions
                             offered, that resolves to the 900px medium on mobile instead of
                             the 1800px preview it used to pull down. -->
                        <picture>
                            {#if heroSrcset?.webp}
                                <source type="image/webp" srcset={heroSrcset.webp} sizes={HERO_SIZES} />
                            {:else if heroBackgroundWebp}
                                <source type="image/webp" srcset={heroBackgroundWebp} />
                            {/if}
                            <img src={heroDisplayImage} srcset={heroSrcset?.jpeg}
                                 sizes={heroSrcset ? HERO_SIZES : undefined}
                                 alt={heroDisplayFigurine?.name ?? 'Gothic Cabinet'} class="hero-img"
                                 fetchpriority="high" decoding="async" draggable="false" />
                        </picture>
                    </div>
                    <div class="img-vignette"></div>
                    <div class="img-grade"></div>
                    <div class="img-noise"></div>

                    {#if showHeroCaption}
                    <a class="art-caption" href={heroObjectHref} aria-label="{heroObjectCta}: {heroObjectName}">
                        <span class="art-caption-kicker">{$t('homeHeroObjectLabel')}</span>
                        <span class="art-caption-name">{heroObjectName}</span>
                        <span class="art-caption-meta">{heroObjectMeta}</span>
                        <span class="art-caption-open">{heroObjectCta} →</span>
                    </a>
                    {/if}
                </div>

                {#if showHint && !hintDismissed}
                    <a href="#gallery" class="scroll-cue" in:fade={{ duration: 400 }}>
                        <span class="sc-line"></span>
                        <span>{$t('homeScrollCue')}</span>
                    </a>
                {/if}
            </div>

        

        </section>
        </div>
        {/if}

        <!-- Returning-visitor band, kept HIGH on purpose: the retention hooks —
             what changed since the last visit, today's single exhibit, and what is
             opening soon — are the reason a returning fan came back, so they must be
             seen fast. First-time visitors skip this entirely and go straight to the
             acquisition arc (maker → works → proof → commission). The band is one
             compound layout block: the admin moves it as a unit and orders the
             ledger/notice-board inside it. -->
        {#if isLoaded && effectiveReturning && hlVisible('returningBand')}
            <div class={hlClasses('returningBand')} style={hlStyle('returningBand')} data-hl="returningBand">
                <div class="hl-band">
                    {#if hlVisible('visitLedger')}
                        <!-- "Since your visit" + "Exhibit of the day" folded into one light
                             ledger: quick facts worth a daily glance, not a monument. -->
                        <div class={hlClasses('visitLedger')} style={hlSubStyle('visitLedger', hlBandOrder.indexOf('visitLedger'))} data-hl="visitLedger">
                            <VisitLedger
                                figurines={collectionFigurines}
                                rooms={showingRooms.list.map((r) => ({ id: r.id, name: r.name }))}
                                inProgressCount={inProgressFigurines.length}
                            />
                        </div>
                    {/if}

                    {#if hlVisible('noticeBoard')}
                        <!-- "Opening soon": timed reveals — come at the appointed hour. Hides
                             itself when nothing is opening within the week. This is the one
                             real ceremony in the band — the showing programme runs regularly
                             and earns the theatre. -->
                        <div class={hlClasses('noticeBoard')} style={hlSubStyle('noticeBoard', hlBandOrder.indexOf('noticeBoard'))} data-hl="noticeBoard">
                            <HouseNoticeBoard figurines={collectionFigurines} />
                        </div>
                    {/if}
                </div>
            </div>
        {/if}

        {#if hlVisible('gallery')}
        <div class={hlClasses('gallery')} style={hlStyle('gallery')} data-hl="gallery">
        <!-- Gallery: the site's actual purpose, given room to work. One wide,
             unfiltered wall of the maker's pieces — not a tabbed shop shelf —
             so a visitor can simply stop and look through the work. -->
        <section id="gallery" class="context-section work-hub" aria-labelledby="context-title">
            <div class="context-hd work-hd">
                <div>
                    <p class="eyebrow">
                        <span class="eyebrow-rule"></span>
                        {$t('homeGalleryEyebrow')}
                    </p>
                    <h2 id="context-title" class="context-title">{$t('homeGalleryTitle')}</h2>
                </div>
                <div class="context-side">
                    <div class="context-side-main">
                        <p class="context-desc">{$t('homeGalleryText')}</p>
                        <p class="context-meta">
                            <span class="context-meta-kicker">{$t('homeHowEyebrow')}</span>
                            {$t('homeWorksGuideTitle')}
                        </p>
                    </div>
                    <div class="context-side-links">
                        <a href="/figurines" class="all-link">
                            {$t('homeGalleryCta')}
                            <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                                <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                            </svg>
                        </a>
                        <a href="/commission" class="all-link">
                            {$t('commissionInvite')}
                            <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                                <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                            </svg>
                        </a>
                    </div>
                </div>
            </div>

            <div class="work-content">
                {#if visibleGalleryFigurines.length > 0}
                    <!-- The works, one pane per row, alternating sides. They sit on
                         the page's own parchment — no band of their own — so they are
                         paper, not glass: a warm card with a real edge and a real
                         shadow. See ReelWorkCard. -->
                    <div class="work-spread">
                        <!-- The margin: works already passed, entered as small
                             plates. Pure orientation — it links back up the reel,
                             never out to a detail page. -->
                        <WorkMarginIndex figurines={visibleGalleryFigurines} count={passedCount} />

                        <div class="work-reel" bind:this={reelEl}>
                            {#each visibleGalleryFigurines as fig, i (fig.id)}
                                <div class="reel-slot" id="work-{fig.id}" data-reel-slot={i}>
                                    <ReelWorkCard
                                        {fig}
                                        index={i + 1}
                                        story={galleryStories[fig.id]}
                                        flip={i % 2 === 1}
                                    />
                                </div>
                            {/each}

                            <!-- The reel does not trail off into a caption: its last
                                 pane IS the archive. Same paper, same width, same
                                 rhythm — so the eye that read sixteen plates cannot
                                 miss the seventeenth. -->
                            {#if galleryRemaining > 0}
                                <a href="/figurines" class="archive-end">
                                    <div class="archive-end__text">
                                        <span class="archive-end__count">{galleryRemaining}</span>
                                        <span class="archive-end__label">{$t('homeMoreInArchive')}</span>
                                    </div>

                                    {#if archivePeek.length > 0}
                                        <div class="archive-end__drawer" aria-hidden="true">
                                            {#each archivePeek as peek, i (peek.id)}
                                                <span class="archive-end__plate" style="--peek-i: {i}">
                                                    <AppImage
                                                        src={peek.thumbUrl ?? peek.faceImageUrl}
                                                        alt=""
                                                        class="archive-end__plate-img"
                                                    />
                                                </span>
                                            {/each}
                                        </div>
                                    {/if}

                                    <span class="archive-end__cta">
                                        {$t('homeGalleryCta')}
                                        <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                                            <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                                        </svg>
                                    </span>
                                </a>
                            {/if}
                        </div>
                    </div>
                {:else}
                    <div class="work-empty">
                        <p>{$t('homeWorksEmptyArchive')}</p>
                        <a href="/commission" class="all-link">
                            {$t('commissionInvite')}
                            <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                                <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                            </svg>
                        </a>
                    </div>
                {/if}
            </div>

        </section>
        </div>
        {/if}

        <!-- The maker, after the work has already made its case: a reputation
             is proven by the pieces first, then given a face. Content is reused
             from the admin-editable AuthorProfile. -->
        {#if hlVisible('authorStory')}
        <div class={hlClasses('authorStory')} style={hlStyle('authorStory')} data-hl="authorStory">
        {#if isLoaded}
            <AuthorStory name={authorName} bio={authorProfile?.bio ?? null} photoUrl={authorProfile?.photoUrl ?? null} />
        {/if}
        </div>
        {/if}

        <!-- Social proof (Concept B, Variant A): curator-picked reactions from the
             Book of Impressions, lifted up to support the commission decision
             rather than sitting below it in the page's basement. -->
        {#if hlVisible('impressions')}
        <div class={hlClasses('impressions')} style={hlStyle('impressions')} data-hl="impressions">
        <ImpressionsQuoteStrip />
        </div>
        {/if}

        {#if hlVisible('requestSteps')}
        <div class={hlClasses('requestSteps')} style={hlStyle('requestSteps')} data-hl="requestSteps">
        <section id="request-steps" class="request-path compact-request" aria-labelledby="request-steps-title">
            <div class="request-copy">
                <p class="eyebrow">
                    <span class="eyebrow-rule"></span>
                    {$t('homeHowEyebrow')}
                </p>
                <h2 id="request-steps-title" class="request-title">{$t('homeHowTitle')}</h2>
                <a href="/commission" class="all-link request-cta">
                    {$t('homeHowCta')}
                    <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                        <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                    </svg>
                </a>
            </div>

            <div class="request-steps" aria-label={$t('homeHowTitle')}>
                <span><b>01</b><strong>{$t('homeHowStep1Title')}</strong><em>{$t('homeHowStep1Text')}</em></span>
                <span><b>02</b><strong>{$t('homeHowStep2Title')}</strong><em>{$t('homeHowStep2Text')}</em></span>
                <span><b>03</b><strong>{$t('homeHowStep3Title')}</strong><em>{$t('homeHowStep3Text')}</em></span>
            </div>
        </section>
        </div>
        {/if}

        

        <!-- The house guest book: sign it to receive the workshop's letters first.
             Quiet email capture — the one channel the house owns. Placed right
             after the commission path, as the soft secondary conversion. -->
        {#if hlVisible('visitorBook')}
        <div class={hlClasses('visitorBook')} style={hlStyle('visitorBook')} data-hl="visitorBook">
        <VisitorBook figurines={collectionFigurines} />
        </div>
        {/if}

        <!-- Personal shelves, returning visitors only: the book-holders' first
             look, the visitor's own marks, and quietly-noticed works. A first-time
             stranger never sees these — they are a returning ritual, not an
             acquisition surface. One compound layout block: the "Lately" divider
             travels with it, the shelves are ordered inside it. -->
        {#if effectiveReturning && hlVisible('latelyShelves')}
            <div class={hlClasses('latelyShelves')} style={hlStyle('latelyShelves')} data-hl="latelyShelves">
                <div class="house-lately-divider" aria-hidden="true">
                    <span class="hld-rule"></span>
                    <span class="hld-label">{$t('homeHouseLatelyLabel')}</span>
                    <span class="hld-rule"></span>
                </div>

                <div class="hl-band">
                    <!-- First look: book-holders' privilege — shown only to a signed
                         visitor (the editor's forced returning-preview counts as signed
                         so the admin can actually see the shelf). -->
                    {#if hlVisible('firstLook') && (visitorBook.signed || previewVisitorMode === 'returning')}
                        <div class={hlClasses('firstLook')} style={hlSubStyle('firstLook', hlShelfOrder.indexOf('firstLook'))} data-hl="firstLook">
                            <FirstLook works={homeShelves.firstLook} greetName={visitorBook.name} />
                        </div>
                    {/if}

                    <!-- The visitor's own private marks, resolved locally. -->
                    {#if hlVisible('markedByYou')}
                        <div class={hlClasses('markedByYou')} style={hlSubStyle('markedByYou', hlShelfOrder.indexOf('markedByYou'))} data-hl="markedByYou">
                            <MarkedByYou figurines={homeShelves.marked} />
                        </div>
                    {/if}

                    <!-- Hybrid editorial+algorithmic shelf: admin pins + top of the private mark ranking. -->
                    {#if hlVisible('noticedByGuests')}
                        <div class={hlClasses('noticedByGuests')} style={hlSubStyle('noticedByGuests', hlShelfOrder.indexOf('noticedByGuests'))} data-hl="noticedByGuests">
                            <NoticedByGuests figurines={homeShelves.noticed} />
                        </div>
                    {/if}
                </div>
            </div>
        {/if}

        <!-- WORKSHOP section — disabled for now, kept here to restore easily later.
        {#if workshopFeature.visible}
        <section class="workshop-feature" aria-labelledby="workshop-feature-title" bind:this={workshopSectionEl}>
            <div class="workshop-photos">
                <div class="workshop-stage">
                    <button
                        type="button"
                        class="workshop-plate workshop-plate-a"
                        class:is-front={frontReel === 'a'}
                        onclick={() => { frontReel = 'a'; workshopActivated = true; }}
                        aria-label={$t('homeWorkshopBringForward')}
                    >
                        {#if workshopActivated}
                            <AtelierReel />
                        {:else}
                            <img src="/images/workshop/atelier-reel-poster.jpg" alt="" class="workshop-plate-poster" loading="lazy" />
                        {/if}
                    </button>
                    <button
                        type="button"
                        class="workshop-plate workshop-plate-b"
                        class:is-front={frontReel === 'b'}
                        onclick={() => { frontReel = 'b'; workshopActivated = true; }}
                        aria-label={$t('homeWorkshopBringForward')}
                    >
                        {#if workshopActivated}
                            <AtelierReel
                                webm="/images/workshop/atelier-reel-2.webm"
                                mp4="/images/workshop/atelier-reel-2.mp4"
                                poster="/images/workshop/atelier-reel-2-poster.jpg"
                            />
                        {:else}
                            <img src="/images/workshop/atelier-reel-2-poster.jpg" alt="" class="workshop-plate-poster" loading="lazy" />
                        {/if}
                    </button>
                </div>
                <p class="workshop-plate-label" aria-hidden="true">
                    <span class="wpl-rule"></span>
                    {$t('homeWorkshopReelCaption')}
                </p>
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
        -->

    </main>

    {#if reelModalOpen}
        <WorkshopReelModal
            webm={reelModalClip === 'a' ? '/images/workshop/atelier-reel.webm' : '/images/workshop/atelier-reel-2.webm'}
            mp4={reelModalClip === 'a' ? '/images/workshop/atelier-reel.mp4' : '/images/workshop/atelier-reel-2.mp4'}
            poster={reelModalClip === 'a' ? '/images/workshop/atelier-reel-poster.jpg' : '/images/workshop/atelier-reel-2-poster.jpg'}
            caption={$t('homeWorkshopReelCaption')}
            closeLabel={$t('figurineGrimoireClose')}
            origin={reelModalOrigin}
            onClose={() => reelModalOpen = false}
        />
    {/if}
</div>

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

    @media (prefers-reduced-motion: no-preference) {
        :global(html) { scroll-behavior: smooth; }
    }

    /* Divider opening the returning-visitor appendix ("Lately at the cabinet"). */
    .house-lately-divider {
        max-width: 1520px;
        margin: clamp(48px, 7vw, 96px) auto clamp(8px, 2vw, 24px);
        padding: 0 clamp(20px, 4.5vw, 64px);
        display: flex;
        align-items: center;
        gap: clamp(14px, 2vw, 28px);
    }
    .hld-rule {
        flex: 1;
        height: 1px;
        background: color-mix(in srgb, var(--color-ink-primary) 16%, transparent);
    }
    .hld-label {
        font-family: 'Instrument Sans', system-ui, sans-serif;
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: var(--color-ink-tertiary);
        white-space: nowrap;
    }

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
        /* `clip` (not `hidden`) — `hidden` forces the paired overflow-y to
           `auto` per spec, silently turning .root into its own scroll
           container and breaking position:sticky calculations inside it
           (e.g. the collection header). */
        overflow-x: clip;
        position: relative;
        background:
            radial-gradient(ellipse 70% 55% at 72% 38%, rgba(198,95,60,0.07) 0%, transparent 65%),
            radial-gradient(ellipse 50% 70% at 18% 72%, rgba(201,168,117,0.06) 0%, transparent 60%),
            var(--cream);
    }

    /* ── CURSOR GLOW ─────────────────────────────── */
    .cursor-glow {
        position: fixed;
        top: 0; left: 0;
        width: 500px;
        height: 500px;
        border-radius: 50%;
        background: radial-gradient(circle, rgba(198,95,60,0.07) 0%, transparent 70%);
        pointer-events: none;
        z-index: 0;
        transition: transform 0.8s ease;
        will-change: transform;
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
        /* Column flex so the admin-configured block `order` (Home Layout
           editor) can rearrange the page without touching the markup. */
        display: flex;
        flex-direction: column;
        align-items: stretch;
    }

    /* ── HOME LAYOUT WRAPPERS ─────────────────────
       One wrapper per admin-arrangeable block. Background paints on the
       wrapper (edge-to-edge for `full`); text overrides cascade to prose
       only — buttons, links and controls keep the house styling. */
    .hl-block {
        width: 100%;
        background: var(--hl-bg, transparent);
    }
    .hl-band {
        display: flex;
        flex-direction: column;
        align-items: stretch;
    }
    .hl-size-compact {
        max-width: 980px;
        margin-inline: auto;
    }
    .hl-has-color :global(:is(h1, h2, h3, h4, p, em, li, blockquote, figcaption)) {
        color: var(--hl-color) !important;
    }
    .hl-has-font :global(*) {
        font-family: var(--hl-font) !important;
    }
    .hl-has-size :global(:is(p, li, em)) {
        font-size: var(--hl-size) !important;
    }
    /* Vertical rhythm presets. `tight` pulls neighbours closer with a small
       negative margin (sections carry generous internal padding, so nothing
       clips); roomy/spacious add breathing room around the block. */
    .hl-pad-tight    { margin-block: clamp(-22px, -1.6vw, -10px); }
    .hl-pad-roomy    { padding-block: clamp(20px, 2.6vw, 44px); }
    .hl-pad-spacious { padding-block: clamp(44px, 5.4vw, 96px); }
    /* Letterpress rule above a block. */
    .hl-has-divider::before {
        content: '';
        display: block;
        width: min(92%, 1440px);
        height: 1px;
        margin: 0 auto clamp(18px, 2.4vw, 36px);
        background: color-mix(in srgb, var(--color-ink-primary) 18%, transparent);
    }
    /* Per-device visibility (breakpoints match this page's own: 680 / 1080). */
    @media (max-width: 680px) {
        .hl-hide-mobile { display: none; }
    }
    @media (min-width: 681px) and (max-width: 1080px) {
        .hl-hide-tablet { display: none; }
    }
    @media (min-width: 1081px) {
        .hl-hide-desktop { display: none; }
    }

    /* ── HERO LAYOUT (split: text left, letterboxed frame right) ───
       The frame used to run full-width with the text stacked under it,
       which left the pillarboxed side-bars (from object-fit: contain)
       doing nothing. Side by side, that same width goes to the text
       column instead of sitting empty. */
    .hero-cine {
        display: grid;
        grid-template-columns: minmax(340px, 0.86fr) minmax(420px, 1.14fr);
        align-items: start;
        gap: clamp(28px, 4vw, 64px);
        /* Главная без header-offset — добавляем верхний воздух, чтобы фото и мета-строка
           не уходили под фиксированную шапку. */
        padding:
            calc(var(--site-header-height) + clamp(18px, 2.4vw, 34px))
            clamp(20px, 4.5vw, 64px)
            clamp(28px, 3.2vw, 44px);
        max-width: 1320px;
        margin: 0 auto;
    }

    /* ── HERO TEXT ───────────────────────────────── */
    .hero-text {
        position: relative;
        z-index: 10;
        min-width: 0;
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
        font-size: clamp(48px, 7vw, 108px);
        font-weight: 300;
        line-height: 0.9;
        letter-spacing: 0;
        color: var(--ink);
        max-width: 100%;
        margin: 0 0 22px;
        word-break: keep-all;
        overflow-wrap: normal;
        hyphens: none;
        display: grid;
        gap: 0.02em;
        overflow: visible;
        padding-bottom: 0.12em;
    }

    .hero-title:lang(ru) {
        font-size: clamp(44px, 6.4vw, 96px);
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
        .grain,
        .sc-line {
            animation: none;
            transform: none;
            opacity: 1;
        }

        .cursor-glow {
            display: none;
        }

        .hero-img {
            animation: none;
        }

        .hero-lens {
            transform: none;
        }

        .cine-rec i,
        .cine-scrub i {
            animation: none;
        }
    }

    /* Now that the text sits in its own (narrower) column beside the photo,
       there's no spare width for the pinned specimen card to sit side by
       side with the lead copy — it stacks underneath instead. */
    .hero-body {
        max-width: 480px;
    }

    .hero-lead {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(16px, 1.5vw, 20px);
        font-weight: 300;
        font-style: italic;
        line-height: 1.42;
        color: var(--color-ink-secondary);
        max-width: 420px;
        margin: 0 0 22px;
    }

    /* ── CTAs ────────────────────────────────────── */
    .hero-ctas {
        display: flex;
        align-items: center;
        gap: 14px;
        flex-wrap: wrap;
        margin-bottom: 22px;
    }

    .cta-primary {
        display: inline-flex;
        align-items: center;
        gap: 12px;
        height: 40px;
        padding: 0 22px;
        background: var(--ink);
        color: var(--cream2);
        font-size: 12px;
        font-weight: 600;
        letter-spacing: 0.09em;
        text-transform: uppercase;
        text-decoration: none;
        border-radius: 999px;
        transition:
            background 0.22s ease,
            box-shadow 0.22s ease,
            transform 0.18s ease;
    }

    .cta-arrow {
        flex-shrink: 0;
        transition: transform 0.22s ease;
    }

    .cta-primary:hover {
        background: var(--mid);
        box-shadow: 0 12px 22px -8px rgba(68,37,20,0.4);
        transform: translateY(-2px);
    }

    .cta-primary:hover .cta-arrow {
        transform: translateX(4px);
    }

    .cta-primary:active {
        transform: translateY(0);
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
        gap: 6px 14px;
        max-width: 520px;
        color: var(--muted);
        font-size: 12.5px;
        letter-spacing: 0.01em;
        line-height: 1.5;
    }

    .hero-proof span {
        position: relative;
        padding-left: 13px;
    }

    .hero-proof span::before {
        content: "\00b7";
        position: absolute;
        left: 0;
        top: -0.2em;
        font-size: 16px;
        line-height: 1;
        color: var(--copper);
    }

    .hw-teasers {
        display: flex;
        align-items: center;
        gap: 14px;
        width: fit-content;
        margin-top: 20px;
    }

    .hw-teasers-label {
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.1em;
        line-height: 1.3;
        text-transform: uppercase;
        max-width: 130px;
        color: var(--mid);
        border-bottom: 1px solid rgba(198,95,60,0.25);
        padding-bottom: 2px;
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

    /* Honest facts doubling as quick jumps — only shown alongside the pin
       (otherwise the tightened single column above is already complete). */
    .hero-stats {
        display: flex;
        gap: 30px;
        margin-top: 30px;
        padding-top: 20px;
        border-top: 1px solid var(--border);
    }

    .hero-stat {
        display: block;
        text-decoration: none;
        cursor: pointer;
        transition: transform 0.16s ease;
    }

    .hero-stat:hover,
    .hero-stat:focus-visible {
        transform: translateY(-2px);
    }

    .hero-stat:focus-visible {
        outline: 2px solid rgba(198,95,60,0.56);
        outline-offset: 3px;
    }

    .hero-stat b {
        display: block;
        font-family: 'Cormorant Garamond', serif;
        font-weight: 300;
        font-size: 26px;
        line-height: 1;
        color: var(--mid);
        margin-bottom: 5px;
        transition: color 0.16s ease;
    }

    .hero-stat:hover b,
    .hero-stat:focus-visible b {
        color: var(--copper);
    }

    /* Underline is always faintly present (this is a link), and the
       arrow only becomes legible on interaction so the row stays quiet
       until someone's pointer actually lands on it. */
    .hero-stat span {
        display: inline-flex;
        align-items: center;
        gap: 5px;
        font-size: 10.5px;
        font-weight: 600;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: var(--muted);
        border-bottom: 1px solid rgba(198,95,60,0.28);
        padding-bottom: 2px;
        transition: border-color 0.16s ease, color 0.16s ease;
    }

    .hero-stat:hover span,
    .hero-stat:focus-visible span {
        color: var(--copper);
        border-color: rgba(198,95,60,0.7);
    }

    .hero-stat-arrow {
        flex-shrink: 0;
        opacity: 0;
        transform: translateX(-3px);
        transition: opacity 0.18s ease, transform 0.18s ease;
    }

    .hero-stat:hover .hero-stat-arrow,
    .hero-stat:focus-visible .hero-stat-arrow {
        opacity: 1;
        transform: translateX(0);
    }

    /* ── PINNED SPECIMEN: today's vitrine pick, tilted like a specimen
       card pinned under the lead column. ────────────────────────────── */
    /* Full-width row under both hero columns. */
    .hero-pin-wrap {
        grid-column: 1 / -1;
        display: flex;
        justify-content: stretch;
        padding-top: clamp(26px, 3vw, 44px);
    }

    /* A mounted plate rather than a snapshot: the whole diorama is shown
       uncropped on its own card stock, with the label set beside it. */
    .hero-pin {
        position: relative;
        display: grid;
        grid-template-columns: minmax(220px, 30%) 1fr;
        align-items: center;
        gap: clamp(24px, 4vw, 56px);
        width: 100%;
        max-width: 100%;
        background: var(--cream2, #fff9ee);
        padding: clamp(18px, 2vw, 26px) clamp(24px, 4vw, 56px) clamp(18px, 2vw, 26px) clamp(18px, 2vw, 26px);
        text-decoration: none;
        box-shadow:
            0 22px 40px -18px rgba(20,11,7,0.35),
            0 4px 10px -4px rgba(20,11,7,0.2);
        transform: rotate(-0.8deg);
        transition: transform 0.3s ease, box-shadow 0.3s ease;
    }

    .hero-pin:hover,
    .hero-pin:focus-visible {
        transform: rotate(-0.2deg) translateY(-4px);
        box-shadow:
            0 30px 54px -18px rgba(20,11,7,0.4),
            0 6px 14px -6px rgba(20,11,7,0.24);
    }

    .hero-pin:focus-visible {
        outline: 2px solid rgba(198,95,60,0.56);
        outline-offset: 3px;
    }

    /* Thin engraved keyline around the photo — the piece sits *in* the plate. */
    .hero-pin-plate {
        display: block;
        position: relative;
        box-shadow: inset 0 0 0 1px rgba(20,11,7,0.28);
    }

    .hero-pin-img {
        display: block;
        width: 100%;
        aspect-ratio: 1 / 1;
        object-fit: cover;
        object-position: center;
    }

    .hero-pin-seal {
        position: absolute;
        top: -13px;
        left: clamp(60px, 20%, 110px);
        transform: rotate(-2deg);
        width: 26px;
        height: 26px;
        border-radius: 50%;
        background: radial-gradient(circle at 34% 30%, #d8734c, #96351c 72%);
        box-shadow: 0 4px 10px rgba(20,11,7,0.4);
    }

    .hero-pin-cap {
        display: grid;
        gap: 8px;
        align-content: center;
        min-width: 0;
    }

    .hero-pin-k {
        font-size: 10.5px;
        font-weight: 700;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: var(--muted);
    }

    .hero-pin-v {
        font-family: 'Cormorant Garamond', serif;
        font-style: italic;
        font-size: clamp(28px, 3.6vw, 48px);
        line-height: 1.1;
        color: var(--ink);
    }

    /* Leader rule runs out to the edge of the plate — a ledger line, not a dash. */
    .hero-pin-rule {
        display: block;
        height: 1px;
        width: 100%;
        margin: 10px 0 6px;
        background: linear-gradient(to right, rgba(111,59,36,0.5), rgba(111,59,36,0.08));
    }

    .hero-pin-m {
        display: flex;
        flex-wrap: wrap;
        gap: 6px 28px;
        font-size: 12px;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        color: var(--muted);
    }

    @media (max-width: 560px) {
        .hero-pin {
            grid-template-columns: 1fr;
            gap: 12px;
            padding: 12px 12px 16px;
            max-width: 300px;
        }

        .hero-pin-seal {
            left: 50%;
            transform: translateX(-50%) rotate(-2deg);
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .hero-pin,
        .hero-pin:hover,
        .hero-pin:focus-visible {
            transition: none;
        }
    }

    /* ── HERO VISUAL: cinematic letterboxed frame ─────────────────
       Site palette throughout (no near-black cinema ground) — the
       "cinema" comes from the letterbox proportions, the HUD strip,
       and the slow drift on the photo, not from a foreign color world. */
    .cine-frame {
         position: relative;
        border-radius: clamp(10px, 1.2vw, 16px);
        overflow: hidden;
        background: var(--ink);
        box-shadow:
            0 44px 84px -34px rgba(20,11,7,0.4),
            0 16px 30px -18px rgba(20,11,7,0.3);
    }

    .cine-photo {
        position: relative;
        width: 100%;
        /* Matches the current hero photo's own aspect ratio (1509×822). Now
           that the frame lives in its own flexible-width column instead of
           spanning the full page, it can just take the photo's real shape —
           no more dark contain-bars filling in a mismatched box. */
        aspect-ratio: 1509 / 822;
        max-height: 460px;
        overflow: hidden;
    }

    /* Lens layer: carries the dwell push-in, written every frame from rAF — no
       transition, it would only add lag on top of motion that is already smooth. */
    .hero-lens {
        position: absolute;
        inset: 0;
        z-index: 1;
        transform: scale(var(--lens-scale, 1));
        transform-origin: center center;
        will-change: transform;
    }

    .hero-img {
        position: absolute;
        inset: 0;
        width: 100%;
        height: 100%;
        object-fit: cover;
        object-position: center center;
        display: block;
        filter: saturate(0.94) contrast(1.12) brightness(1.02);
        backface-visibility: hidden;
    }

    .img-vignette {
        position: absolute;
        inset: 0; z-index: 2;
        background: radial-gradient(ellipse at center, transparent 46%, rgba(20,11,7,0.28) 100%);
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
        opacity: 0.03;
        mix-blend-mode: overlay;
        pointer-events: none;
        background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
    }

    /* HUD strip — the letterbox bar doubling as a status readout */
    .cine-hud {
        display: flex;
        align-items: center;
        gap: clamp(16px, 2.2vw, 26px);
        padding: 13px clamp(18px, 2.3vw, 28px);
        background: var(--ink);
        color: #fff7ea;
    }

    .cine-rec {
        display: inline-flex;
        align-items: center;
        gap: 9px;
        font-size: 11px;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: #fff7ea;
        flex-shrink: 0;
    }

    .cine-rec i {
        width: 7px; height: 7px;
        border-radius: 50%;
        background: var(--copper);
        display: block;
        animation: cine-blink 1.8s ease-in-out infinite;
    }

    @keyframes cine-blink {
        0%, 100% { opacity: 1; }
        50% { opacity: 0.28; }
    }

    .cine-scrub {
        flex: 1;
        height: 2px;
        border-radius: 2px;
        background: rgba(255,247,234,0.16);
        position: relative;
        overflow: hidden;
    }

    .cine-scrub i {
        position: absolute;
        inset: 0;
        width: 40%;
        background: var(--copper);
        display: block;
        animation: cine-scrub-move 7s ease-in-out infinite;
    }

    @keyframes cine-scrub-move {
        0%   { transform: translateX(-120%); }
        50%  { transform: translateX(60%); }
        100% { transform: translateX(240%); }
    }

    .cine-count {
        font-size: 11px;
        letter-spacing: 0.1em;
        text-transform: uppercase;
        color: rgba(255,247,234,0.68);
        white-space: nowrap;
        flex-shrink: 0;
    }

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

    /* ── WORK HUB ────────────────────────────────── */
    .all-link:focus-visible {
        outline: 2px solid rgba(198,95,60,0.56);
        outline-offset: 3px;
    }

    /* ── CONTEXT SECTION ─────────────────────────── */
    .context-section {
        padding: clamp(14px, 2vw, 26px) clamp(20px, 4.5vw, 64px) clamp(42px, 5.5vw, 72px);
        max-width: 1520px;
        margin: 0 auto;
        scroll-margin-top: calc(var(--site-header-height) + 12px);
    }

    .context-hd {
        display: grid;
        grid-template-columns: minmax(220px, 0.42fr) minmax(420px, 0.58fr);
        gap: clamp(18px, 2.4vw, 36px);
        align-items: end;
        margin-bottom: 12px;
        padding-bottom: 12px;
        border-bottom: 1px solid var(--border);
        /* Pins flush under the fixed site header while the cards below scroll
           past, releasing on its own once .context-section's end reaches this
           offset (i.e. once the visitor has scrolled through every card).
           54px matches SiteHeader's shrunk `.is-scrolled` height (the state
           it's always in by the time this section is reachable) — not the
           shared --site-header-height var, which is a larger padding buffer
           used elsewhere and would leave a gap here. */
        position: sticky;
        top: 54px;
        z-index: 3;
        /* No fill of its own — see ::before. A solid --cream here painted a strip
           that was both the wrong colour (--cream is #FAF6EE; the page is #F8F1E7)
           and the wrong width (it stopped at the section's padding), so it read as
           a pale band with cut edges. The old tile grid hid it; a single column of
           cards does not. */
        background: transparent;
    }

    /* The bar the header pins against: the page's own colour, bled to the full
       window width so it has no edges to see. */
    .context-hd::before {
        content: '';
        position: absolute;
        top: 0;
        bottom: 0;
        left: 50%;
        width: 100vw;
        transform: translateX(-50%);
        background: #f8f1e7;
        z-index: -1;
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
        display: flex;
        align-items: flex-end;
        justify-content: space-between;
        gap: 24px;
        padding-bottom: 0;
    }

    .context-side-main {
        display: flex;
        flex-direction: column;
        gap: 4px;
        min-width: 0;
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

    .context-meta {
        display: flex;
        align-items: baseline;
        flex-wrap: wrap;
        gap: 8px;
        font-family: 'Cormorant Garamond', serif;
        font-size: 14px;
        font-style: italic;
        font-weight: 300;
        color: var(--muted2);
    }

    .context-meta-kicker {
        font-family: 'Inter', sans-serif;
        font-size: 10px;
        font-weight: 600;
        font-style: normal;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: var(--copper);
    }

    .context-side-links {
        display: flex;
        flex-direction: column;
        align-items: flex-end;
        gap: 10px;
        flex-shrink: 0;
    }

    .work-content {
        display: grid;
        grid-template-columns: 1fr;
    }

    /* The reel and its margin, as one spread. On a narrow screen there is no
       margin to write in, so the reel simply takes the whole page — the index is
       not rendered at all. */
    .work-spread {
        display: grid;
        grid-template-columns: minmax(0, 1fr);
        max-width: var(--reel-card-width, 64rem);
        /* The old masonry got its breathing room under the section rule from the
           tiles' own margins. A grid has none, so the spread states it. */
        margin: clamp(2rem, 4vw, 3.5rem) auto 0;
    }

    .work-spread > :global(.margin-index) { display: none; }

    @media (min-width: 1280px) {
        .work-spread {
            grid-template-columns: 5.5rem minmax(0, 1fr);
            gap: clamp(1.5rem, 3vw, 3rem);
            /* Widened by exactly the margin column, so the reel itself stays
               where it was on the page. */
            max-width: calc(var(--reel-card-width, 64rem) + 8.5rem);
        }

        .work-spread > :global(.margin-index) { display: block; }
    }

    /* A vertical reel of panes, one per row. Their own look lives in
       ReelWorkCard; this only spaces them. */
    .work-reel {
        display: grid;
        gap: var(--reel-card-gap, 2.25rem);
        min-width: 0;
    }

    /* The observed unit, and the anchor the margin index scrolls back to. */
    .reel-slot {
        scroll-margin-top: 96px;
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

    /* ── ARCHIVE: THE REEL'S CLOSING PLATE ───────── */
    .archive-end {
        display: grid;
        grid-template-columns: minmax(0, 1fr);
        gap: clamp(20px, 2.6vw, 30px);
        align-items: center;
        padding: clamp(26px, 3.4vw, 40px) clamp(24px, 3.2vw, 40px);
        border: 1px solid var(--border2);
        /* Double rule — the drawer's own edge, as on the modals. */
        box-shadow: inset 0 0 0 4px var(--cream), inset 0 0 0 5px var(--border);
        border-radius: 3px;
        background: var(--cream2);
        text-decoration: none;
        color: var(--ink);
        transition: border-color 0.32s ease, background 0.32s ease;
    }
    .archive-end:hover { border-color: var(--copper); background: var(--cream); }

    @media (min-width: 760px) {
        .archive-end {
            grid-template-columns: minmax(0, 0.9fr) minmax(0, 1.1fr);
            grid-template-areas: 'text drawer' 'cta drawer';
            gap: clamp(18px, 2.4vw, 26px) clamp(28px, 4vw, 56px);
            align-content: center;
        }
        .archive-end__text   { grid-area: text; align-self: end; }
        .archive-end__cta    { grid-area: cta; align-self: start; }
        .archive-end__drawer { grid-area: drawer; }
    }

    .archive-end__count {
        display: block;
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(52px, 6.5vw, 82px);
        font-weight: 400;
        line-height: 0.9;
        color: var(--ink);
    }

    .archive-end__label {
        display: block;
        margin-top: 10px;
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.16em;
        text-transform: uppercase;
        color: var(--muted);
    }

    /* The drawer: plates of the next works, each sliding a little further out of
       the box, the last one cut by the frame — there is more behind it. */
    .archive-end__drawer {
        display: flex;
        min-width: 0;
        overflow: hidden;
    }

    .archive-end__plate {
        position: relative;
        flex: 0 0 auto;
        width: clamp(72px, 9vw, 108px);
        aspect-ratio: 3 / 4;
        border: 1px solid var(--border2);
        border-radius: 3px;
        overflow: hidden;
        background: var(--cream);
        /* Overlapped like sheets pulled from a drawer; the later ones lift. */
        margin-left: calc(var(--peek-i) * -0.5px);
        transform: translateY(calc(var(--peek-i) * -3px)) rotate(calc(var(--peek-i) * 0.6deg));
        transition: transform 0.4s cubic-bezier(0.22, 1, 0.36, 1);
    }
    .archive-end__plate + .archive-end__plate { margin-left: clamp(-22px, -1.4vw, -10px); }

    .archive-end:hover .archive-end__plate {
        transform: translateY(calc(var(--peek-i) * -5px)) rotate(calc(var(--peek-i) * 0.9deg));
    }

    .archive-end__plate :global(.archive-end__plate-img),
    .archive-end__plate :global(img) {
        width: 100%;
        height: 100%;
        object-fit: cover;
        object-position: center top;
        display: block;
        /* The archive is remembered, not lit — colour returns on approach. */
        filter: grayscale(0.35) contrast(0.96);
        transition: filter 0.4s ease;
    }
    .archive-end:hover .archive-end__plate :global(img) { filter: none; }

    .archive-end__cta {
        display: inline-flex;
        align-items: center;
        justify-self: start;
        gap: 10px;
        padding: 12px 22px;
        border: 1px solid var(--copper);
        border-radius: 3px;
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.16em;
        text-transform: uppercase;
        color: var(--copper);
        transition: background 0.28s ease, color 0.28s ease, gap 0.28s ease;
    }
    .archive-end:hover .archive-end__cta {
        background: var(--copper);
        color: var(--cream);
        gap: 14px;
    }

    @media (prefers-reduced-motion: reduce) {
        .archive-end__plate,
        .archive-end__cta,
        .archive-end__plate :global(img) { transition: none; }
        .archive-end:hover .archive-end__plate { transform: none; }
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

    .request-cta {
        margin-top: clamp(14px, 2vw, 22px);
    }

    .request-steps {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: 12px;
    }

    .request-steps span {
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
        display: grid;
        justify-items: center;
        width: 100%;
    }

    .workshop-stage {
        position: relative;
        width: 100%;
        min-height: clamp(420px, 42vw, 680px);
    }

    .workshop-plate {
        position: absolute;
        display: block;
        width: 69%;
        height: 74%;
        padding: 0;
        background: none;
        border: 1px solid rgba(52,37,28,0.08);
        box-shadow: 0 28px 76px rgba(52,37,28,0.14);
        overflow: hidden;
        cursor: pointer;
        transition: transform 0.4s cubic-bezier(0.16,1,0.3,1), box-shadow 0.4s ease, filter 0.4s ease;
    }

    .workshop-plate-a { left: 0; top: 0; }
    .workshop-plate-b { right: 0; bottom: 0; }

    .workshop-plate.is-front {
        z-index: 2;
        box-shadow: 0 34px 92px rgba(52,37,28,0.22);
    }

    .workshop-plate:not(.is-front) {
        z-index: 1;
        transform: scale(0.96);
        filter: saturate(0.8);
    }

    .workshop-plate:focus-visible {
        outline: 2px solid rgba(198,95,60,0.6);
        outline-offset: 3px;
    }

    .workshop-plate-poster {
        position: absolute;
        inset: 0;
        width: 100%;
        height: 100%;
        object-fit: cover;
    }

    .workshop-plate-label {
        display: flex;
        align-items: center;
        gap: 10px;
        margin-top: 18px;
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.14em;
        text-transform: uppercase;
        color: var(--mid);
    }

    .wpl-rule {
        width: 22px;
        height: 1px;
        background: rgba(198,95,60,0.4);
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
        .hero-cine {
            grid-template-columns: 1fr;
            padding-top: calc(var(--site-header-height) + 24px);
            gap: 22px;
        }

        /* Stacked, the photo leads: it is the thing worth seeing first, and the
           copy reads as its caption rather than as a wall to scroll past. */
        .cine-frame { order: -1; }

        /* Stacked, the text has the whole page width to itself — the 580px cap
           belonged to the two-column layout and here it just walls off the
           right half of the screen. */
        .hero-text { max-width: none; }

        .cine-photo { max-height: 420px; }

        .request-path {
            grid-template-columns: 1fr;
            padding-top: 16px;
        }

        .request-steps {
            grid-template-columns: repeat(3, minmax(0, 1fr));
        }

        .workshop-feature {
            grid-template-columns: 1fr;
            gap: 34px;
        }

        .workshop-stage {
            min-height: min(72vw, 560px);
        }
    }

    /* Tablet band: the hero has stacked, so the copy suddenly has ~1000px of
       width and nothing to do with it. Split it in two — the pitch (lead + CTAs)
       reads left, the evidence (trust line, process clips) sits right, and the
       ledger figures run the full width underneath as a footer rule. */
    @media (min-width: 781px) and (max-width: 1080px) {
        .hero-body {
            max-width: none;
        }

        .hero-main {
            display: grid;
            grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
            column-gap: clamp(32px, 5vw, 64px);
            align-items: start;
        }

        .hero-lead {
            grid-column: 1;
            grid-row: 1;
            max-width: none;
        }

        .hero-ctas {
            grid-column: 1;
            grid-row: 2;
            align-self: end;
            margin-bottom: 0;
        }

        .hero-proof {
            grid-column: 2;
            grid-row: 1;
            margin-top: 4px;
        }

        .hw-teasers {
            grid-column: 2;
            grid-row: 2;
            margin-top: 0;
        }

        .release-note {
            grid-column: 2;
            grid-row: 3;
        }

        .hero-stats {
            grid-column: 1 / -1;
            justify-content: flex-start;
            gap: clamp(40px, 8vw, 96px);
        }
    }

    /* Below the tablet band the side-by-side context header genuinely runs out
       of room and has to stack. */
    @media (max-width: 780px) {
        .context-hd {
            grid-template-columns: 1fr;
        }

        .context-side {
            flex-direction: column;
            align-items: flex-start;
            gap: 14px;
        }

        .context-side-links {
            align-items: flex-start;
        }
    }

    @media (max-width: 680px) {
        :root {
            --site-header-height: 58px;
        }

        .hero-cine {
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

        .cine-photo { min-height: 230px; max-height: 320px; }

        .cine-hud { gap: 12px; }
        .cine-count { display: none; }

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

        /* the action reads these gaps back; column count is chosen by width */
        .work-grid {
            --wg-col-gap: 12px;
            --wg-row-gap: 12px;
        }

        .request-path {
            padding: 24px 16px 70px;
        }

        .request-steps {
            grid-template-columns: 1fr;
        }

        .workshop-feature {
            padding: 36px 16px 74px;
        }

        .workshop-stage {
            min-height: 360px;
        }

        .workshop-plate {
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
        .cursor-glow { display: none; }
        .cta-primary,
        .cta-ghost {
            transform: none !important;
        }
    }

    @media (pointer: coarse) {
        /* Suppress compositing-heavy animations on touch devices */
        .grain { animation: none; }
    }

    @media (max-width: 460px) {
        /* keep two columns even on the smallest phones — a single-column stack
           reads as a shop list, not a mosaic wall. Feature plates still span
           both (full width) as a deliberate spread. */
        .work-grid {
            --wg-col-gap: 8px;
            --wg-row-gap: 8px;
        }
    }
</style>
