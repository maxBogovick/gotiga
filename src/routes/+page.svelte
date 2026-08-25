<script lang="ts">
    import { onMount } from 'svelte';
    import { fade, fly } from 'svelte/transition';
    import { cubicOut } from 'svelte/easing';
    import { api, resolveSrcset, resolveBackgroundSrcset, resolveWebpUrl } from '$lib/api';
    import { figurineHref } from '$lib/figurineHref';
    import { createSiteAnalytics } from '$lib/analytics';
    import AppImage from '$lib/components/AppImage.svelte';
    import type { AuthorProfile, FigurineListItem, GazetteHome, HomeContent } from '$lib/types/api';
    import { t, brandName } from '$lib/i18n';
    import ReelWorkCard from '$lib/components/ReelWorkCard.svelte';
    import WorkMarginIndex from '$lib/components/WorkMarginIndex.svelte';
    import VisitorBook from '$lib/components/VisitorBook.svelte';
    import ImpressionsQuoteStrip from '$lib/components/ImpressionsQuoteStrip.svelte';
    import AuthorStory from '$lib/components/AuthorStory.svelte';
    import CorrespondenceInvite from '$lib/components/CorrespondenceInvite.svelte';
    import HeroWorkshopTeaser from '$lib/components/HeroWorkshopTeaser.svelte';
    import { heroImageUrl, pickHeroFigurine, pickLatestAddedWork, sortWorks, visibleWorks } from '$lib/home-hero';
    import { syncAttr } from '$lib/hydrate-image';
    import { afterLoadIdle } from '$lib/after-load-idle';
    import { visitorBook } from '$lib/stores/visitor-book.svelte';
    import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
    import { visitorMarks } from '$lib/stores/visitor-marks.svelte';
    import { houseClock } from '$lib/stores/house-clock.svelte';
    import { showingRooms } from '$lib/stores/showing-rooms.svelte';
    import { SITE_URL } from '$lib/site';
    import { jsonLdSafe } from '$lib/jsonld';
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
    import { injectStyle } from '$lib/inject-style';

    let { data } = $props();

    // WebSite + Organization graph — anchors the brand for search engines and LLMs and
    // ties every other JSON-LD node (figurines, the author) back to one named entity.
    // The Organization carries a logo so Google can show it in brand/knowledge panels.
    let websiteJsonLd = $derived(jsonLdSafe({
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
    // Code-split blotter / gazette plate: they must not be in the SSR tree or
    // the client {#await} pending state fights the prerendered HTML. Flip this
    // on mount so hydration is a no-op, then the chunks load off the critical path.
    let deferHomeExtras = $state(false);
    // The hero's own inputs (the background, and the work it shows) are resolved in
    // load() and seeded here, so the <img> in the prerendered HTML is already the one
    // this component wants — see the note in +page.ts. The bundled photo is only the
    // last resort, for a house with neither a background nor a single catalogued work.
    const FALLBACK_HERO = '/images/cabinet-bg.jpeg';
    let backgroundUrl = $state<string | null>(data.bg);
    // Seeded from load(), so the reel is IN the prerendered HTML — see the note in +page.ts
    // (the archive has always done this; the home page used to start empty and pop in).
    // init() then refreshes them against the live API, sharing load()'s deduped request.
    // $state.raw: these arrays are replaced wholesale (init() reassigns them, never
    // mutates in place), so the deep reactive proxy over every figurine object is pure
    // overhead — raw state tracks the reassignment and skips the proxying.
    let collectionFigurines = $state.raw<FigurineListItem[]>(data.works);
    // Derived subset of the collection — single source of truth. Was a separate $state
    // that init() had to keep in sync by hand (a drift-bug waiting to happen).
    let availableFigurines = $derived(collectionFigurines.filter((f) => f.status === 'available'));
    let inProgressFigurines = $state.raw<FigurineListItem[]>(data.inProgress);
    let heroFigurine = $state<FigurineListItem | null>(data.heroFig ?? null);
    // Author-led hero + story content, reused from the admin-editable profile.
    let authorProfile = $state<AuthorProfile | null>(data.author ?? null);
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

    // The list IS the whole visible collection (see home-hero.ts), so its length is the
    // honest count — and, because the works are seeded from load(), it is already right in
    // the prerendered HTML rather than snapping into place after hydration.
    let collectionTotal = $derived(collectionFigurines.length);
    let homeContent = $state<HomeContent>(data.homeContent);
    let gazetteHome = $state<GazetteHome>(data.gazette);
    let mouseX = $state(0.5);
    let mouseY = $state(0.5);
    let canUseHeroTilt = $state(false);

    // Clicking a hero locket reveals its clip full-size, growing out of the
    // locket's own screen position rather than a plain fade.
    let reelModalOpen = $state(false);
    let reelModalClip = $state<'a' | 'b'>('a');
    let reelModalOrigin = $state<{ x: number; y: number; width: number; height: number } | null>(null);

    function openReelModal(which: 'a' | 'b', e: MouseEvent) {
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

    // SEO title/description: derive from the admin-editable home content (with a stable
    // brand-bearing fallback) instead of a hardcoded string, so the page's two most
    // important on-page signals actually reflect — and can be tuned to — the real content.
    let metaTitle = $derived(
        homeContent.title?.trim()
            ? `${homeContent.title.trim()} · ${$brandName}`
            : `${$brandName} — Cabinet of Gothic Miniatures`
    );
    let metaDescription = $derived(
        (homeContent.lead?.trim()
            || authorProfile?.tagline?.trim()
            || "An author's cabinet of gothic figures and handmade miniatures."
        ).slice(0, 200)
    );

    let primaryCtaHref = '#gallery';
    let primaryCtaText = $derived($t('homeSeeTheWorks'));
    let secondaryCtaHref = '#correspondence';
    let secondaryCtaText = $derived($t('headerContactLabel'));
    // The book-holders' "first look" shelf: works genuinely inside their timed
    // early-release window (held out of the public archive by the server until
    // their hour). Rendered only when signed (see template guard).
    let firstLookFigurines = $state.raw<FigurineListItem[]>([]);
    // Hybrid editorial+algorithmic shelf resolved entirely server-side (admin
    // pins + top of the private mark ranking) — see /figurines/noticed.
    let noticedByGuestsFigurines = $state.raw<FigurineListItem[]>([]);
    // The work ACTUALLY IN the hero photograph — which is nobody when an admin-uploaded
    // background has overridden it. The caption ("Selected work: <name> → Open work") and
    // the alt text hang off this, not off heroFigurine: the latter is only the pick for the
    // photo, and when the background wins, naming a piece the visitor cannot see in the
    // frame — and linking to it — is a caption that lies.
    let heroPhotoFigurine = $derived(backgroundUrl ? null : heroFigurine);
    let heroObjectName = $derived(homeContent.heroCaptionTitle?.trim() || heroPhotoFigurine?.name || homeContent.title?.trim() || '');
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
    let heroObjectHref = $derived(heroPhotoFigurine ? figurineHref(heroPhotoFigurine, 'home_featured') : '/figurines');
    let latestAddedWork = $derived(pickLatestAddedWork([...collectionFigurines, ...inProgressFigurines]));
    let showHeroCaption = $derived(Boolean(heroObjectName));
    // The hero photo. `heroFigurine` is the deterministic pick made by pickHeroFigurine —
    // the same function load() ran at build time, over the same data — so this string is
    // normally IDENTICAL to the one already baked into the prerendered <img>, and no
    // second photograph is downloaded. An admin-uploaded background (Replace Photo) still
    // overrides it: that is an explicit choice, not a fallback.
    //
    // The hero fills the fold, so it takes the preview-sized photo — the 420px thumbnail
    // behind faceImageUrl is built for the archive's small cards and goes to mush here.
    let heroDisplayImage = $derived(heroImageUrl(backgroundUrl, heroFigurine, FALLBACK_HERO));
    const HERO_SIZES = '(max-width: 1080px) 100vw, 58vw';
    let heroSrcset = $derived(
        resolveSrcset(heroDisplayImage) ?? resolveBackgroundSrcset(heroDisplayImage)
    );
    // Format-only fallback when there is neither a figurine srcset nor a 900px
    // background sibling (bundled /images/cabinet-bg.jpeg). Prefer the WebP
    // rewrite when it exists so preload and <picture> point at the same file.
    let heroBackgroundWebp = $derived(heroSrcset ? null : resolveWebpUrl(heroDisplayImage));

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
    // noticed-by-guests) still dedupe against each other — with a catalog this
    // small, the same card twice reads as broken, not as generous curation.
    let homeShelves = $derived.by(() => {
        const used = new Set<string>();
        const claim = (list: FigurineListItem[]) => {
            const picked = list.filter((f) => !used.has(f.id));
            for (const f of picked) used.add(f.id);
            return picked;
        };

        const marked = claim(markedWorkFigurines);
        const firstLook = claim(firstLookFigurines);
        const noticed = claim(noticedByGuestsFigurines);

        return { marked, firstLook, noticed };
    });

    // The main gallery: the site's actual purpose — a generous, unfiltered
    // wall of the maker's work, not a tabbed shop shelf. Everyone gets the
    // same wide cross-section (status reads as a quiet museum-label mark on
    // each card, not a filter axis). Order is the author's `sortOrder`, then
    // newest first — nothing else (no daily rotation, no vitrine carve-out).
    const GALLERY_LIMIT = 16;
    let galleryFigurines = $derived(collectionFigurines);
    let visibleGalleryFigurines = $derived(galleryFigurines.slice(0, GALLERY_LIMIT));
    // Stable ids for the keeper — a `.map()` in the template would allocate a
    // new array on every parent invalidation and re-seed the blotter.
    let galleryReelIds = $derived(visibleGalleryFigurines.map((f) => f.id));
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

    // The hero is seeded from +page.ts's load(), so it paints without waiting on this.
    // Everything that varies with the collection — the works reel above all — is fetched
    // HERE, on the client, and never from load(): this page is prerendered, and a work's
    // photo that was rendered at build time is not replaced during hydration (see the note
    // in +page.ts). homeContent is re-fetched too, so an admin's edit shows up without a
    // rebuild — and so is the background: its URL is baked into the static HTML like every
    // other load() value, so without re-reading it here a new background stays invisible
    // until the next deploy. The works page and the background are the same deduped reads
    // load() already made, so re-asking for them costs no extra request.
    async function init() {
        try {
            const returning =
                visitedBefore
                || visitorBook.signed
                || savedFigurines.ids.length > 0
                || Object.keys(visitorMarks.marks).length > 0
                || previewVisitorMode === 'returning';
            const none: FigurineListItem[] = [];
            const [page, firstLook, noticedByGuests, content, author, layout, savedReelTheme, bg, gazette] = await Promise.all([
                api.getFigurinesPage().catch(() => null),
                returning ? api.getFirstLookFigurines().catch(() => none) : Promise.resolve(none),
                returning ? api.getNoticedByGuests().catch(() => none) : Promise.resolve(none),
                api.getHomeContent().catch(() => null),
                api.getAuthorProfile().catch(() => null),
                api.getHomeLayout().catch(() => null),
                api.getReelTheme().catch(() => null),
                api.getMainBackground().catch(() => undefined),
                api.getGazetteHome().catch(() => ({ leaves: [], cuttings: [] } as GazetteHome)),
            ]);
            if (author) authorProfile = author;
            if (savedReelTheme) reelTheme = savedReelTheme;
            // Editor preview (postMessage) wins over the saved config.
            if (layout && !hlPreviewDriven) homeLayout = layout;
            if (content) homeContent = content;
            if (bg !== undefined) backgroundUrl = bg;

            if (page) {
                const works = sortWorks(visibleWorks(page.items));
                collectionFigurines = works;
                // availableFigurines is $derived from collectionFigurines — no manual sync.
                inProgressFigurines = sortWorks(page.items.filter((f) => f.status === 'in_progress'));
                // The same deterministic pick load() made at build time — normally the same
                // work, and therefore the same <img src>, so the hero is not re-downloaded.
                heroFigurine = pickHeroFigurine(works, content ?? homeContent);
            }
            firstLookFigurines = firstLook;
            noticedByGuestsFigurines = noticedByGuests;
            gazetteHome = gazette;
            isLoaded = true;
        } catch (e) {
            isLoaded = true;
        }
    }

    // The pointer glow follows the cursor. Coalesced onto the next animation frame: the
    // handler used to write two pieces of reactive state on EVERY mousemove event — 60–120
    // times a second, each one flushing an update and a style write for a single 500px
    // gradient. The screen cannot show more than one position per frame anyway.
    let mouseRaf = 0;
    function handleMouseMove(e: MouseEvent) {
        if (!canUseHeroTilt || mouseRaf) return;
        const { clientX, clientY } = e;
        mouseRaf = requestAnimationFrame(() => {
            mouseRaf = 0;
            mouseX = clientX / window.innerWidth;
            mouseY = clientY / window.innerHeight;
        });
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
    // The hero's own <picture>/<img>, so the post-hydration repair can reach them.
    let heroPictureEl = $state<HTMLElement | null>(null);
    let heroImgEl = $state<HTMLImageElement | null>(null);
    let heroLensScale = $derived((1 + heroDwellZoom).toFixed(4));

    const HERO_DWELL_CAP = 0.22;   // how far the dwell push can ever go
    const HERO_DWELL_TAU = 26000;  // ms to reach ~63% of the cap

    // The scroll cue, shown once the hero has settled. It needs no dismissal flag: it is
    // positioned inside the hero frame, so it leaves with it the moment the visitor does
    // the very thing it asks for. (There was such a flag; nothing ever set it.)
    let showHint = $state(false);

    // Site-wide analytics for the home page. `trackWorks` so each work tile that
    // scrolls into view (via `siteAnalytics.observeWork` on its .reel-slot) counts
    // toward the visit's works_seen — how many pieces the visitor actually saw
    // before leaving. `start()`/`stop()` measure dwell time + scroll depth.
    const siteAnalytics = createSiteAnalytics({ trackWorks: true });

    onMount(() => {
        // Site-wide view, no figurine attached — respects the same DNT/admin
        // exclusions as the figurine-detail tracking automatically. Dedupes
        // internally, so this stays a no-op if the component ever re-mounts.
        siteAnalytics.pageView();
        siteAnalytics.start();
        const stopExtras = afterLoadIdle(() => { deferHomeExtras = true; });

        // Hydration does not touch src/srcset (see hydrate-image.ts): on this prerendered
        // page the hero <img> holds whatever the BUILD resolved, and load() has since
        // re-run against the live API. Usually the two agree — the pick is deterministic —
        // and every line here is a no-op. When they don't (the admin changed the pinned
        // work, or replaced the background) this is what puts the right photograph on
        // screen instead of leaving the build's one there until the next deploy.
        syncAttr(heroPictureEl?.querySelector('source'), 'srcset', heroSrcset?.webp || heroBackgroundWebp);
        syncAttr(heroImgEl, 'srcset', heroSrcset?.jpeg ?? null);
        syncAttr(heroImgEl, 'src', heroDisplayImage);

        savedFigurines.load();
        visitorMarks.load();
        houseClock.start();
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
            // Only the admin editor, running on this very origin, may drive the page.
            // Without this check ANY site that embeds the home page in an iframe can post
            // this message and rearrange, recolour or hide the blocks of the real site —
            // the listener is on the public page, not on the admin one.
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
        void init();
        void showingRooms.load();
        const reduceMq = window.matchMedia('(prefers-reduced-motion: reduce)');
        const pointerMq = window.matchMedia('(pointer: fine)');
        // Dwell push-in: only accumulates while the photo is actually on screen,
        // and the rAF loop is torn down entirely when it leaves. Gated by the same
        // `pointer: fine` check as the tilt effect, not just reduced-motion — a touch
        // device can never produce the tilt this dwell zoom is paired with, and the
        // rAF loop is a continuous main-thread cost with nothing to show for it on
        // exactly the class of device (mobile) where that cost matters most.
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
            if (dwellRaf || reduceMq.matches || !pointerMq.matches) return;
            lastTick = 0;
            dwellRaf = requestAnimationFrame(tickDwell);
        };
        const stopDwell = () => {
            if (dwellRaf) cancelAnimationFrame(dwellRaf);
            dwellRaf = 0;
        };
        const syncTiltPreference = () => {
            canUseHeroTilt = pointerMq.matches && !reduceMq.matches;
            if (!canUseHeroTilt) {
                mouseX = 0.5;
                mouseY = 0.5;
            }
            // A touch device (or a session that just lost its fine pointer) loses the
            // dwell zoom too. Not restarted on the reverse transition here — the
            // IntersectionObserver below already calls startDwell() on the next scroll
            // that brings the hero into view, which is the common case.
            if (!canUseHeroTilt) {
                stopDwell();
                heroDwellZoom = 0;
            }
        };
        syncTiltPreference();
        reduceMq.addEventListener('change', syncTiltPreference);
        pointerMq.addEventListener('change', syncTiltPreference);

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
            stopExtras();
            clearTimeout(hintTimer);
            if (mouseRaf) cancelAnimationFrame(mouseRaf);
            stopDwell();
            heroObserver?.disconnect();
            reduceMq.removeEventListener('change', syncTiltPreference);
            pointerMq.removeEventListener('change', syncTiltPreference);
            window.removeEventListener('message', onHlMessage);
            siteAnalytics.stop();
        };
    });

    // The admin's reel-theme panel drives this page live over BroadcastChannel.
    onMount(() => startListeningForReelPreview());

    // The two stylesheets the admin drives (the reel theme, and the layout's element-level
    // overrides) both arrive from the API after the first render, and again — live — from
    // the editor's preview. They used to be injected two different ways for no reason that
    // survives inspection; see injectStyle.
    $effect(() => injectStyle('reel-theme', reelCSSBlock));
    $effect(() => injectStyle('hl-element-overrides', hlElementCSS));

</script>

<svelte:head>
    <title>{metaTitle}</title>
    <meta name="description" content={metaDescription} />
    <meta property="og:site_name" content={$brandName} />
    <meta property="og:locale" content="en_US" />
    <meta property="og:title" content={metaTitle} />
    <meta property="og:description" content={metaDescription} />
    <meta property="og:image" content={data.ogImage} />
    <meta property="og:type" content="website" />
    <meta property="og:url" content={SITE_URL} />
    <meta name="twitter:card" content="summary_large_image" />
    <meta name="twitter:title" content={metaTitle} />
    <meta name="twitter:image" content={data.ogImage} />
    <meta name="theme-color" content="#f8f1e7" />
    <!-- Preload the LCP hero. fetchpriority on the <img> raises its PRIORITY once found;
         this preload solves early DISCOVERY (web.dev 2026 says use both). Prefer the WebP
         rendition modern browsers actually paint (type-gated, so a no-WebP browser ignores
         it and loads the JPEG from the <img> as before); fall back to the single hero URL
         when there is no responsive srcset (admin background / bundled fallback). -->
    {#if heroSrcset?.webp}
        <link rel="preload" as="image" type="image/webp" imagesrcset={heroSrcset.webp} imagesizes={HERO_SIZES} fetchpriority="high" />
    {:else if heroBackgroundWebp}
        <!-- Admin-background hero: the <picture> paints heroBackgroundWebp, so preload THAT
             (not the JPEG), matching the source the browser actually chooses. -->
        <link rel="preload" as="image" type="image/webp" href={heroBackgroundWebp} fetchpriority="high" />
    {:else if heroDisplayImage}
        <link rel="preload" as="image" href={heroDisplayImage} fetchpriority="high" />
    {/if}
    {@html `<script type="application/ld+json">${websiteJsonLd}<\/script>`}
    <!-- Fonts are injected after load from +layout.svelte so they do not
         share the Slow 4G pipe with this photograph. -->
</svelte:head>

<svelte:window onmousemove={handleMouseMove} />

<div class="root">
    <div class="cursor-glow" style="transform:translate(calc({mouseX*100}vw - 250px),calc({mouseY*100}vh - 250px))"></div>

    <!-- Not a <main>: the layout (+layout.svelte) already renders the page's single
         <main> landmark around {@render children()}. A second <main> here nested one
         inside it is invalid (there must be exactly one per document) and hands screen
         readers two "main" landmarks to choose between. This is the page's own ground and
         block-ordering container; the landmark role stays with the layout. -->
    <div class="home-main" in:fade={{ duration: 700, delay: 40 }} style={homePageStyle(homeLayout)}>

        <!-- HERO -->
        {#if hlVisible('hero')}
        <div class={hlClasses('hero')} style={hlStyle('hero')} data-hl="hero">
        <section class="hero hero-cine" aria-labelledby="home-title">

            <!-- Photo, left: the work is the first thing the eye meets. -->
            <div class="cine-frame">
                <div class="cine-photo" style="--lens-scale:{heroLensScale}">
                    <div class="hero-lens" bind:this={heroPhotoEl}>
                        <!-- The hero is the LCP element, so what the browser picks here sets
                             the page's headline number. It fills the fold on a phone and about
                             half the width on a wide screen; with the 420/900/1800 renditions
                             offered, that resolves to the 900px medium on mobile instead of
                             the 1800px preview it used to pull down. -->
                        <picture bind:this={heroPictureEl}>
                            {#if heroSrcset?.webp}
                                <source type="image/webp" srcset={heroSrcset.webp} sizes={HERO_SIZES} />
                            {:else if heroBackgroundWebp}
                                <source type="image/webp" srcset={heroBackgroundWebp} />
                            {/if}
                            <img bind:this={heroImgEl}
                                 src={heroDisplayImage} srcset={heroSrcset?.jpeg}
                                 sizes={heroSrcset ? HERO_SIZES : undefined}
                                 alt={heroPhotoFigurine?.name ?? 'Gothic Cabinet'} class="hero-img"
                                 fetchpriority="high" decoding="async" draggable="false" />
                        </picture>
                    </div>
                    <div class="img-vignette"></div>
                    <div class="img-grade"></div>

                    {#if showHeroCaption}
                    <a class="art-caption" href={heroObjectHref} aria-label="{heroObjectCta}: {heroObjectName}">
                        <span class="art-caption-kicker">{$t('homeHeroObjectLabel')}</span>
                        <span class="art-caption-name">{heroObjectName}</span>
                        <span class="art-caption-open">{heroObjectCta} →</span>
                    </a>
                    {/if}
                </div>

                {#if showHint}
                    <a href="#gallery" class="scroll-cue" in:fade={{ duration: 400 }}>
                        <span class="sc-line"></span>
                        <span>{$t('homeScrollCue')}</span>
                    </a>
                {/if}
            </div>

            <!-- Copy, right column -->
            <div class="hero-text" in:fly={{ y: 20, duration: 900, delay: 350, easing: cubicOut }}>
                <p class="hero-kicker">{homeContent.kicker?.trim() || $t('homeKicker')}</p>
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

                        <div class="hero-doors">
                            <button
                                type="button"
                                class="hero-door"
                                onclick={(e) => openReelModal('a', e)}
                            >
                                <HeroWorkshopTeaser
                                    webm="/images/workshop/atelier-reel-tiny.webm"
                                    mp4="/images/workshop/atelier-reel-tiny.mp4"
                                    poster="/images/workshop/atelier-reel-tiny-poster.jpg"
                                    label={$t('homeDoorHandsTitle')}
                                    size="door"
                                    interactive={false}
                                />
                                <span class="hero-door-copy">
                                    <b>{$t('homeDoorHandsTitle')}</b>
                                    <span>{$t('homeDoorHandsHint')}</span>
                                </span>
                            </button>
                            <button
                                type="button"
                                class="hero-door"
                                onclick={(e) => openReelModal('b', e)}
                            >
                                <HeroWorkshopTeaser
                                    webm="/images/workshop/atelier-reel-2-tiny.webm"
                                    mp4="/images/workshop/atelier-reel-2-tiny.mp4"
                                    poster="/images/workshop/atelier-reel-2-tiny-poster.jpg"
                                    label={$t('homeDoorBenchTitle')}
                                    size="door"
                                    interactive={false}
                                    delayMs={400}
                                />
                                <span class="hero-door-copy">
                                    <b>{$t('homeDoorBenchTitle')}</b>
                                    <span>{$t('homeDoorBenchHint')}</span>
                                </span>
                            </button>
                            {#if deferHomeExtras}
                            {#await import('$lib/components/HeroGazettePlate.svelte') then { default: HeroGazettePlate }}
                            <div class="hero-door-row">
                                <HeroGazettePlate
                                    variant="door"
                                    leaves={gazetteHome.leaves}
                                    cuttings={gazetteHome.cuttings}
                                    latestWork={latestAddedWork}
                                />
                            </div>
                            {/await}
                            {#await import('$lib/components/HeroTalesPlate.svelte') then { default: HeroTalesPlate }}
                            <div class="hero-door-row">
                                <HeroTalesPlate />
                            </div>
                            {/await}
                            {:else}
                                <div class="hero-door-slot" aria-hidden="true"></div>
                            {/if}
                        </div>
                        {#if availableFigurines.length === 0}
                            <p class="release-note">{$t('homeReleaseNote')}</p>
                        {/if}
                    </div>

                </div>

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
                            {#await import('$lib/components/VisitLedger.svelte') then { default: VisitLedger }}
                            <VisitLedger
                                figurines={collectionFigurines}
                                rooms={showingRooms.list.map((r) => ({ id: r.id, name: r.name }))}
                                inProgressCount={inProgressFigurines.length}
                            />
                            {/await}
                        </div>
                    {/if}

                    {#if hlVisible('noticeBoard')}
                        <!-- "Opening soon": timed reveals — come at the appointed hour. Hides
                             itself when nothing is opening within the week. This is the one
                             real ceremony in the band — the showing programme runs regularly
                             and earns the theatre. -->
                        <div class={hlClasses('noticeBoard')} style={hlSubStyle('noticeBoard', hlBandOrder.indexOf('noticeBoard'))} data-hl="noticeBoard">
                            {#await import('$lib/components/HouseNoticeBoard.svelte') then { default: HouseNoticeBoard }}
                            <HouseNoticeBoard figurines={collectionFigurines} source="home_afisha" />
                            {/await}
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

            {#if deferHomeExtras}
            {#await import('$lib/components/KeeperNote.svelte') then { default: KeeperNote }}
            <KeeperNote figurines={collectionFigurines} reelIds={galleryReelIds} />
            {/await}
            {/if}

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
                                <div class="reel-slot" id="work-{fig.id}" data-reel-slot={i} use:siteAnalytics.observeWork={fig.id}>
                                    <!-- The pane's paragraph is the work's own short text,
                                         which the list payload already carries — no lookup
                                         table in between (there was one; it rebuilt an
                                         id→text map out of the very objects being passed). -->
                                    <ReelWorkCard
                                        {fig}
                                        index={i + 1}
                                        story={fig.shortText}
                                        flip={i % 2 === 1}
                                        source="home_grid"
                                        onLike={() => siteAnalytics.cta('wishlist')}
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
        <!-- Rendered as soon as there IS a profile — which, since load() now resolves it,
             means in the prerendered HTML. It used to wait for isLoaded, i.e. for a client
             fetch, so this section simply did not exist for anything that doesn't run JS. -->
        {#if authorProfile || isLoaded}
            <AuthorStory name={authorName} bio={authorProfile?.bio ?? null} photoUrl={authorProfile?.photoUrl ?? null} />
        {/if}
        </div>
        {/if}

        <!-- "Write to the author": the low-commitment touchpoint the page was
             missing between the author's story and the commission funnel — a
             stranger who isn't ready for the full /commission wizard still
             gets an obvious, immediate way to say something. -->
        {#if hlVisible('correspondence')}
        <div class={hlClasses('correspondence')} style={hlStyle('correspondence')} data-hl="correspondence">
        <CorrespondenceInvite authorName={authorName} />
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
                            {#await import('$lib/components/FirstLook.svelte') then { default: FirstLook }}
                            <FirstLook works={homeShelves.firstLook} greetName={visitorBook.name} source="home_first_look" />
                            {/await}
                        </div>
                    {/if}

                    <!-- The visitor's own private marks, resolved locally. -->
                    {#if hlVisible('markedByYou')}
                        <div class={hlClasses('markedByYou')} style={hlSubStyle('markedByYou', hlShelfOrder.indexOf('markedByYou'))} data-hl="markedByYou">
                            {#await import('$lib/components/MarkedByYou.svelte') then { default: MarkedByYou }}
                            <MarkedByYou figurines={homeShelves.marked} source="home_marked" />
                            {/await}
                        </div>
                    {/if}

                    <!-- Hybrid editorial+algorithmic shelf: admin pins + top of the private mark ranking. -->
                    {#if hlVisible('noticedByGuests')}
                        <div class={hlClasses('noticedByGuests')} style={hlSubStyle('noticedByGuests', hlShelfOrder.indexOf('noticedByGuests'))} data-hl="noticedByGuests">
                            {#await import('$lib/components/NoticedByGuests.svelte') then { default: NoticedByGuests }}
                            <NoticedByGuests figurines={homeShelves.noticed} source="home_noticed" />
                            {/await}
                        </div>
                    {/if}
                </div>
            </div>
        {/if}

    </div>

    {#if reelModalOpen}
        <!-- Dynamically imported: a modal that opens on click has no reason to be part
             of the home route's initial JS payload (see the same pattern for
             BrassLens/LivingDaguerreotype/RakingLight in the figurine-detail layouts). -->
        {#await import('$lib/components/WorkshopReelModal.svelte') then { default: WorkshopReelModal }}
            <WorkshopReelModal
                webm={reelModalClip === 'a' ? '/images/workshop/atelier-reel.webm' : '/images/workshop/atelier-reel-2.webm'}
                mp4={reelModalClip === 'a' ? '/images/workshop/atelier-reel.mp4' : '/images/workshop/atelier-reel-2.mp4'}
                poster={reelModalClip === 'a' ? '/images/workshop/atelier-reel-poster.jpg' : '/images/workshop/atelier-reel-2-poster.jpg'}
                caption={$t('homeWorkshopReelCaption')}
                closeLabel={$t('figurineGrimoireClose')}
                origin={reelModalOrigin}
                onClose={() => reelModalOpen = false}
            />
        {/await}
    {/if}
</div>

<style>
    /* Tokens (--cream, --ink, --muted, --site-header-height, --ease …) now live in
       app.css. They were declared here, in a `:root` block — which Svelte cannot scope,
       so they were global variables owned by a single route: the rest of the site saw
       them, or didn't, depending on whether this page's CSS chunk happened to be loaded.
       Same for the page's `scroll-behavior` and its `body` typography, which are now the
       site's (app.css) and this page's own (.root) respectively. */

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
    /* The page's own ground and typography. This was a `:global(body)` rule, which set the
       whole site's body font to Instrument Sans from inside one route's stylesheet. The
       page wraps everything it renders, so it can simply say it here — and `body` keeps
       app.css's canvas colour underneath for the overscroll area. */
    .root {
        color: var(--brown);
        font-family: 'Instrument Sans', sans-serif;
        -webkit-font-smoothing: antialiased;
        /* 100vw includes the scrollbar's width, so on any desktop browser that reserves
           one this box was ~15px wider than the viewport — overflow that `overflow-x: clip`
           then hid, after the layout had already been computed against the wrong width. */
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

    /* Was a bare `main {}` selector; renamed with the element (see the markup note) so
       it still matches after the tag became a <div class="home-main">. */
    .home-main {
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

    /* ── HERO LAYOUT (photo left, copy right) ─────────────────────
       The photograph is the first thing a visitor meets; the words sit
       beside it as a caption, not as a wall to read before the work. */
    .hero-cine {
        display: grid;
        grid-template-columns: minmax(420px, 1.2fr) minmax(300px, 0.8fr);
        align-items: center;
        gap: clamp(28px, 4vw, 56px);
        padding:
            calc(var(--site-header-height) + clamp(18px, 2.4vw, 34px))
            clamp(20px, 4.5vw, 64px)
            clamp(28px, 3.2vw, 44px);
        max-width: 1480px;
        margin: 0 auto;
    }

    /* ── HERO TEXT ───────────────────────────────── */
    .hero-text {
        position: relative;
        z-index: 10;
        min-width: 0;
        padding-block: 8px;
    }

    .hero-kicker {
        margin: 0 0 14px;
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: var(--copper);
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
        font-size: clamp(40px, 5.2vw, 76px);
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

    .hero-doors {
        display: flex;
        flex-direction: column;
        width: 100%;
        margin-top: 8px;
        border-bottom: 1px solid var(--border);
    }

    .hero-door,
    .hero-door-row,
    .hero-door-slot {
        border-top: 1px solid var(--border);
    }

    .hero-door {
        display: grid;
        grid-template-columns: 64px minmax(0, 1fr);
        gap: 14px;
        align-items: center;
        width: 100%;
        padding: 12px 0;
        border: none;
        border-top: 1px solid var(--border);
        background: none;
        text-align: left;
        color: inherit;
        font: inherit;
        cursor: pointer;
        -webkit-appearance: none;
        appearance: none;
    }

    .hero-door:hover .hero-door-copy b,
    .hero-door:focus-visible .hero-door-copy b {
        color: var(--copper);
    }

    .hero-door:focus-visible {
        outline: 2px solid rgba(198, 95, 60, 0.56);
        outline-offset: 3px;
    }

    .hero-door-copy {
        min-width: 0;
        display: flex;
        flex-direction: column;
        gap: 3px;
    }

    .hero-door-copy b {
        font-family: 'Cormorant Garamond', serif;
        font-size: 22px;
        font-weight: 500;
        line-height: 1.15;
        color: var(--ink);
        transition: color 0.18s ease;
    }

    .hero-door-copy span {
        font-size: 12px;
        line-height: 1.35;
        color: var(--muted);
    }

    .hero-door-row {
        min-width: 0;
    }

    .hero-door-slot {
        height: 88px;
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
        aspect-ratio: 4 / 3;
        min-height: 380px;
        max-height: min(72vh, 680px);
        overflow: hidden;
        border-radius: inherit;
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
        left: clamp(16px, 2vw, 28px);
        bottom: clamp(16px, 2vw, 28px);
        z-index: 28;
        display: grid;
        gap: 6px;
        max-width: min(320px, calc(100% - 32px));
        padding: 14px 16px 13px;
        color: var(--ink);
        text-decoration: none;
        background: var(--cream);
        border: 1px solid color-mix(in srgb, var(--ink) 12%, transparent);
        box-shadow: none;
        transition: border-color 0.22s ease, transform 0.18s ease;
    }

    .art-caption:hover {
        border-color: color-mix(in srgb, var(--copper) 55%, transparent);
        transform: translateY(-1px);
    }

    .art-caption-kicker {
        font-size: 10px;
        letter-spacing: 0.16em;
        line-height: 1;
        text-transform: uppercase;
        color: var(--copper);
    }

    .art-caption-name {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(22px, 2vw, 30px);
        font-weight: 500;
        font-style: normal;
        line-height: 1;
        color: var(--ink);
    }

    .art-caption-open {
        width: fit-content;
        margin-top: 4px;
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.12em;
        line-height: 1;
        text-transform: uppercase;
        color: var(--copper);
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
        /* A section title, not persistent chrome. It used to stick under the
           site header and stack with the keeper dock — three bars over the
           first work. The loupe in the header is how the visitor asks again. */
        background: transparent;
    }

    .context-title {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(30px, 2.8vw, 42px);
        font-weight: 300;
        line-height: 0.96;
        color: var(--ink);
        margin-top: 6px;
        font-synthesis: none;
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
        font-family: 'Instrument Sans', sans-serif;
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

    /* ── RESPONSIVE ──────────────────────────────── */
    @media (max-width: 1080px) {
        .hero-cine {
            grid-template-columns: 1fr;
            padding-top: calc(var(--site-header-height) + 24px);
            gap: 22px;
        }

        /* Stacked, the photo already leads in the markup. */

        /* Stacked, the text has the whole page width to itself — the 580px cap
           belonged to the two-column layout and here it just walls off the
           right half of the screen. */
        .hero-text { max-width: none; }

        .cine-photo { min-height: 280px; max-height: 420px; }

        .request-path {
            grid-template-columns: 1fr;
            padding-top: 16px;
        }

        .request-steps {
            grid-template-columns: repeat(3, minmax(0, 1fr));
        }
    }

    /* Tablet band: the hero has stacked, so the copy suddenly has ~1000px of
       width and nothing to do with it. Split it in two — the pitch (lead + CTAs)
       reads left, the three doors sit right. */
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

        .hero-doors {
            grid-column: 2;
            grid-row: 1 / span 2;
            margin-top: 0;
        }

        .release-note {
            grid-column: 1 / -1;
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

        /* The Works header specifically: on mobile it has no side-by-side room
           to hide its height in, and both of its links duplicate CTAs the
           visitor meets again in a moment — "Open the full archive" as the
           reel's own last card, "Propose your own idea" in the commission
           section further down. So this one header drops its meta line and
           links rather than stacking all of it above the first figurine. */
        .work-hd .context-meta,
        .work-hd .context-side-links {
            display: none;
        }
    }

    @media (max-width: 680px) {
        /* (--site-header-height drops to 58px here — in app.css, with the token itself.) */

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

        .release-note {
            margin-top: 26px;
            font-size: 16px;
        }

        .cine-photo { min-height: 230px; max-height: 320px; }

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

        .request-path {
            padding: 24px 16px 70px;
        }

        .request-steps {
            grid-template-columns: 1fr;
        }
    }

    @media (hover: none) {
        .cursor-glow { display: none; }
        .cta-primary,
        .cta-ghost {
            transform: none !important;
        }
    }

</style>
