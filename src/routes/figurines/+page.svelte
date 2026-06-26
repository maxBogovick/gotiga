<script lang="ts">
  import { onMount } from 'svelte';
  import { beforeNavigate, afterNavigate, invalidateAll, goto } from '$app/navigation';
  import { fade, slide } from 'svelte/transition';
  import { t, brandName } from '$lib/i18n';
  import { SITE_URL } from '$lib/site';
  import AppImage from '$lib/components/AppImage.svelte';
  import KeyholeVeil from '$lib/components/KeyholeVeil.svelte';
  import SealedDoor from '$lib/components/SealedDoor.svelte';
  import Lightbox from '$lib/components/Lightbox.svelte';
  import OrderModal from '$lib/components/OrderModal.svelte';
  import FilterPopover from '$lib/components/FilterPopover.svelte';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
  import { revealedFigurines } from '$lib/stores/revealed-figurines.svelte';
  import { themeConfig } from '$lib/stores/theme.svelte';
  import { dwellReveal } from '$lib/actions/dwell-reveal';
  import { houseClock } from '$lib/stores/house-clock.svelte';
  import { showingRooms } from '$lib/stores/showing-rooms.svelte';
  import { isGated, isShowingOpen, resolveWindow } from '$lib/showing-window';
  import { SvelteSet } from 'svelte/reactivity';
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
  let batchOffset  = $state(0);

  // Declared up here (not next to its onMount loader) because the filter/count
  // $derived blocks below reference it. Under SSR/prerender deriveds evaluate
  // eagerly in source order, so a later `let` would be in the temporal dead zone.
  let viewedIds = $state(new Set<string>());

  // Dwell-to-reveal: a sustained look thins a sealed card's shadow to "half-lit"
  // over `dwellSec` seconds (never fully — only opening clears it). `glancedIds`
  // holds completed glances; `dwellingId` is the card currently being looked at.
  let dwellSec = $derived(Number($themeConfig.effects?.keyholeDwellReveal ?? 0));
  let glancedIds = new SvelteSet<string>();
  let dwellingId = $state<string | null>(null);

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
    batchOffset  = 0;
  });

  function loadMore() {
    batchOffset  = displayLimit;
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
    revealedFigurines.load();
    houseClock.start();
    showingRooms.load();
    try {
      const viewed: string[] = JSON.parse(localStorage.getItem('gotiga_viewed') ?? '[]');
      viewedIds = new Set(viewed);
    } catch {}
  });

  function toggleLike(e: MouseEvent, id: string) {
    e.preventDefault();
    e.stopPropagation();
    savedFigurines.toggle(id);
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
    // Keyhole memory: lift this work's seal now (and let an older one settle back).
    // Always runs, even for works already in the permanent ledger.
    revealedFigurines.reveal(id);
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
    const url = `${window.location.origin}/figurines/${fig.id}`;
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
  <meta property="og:image" content="{SITE_URL}/images/cabinet-room.jpg" />
  <meta property="og:url" content="{SITE_URL}/figurines" />
  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:title" content="Archive — Gothic Miniatures Collection — {$brandName}" />
  <meta name="twitter:image" content="{SITE_URL}/images/cabinet-room.jpg" />
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
        <ul class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-x-8 gap-y-16">
          {#each visible as figurine, i (figurine.id)}
            <li class="group perspective-container" in:fade={{ delay: Math.max(0, i - batchOffset) * 40, duration: 600 }}
                onmousemove={onTiltMove} onmouseleave={onTiltLeave}>
              <a
                href={doorShut(figurine) ? undefined : `/figurines/${figurine.id}`}
                class="block w-full text-left relative focus:outline-none"
                aria-label="{figurine.name}"
                data-sveltekit-preload-data="hover"
                onclick={(e) => { if (doorShut(figurine)) { e.preventDefault(); return; } markViewed(figurine.id); }}
                use:dwellReveal={{
                  ms: !revealedFigurines.has(figurine.id) && !glancedIds.has(figurine.id) && dwellSec > 0 ? dwellSec * 1000 : 0,
                  onStart: () => dwellingId = figurine.id,
                  onStop: () => { if (dwellingId === figurine.id) dwellingId = null; },
                  onReveal: () => { glancedIds.add(figurine.id); if (dwellingId === figurine.id) dwellingId = null; },
                }}
              >
                <div
                  class="relative aspect-[3/4] mb-6 overflow-hidden bg-[#fff9f0] border border-[#34251c]/10 shadow-2xl transition-all duration-700 group-hover:border-[#34251c]/30 group-hover:shadow-[0_0_30px_-10px_rgba(198,95,60,0.15)] group-hover:-translate-y-2"
                  style={doorShut(figurine) ? '' : `view-transition-name: figurine-${figurine.id}`}
                >

                  {#if doorShut(figurine)}
                    <SealedDoor
                      openFromMin={winOf(figurine).openFromMin}
                      openUntilMin={winOf(figurine).openUntilMin}
                      daysMask={winOf(figurine).daysMask}
                      monthDay={winOf(figurine).monthDay}
                      dateFrom={winOf(figurine).dateFrom}
                      dateUntil={winOf(figurine).dateUntil}
                      doorImageUrl={figurine.sealedDoorImage}
                      name={figurine.name}
                    />
                  {:else}
                  {#if figurine.faceImageUrl}
                    <AppImage
                            src={figurine.faceImageUrl}
                            thumbUrl={figurine.thumbUrl}
                            alt={figurine.name}
                            class="w-full h-full object-cover transition-all duration-700 ease-out group-hover:scale-105 fig-img--{figurine.status}"
                            loading="lazy"
                    />
                  {:else}
                    <div class="w-full h-full flex items-center justify-center opacity-20">
                      <span class="font-['Fraunces'] text-2xl text-[#34251c]">?</span>
                    </div>
                  {/if}

                  {#if figurine.faceImageUrl}
                    <KeyholeVeil
                      show={!revealedFigurines.has(figurine.id)}
                      dwelling={dwellingId === figurine.id}
                      partial={glancedIds.has(figurine.id)}
                      dwellMs={dwellSec * 1000}
                      focalX={figurine.focalX} focalY={figurine.focalY} revealRadius={figurine.revealRadius} darkness={figurine.darkness} />
                  {/if}
                  {/if}

                  <div class="absolute inset-0 bg-[radial-gradient(circle_at_center,transparent_0%,rgba(111,59,36,0.8)_100%)] pointer-events-none transition-opacity duration-500 fig-vignette--{figurine.status}"></div>

                  <div class="absolute top-2 left-2 w-4 h-4 border-t border-l border-[#34251c]/20 group-hover:border-[#34251c]/60 transition-colors pointer-events-none"></div>
                  <div class="absolute bottom-2 right-2 w-4 h-4 border-b border-r border-[#34251c]/20 group-hover:border-[#34251c]/60 transition-colors pointer-events-none"></div>

                  <!-- New arrival badge — top-left, only if created within 21 days -->
                  {#if figurine.createdAt && (Date.now() - new Date(figurine.createdAt).getTime()) < 21 * 86400_000}
                    <span class="absolute top-3 left-3 z-10 px-1.5 py-0.5 bg-[#c65f3c] text-[#f8f1e7] font-['Instrument_Sans',system-ui,sans-serif] text-[7px] uppercase tracking-[0.14em] pointer-events-none select-none">
                      {$t('archiveCardNew')}
                    </span>
                  {/if}

                  <!-- Heart button — always visible, top-right -->
                  <button
                    class="absolute top-3 right-3 z-10 flex items-center justify-center w-[30px] h-[30px] rounded-full backdrop-blur-sm cursor-pointer transition-all duration-250
                      {savedFigurines.has(figurine.id)
                        ? 'bg-[rgba(198,95,60,0.12)] border border-[rgba(198,95,60,0.32)] text-[#c65f3c]'
                        : 'bg-[rgba(255,249,240,0.65)] border border-[rgba(52,37,28,0.13)] text-[rgba(95,70,54,0.55)] hover:bg-[rgba(255,249,240,0.92)] hover:text-[#c65f3c] hover:border-[rgba(198,95,60,0.25)]'}"
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

                <div class="pl-2 border-l border-transparent group-hover:border-[#34251c]/40 transition-all duration-500">
                  <h2 class="font-['Fraunces'] text-xl sm:text-2xl text-[#34251c] mb-1 group-hover:text-[#6f3b24] transition-colors tracking-wide line-clamp-2 leading-snug">
                    {figurine.name}
                  </h2>
                  <div class="flex flex-wrap items-center gap-2">
                    <p class="text-[10px] tracking-[0.06em] uppercase text-[#5f4636] group-hover:text-[#34251c]/70 transition-colors">
                      {$t('archiveExhibit')}{figurine.sortOrder ?? i + 1}
                    </p>
                    {#if figurine.year}
                      <span class="text-[#5f4636]/30">·</span>
                      <span class="text-[10px] tracking-[0.10em] uppercase text-[#7c6554]">
                        {figurine.year}
                      </span>
                    {/if}
                    <span class="text-[#5f4636]/30">·</span>
                    <span class="flex items-center gap-1 text-[10px] tracking-[0.15em] uppercase
                      {figurine.status === 'available' ? 'text-emerald-600/70' : figurine.status === 'reserved' ? 'text-amber-600/70' : 'text-[#7c6554]'}">
                      <span class="w-1 h-1 rounded-full flex-shrink-0
                        {figurine.status === 'available' ? 'bg-emerald-500/60' : figurine.status === 'reserved' ? 'bg-amber-500/60' : 'bg-[#7c6554]'}
                      "></span>
                      {figurine.status === 'available' ? $t('archiveStatusAvailableLabel') : figurine.status === 'reserved' ? $t('archiveStatusReservedLabel') : $t('archiveStatusSoldLabel')}
                    </span>
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
