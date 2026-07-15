<script lang="ts">
  import { onMount } from 'svelte';
  import { beforeNavigate, afterNavigate, invalidateAll, goto } from '$app/navigation';
  import { fade, slide } from 'svelte/transition';
  import { t, lang, brandName } from '$lib/i18n';
  import { SITE_URL } from '$lib/site';
  import { figurineHref } from '$lib/figurineHref';
  import AppImage from '$lib/components/AppImage.svelte';
  import SealedDoor from '$lib/components/SealedDoor.svelte';
  import Lightbox from '$lib/components/Lightbox.svelte';
  import OrderModal from '$lib/components/OrderModal.svelte';
  import FilterPopover from '$lib/components/FilterPopover.svelte';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
  import { houseClock } from '$lib/stores/house-clock.svelte';
  import { showingRooms } from '$lib/stores/showing-rooms.svelte';
  import { isGated, isShowingOpen, resolveWindow, openingHeadline } from '$lib/showing-window';
  import type { FigurineListItem } from '$lib/types/api';

  // "The house wakes": a gated work is sealed behind a carved door while the
  // visitor's local clock is outside its window. The effective window is the
  // work's own hours OR its showing room's. Reads houseClock.now so a door lifts
  // live on the minute its window opens.
  function winOf(f: FigurineListItem) {
    return resolveWindow(f, showingRooms.list);
  }
  function doorShut(f: FigurineListItem): boolean {
    const w = winOf(f);
    return isGated(w) && !isShowingOpen(w, houseClock.nowDate);
  }
  let doorLocale = $derived($lang === 'ru' ? 'ru-RU' : 'en-US');
  function doorHeadlineOf(f: FigurineListItem): string {
    return openingHeadline(winOf(f), $t, doorLocale, houseClock.nowDate);
  }

  type MainFilter = 'all' | 'available' | 'reserved' | 'sold' | 'saved' | 'viewed';
  type SortMode = 'curated' | 'available' | 'newest' | 'oldest' | 'name';

  let { data } = $props();
  let figurines = $derived(data.figurines);

  let searchQuery = $state('');
  let mainFilter = $state<MainFilter>('all');
  let sortMode = $state<SortMode>('curated');
  let yearFilter = $state('all');
  let techniqueFilter = $state('all');
  let seriesFilter = $state('all');
  let materialFilter = $state('all');

  const PAGE_SIZE = 12;
  let displayLimit = $state(PAGE_SIZE);

  // Declared up here (not next to its onMount loader) because the filter/count
  // $derived blocks below reference it. Under SSR/prerender deriveds evaluate
  // eagerly in source order, so a later `let` would be in the temporal dead zone.
  let viewedIds = $state(new Set<string>());

  // ── Derived filter data ────────────────────────────────────────
  type FigItem = import('$lib/types/api').FigurineListItem;

  let filtered = $derived(
    (figurines as FigItem[])
      .filter((f) => {
        if (mainFilter === 'saved')    return savedFigurines.has(f.id);
        if (mainFilter === 'viewed')   return viewedIds.has(f.id);
        if (mainFilter !== 'all')      return f.status === mainFilter;
        return true;
      })
      .filter((f) => yearFilter === 'all' || String(f.year ?? '') === yearFilter)
      .filter((f) => techniqueFilter === 'all' || f.technique === techniqueFilter)
      .filter((f) => seriesFilter === 'all' || f.series === seriesFilter)
      .filter((f) => materialFilter === 'all' || f.material === materialFilter)
      .filter((f) => {
        const query = searchQuery.trim().toLowerCase();
        if (!query) return true;
        return [f.name, f.year ? String(f.year) : '', f.technique ?? '', f.material ?? '', f.series ?? '']
          .some(v => v.toLowerCase().includes(query));
      })
  );

  let sorted = $derived(
    filtered.slice().sort((a, b) => {
      const byCurated = (a.sortOrder ?? 0) - (b.sortOrder ?? 0);
      const byName = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
      const byNewest = (b.year ?? -Infinity) - (a.year ?? -Infinity);
      const byOldest = (a.year ?? Infinity) - (b.year ?? Infinity);
      const statusRank: Record<string, number> = { available: 0, reserved: 1, sold: 2 };
      if (sortMode === 'available') return statusRank[a.status] - statusRank[b.status] || byCurated || byName;
      if (sortMode === 'newest') return byNewest || byCurated || byName;
      if (sortMode === 'oldest') return byOldest || byCurated || byName;
      if (sortMode === 'name') return byName || byCurated;
      return byCurated || byName;
    })
  );

  let visible = $derived(sorted.slice(0, displayLimit));
  let hasMore = $derived(sorted.length > displayLimit);

  let availableYears = $derived(
    [...new Set((figurines as FigItem[]).map((f) => f.year).filter((y): y is number => typeof y === 'number'))]
      .sort((a, b) => b - a)
  );

  let availableTechniques = $derived(
    [...new Set((figurines as FigItem[]).map((f) => f.technique).filter((t): t is string => Boolean(t?.trim())))]
      .sort((a, b) => a.localeCompare(b, undefined, { sensitivity: 'base' }))
  );

  let availableSeries = $derived(
    [...new Set((figurines as FigItem[]).map((f) => f.series).filter((s): s is string => Boolean(s?.trim())))]
      .sort((a, b) => a.localeCompare(b, undefined, { sensitivity: 'base' }))
  );

  let availableMaterials = $derived(
    [...new Set((figurines as FigItem[]).map((f) => f.material).filter((m): m is string => Boolean(m?.trim())))]
      .sort((a, b) => a.localeCompare(b, undefined, { sensitivity: 'base' }))
  );

  let statusCounts = $derived({
    all:       (figurines as FigItem[]).length,
    available: (figurines as FigItem[]).filter((f) => f.status === 'available').length,
    reserved:  (figurines as FigItem[]).filter((f) => f.status === 'reserved').length,
    sold:      (figurines as FigItem[]).filter((f) => f.status === 'sold').length,
  });

  let savedCount  = $derived((figurines as FigItem[]).filter((f) => savedFigurines.has(f.id)).length);
  let viewedCount = $derived((figurines as FigItem[]).filter((f) => viewedIds.has(f.id)).length);

  let hasActiveFilters = $derived(
    searchQuery.trim() !== '' || mainFilter !== 'all' ||
    sortMode !== 'curated' || yearFilter !== 'all' || techniqueFilter !== 'all' ||
    seriesFilter !== 'all' || materialFilter !== 'all'
  );

  // ── Popover option lists (single chip vocabulary, no native <select>) ──────
  let sortOptions = $derived([
    { value: 'curated',   label: $t('archiveSortCurated') },
    { value: 'available', label: $t('archiveSortAvailable') },
    { value: 'newest',    label: $t('archiveSortNewest') },
    { value: 'oldest',    label: $t('archiveSortOldest') },
    { value: 'name',      label: $t('archiveSortName') },
  ]);

  const countBy = (key: 'technique' | 'material' | 'series', val: string) =>
    (figurines as FigItem[]).filter((f) => (f[key] ?? '') === val).length;

  let techniqueOptions = $derived(
    availableTechniques.map((t) => ({ value: t, label: t, count: countBy('technique', t) }))
  );
  let materialOptions = $derived(
    availableMaterials.map((m) => ({ value: m, label: m, count: countBy('material', m) }))
  );
  let seriesOptions = $derived(
    availableSeries.map((s) => ({ value: s, label: s, count: countBy('series', s) }))
  );

  // Secondary axes only exist if there is more than one value to choose between.
  let hasRefineAxes = $derived(
    availableYears.length > 0 || techniqueOptions.length > 0 ||
    seriesOptions.length > 0 || materialOptions.length > 0
  );
  let secondaryActive = $derived(
    yearFilter !== 'all' || techniqueFilter !== 'all' ||
    seriesFilter !== 'all' || materialFilter !== 'all'
  );
  let refineOpen = $state(false);
  // Keep refinements visible whenever one is active (e.g. restored on back-nav).
  $effect(() => { if (secondaryActive) refineOpen = true; });

  $effect(() => {
    void mainFilter; void searchQuery; void sortMode; void yearFilter; void techniqueFilter;
    void seriesFilter; void materialFilter;
    displayLimit = PAGE_SIZE;
  });

  function loadMore() {
    displayLimit += PAGE_SIZE;
  }

  let countText = $derived(() => {
    const total = (figurines as FigItem[]).length;
    const shown = filtered.length;
    if (total === 0) return $t('archiveEmpty');
    if (mainFilter !== 'all' || searchQuery.trim()) return `${shown} / ${toRoman(total)}`;
    return toRoman(total);
  });

  function toRoman(num: number): string {
    const lookup: Record<string, number> = {M:1000,CM:900,D:500,CD:400,C:100,XC:90,L:50,XL:40,X:10,IX:9,V:5,IV:4,I:1};
    let roman = '', i;
    for (i in lookup) { while (num >= lookup[i]) { roman += i; num -= lookup[i]; } }
    return roman;
  }

  function clearFilters() {
    searchQuery = ''; mainFilter = 'all'; sortMode = 'curated';
    yearFilter = 'all'; techniqueFilter = 'all'; seriesFilter = 'all'; materialFilter = 'all';
  }

  // ── Card actions ─────────────────────────────────────────────────────────
  let lightboxFig = $state<FigurineListItem | null>(null);
  let orderFig = $state<FigurineListItem | null>(null);
  let shareCopiedId = $state('');

  onMount(() => {
    savedFigurines.load();
    houseClock.start();
    showingRooms.load();
    try {
      const viewed: string[] = JSON.parse(localStorage.getItem('gotiga_viewed') ?? '[]');
      viewedIds = new Set(viewed);
    } catch {}
  });

  let justSavedId = $state('');
  function toggleLike(e: MouseEvent, id: string) {
    e.preventDefault();
    e.stopPropagation();
    savedFigurines.toggle(id);
    if (savedFigurines.has(id)) {
      justSavedId = id;
      setTimeout(() => { if (justSavedId === id) justSavedId = ''; }, 650);
    }
  }

  function isNewArrival(f: FigurineListItem): boolean {
    if (!f.createdAt) return false;
    return Date.now() - new Date(f.createdAt).getTime() < 21 * 86400_000;
  }

  // Scroll-reveal: same live IntersectionObserver used by the home gallery's
  // "rise" card-fx — a card climbs into place as it crosses ~65% up into view.
  function revealOnEnter(node: HTMLElement) {
    if (typeof IntersectionObserver === 'undefined') { node.classList.add('fx-revealed'); return; }

    const reveal = () => node.classList.add('fx-revealed');

    // A plate never hides again once it has climbed into place: the reveal is a
    // one-way door. Anything already inside the viewport is shown right away —
    // otherwise a first callback that lands before the images give the row its
    // height would leave the whole grid at opacity 0 until something nudges it.
    const io = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) { reveal(); io.disconnect(); }
    }, { rootMargin: '0px 0px -12% 0px', threshold: 0 });
    io.observe(node);

    // Safety net: nothing above the fold may stay invisible.
    const t = setTimeout(() => {
      if (node.getBoundingClientRect().top < window.innerHeight) { reveal(); io.disconnect(); }
    }, 400);

    return { destroy() { clearTimeout(t); io.disconnect(); } };
  }

  // ── 3D tilt ────────────────────────────────────────────────────
  function onTiltMove(e: MouseEvent) {
    const el = e.currentTarget as HTMLElement;
    const r  = el.getBoundingClientRect();
    const x  = (e.clientX - r.left)  / r.width  - 0.5;
    const y  = (e.clientY - r.top)   / r.height - 0.5;
    el.style.transition = 'none';
    el.style.transform  = `perspective(900px) rotateX(${-y * 7}deg) rotateY(${x * 7}deg) translateZ(4px)`;
  }

  function onTiltLeave(e: MouseEvent) {
    const el = e.currentTarget as HTMLElement;
    el.style.transition = 'transform 0.55s cubic-bezier(0.16,1,0.3,1)';
    el.style.transform  = '';
  }

  function markViewed(id: string) {
    if (viewedIds.has(id)) return;
    const next = new Set(viewedIds);
    next.add(id);
    viewedIds = next;
    try { localStorage.setItem('gotiga_viewed', JSON.stringify([...next])); } catch {}
  }

  function openQuickView(e: MouseEvent, fig: FigurineListItem) {
    e.preventDefault();
    e.stopPropagation();
    if (fig.faceImageUrl) { lightboxFig = fig; markViewed(fig.id); }
  }

  async function handleShare(e: MouseEvent, fig: FigurineListItem) {
    e.preventDefault();
    e.stopPropagation();
    const url = `${window.location.origin}${figurineHref(fig)}`;
    try {
      if (navigator.share) {
        await navigator.share({ title: fig.name, url });
      } else {
        await navigator.clipboard.writeText(url);
        shareCopiedId = fig.id;
        setTimeout(() => { shareCopiedId = ''; }, 2000);
      }
    } catch {}
  }

  function openOrder(e: MouseEvent, fig: FigurineListItem) {
    e.preventDefault();
    e.stopPropagation();
    orderFig = fig;
  }

  function openSimilarCommission(e: MouseEvent, fig: FigurineListItem) {
    e.preventDefault();
    e.stopPropagation();
    markViewed(fig.id);
    goto(`/commission?source=${encodeURIComponent(fig.id)}`);
  }

  // ── Scroll + filter state restoration ────────────────────────────────────
  const SCROLL_KEY  = 'figurines-scroll';
  const FILTER_KEY  = 'figurines-filters';

  beforeNavigate(({ to }) => {
    if (to?.url.pathname.startsWith('/figurines/')) {
      sessionStorage.setItem(SCROLL_KEY, String(window.scrollY));
      sessionStorage.setItem(FILTER_KEY, JSON.stringify({
        searchQuery, mainFilter, sortMode,
        yearFilter, techniqueFilter, seriesFilter, materialFilter,
        displayLimit,
      }));
    }
  });

  afterNavigate(({ type }) => {
    if (type === 'popstate') {
      const savedScroll = sessionStorage.getItem(SCROLL_KEY);
      if (savedScroll) {
        requestAnimationFrame(() => window.scrollTo({ top: parseInt(savedScroll), behavior: 'instant' }));
        sessionStorage.removeItem(SCROLL_KEY);
      }
      try {
        const raw = sessionStorage.getItem(FILTER_KEY);
        if (raw) {
          const f = JSON.parse(raw);
          searchQuery    = f.searchQuery    ?? '';
          mainFilter     = f.mainFilter     ?? 'all';
          sortMode       = f.sortMode       ?? 'curated';
          yearFilter     = f.yearFilter     ?? 'all';
          techniqueFilter= f.techniqueFilter?? 'all';
          seriesFilter   = f.seriesFilter   ?? 'all';
          materialFilter = f.materialFilter ?? 'all';
          displayLimit   = f.displayLimit   ?? PAGE_SIZE;
          sessionStorage.removeItem(FILTER_KEY);
        }
      } catch { /* ignore */ }
    }
  });
</script>

<svelte:head>
  <title>Archive — Gothic Miniatures Collection — {$brandName}</title>
  <meta name="description" content={$t('archiveMetaDescription')} />
  <meta property="og:site_name" content={$brandName} />
  <meta property="og:locale" content="en_US" />
  <meta property="og:type" content="website" />
  <meta property="og:title" content="Archive — Gothic Miniatures Collection — {$brandName}" />
  <meta property="og:description" content={$t('archiveMetaDescription')} />
  <meta property="og:image" content="{SITE_URL}/images/cabinet-bg.jpeg" />
  <meta property="og:url" content="{SITE_URL}/figurines" />
  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:title" content="Archive — Gothic Miniatures Collection — {$brandName}" />
  <meta name="twitter:image" content="{SITE_URL}/images/cabinet-bg.jpeg" />
  {@html `<script type="application/ld+json">${JSON.stringify({ '@context': 'https://schema.org', '@type': 'BreadcrumbList', itemListElement: [ { '@type': 'ListItem', position: 1, name: $brandName, item: SITE_URL }, { '@type': 'ListItem', position: 2, name: 'Archive', item: `${SITE_URL}/figurines` } ] })}<\/script>`}
  <!-- Fonts loaded once globally in app.html -->
</svelte:head>

<div class="fixed inset-0 bg-[#f8f1e7] -z-50"></div>
<div class="fixed inset-0 pointer-events-none z-0 bg-noise opacity-[0.07] mix-blend-overlay"></div>
<div class="fixed inset-0 pointer-events-none z-0 bg-[radial-gradient(circle_at_50%_50%,transparent_0%,#f8f1e7_90%)]"></div>

<div class="min-h-screen relative z-10 overflow-hidden font-['Inter'] text-[#34251c]">

    <div class="container mx-auto px-6 sm:px-12 py-12">
      <!-- ── FILTER BAR ─────────────────────────────────────────────── -->
      <div class="filter-bar" in:fade={{ duration: 1000 }}>

        <!-- Row 1: Title + count -->
        <div class="filter-bar__head">
          <div>
            <a href="/" class="filter-bar__back">{$t('archiveBackLink')}</a>
            <h1 class="filter-bar__title">{$t('archivePageTitle')}</h1>
          </div>
          <div class="filter-bar__count">
            <p class="filter-bar__count-label">{$t('archiveStatusCount')}</p>
            <p class="filter-bar__count-value">{countText()}</p>
          </div>
        </div>

        <!-- Row 2: Search + Sort -->
        <div class="filter-bar__search-row">
          <label class="filter-bar__search-wrap">
            <svg class="filter-bar__search-icon" width="13" height="13" viewBox="0 0 13 13" fill="none" aria-hidden="true">
              <circle cx="5.5" cy="5.5" r="4" stroke="currentColor" stroke-width="1"/>
              <path d="M9 9L12 12" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
            </svg>
            <input
              bind:value={searchQuery}
              type="search"
              placeholder={$t('archiveSearchPlaceholder')}
              class="filter-bar__search-input"
              aria-label={$t('archiveSearchPlaceholder')}
            />
            {#if searchQuery}
              <button
                type="button"
                onclick={() => searchQuery = ''}
                class="filter-bar__search-clear"
                aria-label={$t('archiveClearFilters')}
              >
                <svg width="9" height="9" viewBox="0 0 9 9" fill="none" aria-hidden="true">
                  <path d="M1 1l7 7M8 1L1 8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                </svg>
              </button>
            {/if}
          </label>

          <!-- Sort — same chip vocabulary, not a native select -->
          <FilterPopover
            label={$t('archiveSortLabel')}
            bind:value={sortMode}
            options={sortOptions}
            allValue="curated"
            allLabel={$t('archiveSortCurated')}
          />
        </div>

        <!-- Row 3: Primary filter chips (status + saved + viewed) -->
        <div class="filter-chips" role="group" aria-label={$t('archivePrimaryFilters')}>

          <!-- All -->
          <button
            class="fchip {mainFilter === 'all' ? 'fchip--active-default' : 'fchip--off'}"
            onclick={() => mainFilter = 'all'}
            aria-pressed={mainFilter === 'all'}
          >
            {$t('archiveStatusAll')}
            <span class="fchip__count">{statusCounts.all}</span>
          </button>

          <!-- Available -->
          {#if statusCounts.available > 0}
          <button
            class="fchip {mainFilter === 'available' ? 'fchip--active-avail' : 'fchip--off'}"
            onclick={() => mainFilter = mainFilter === 'available' ? 'all' : 'available'}
            aria-pressed={mainFilter === 'available'}
          >
            <span class="fchip__dot fchip__dot--avail"></span>
            {$t('archiveStatusAvailable')}
            <span class="fchip__count">{statusCounts.available}</span>
          </button>
          {/if}

          <!-- Reserved -->
          {#if statusCounts.reserved > 0}
          <button
            class="fchip {mainFilter === 'reserved' ? 'fchip--active-res' : 'fchip--off'}"
            onclick={() => mainFilter = mainFilter === 'reserved' ? 'all' : 'reserved'}
            aria-pressed={mainFilter === 'reserved'}
          >
            <span class="fchip__dot fchip__dot--res"></span>
            {$t('archiveStatusReserved')}
            <span class="fchip__count">{statusCounts.reserved}</span>
          </button>
          {/if}

          <!-- Sold -->
          {#if statusCounts.sold > 0}
          <button
            class="fchip {mainFilter === 'sold' ? 'fchip--active-sold' : 'fchip--off'}"
            onclick={() => mainFilter = mainFilter === 'sold' ? 'all' : 'sold'}
            aria-pressed={mainFilter === 'sold'}
          >
            <span class="fchip__dot fchip__dot--sold"></span>
            {$t('archiveStatusSold')}
            <span class="fchip__count">{statusCounts.sold}</span>
          </button>
          {/if}

          <!-- Divider always before special chips -->
          <span class="fchip-sep" aria-hidden="true"></span>

          <!-- Saved (♡) — always visible, disabled when empty -->
          <button
            class="fchip {mainFilter === 'saved' ? 'fchip--active-saved' : savedCount > 0 ? 'fchip--off' : 'fchip--empty'}"
            onclick={() => { if (savedCount > 0) mainFilter = mainFilter === 'saved' ? 'all' : 'saved'; }}
            aria-pressed={mainFilter === 'saved'}
            disabled={savedCount === 0}
            title={savedCount === 0 ? 'Press ♡ on any card to save' : undefined}
          >
            <svg width="11" height="11" viewBox="0 0 14 14" fill="none" aria-hidden="true">
              <path
                d="M7 12.5C7 12.5 1 8.5 1 4.5C1 2.5 2.5 1 4.5 1C5.5 1 6.5 1.8 7 3C7.5 1.8 8.5 1 9.5 1C11.5 1 13 2.5 13 4.5C13 8.5 7 12.5 7 12.5Z"
                fill={mainFilter === 'saved' ? 'currentColor' : 'none'}
                stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"
              />
            </svg>
            {$t('archiveFilterSaved')}
            <span class="fchip__count">{savedCount}</span>
          </button>

          <!-- Viewed (👁) — always visible, disabled when empty -->
          <button
            class="fchip {mainFilter === 'viewed' ? 'fchip--active-viewed' : viewedCount > 0 ? 'fchip--off' : 'fchip--empty'}"
            onclick={() => { if (viewedCount > 0) mainFilter = mainFilter === 'viewed' ? 'all' : 'viewed'; }}
            aria-pressed={mainFilter === 'viewed'}
            disabled={viewedCount === 0}
            title={viewedCount === 0 ? 'Open figurines to track viewed' : undefined}
          >
            <svg width="12" height="11" viewBox="0 0 14 14" fill="none" aria-hidden="true">
              <path d="M1 7C1 7 3.5 3 7 3C10.5 3 13 7 13 7C13 7 10.5 11 7 11C3.5 11 1 7 1 7Z" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round"/>
              <circle cx="7" cy="7" r="2" stroke="currentColor" stroke-width="1.1"/>
            </svg>
            {$t('archiveFilterViewed')}
            <span class="fchip__count">{viewedCount}</span>
          </button>
        </div>

        <!-- Row 4: Refine — secondary axes folded as marginalia, calm by default -->
        {#if hasRefineAxes || hasActiveFilters}
        <div class="filter-refine">
          <div class="filter-refine__head">
            {#if hasRefineAxes}
            <button
              type="button"
              class="refine-toggle {refineOpen ? 'refine-toggle--open' : ''}"
              onclick={() => refineOpen = !refineOpen}
              aria-expanded={refineOpen}
            >
              <span class="refine-toggle__mark">{refineOpen ? '–' : '+'}</span>
              {refineOpen ? $t('archiveRefineHide') : $t('archiveRefine')}
            </button>
            {/if}

            {#if hasActiveFilters}
            <button type="button" onclick={clearFilters} class="filter-clear">
              <svg width="9" height="9" viewBox="0 0 10 10" fill="none" aria-hidden="true">
                <path d="M1 5h8M5 1v8" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" transform="rotate(45 5 5)"/>
              </svg>
              {$t('archiveClearFilters')}
            </button>
            {/if}
          </div>

          {#if refineOpen && hasRefineAxes}
          <div class="filter-secondary" transition:slide={{ duration: 260 }}>

            <!-- Year chips -->
            {#if availableYears.length > 0}
            <div class="filter-secondary__years">
              {#each availableYears as year}
              <button
                class="fchip fchip--sm {yearFilter === String(year) ? 'fchip--active-default' : 'fchip--off'}"
                onclick={() => yearFilter = yearFilter === String(year) ? 'all' : String(year)}
                aria-pressed={yearFilter === String(year)}
              >{year}</button>
              {/each}
            </div>
            {/if}

            {#if availableYears.length > 0 && (techniqueOptions.length > 0 || seriesOptions.length > 0 || materialOptions.length > 0)}
              <span class="filter-secondary__sep" aria-hidden="true"></span>
            {/if}

            <!-- Technique -->
            {#if techniqueOptions.length > 0}
            <FilterPopover
              label={$t('archiveTechniqueLabel')}
              bind:value={techniqueFilter}
              options={techniqueOptions}
              allLabel={$t('archiveTechniqueAll')}
            />
            {/if}

            <!-- Series -->
            {#if seriesOptions.length > 0}
            <FilterPopover
              label={$t('archiveSeriesLabel')}
              bind:value={seriesFilter}
              options={seriesOptions}
              allLabel={$t('archiveSeriesAll')}
            />
            {/if}

            <!-- Material -->
            {#if materialOptions.length > 0}
            <FilterPopover
              label={$t('archiveMaterialLabel')}
              bind:value={materialFilter}
              options={materialOptions}
              allLabel={$t('archiveMaterialAll')}
            />
            {/if}
          </div>
          {/if}
        </div>
        {/if}

      </div>

      {#if filtered.length > 0}
        <ul class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-x-8 gap-y-10">
          {#each visible as figurine, i (figurine.id)}
            <li class="group perspective-container fig-tile" use:revealOnEnter
                onmousemove={onTiltMove} onmouseleave={onTiltLeave}>
              <a
                href={doorShut(figurine) ? undefined : figurineHref(figurine)}
                class="block w-full text-left relative focus:outline-none"
                aria-label="{figurine.name}"
                data-sveltekit-preload-data="hover"
                onclick={(e) => { if (doorShut(figurine)) { e.preventDefault(); return; } markViewed(figurine.id); }}
              >
                <div
                  class="fig-media relative aspect-[3/4] transition-shadow duration-500"
                  style={doorShut(figurine) ? '' : `view-transition-name: figurine-${figurine.id}`}
                >

                  {#if doorShut(figurine)}
                    <!-- The sealed-door plate is its own bespoke design (eyebrow,
                         wax seal, schedule) — left unmasked so its edge text
                         doesn't fade with the "impressed figure" mask below. -->
                    <SealedDoor
                      openFromMin={winOf(figurine).openFromMin}
                      openUntilMin={winOf(figurine).openUntilMin}
                      daysMask={winOf(figurine).daysMask}
                      monthDay={winOf(figurine).monthDay}
                      dateFrom={winOf(figurine).dateFrom}
                      dateUntil={winOf(figurine).dateUntil}
                      imageUrl={figurine.faceImageUrl}
                      thumbUrl={figurine.thumbUrl}
                      name={figurine.name}
                      showSchedule={false}
                    />
                  {:else}
                    <!-- The figure itself, masked to a soft irregular fade + carved
                         drop-shadow — reads as pressed INTO the parchment rather
                         than a photo pasted on top of it. Badges/plaque/buttons
                         stay outside this wrapper so they stay crisp on top. -->
                    <div class="fig-photo">
                    {#if figurine.faceImageUrl}
                      <!-- Dense archive grid: two cards across on a phone, up to four on a
                           wide screen. Told to the browser so it can pick the 420px thumb
                           here rather than a rendition sized for a full-width hero. -->
                      <AppImage
                              src={figurine.faceImageUrl}
                              thumbUrl={figurine.thumbUrl}
                              alt={figurine.name}
                              class="fig-img-main w-full h-full object-cover transition-all duration-700 ease-out group-hover:scale-105 fig-img--{figurine.status}"
                              loading="lazy"
                              sizes="(max-width: 680px) 50vw, (max-width: 1024px) 33vw, 25vw"
                      />
                      {#if figurine.detailImageUrl}
                        <!-- a second angle, held in reserve for a lingering look -->
                        <AppImage
                          src={figurine.detailImageUrl}
                          alt=""
                          class="fig-img-alt"
                          loading="lazy"
                          sizes="(max-width: 680px) 50vw, (max-width: 1024px) 33vw, 25vw"
                        />
                      {/if}
                    {:else}
                      <div class="w-full h-full flex items-center justify-center opacity-20">
                        <span class="font-['Fraunces'] text-2xl text-[#34251c]">?</span>
                      </div>
                    {/if}
                    </div>
                  {/if}

                  {#if doorShut(figurine)}
                    <div class="fig-glass fig-glass-sealed">
                      <span class="fig-cap-meta">
                        <span class="fig-cap-status status-soon">{$t('posterSoon')}</span>
                        <span class="fig-cap-dot" aria-hidden="true">·</span>
                        <span class="fig-cap-soon">{doorHeadlineOf(figurine)}</span>
                      </span>
                      <h2 class="fig-cap-name"><span class="fig-cap-name-text">{figurine.name}</span></h2>
                    </div>
                  {/if}

                  <div class="absolute inset-0 bg-[radial-gradient(circle_at_center,transparent_0%,rgba(111,59,36,0.8)_100%)] pointer-events-none transition-opacity duration-500 fig-vignette--{figurine.status}"></div>

                  <div class="absolute top-2 left-2 w-4 h-4 border-t border-l border-[#34251c]/20 group-hover:border-[#34251c]/60 transition-colors pointer-events-none"></div>
                  <div class="absolute bottom-2 right-2 w-4 h-4 border-b border-r border-[#34251c]/20 group-hover:border-[#34251c]/60 transition-colors pointer-events-none"></div>

                  <!-- Badge stack — top-left: house-favorite medal, then "new" wax seal -->
                  <div class="fig-badges">
                    {#if figurine.houseFavorite}
                      <span class="fig-favorite" title={$t('houseFavoriteBadge')}>
                        <svg width="11" height="11" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                          <path d="M2 12C2 7 4 3 4 3M2 12L4 9.5M2 12L4.5 11" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
                          <path d="M12 12C12 7 10 3 10 3M12 12L10 9.5M12 12L9.5 11" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
                          <circle cx="7" cy="3" r="1.1" fill="currentColor"/>
                        </svg>
                        {$t('houseFavoriteBadge')}
                      </span>
                    {/if}
                    {#if isNewArrival(figurine)}
                      <span class="fig-seal">{$t('archiveCardNew')}</span>
                    {/if}
                  </div>

                  <!-- Heart button — always visible, top-right -->
                  <button
                    class="absolute top-3 right-3 z-10 flex items-center justify-center w-[30px] h-[30px] rounded-full backdrop-blur-sm cursor-pointer transition-all duration-250
                      {savedFigurines.has(figurine.id)
                        ? 'bg-[rgba(198,95,60,0.12)] border border-[rgba(198,95,60,0.32)] text-[#c65f3c]'
                        : 'bg-[rgba(255,249,240,0.65)] border border-[rgba(52,37,28,0.13)] text-[rgba(95,70,54,0.55)] hover:bg-[rgba(255,249,240,0.92)] hover:text-[#c65f3c] hover:border-[rgba(198,95,60,0.25)]'}"
                    class:fig-just-saved={justSavedId === figurine.id}
                    onclick={(e) => toggleLike(e, figurine.id)}
                    aria-label={savedFigurines.has(figurine.id) ? $t('cardSaved') : $t('cardSave')}
                    title={savedFigurines.has(figurine.id) ? $t('cardSaved') : $t('cardSave')}
                  >
                    <svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                      <path
                        d="M7 12.5C7 12.5 1 8.5 1 4.5C1 2.5 2.5 1 4.5 1C5.5 1 6.5 1.8 7 3C7.5 1.8 8.5 1 9.5 1C11.5 1 13 2.5 13 4.5C13 8.5 7 12.5 7 12.5Z"
                        fill={savedFigurines.has(figurine.id) ? 'currentColor' : 'none'}
                        stroke="currentColor"
                        stroke-width="1.1"
                        stroke-linejoin="round"
                      />
                    </svg>
                  </button>

                  {#if !doorShut(figurine)}
                  <!-- Frosted glass caption plaque — status · year · roman + hover-arrow -->
                  <div class="fig-glass">
                    <span class="fig-cap-meta">
                      <span class="fig-cap-status status-{figurine.status}">
                        {figurine.status === 'available' ? $t('archiveStatusAvailableLabel') : figurine.status === 'reserved' ? $t('archiveStatusReservedLabel') : figurine.status === 'in_progress' ? $t('profileWishInProgress') : $t('archiveStatusSoldLabel')}
                      </span>
                      {#if figurine.year}
                        <span class="fig-cap-dot" aria-hidden="true">·</span>
                        <span class="fig-cap-year">{figurine.year}</span>
                      {/if}
                      <span class="fig-cap-roman">{toRoman(figurine.sortOrder ?? i + 1)}</span>
                    </span>
                    <h2 class="fig-cap-name">
                      <span class="fig-cap-name-text">{figurine.name}</span>
                      <svg class="fig-cap-arrow" width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                        <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                      </svg>
                    </h2>
                  </div>
                  {/if}

                  <!-- Slide-up action bar — appears on hover (and stays open on touch, see <style>) -->
                  <div class="card-actions-bar absolute bottom-0 left-0 right-0 z-10 flex items-center justify-between px-3.5 py-2.5
                              bg-gradient-to-t from-[rgba(44,23,16,0.72)] to-transparent
                              translate-y-full group-hover:translate-y-0
                              transition-transform duration-500 ease-[cubic-bezier(0.16,1,0.3,1)]">

                    <span class="text-[rgba(255,249,240,0.62)] text-[8.5px] tracking-[0.16em] uppercase select-none">
                      {$t('cardActions')}
                    </span>

                    <div class="flex items-center gap-1.5">
                      <!-- Quick View -->
                      {#if figurine.faceImageUrl && !doorShut(figurine)}
                      <button
                        class="flex items-center justify-center w-7 h-7 rounded-full
                               bg-[rgba(255,249,240,0.11)] border border-[rgba(255,249,240,0.20)]
                               text-[rgba(255,249,240,0.62)] hover:text-white hover:bg-[rgba(255,249,240,0.22)] hover:border-[rgba(255,249,240,0.38)]
                               cursor-pointer transition-all duration-200
                               translate-y-1 opacity-0 group-hover:translate-y-0 group-hover:opacity-100
                               [transition-delay:50ms]"
                        onclick={(e) => openQuickView(e, figurine)}
                        title={$t('cardQuickView')}
                        aria-label={$t('cardQuickView')}
                      >
                        <svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                          <path d="M1 7C1 7 3.5 3 7 3C10.5 3 13 7 13 7C13 7 10.5 11 7 11C3.5 11 1 7 1 7Z" stroke="currentColor" stroke-width="1" stroke-linejoin="round"/>
                          <circle cx="7" cy="7" r="2" stroke="currentColor" stroke-width="1"/>
                        </svg>
                      </button>
                      {/if}

                      <!-- Share -->
                      <button
                        class="flex items-center justify-center w-7 h-7 rounded-full
                               bg-[rgba(255,249,240,0.11)] border border-[rgba(255,249,240,0.20)]
                               cursor-pointer transition-all duration-200
                               translate-y-1 opacity-0 group-hover:translate-y-0 group-hover:opacity-100
                               [transition-delay:90ms]
                               {shareCopiedId === figurine.id
                                 ? 'text-[rgba(130,210,130,0.9)] border-[rgba(100,180,100,0.4)]'
                                 : 'text-[rgba(255,249,240,0.62)] hover:text-white hover:bg-[rgba(255,249,240,0.22)] hover:border-[rgba(255,249,240,0.38)]'}"
                        onclick={(e) => handleShare(e, figurine)}
                        title={shareCopiedId === figurine.id ? $t('cardLinkCopied') : $t('cardShare')}
                        aria-label={$t('cardShare')}
                      >
                        {#if shareCopiedId === figurine.id}
                        <svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                          <path d="M2 7L5.5 10.5L12 3.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                        {:else}
                        <svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                          <path d="M9 2H12V5" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
                          <path d="M12 2L7 7" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
                          <path d="M6 3H3C2.4 3 2 3.4 2 4V11C2 11.6 2.4 12 3 12H10C10.6 12 11 11.6 11 11V8" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
                        </svg>
                        {/if}
                      </button>

                      {#if figurine.status === 'available'}
                        <!-- Artifact Request -->
                        <button
                          class="flex items-center justify-center w-7 h-7 rounded-full
                                 bg-[rgba(198,95,60,0.18)] border border-[rgba(198,95,60,0.32)]
                                 text-[rgba(255,200,170,0.88)] hover:bg-[rgba(198,95,60,0.38)] hover:border-[rgba(198,95,60,0.55)] hover:text-[rgba(255,224,208,1)]
                                 cursor-pointer transition-all duration-200
                                 translate-y-1 opacity-0 group-hover:translate-y-0 group-hover:opacity-100
                                 [transition-delay:130ms]"
                          onclick={(e) => openOrder(e, figurine)}
                          title={$t('cardRequest')}
                          aria-label={$t('cardRequest')}
                        >
                          <svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                            <rect x="2" y="4" width="10" height="7" rx="0.5" stroke="currentColor" stroke-width="1"/>
                            <path d="M2 5L7 8.5L12 5" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
                          </svg>
                        </button>
                      {:else}
                        <!-- Similar commission -->
                        <button
                          class="flex items-center justify-center w-7 h-7 rounded-full
                                 bg-[rgba(198,95,60,0.18)] border border-[rgba(198,95,60,0.32)]
                                 text-[rgba(255,200,170,0.88)] hover:bg-[rgba(198,95,60,0.38)] hover:border-[rgba(198,95,60,0.55)] hover:text-[rgba(255,224,208,1)]
                                 cursor-pointer transition-all duration-200
                                 translate-y-1 opacity-0 group-hover:translate-y-0 group-hover:opacity-100
                                 [transition-delay:130ms]"
                          onclick={(e) => openSimilarCommission(e, figurine)}
                          title={$t('commissionCreateSimilarCta')}
                          aria-label={$t('commissionCreateSimilarCta')}
                        >
                          <svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                            <path d="M3 7.5C3 4.8 5.1 2.8 7.8 2.8H11" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
                            <path d="M8.5 1.4L11 2.8L8.5 4.4" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
                            <path d="M11 6.5C11 9.2 8.9 11.2 6.2 11.2H3" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
                            <path d="M5.5 9.6L3 11.2L5.5 12.6" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
                          </svg>
                        </button>
                      {/if}
                    </div>
                  </div>
                </div>
              </a>
            </li>
          {/each}
        </ul>

        {#if hasMore}
          <div class="mt-20 flex justify-center" in:fade>
            <button
              onclick={loadMore}
              class="group flex items-center gap-4 px-10 py-4 border border-[#34251c]/20 hover:border-[#34251c]/50 text-[#5f4636] hover:text-[#34251c] font-['Inter'] text-xs tracking-[0.08em] uppercase transition-all duration-500"
            >
              <span>{$t('archiveLoadMore')}</span>
              <span class="text-[#34251c]/30 group-hover:text-[#34251c]/85 transition-colors">{Math.min(PAGE_SIZE, filtered.length - displayLimit)}</span>
              <span class="transition-transform group-hover:translate-y-0.5">↓</span>
            </button>
          </div>
        {/if}
      {:else if data.loadError}
        <div class="flex flex-col items-center justify-center py-32 border border-dashed border-[#c65f3c]/25 rounded-lg" in:fade>
          <p class="font-['Fraunces'] text-3xl text-[#6f3b24] mb-2 opacity-85">{$t('loadErrorTitle')}</p>
          <p class="text-xs tracking-wide text-[#7c6554] uppercase max-w-md text-center mb-6">{$t('loadErrorHint')}</p>
          <button
            onclick={() => invalidateAll()}
            class="px-8 py-3 border border-[#34251c]/25 hover:border-[#34251c]/55 text-[#5f4636] hover:text-[#34251c] text-xs tracking-[0.08em] uppercase transition-all duration-500"
          >
            {$t('loadErrorRetry')}
          </button>
        </div>
      {:else if searchQuery}
        <div class="flex flex-col items-center justify-center py-32 border border-dashed border-[#34251c]/10 rounded-lg" in:fade>
          <p class="font-['Fraunces'] text-3xl text-[#5f4636] mb-2 opacity-75">{$t('archiveNotFound')}</p>
          <p class="text-xs tracking-wide text-[#7c6554] uppercase">{$t('archiveNoEntry')} «{searchQuery}»</p>
        </div>
      {:else}
        <div class="flex flex-col items-center justify-center py-32 border border-dashed border-[#34251c]/10 rounded-lg">
          <p class="font-['Fraunces'] text-3xl text-[#5f4636] mb-2 opacity-75">{$t('archiveEmpty')}</p>
          <p class="text-xs tracking-wide text-[#7c6554] uppercase">{$t('archiveEmptyHint')}</p>
        </div>
      {/if}

      <div class="h-32"></div>
    </div>
  </div>

{#if lightboxFig?.faceImageUrl}
  <Lightbox
    images={[{ url: lightboxFig.faceImageUrl!, alt: lightboxFig.name }]}
    onClose={() => { lightboxFig = null; }}
  />
{/if}

{#if orderFig}
  <OrderModal
    isOpen={!!orderFig}
    figurineName={orderFig.name}
    figurineId={orderFig.id}
    mode="request"
    onClose={() => { orderFig = null; }}
  />
{/if}

<style>
  @keyframes shimmer {
    100% { transform: translateX(200%); }
  }

  /* ── SCROLL-REVEAL: same live IntersectionObserver "rise" card-fx used by
     the home gallery — a plate climbs into place as it crosses into view. ── */
  .fig-tile {
    opacity: 0;
    transform: translateY(52px) rotate(-2.2deg) scale(0.95);
    transition: transform 0.8s cubic-bezier(0.22, 0.9, 0.3, 1.28), opacity 0.55s ease;
  }
  .fig-tile.fx-revealed {
    opacity: 1;
    transform: none;
  }

  /* ── SECOND ANGLE: cross-fades in over the face photo on a sustained hover ── */
  .fig-media :global(.fig-img-alt) {
    position: absolute;
    inset: 0;
    z-index: 1;
    width: 100%;
    height: 100%;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.55s ease;
  }
  .fig-media :global(.fig-img-alt .app-image-thumb),
  .fig-media :global(.fig-img-alt .app-image-main) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: cover;
  }
  .group:hover .fig-media :global(.fig-img-alt),
  .group:focus-within .fig-media :global(.fig-img-alt) {
    opacity: 1;
  }
  @media (hover: none) {
    .fig-media :global(.fig-img-alt) { display: none; }
  }

  /* ── GLASS PLATE: frosted caption plaque over the photo's foot ──────────── */
  .fig-glass {
    position: absolute;
    left: 10px;
    right: 10px;
    bottom: 46px;
    z-index: 3;
    display: grid;
    gap: 5px;
    padding: 12px 14px 11px;
    border-radius: 3px;
    background: linear-gradient(165deg, rgba(52,37,28,0.4), rgba(14,9,6,0.62));
    backdrop-filter: blur(14px) saturate(150%);
    -webkit-backdrop-filter: blur(14px) saturate(150%);
    border: 1px solid rgba(255,247,234,0.28);
    box-shadow:
        0 10px 26px rgba(12,7,4,0.28),
        inset 0 1px 0 rgba(255,255,255,0.2);
    pointer-events: none;
    transition: background 0.3s ease, border-color 0.3s ease;
  }
  .group:hover .fig-glass {
    background: linear-gradient(165deg, rgba(52,37,28,0.48), rgba(14,9,6,0.7));
    border-color: rgba(255,247,234,0.4);
  }

  .fig-cap-meta {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 7px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.14em;
    line-height: 1.2;
    text-transform: uppercase;
    color: rgba(255,247,234,0.86);
  }
  .fig-cap-dot { opacity: 0.5; }
  .fig-cap-status::before {
    content: '';
    display: inline-block;
    width: 5px;
    height: 5px;
    margin-right: 5px;
    border-radius: 50%;
    border: 1px solid currentColor;
    vertical-align: middle;
  }
  .fig-cap-status.status-available::before,
  .fig-cap-status.status-sold::before { background: currentColor; }
  .fig-cap-status.status-reserved::before,
  .fig-cap-status.status-in_progress::before { background: transparent; }
  .fig-cap-status.status-sold { color: rgba(255,247,234,0.6); }
  .fig-cap-status.status-sold::before { opacity: 0.4; }
  .fig-cap-status.status-soon { color: #f0b48c; }
  .fig-cap-status.status-soon::before { background: currentColor; }
  .fig-cap-soon {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.02em;
    text-transform: none;
    color: #f0b48c;
  }
  .fig-glass-sealed {
    background: linear-gradient(165deg, rgba(84,58,40,0.55), rgba(38,24,15,0.62));
    border-color: rgba(255,247,234,0.4);
    box-shadow:
        0 10px 26px rgba(12,7,4,0.4),
        inset 0 1px 0 rgba(255,255,255,0.24);
  }
  .fig-cap-year { font-variant-numeric: tabular-nums; }
  .fig-cap-roman {
    margin-left: auto;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: 14px;
    font-weight: 400;
    letter-spacing: 0.01em;
    text-transform: none;
    color: rgba(255,247,234,0.85);
  }
  .fig-cap-name {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin: 0;
    min-width: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(19px, 1.4vw, 26px);
    font-weight: 400;
    line-height: 0.98;
    color: #fdf5e8;
    text-shadow: 0 1px 10px rgba(12,7,4,0.5);
  }
  .fig-cap-name-text {
    display: -webkit-box;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .fig-cap-arrow {
    flex-shrink: 0;
    align-self: center;
    color: rgba(255,247,234,0.82);
    opacity: 0;
    transform: translateX(-4px);
    transition: opacity 0.28s ease, transform 0.28s ease;
  }
  .group:hover .fig-cap-arrow,
  .group:focus-within .fig-cap-arrow {
    opacity: 1;
    transform: none;
  }
  @media (hover: none) {
    .fig-cap-arrow { opacity: 1; transform: none; }
  }

  /* ── BADGES (top-left) ───────────────────────────────────────────────── */
  .fig-badges {
    position: absolute;
    left: 8px;
    top: 8px;
    z-index: 4;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    max-width: calc(100% - 74px);
  }
  .fig-seal {
    padding: 5px 9px;
    border-radius: 4px;
    background: linear-gradient(150deg, rgba(198,95,60,0.94), rgba(111,59,36,0.94));
    color: #fff7ea;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 8px;
    font-weight: 700;
    letter-spacing: 0.16em;
    line-height: 1;
    text-transform: uppercase;
    transform: rotate(-2deg);
  }
  .fig-favorite {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 9px 5px 7px;
    border-radius: 4px;
    background: rgba(52,37,28,0.94);
    color: #f6e6c8;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 8px;
    font-weight: 700;
    letter-spacing: 0.14em;
    line-height: 1;
    text-transform: uppercase;
  }

  /* ── Heart "just saved" pop ──────────────────────────────────────────── */
  .fig-just-saved {
    animation: fig-heart-pop 0.62s cubic-bezier(0.34,1.56,0.64,1);
  }
  @keyframes fig-heart-pop {
    0% { transform: scale(1); }
    35% { transform: scale(1.32); }
    60% { transform: scale(0.92); }
    100% { transform: scale(1); }
  }

  @media (prefers-reduced-motion: reduce) {
    .fig-tile,
    .fig-just-saved,
    .fig-cap-arrow {
      animation: none !important;
      transition: opacity 0.2s ease, color 0.2s ease, background 0.2s ease, border-color 0.2s ease !important;
    }
    .fig-tile { opacity: 1 !important; transform: none !important; }
    .fig-media :global(.fig-img-alt) { display: none !important; }
    .fig-photo { filter: none !important; }
  }

  /* ── IMPRESSED INTO THE PARCHMENT: the figure itself — not a card behind
     it — carries the depth. No rectangle, no plate: the photo's edges fade
     to nothing on a soft irregular mask (revealing the page's own parchment
     underneath, unbroken), and a dark, close drop-shadow gathers around
     whatever's left visible, as though it had been pressed down into the
     paper. Badges/plaque/buttons live outside this wrapper so they stay
     crisp, unmasked, sitting "above" the impression. */
  .fig-media {
    background: transparent;
  }
  .fig-photo {
    position: absolute;
    inset: 0;
    /* radial-gradient <size> percentages are the ellipse's RADIUS as a share
       of the box (so 50%/50% just reaches the edges) — the previous 72%/78%
       pushed the whole fade band past the box entirely, so nothing visibly
       faded. Radius must stay ≤50% for the fade to land inside the box. */
    -webkit-mask-image: radial-gradient(ellipse 50% 54% at 50% 46%, #000 52%, transparent 100%);
    mask-image: radial-gradient(ellipse 50% 54% at 50% 46%, #000 52%, transparent 100%);
    filter:
      drop-shadow(0 3px 5px rgba(20,13,9,0.4))
      drop-shadow(0 -1px 2px rgba(255,250,240,0.22))
      drop-shadow(2px 0 4px rgba(20,13,9,0.22));
    transition: filter 0.5s ease;
  }
  .group:hover .fig-photo,
  .group:focus-within .fig-photo {
    filter:
      drop-shadow(0 4px 7px rgba(20,13,9,0.48))
      drop-shadow(0 -1px 2px rgba(255,250,240,0.26))
      drop-shadow(2px 0 5px rgba(20,13,9,0.26));
  }

  /* ── STATUS-BASED IMAGE TREATMENT ──────────────────────────────── */

  /* Available: full colour, light vignette — это доступные работы, они должны звать */
  :global(.fig-img--available) { opacity: 1; filter: none; }
  :global(.group:hover .fig-img--available) { opacity: 1; filter: none; }
  :global(.fig-vignette--available) { opacity: 0.30; }
  :global(.group:hover .fig-vignette--available) { opacity: 0.18; }

  /* Reserved: лёгкий налёт — не мертво, но занято */
  :global(.fig-img--reserved) { opacity: 0.88; filter: grayscale(0.40) saturate(0.72); }
  :global(.group:hover .fig-img--reserved) { opacity: 1; filter: grayscale(0) saturate(1); }
  :global(.fig-vignette--reserved) { opacity: 0.52; }
  :global(.group:hover .fig-vignette--reserved) { opacity: 0.35; }

  /* Sold: полный цвет — это часть архива, не могила */
  :global(.fig-img--sold) { opacity: 1; filter: none; }
  :global(.group:hover .fig-img--sold) { opacity: 1; filter: none; }
  :global(.fig-vignette--sold) { opacity: 0.30; }
  :global(.group:hover .fig-vignette--sold) { opacity: 0.18; }

  /* In progress: приглушённее available, но не серое */
  :global(.fig-img--in_progress) { opacity: 0.82; filter: saturate(0.80); }
  :global(.group:hover .fig-img--in_progress) { opacity: 1; filter: saturate(1); }
  :global(.fig-vignette--in_progress) { opacity: 0.48; }
  :global(.group:hover .fig-vignette--in_progress) { opacity: 0.30; }

  /* Mobile: без ховера — показываем всё в цвете */
  @media (hover: none) {
    :global(.fig-img--available),
    :global(.fig-img--reserved),
    :global(.fig-img--sold),
    :global(.fig-img--in_progress) { opacity: 1; filter: none; }
    :global(.fig-vignette--available),
    :global(.fig-vignette--reserved),
    :global(.fig-vignette--in_progress) { opacity: 0.25; }
    :global(.fig-vignette--sold) { opacity: 0.50; }
  }

  /* Touch devices have no hover: keep the action bar (quick view / share / request)
     reachable instead of hidden off-screen. Scoped specificity beats the Tailwind utilities. */
  @media (hover: none) {
    .card-actions-bar {
      transform: none;
    }
    .card-actions-bar button {
      opacity: 1;
      transform: none;
    }
  }

  @media (pointer: coarse) {
    /* iOS Safari auto-zooms on any input with font-size < 16px */
    .filter-bar__search-input {
      font-size: 16px;
      letter-spacing: 0.04em;
      text-transform: none;
    }
    /* Larger filter chip tap targets */
    .fchip {
      padding: 9px 12px;
    }
    .fchip--sm {
      padding: 7px 10px;
    }
    /* Increase back-link hit area */
    .filter-bar__back {
      min-height: 44px;
      align-items: center;
    }
  }

  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }

  /* Skip rendering offscreen cards — major FCP improvement on long lists */
  :global(.perspective-container) {
    content-visibility: auto;
    contain-intrinsic-size: 0 380px;
  }

  /* ── FILTER BAR LAYOUT ─────────────────────────────────────────── */
  .filter-bar {
    margin-bottom: clamp(56px, 8vw, 96px);
    border-bottom: 1px solid rgba(52,37,28,0.10);
    padding-bottom: 28px;
  }

  .filter-bar__head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    margin-bottom: 28px;
  }

  .filter-bar__back {
    display: inline-flex;
    align-items: center;
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.60);
    text-decoration: none;
    margin-bottom: 14px;
    transition: color 0.22s, opacity 0.22s;
    opacity: 0.7;
  }
  .filter-bar__back:hover { color: #34251c; opacity: 1; }

  .filter-bar__title {
    font-family: 'Fraunces', serif;
    font-size: clamp(38px, 6vw, 72px);
    color: rgba(111,59,36,0.90);
    letter-spacing: 0.02em;
    line-height: 1;
    margin: 0;
  }

  .filter-bar__count {
    display: none;
    text-align: right;
  }
  @media (min-width: 640px) { .filter-bar__count { display: block; } }

  .filter-bar__count-label {
    font-size: 9px;
    letter-spacing: 0.10em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.65);
    margin-bottom: 4px;
  }

  .filter-bar__count-value {
    font-size: 18px;
    color: #34251c;
    border-left: 2px solid rgba(52,37,28,0.18);
    padding-left: 12px;
  }

  /* ── SEARCH + SORT ROW ─────────────────────────────────────────── */
  .filter-bar__search-row {
    display: flex;
    gap: 12px;
    align-items: center;
    margin-bottom: 18px;
  }

  .filter-bar__search-wrap {
    position: relative;
    flex: 1;
    max-width: 420px;
  }

  .filter-bar__search-icon {
    position: absolute;
    left: 10px;
    top: 50%;
    transform: translateY(-50%);
    color: rgba(95,70,54,0.45);
    pointer-events: none;
  }

  .filter-bar__search-input {
    width: 100%;
    background: transparent;
    border: 1px solid rgba(52,37,28,0.15);
    border-bottom-color: rgba(52,37,28,0.28);
    padding: 8px 32px 8px 30px;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #34251c;
    font-family: 'Inter', sans-serif;
    outline: none;
    transition: border-color 0.22s;
  }
  .filter-bar__search-input::placeholder { color: rgba(95,70,54,0.50); }
  .filter-bar__search-input:focus { border-color: rgba(52,37,28,0.45); }
  .filter-bar__search-input::-webkit-search-cancel-button { display: none; }

  .filter-bar__search-clear {
    position: absolute;
    right: 4px;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    color: rgba(95,70,54,0.50);
    background: none;
    border: none;
    cursor: pointer;
    transition: color 0.18s;
  }
  .filter-bar__search-clear:hover { color: #34251c; }

  /* ── PRIMARY FILTER CHIPS ──────────────────────────────────────── */
  .filter-chips {
    display: flex;
    align-items: center;
    gap: 6px;
    overflow-x: auto;
    padding-bottom: 4px;
    margin-bottom: 14px;
    scrollbar-width: none;
  }
  .filter-chips::-webkit-scrollbar { display: none; }

  .fchip-sep {
    width: 1px;
    height: 14px;
    background: rgba(52,37,28,0.14);
    flex-shrink: 0;
    margin: 0 2px;
  }

  /* ── CHIP BASE ─────────────────────────────────────────────────── */
  .fchip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 11px;
    font-size: 9.5px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    border: 1px solid;
    white-space: nowrap;
    user-select: none;
    flex-shrink: 0;
    cursor: pointer;
    background: transparent;
    font-family: 'Inter', sans-serif;
    line-height: 1;
    transition: border-color 0.18s, color 0.18s, background 0.18s;
  }

  .fchip--sm {
    padding: 4px 9px;
    font-size: 9px;
    letter-spacing: 0.10em;
  }

  .fchip--off {
    border-color: rgba(52,37,28,0.13);
    color: rgba(95,70,54,0.68);
  }
  .fchip--off:hover {
    border-color: rgba(52,37,28,0.30);
    color: rgba(52,37,28,0.85);
  }

  /* Empty / not-yet-populated special chips */
  .fchip--empty {
    border-color: rgba(52,37,28,0.08);
    color: rgba(95,70,54,0.30);
    cursor: default;
  }

  .fchip--active-default {
    border-color: rgba(52,37,28,0.52);
    color: #34251c;
    background: rgba(52,37,28,0.06);
  }
  .fchip--active-avail {
    border-color: rgba(20,100,55,0.48);
    color: rgb(18,95,52);
    background: rgba(20,100,55,0.06);
  }
  .fchip--active-res {
    border-color: rgba(150,100,15,0.48);
    color: rgb(135,88,10);
    background: rgba(150,100,15,0.07);
  }
  .fchip--active-sold {
    border-color: rgba(95,70,54,0.42);
    color: #7c6554;
    background: rgba(52,37,28,0.04);
  }
  .fchip--active-saved {
    border-color: rgba(198,95,60,0.52);
    color: #c65f3c;
    background: rgba(198,95,60,0.08);
  }
  .fchip--active-viewed {
    border-color: rgba(70,95,85,0.48);
    color: #3e5a52;
    background: rgba(70,95,85,0.07);
  }

  .fchip__count {
    opacity: 0.58;
    font-weight: 400;
  }

  .fchip__dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .fchip__dot--avail { background: rgba(20,135,70,0.75); }
  .fchip__dot--res   { background: rgba(175,120,20,0.75); }
  .fchip__dot--sold  { background: rgba(120,95,80,0.65); }

  /* ── REFINE DISCLOSURE ─────────────────────────────────────────── */
  .filter-refine__head {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .refine-toggle {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 4px 2px;
    font-family: 'Inter', sans-serif;
    font-size: 9.5px;
    letter-spacing: 0.10em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.55);
    background: none;
    border: none;
    cursor: pointer;
    transition: color 0.2s;
  }
  .refine-toggle:hover { color: #34251c; }
  .refine-toggle--open { color: rgba(52,37,28,0.78); }

  .refine-toggle__mark {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 13px;
    height: 13px;
    font-size: 12px;
    line-height: 1;
    color: rgba(95,70,54,0.55);
    border: 1px solid rgba(52,37,28,0.20);
    border-radius: 50%;
  }
  .refine-toggle:hover .refine-toggle__mark { border-color: rgba(52,37,28,0.45); color: #34251c; }

  /* ── SECONDARY FILTERS ─────────────────────────────────────────── */
  .filter-secondary {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    padding-top: 14px;
  }

  .filter-secondary__years {
    display: flex;
    align-items: center;
    gap: 5px;
    flex-wrap: wrap;
  }

  .filter-secondary__sep {
    width: 1px;
    height: 13px;
    background: rgba(52,37,28,0.14);
    flex-shrink: 0;
  }

  .filter-clear {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-left: auto;
    padding: 5px 11px;
    font-size: 9px;
    letter-spacing: 0.10em;
    text-transform: uppercase;
    font-family: 'Inter', sans-serif;
    color: rgba(95,70,54,0.60);
    border: 1px solid rgba(52,37,28,0.12);
    background: transparent;
    cursor: pointer;
    transition: color 0.18s, border-color 0.18s;
  }
  .filter-clear:hover {
    color: rgba(198,95,60,0.85);
    border-color: rgba(198,95,60,0.30);
  }
</style>
