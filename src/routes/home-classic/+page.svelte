<script lang="ts">
    import { onMount } from 'svelte';
    import { fade, fly } from 'svelte/transition';
    import { cubicOut } from 'svelte/easing';
    import { spring, type Spring } from 'svelte/motion';
    import { api } from '$lib/api';
    import { figurineHref } from '$lib/figurineHref';
    import type { AuthorProfile, FigurineListItem, HomeContent, WorkshopFeature } from '$lib/types/api';
    import { t, brandName, lang } from '$lib/i18n';
    import AppImage from '$lib/components/AppImage.svelte';
    import HomeFigurineTile from '$lib/components/HomeFigurineTile.svelte';
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
    import type { HomeLayoutConfig, HomeBlockId } from '$lib/types/api';

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

    let isLoaded = $state(false);
    let imageUrl = $state('/images/cabinet-bg.jpeg');
    let hasCustomHeroPhoto = $state(false);
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
    let homeContent = $state<HomeContent>({
        title: null,
        kicker: null,
        lead: null,
        heroFigurineId: null,
        heroCaptionTitle: null,
        heroCaptionMeta: null,
        heroCaptionCta: null,
        heroMode: null,
        vitrineFigurineId: null,
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

    let parallaxX = $state(0);
    let parallaxY = $state(0);
    let _tiltSpring: Spring<{ x: number; y: number }> | null = null;
    let _tiltUnsub: (() => void) | null = null;

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
    let heroObjectHref = $derived(heroFigurine ? figurineHref(heroFigurine) : '/figurines');
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
    let heroDisplayImage = $derived(
        hasCustomHeroPhoto ? imageUrl : (heroDisplayFigurine?.faceImageUrl?.trim() || imageUrl)
    );
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

    // THE COLLECTION card scroll-reveal treatment — admin-selectable (Home
    // Layout Editor → "Эффект карточек"), persisted in HomeLayoutConfig.
    // Each card triggers its own reveal via a live IntersectionObserver (see
    // revealOnEnter in HomeFigurineTile.svelte) as it crosses into view.
    let cardEffectVariant = $derived(homeLayout?.cardEffect ?? 'rise');

    // Daily "re-hang" index: advances once per calendar day. Every visitor on
    // the same day sees the same arrangement (no layout-shift noise on refresh),
    // but returning the next day finds a freshly rotated wall.
    function dayIndex(d = new Date()): number {
        return Math.floor(d.getTime() / 86_400_000);
    }

    function dailyRotate<T>(items: T[]): T[] {
        if (items.length < 2) return items;
        const offset = ((dayIndex() % items.length) + items.length) % items.length;
        return [...items.slice(offset), ...items.slice(0, offset)];
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

    // Pin the 2 newest works (by createdAt) at the front so a returning visitor
    // immediately sees the latest additions. The remaining works rotate daily.
    const PINNED_NEWEST = 2;

    function sortFeaturedFigurines(items: FigurineListItem[]) {
        const byDate = items.slice().sort((a, b) => {
            const da = a.createdAt ? new Date(a.createdAt).getTime() : 0;
            const db = b.createdAt ? new Date(b.createdAt).getTime() : 0;
            return db - da;
        });
        const pinned = byDate.slice(0, PINNED_NEWEST);
        const pinnedIds = new Set(pinned.map((f) => f.id));
        const rest = items
            .filter((f) => !pinnedIds.has(f.id))
            .sort((a, b) => (a.sortOrder ?? 0) - (b.sortOrder ?? 0));
        return [...pinned, ...dailyRotate(rest)];
    }

    // Pinterest-style masonry for the gallery. Rather than a dense CSS grid —
    // which places row-by-row and leaves dead vertical gaps whenever a tall card
    // (e.g. a sealed door) can't be backfilled — we run the real shortest-column
    // algorithm: measure every card and drop it into whichever column is
    // currently shortest, positioning it absolutely so columns stay flush. Wide
    // isFeatured cards span two columns (placed on the adjacent pair whose taller
    // side is lowest). A ResizeObserver on the container reflows on viewport
    // change, a second one on each card reflows as images load, and a
    // MutationObserver re-observes children when the gallery list re-renders.
    function masonryGrid(grid: HTMLElement) {
        // Take over from the CSS multi-column fallback (which the prerendered HTML
        // renders before hydration so cards never pile at the origin) and drive
        // absolute placement ourselves.
        grid.classList.add('is-js-masonry');

        let raf = 0;
        let lastWidth = -1;

        const columnsFor = (width: number): number =>
            width >= 1100 ? 4 : width >= 760 ? 3 : 2;

        const layout = () => {
            const children = Array.from(grid.children) as HTMLElement[];
            if (children.length === 0) { grid.style.height = ''; return; }

            const cs = getComputedStyle(grid);
            const colGap = parseFloat(cs.columnGap) || 0;
            const rowGap = parseFloat(cs.rowGap) || 0;
            const width = grid.clientWidth;
            if (!width) return;
            lastWidth = width;

            const fullCols = columnsFor(width);
            const cols = Math.max(1, Math.min(fullCols, children.length));
            let colW = (width - colGap * (cols - 1)) / cols;
            // A near-empty gallery must not balloon its few cards to fill the whole
            // width — hold them to the width they'd have at the full column count
            // and let them left-align.
            if (cols < fullCols) {
                colW = Math.min(colW, (width - colGap * (fullCols - 1)) / fullCols);
            }

            const isWide = (el: HTMLElement) => cols > 1 && el.classList.contains('tile-feature');

            // Pass 1 — set every width up front so the height reads in pass 2 cost
            // a single reflow rather than one per card (no write/read interleave).
            for (const el of children) {
                el.style.width = isWide(el) ? `${colW * 2 + colGap}px` : `${colW}px`;
            }
            // Pass 2 — measure all heights (reads only) …
            const measured = children.map((el) => el.offsetHeight);

            // … then place (writes only).
            const heights: number[] = new Array(cols).fill(0);
            const xOf = (c: number) => c * (colW + colGap);

            children.forEach((el, idx) => {
                const h = measured[idx];
                if (!isWide(el)) {
                    let c = 0;
                    for (let i = 1; i < cols; i++) if (heights[i] < heights[c]) c = i;
                    el.style.left = `${xOf(c)}px`;
                    el.style.top = `${heights[c]}px`;
                    heights[c] += h + rowGap;
                } else {
                    // A wide card spans an adjacent pair. Two forces pull against
                    // each other: keep the wall low (small `top`) but don't strand
                    // a tall empty "ledge" above the shorter of the two columns
                    // (small `waste`). Scoring `top + waste` together both spreads
                    // wide cards across the wall AND steers them onto level pairs,
                    // so neither failure mode (piling / ledges) wins.
                    let best = 0;
                    let bestScore = Infinity;
                    let bestTop = Infinity;
                    for (let i = 0; i < cols - 1; i++) {
                        const top = Math.max(heights[i], heights[i + 1]);
                        const waste = (top - heights[i]) + (top - heights[i + 1]);
                        const score = top + waste;
                        if (score < bestScore - 0.5 ||
                            (score <= bestScore + 0.5 && top < bestTop)) {
                            bestScore = score; bestTop = top; best = i;
                        }
                    }
                    const top = Math.max(heights[best], heights[best + 1]);
                    el.style.left = `${xOf(best)}px`;
                    el.style.top = `${top}px`;
                    heights[best] = heights[best + 1] = top + h + rowGap;
                }
            });

            // Trim the trailing rowGap baked into each column's running height so
            // the gallery doesn't leave an empty band before the next section.
            grid.style.height = `${Math.max(...heights) - rowGap}px`;
        };

        const schedule = () => {
            cancelAnimationFrame(raf);
            raf = requestAnimationFrame(layout);
        };

        // Width-only observer: relayout when the container's *width* changes
        // (viewport resize). Height-only changes are ignored — layout() writes the
        // container height itself, so reacting to it would relayout for nothing
        // (and risk a feedback cycle).
        const containerRO = new ResizeObserver(() => {
            if (grid.clientWidth !== lastWidth) schedule();
        });
        containerRO.observe(grid);

        // Each card's height changes as its image loads; re-pack when it does.
        const cardRO = new ResizeObserver(schedule);
        const observeChildren = () => {
            cardRO.disconnect();
            for (const child of Array.from(grid.children)) cardRO.observe(child as HTMLElement);
            schedule();
        };
        observeChildren();

        const mo = new MutationObserver(observeChildren);
        mo.observe(grid, { childList: true });

        return {
            destroy() {
                containerRO.disconnect();
                cardRO.disconnect();
                mo.disconnect();
                cancelAnimationFrame(raf);
            },
        };
    }

    async function init() {
        try {
            const [bgPath, page, inProgress, firstLook, noticedByGuests, content, workshop, author, layout] = await Promise.all([
                api.getMainBackground().catch(() => null),
                // Uncapped: this page resolves the admin's pinned hero/vitrine, the visitor's
                // marks and the notice board's showings against this list, so a page-sized
                // slice silently drops whichever of them falls outside it (see home-hero.ts).
                api.getFigurinesPage().catch(() => ({ items: [] as FigurineListItem[], total: 0 })),
                api.getInProgressFigurines().catch(() => [] as FigurineListItem[]),
                api.getFirstLookFigurines().catch(() => [] as FigurineListItem[]),
                api.getNoticedByGuests().catch(() => [] as FigurineListItem[]),
                api.getHomeContent().catch(() => ({
                    title: null,
                    kicker: null,
                    lead: null,
                    heroFigurineId: null,
                    heroCaptionTitle: null,
                    heroCaptionMeta: null,
                    heroCaptionCta: null,
                    heroMode: null,
                    vitrineFigurineId: null,
                } satisfies HomeContent)),
                api.getWorkshopFeature().catch(() => null),
                api.getAuthorProfile().catch(() => null),
                api.getHomeLayout().catch(() => null)
            ]);
            if (author) authorProfile = author;
            // Editor preview (postMessage) wins over the saved config.
            if (layout && !hlPreviewDriven) homeLayout = layout;
            if (bgPath) { imageUrl = bgPath; hasCustomHeroPhoto = true; }
            homeContent = content;
            if (workshop) workshopFeature = workshop;
            const figurines = page.items;
            const visibleFigurines = figurines.filter(f => f.status !== 'in_progress');
            collectionFigurines = sortFeaturedFigurines(visibleFigurines);
            collectionTotal = visibleFigurines.length;
            availableTotal = figurines.filter((item) => item.status === 'available').length;
            availableFigurines = sortFeaturedFigurines(visibleFigurines.filter((item) => item.status === 'available'));
            inProgressFigurines = sortFeaturedFigurines(inProgress);
            firstLookFigurines = firstLook;
            noticedByGuestsFigurines = noticedByGuests;
            heroFigurine = content.heroFigurineId
                ? visibleFigurines.find((item) => item.id === content.heroFigurineId) ?? null
                : null;
            // Today's vitrine: the admin-pinned vitrine figure if set (independent
            // of the hero banner's own pin), else a daily-rotating available work
            // (falling back to the wider collection when none are available). A
            // sealed work is skipped in favour of the next candidate — the vitrine
            // never spotlights a piece the rest of the house keeps behind a closed
            // door.
            const pinnedVitrineFigurine = content.vitrineFigurineId
                ? visibleFigurines.find((item) => item.id === content.vitrineFigurineId) ?? null
                : null;
            vitrineFig = (pinnedVitrineFigurine && isOpenNow(pinnedVitrineFigurine) ? pinnedVitrineFigurine : null)
                ?? dailyPick(availableFigurines.filter(isOpenNow))
                ?? dailyPick(collectionFigurines.filter(isOpenNow));
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
        _tiltSpring?.set({ x: (mouseX - 0.5) * 2, y: (mouseY - 0.5) * 2 });
    }

    let showHint = $state(false);
    let hintDismissed = $state(false);

    onMount(() => {
        savedFigurines.load();
        visitorMarks.load();
        houseClock.start();
        showingRooms.load();
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
            // Only the admin editor, running on this very origin, may drive the page —
            // otherwise any site that embeds this one in an iframe can rearrange it.
            if (e.origin !== window.location.origin) return;
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
            if (canUseHeroTilt && !_tiltSpring) {
                _tiltSpring = spring({ x: 0, y: 0 }, { stiffness: 0.04, damping: 0.45 });
                _tiltUnsub = _tiltSpring.subscribe(v => { parallaxX = v.x; parallaxY = v.y; });
            } else if (!canUseHeroTilt) {
                mouseX = 0.5;
                mouseY = 0.5;
                _tiltSpring?.set({ x: 0, y: 0 });
                parallaxX = 0;
                parallaxY = 0;
            }
        };
        syncTiltPreference();
        reduceMq.addEventListener('change', syncTiltPreference);
        pointerMq.addEventListener('change', syncTiltPreference);
        const hintTimer = setTimeout(() => { showHint = true; }, 3000);
        return () => {
            clearTimeout(hintTimer);
            reduceMq.removeEventListener('change', syncTiltPreference);
            pointerMq.removeEventListener('change', syncTiltPreference);
            window.removeEventListener('message', onHlMessage);
            _tiltUnsub?.();
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
        <section class="hero" aria-labelledby="home-title">

            <!-- Left: text -->
            <div class="hero-text" in:fly={{ x: -20, duration: 900, delay: 350, easing: cubicOut }}>
                <div class="hero-orn" aria-hidden="true">
                    <svg class="hero-orn-svg" viewBox="0 0 280 22" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <line x1="0" y1="11" x2="96" y2="11" stroke="currentColor" stroke-width="0.5" opacity="0.65"/>
                        <polygon points="96,11 102,7 108,11 102,15" fill="currentColor" opacity="0.52"/>
                        <circle cx="140" cy="11" r="10" stroke="currentColor" stroke-width="0.65" opacity="0.68"/>
                        <circle cx="140" cy="11" r="4.5" stroke="currentColor" stroke-width="0.5" opacity="0.46"/>
                        <line x1="140" y1="1" x2="140" y2="21" stroke="currentColor" stroke-width="0.4" opacity="0.42"/>
                        <line x1="130" y1="11" x2="150" y2="11" stroke="currentColor" stroke-width="0.4" opacity="0.42"/>
                        <circle cx="140" cy="1" r="1.2" fill="currentColor" opacity="0.52"/>
                        <circle cx="140" cy="21" r="1.2" fill="currentColor" opacity="0.52"/>
                        <circle cx="130" cy="11" r="1.2" fill="currentColor" opacity="0.52"/>
                        <circle cx="150" cy="11" r="1.2" fill="currentColor" opacity="0.52"/>
                        <circle cx="140" cy="11" r="1.8" fill="currentColor" opacity="0.36"/>
                        <polygon points="172,11 178,7 184,11 178,15" fill="currentColor" opacity="0.52"/>
                        <line x1="184" y1="11" x2="280" y2="11" stroke="currentColor" stroke-width="0.5" opacity="0.65"/>
                    </svg>
                </div>

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
            </div>

            <!-- Right: the work itself — a real piece, not a room to explore.
                 The goal of this page is to show and enchant with the work, so
                 the very first thing on screen is a large photograph of one. -->
            <div class="hero-visual">
                <div
                    class="img-frame"
                    style="
                        transform:
                            perspective(2200px)
                            rotateY({canUseHeroTilt ? parallaxX * -1.2 : 0}deg)
                            rotateX({canUseHeroTilt ? parallaxY * 1.2 : 0}deg)
                            scale(1.02);
                    "
                >
                    <img src={heroDisplayImage} alt={heroDisplayFigurine?.name ?? 'Gothic Cabinet'} class="hero-img"
                         fetchpriority="high" decoding="async" draggable="false" />
                    <div class="img-vignette"></div>
                    <div class="img-grade"></div>
                    <div class="img-noise"></div>
                    <div class="fog fog-a"></div>
                    <div class="fog fog-b"></div>

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
                    <div class="context-side-row">
                        <p class="context-desc">{$t('homeGalleryText')}</p>
                        <a href="/figurines" class="all-link">
                            {$t('homeGalleryCta')}
                            <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                                <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                            </svg>
                        </a>
                    </div>
                    <div class="context-side-row context-side-row--guide">
                        <p class="context-desc context-desc--guide">
                            <span class="context-desc-kicker">{$t('homeHowEyebrow')}</span>
                            {$t('homeWorksGuideTitle')}
                        </p>
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
                    <div
                        class="work-grid"
                        use:masonryGrid
                        data-card-fx={cardEffectVariant}
                    >
                        {#each visibleGalleryFigurines as fig, i (fig.id)}
                            <HomeFigurineTile {fig} index={i} selected={heroFigurine?.id === fig.id} masonry />
                        {/each}
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

            {#if galleryRemaining > 0}
                <a href="/figurines" class="work-more-ledger">
                    <span class="work-more-ledger__rule"></span>
                    <span class="work-more-ledger__label">
                        <span class="work-more-ledger__count">{galleryRemaining}</span>
                        {$t('homeMoreInArchive')}
                    </span>
                    <span class="work-more-ledger__rule"></span>
                    <svg class="work-more-ledger__arrow" width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                        <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                    </svg>
                </a>
            {/if}

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
    /* Tokens live in app.css. Declaring them in a component's `:root` makes them GLOBAL
       (Svelte cannot scope `:root`), so this one route was silently redefining the whole
       site's palette — including the AA-contrast inks — for every page visited after it. */

    * { margin: 0; padding: 0; box-sizing: border-box; }

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

    /* ── ROOT ────────────────────────────────────── */
    /* The page's own ground and typography. This was a `:global(body)` rule — i.e. one route
       setting the whole site's body font, for the rest of the session. */
    .root {
        color: var(--brown);
        font-family: 'Instrument Sans', sans-serif;
        -webkit-font-smoothing: antialiased;
        width: 100%;
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

    .hero-orn {
        margin-bottom: 18px;
        color: var(--copper);
    }

    .hero-orn-svg {
        display: block;
        width: clamp(180px, 20vw, 280px);
        height: auto;
        animation: orn-breathe 11s ease-in-out infinite;
    }

    @keyframes orn-breathe {
        0%, 100% { opacity: 0.62; }
        50% { opacity: 0.38; }
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
        .hero-orn-svg {
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

    /* ── HERO VISUAL ─────────────────────────────── */
    .hero-visual {
        position: relative;
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
    }

    .context-hd {
        display: grid;
        grid-template-columns: minmax(220px, 0.42fr) minmax(420px, 0.58fr);
        gap: clamp(18px, 2.4vw, 36px);
        align-items: center;
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
        background: var(--cream);
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
        flex-direction: column;
        gap: 8px;
        padding-bottom: 0;
    }

    .context-side-row {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: end;
        gap: 18px;
    }

    .context-side-row--guide {
        padding-top: 8px;
        border-top: 1px solid rgba(52,37,28,0.10);
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

    .context-desc--guide {
        display: flex;
        align-items: baseline;
        flex-wrap: wrap;
        gap: 8px;
    }

    .context-desc-kicker {
        font-family: 'Inter', sans-serif;
        font-size: 10px;
        font-weight: 600;
        font-style: normal;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: var(--copper);
    }

    .work-content {
        display: grid;
        grid-template-columns: 1fr;
    }

    /* Pinterest masonry. Before hydration (and in the prerendered web build) the
       grid renders as a plain CSS multi-column flow, so cards never pile at the
       origin and the page keeps a real height — no overlap flash, no layout
       shift. Once the masonryGrid action mounts it adds `.is-js-masonry` and takes
       over: the container becomes a positioning context whose children it places
       absolutely into the shortest column — flush, no gaps. Gaps live in custom
       properties so the mobile overrides win regardless of selector specificity;
       `display: grid` in JS mode exists only so those gaps resolve to px for the
       action to read back via getComputedStyle. */
    .work-grid {
        --wg-col-gap: clamp(10px, 1.2vw, 16px);
        --wg-row-gap: clamp(12px, 1.4vw, 18px);
        column-count: 4;
        column-gap: var(--wg-col-gap);
    }

    .work-grid > :global(*) {
        width: 100%;
        margin: 0 0 var(--wg-row-gap);
        break-inside: avoid;
    }

    @media (max-width: 1100px) { .work-grid { column-count: 3; } }
    @media (max-width: 760px)  { .work-grid { column-count: 2; } }

    .work-grid.is-js-masonry {
        position: relative;
        display: grid;
        column-count: initial;
        column-gap: var(--wg-col-gap);
        row-gap: var(--wg-row-gap);
    }

    .work-grid.is-js-masonry > :global(*) {
        position: absolute;
        top: 0;
        left: 0;
        width: auto;
        margin: 0;
        break-inside: auto;
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
        .context-side-row {
            grid-template-columns: 1fr;
        }

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

    @media (max-width: 680px) {
        /* (--site-header-height drops to 58px here — in app.css, with the token itself.) */

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
        .fog-a, .fog-b { animation: none; }
        /* No 3D tilt on mobile — don't pay for the compositing layer */
        .img-frame {
            transform-style: flat;
            will-change: auto;
        }
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
