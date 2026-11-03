<script lang="ts">
  import { stripLang } from '$lib/i18n/path';
  import { onMount, tick } from 'svelte';
  import { beforeNavigate, afterNavigate, invalidateAll, goto, pushState, replaceState } from '$app/navigation';
  import { fade, slide } from 'svelte/transition';
  import { t, lang, brandName, LANG_META, quote } from '$lib/i18n';
  import { SITE_URL, toAbsoluteUrl } from '$lib/site';
  import { figurineHref } from '$lib/figurineHref';
  import { interact, observeWork } from '$lib/analytics';
  import { api } from '$lib/api';
  import { loadFigurine } from '$lib/figurineFull';
  import AppImage from '$lib/components/AppImage.svelte';
  import SealedDoor from '$lib/components/SealedDoor.svelte';
  import EtsyBuyDialog from '$lib/components/EtsyBuyDialog.svelte';
  import FigurinePeek from '$lib/components/FigurinePeek.svelte';
  import FilterPopover from '$lib/components/FilterPopover.svelte';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
  import { houseClock } from '$lib/stores/house-clock.svelte';
  import { showingRooms } from '$lib/stores/showing-rooms.svelte';
  import { isGated, isShowingOpen, resolveWindow, openingHeadline } from '$lib/showing-window';
  import type { FigurineListItem, SemanticHit } from '$lib/types/api';
  import { page } from '$app/state';
  import { localizeList, workBase } from '$lib/workText';
  import { assembleResults, ARCHIVE_KEEPER_INPUT_ID, isInHouseQuery, KEEPER_ARCHIVE_MAX, localMatch } from '$lib/keeper-search';

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
  let doorLocale = $derived(LANG_META[$lang].locale);
  function doorHeadlineOf(f: FigurineListItem): string {
    return openingHeadline(winOf(f), $t, doorLocale, houseClock.nowDate);
  }

  type MainFilter = 'all' | 'available' | 'reserved' | 'sold' | 'saved' | 'viewed';
  type SortMode = 'curated' | 'available' | 'newest' | 'oldest' | 'name';

  let { data } = $props();
  // Тексты работ на языке читателя: название, техника, серия, материал.
  let figurines = $derived(localizeList(data.figurines ?? [], $lang));

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

  // Facet filters only (status / year / technique / series / material). Both the
  // instant local text filter AND the semantic results are constrained to these,
  // so the chips keep working in every mode.
  let facetFiltered = $derived(
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
  );

  // Zero-latency local match on visible metadata — shows the instant it's typed.
  let localFiltered = $derived(
    searchQuery.trim() ? localMatch(facetFiltered, searchQuery) : facetFiltered
  );

  function createdAtMs(iso?: string | null): number {
    if (!iso) return 0;
    const t = new Date(iso).getTime();
    return Number.isFinite(t) ? t : 0;
  }

  let sorted = $derived(
    localFiltered.slice().sort((a, b) => {
      const byCurated = (a.sortOrder ?? 0) - (b.sortOrder ?? 0);
      const byName = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
      // When the piece entered the archive — not the work's year. Most of the
      // collection shares a year, so year-sort made newest/oldest look identical.
      const byCreated = createdAtMs(b.createdAt) - createdAtMs(a.createdAt);
      const byYear = (b.year ?? -Infinity) - (a.year ?? -Infinity);
      const statusRank: Record<string, number> = { available: 0, reserved: 1, sold: 2 };
      if (sortMode === 'available') return statusRank[a.status] - statusRank[b.status] || byCurated || byName;
      if (sortMode === 'newest') return byCreated || byYear || byName;
      if (sortMode === 'oldest') return -byCreated || -byYear || byName;
      if (sortMode === 'name') return byName || byCurated;
      return byCurated || byCreated || byName;
    })
  );

  // ── Unified search: local metadata instantly, then the hybrid keeper ─────────
  // As you type, the local metadata filter above shows instantly. After a short
  // pause the same phrase is sent to /search (vector + fuzzy + lexical, fused).
  // No Enter needed. Degrades to the local filter when the endpoint is absent.
  let keeperHits = $state<SemanticHit[] | null>(null); // null → semantic layer idle
  let keeperLoading = $state(false);
  let keeperError = $state(false);
  let keeperSeq = 0;                                   // drops out-of-order replies

  let figById = $derived(new Map((figurines as FigItem[]).map((f) => [f.id, f])));

  // Final list: hybrid rank from the keeper first, then any local leftovers.
  // Falls back to the plain local list while the keeper is idle or found nothing.
  let resultList = $derived.by(() => {
    const allowed = new Set(facetFiltered.map((f) => f.id));
    return assembleResults(keeperHits, sorted, figById, allowed, KEEPER_ARCHIVE_MAX);
  });

  let visible = $derived(resultList.slice(0, displayLimit));
  let hasMore = $derived(resultList.length > displayLimit);

  // Крупные плитки журнальной сетки: одна на каждые шесть показанных работ, то есть две на
  // порцию из двенадцати, при любом фильтре и сортировке. В блоке сначала берётся новинка или
  // «любимая» работа, иначе первая, у которой есть лицевое фото и открытая дверь.
  let wideIds = $derived.by(() => {
    const ids = new Set<string>();
    for (let i = 0; i < visible.length; i += 6) {
      const block = visible.slice(i, i + 6).filter((f) => !!f.faceImageUrl && !doorShut(f));
      const pick = block.find((f) => f.houseFavorite || isNewArrival(f)) ?? block[0];
      if (pick) ids.add(pick.id);
    }
    return ids;
  });

  // Ракурсы крупной плитки: полный список снимков берётся с сервера, пока он не
  // пришёл, плитка показывает два снимка из списка.
  let angleShots = $state<Record<string, { url: string; thumb: string }[]>>({});
  $effect(() => {
    for (const id of wideIds) {
      if (angleShots[id]) continue;
      void loadFigurine(id).then((f) => {
        const imgs = f?.images ?? [];
        if (imgs.length < 2) return;
        const rank = (type: string) => (type === 'face' ? 0 : type === 'full' ? 1 : 2);
        angleShots[id] = imgs
          .slice()
          .sort((a, b) => rank(a.imageType) - rank(b.imageType))
          .map((i) => ({ url: i.url, thumb: i.thumbUrl ?? i.url }));
      });
    }
  });

  let searchDebounce: ReturnType<typeof setTimeout> | undefined;

  // Drive the semantic layer from the query, debounced. Reads searchQuery only;
  // never reads the keeper* state it writes, so there is no reactive loop.
  $effect(() => {
    const q = searchQuery.trim();
    clearTimeout(searchDebounce);
    if (q.length < 2 || isInHouseQuery(q)) {
      keeperHits = null;
      keeperError = false;
      keeperSeq++;                     // cancel any in-flight reply
      return;
    }
    searchDebounce = setTimeout(() => runKeeper(q), 350);
    return () => clearTimeout(searchDebounce);
  });

  // What people look for, and whether the archive had it. One row per phrase
  // the visitor settled on (a pause, not every keystroke), with how many works
  // it showed: a phrase that showed none is the list of things to make or to
  // name differently. Reads `resultList` too, so the count is taken after the
  // semantic answer lands rather than from the instant local match.
  let searchSettle: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const q = searchQuery.trim().toLowerCase();
    const found = resultList.length;
    clearTimeout(searchSettle);
    if (q.length < 2) return;
    searchSettle = setTimeout(() => interact('search', q.slice(0, 60), found), 1500);
    return () => clearTimeout(searchSettle);
  });

  // Which facets people reach for. Changes in the first moments after the page
  // opens are the saved view being restored, not a choice, and are skipped.
  const archiveOpenedAt = Date.now();
  let lastFacets: Record<string, string> | null = null;
  $effect(() => {
    const facets = {
      status: mainFilter,
      year: yearFilter,
      technique: techniqueFilter,
      series: seriesFilter,
      material: materialFilter,
      sort: sortMode,
    } as Record<string, string>;
    const before = lastFacets;
    lastFacets = { ...facets };
    if (!before || Date.now() - archiveOpenedAt < 1500) return;
    for (const [key, value] of Object.entries(facets)) {
      if (value !== before[key] && value !== 'all' && value !== 'curated') {
        interact('filter', `${key}=${String(value).slice(0, 40)}`);
      }
    }
  });

  async function runKeeper(q: string) {
    const seq = ++keeperSeq;
    keeperLoading = true;
    keeperError = false;
    try {
      const hits = await api.semanticSearch(q);
      if (seq !== keeperSeq) return;   // superseded by a newer keystroke
      keeperHits = hits.slice(0, KEEPER_ARCHIVE_MAX);
    } catch {
      if (seq !== keeperSeq) return;
      keeperError = true;
      keeperHits = null;
    } finally {
      if (seq === keeperSeq) keeperLoading = false;
    }
  }

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
  // Default ("curated") is the popover's all-row, not a duplicate in this list.
  let sortOptions = $derived([
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

  // Серии обложками: три снимка стопкой и число работ. В наличии — наверху стопки.
  let seriesCards = $derived(
    seriesOptions.map((o) => {
      const works = (figurines as FigItem[])
        .filter((f) => f.series === o.value)
        .sort((a, b) => Number(b.status === 'available') - Number(a.status === 'available'));
      const thumbs = works
        .map((f) => f.thumbUrl ?? f.faceImageUrl)
        .filter((u): u is string => !!u)
        .slice(0, 3);
      return { value: o.value, count: o.count, thumbs };
    }),
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
  $effect(() => {
    if (secondaryActive || sortMode !== 'curated' || ['reserved', 'sold', 'saved', 'viewed'].includes(mainFilter)) refineOpen = true;
  });

  // Значения техники, серии и материала — слова на языке читателя; после смены
  // языка прежнее слово ничего не нашло бы, поэтому эти три отбора снимаются.
  let filterLang: string | null = null;
  $effect(() => {
    const l = $lang;
    if (filterLang !== null && filterLang !== l) {
      techniqueFilter = 'all'; seriesFilter = 'all'; materialFilter = 'all';
    }
    filterLang = l;
  });

  $effect(() => {
    void mainFilter; void searchQuery; void sortMode; void yearFilter; void techniqueFilter;
    void seriesFilter; void materialFilter;
    displayLimit = PAGE_SIZE;
  });

  function loadMore() {
    displayLimit += PAGE_SIZE;
  }

  // Число найденных работ показывается только при отборе; весь архив назван на фишке «Все».
  let isNarrowed = $derived(keeperHits !== null || mainFilter !== 'all' || !!searchQuery.trim() ||
    yearFilter !== 'all' || techniqueFilter !== 'all' || seriesFilter !== 'all' || materialFilter !== 'all');

  function clearFilters() {
    searchQuery = ''; mainFilter = 'all'; sortMode = 'curated';
    yearFilter = 'all'; techniqueFilter = 'all'; seriesFilter = 'all'; materialFilter = 'all';
    keeperHits = null; keeperError = false; keeperSeq++;   // the query effect also clears it
  }

  // ── Список работ для поисковика ──────────────────────────────────────────
  // «Ещё 12» — кнопка, поэтому адреса работ с 13-й видны роботу только отсюда.
  // Product здесь не годится: у работ нет цены на странице, а без неё разметка неполная.
  const ldJson = (o: unknown) => JSON.stringify(o).replace(/</g, '\\u003c');
  let archiveList = $derived({
    '@context': 'https://schema.org',
    '@type': 'ItemList',
    numberOfItems: (figurines as FigItem[]).length,
    itemListElement: (figurines as FigItem[])
      .slice()
      .sort((a, b) => (a.sortOrder ?? 0) - (b.sortOrder ?? 0))
      .map((f, i) => ({
        '@type': 'ListItem',
        position: i + 1,
        name: f.name,
        url: `${SITE_URL}${figurineHref(f)}`,
        ...(f.faceImageUrl ? { image: toAbsoluteUrl(f.faceImageUrl) } : {}),
      })),
  });

  // ── Отбор в адресе ──────────────────────────────────────────────────────
  // Ссылку с отбором можно переслать: ?status, ?series, ?technique, ?material, ?year,
  // ?sort, ?q и ?page (сколько порций показано). «Сохранённые» и «Просмотренные» —
  // личные списки и в адрес не попадают.
  const PUBLIC_STATUS = ['available', 'reserved', 'sold'];
  const SORTS: SortMode[] = ['available', 'newest', 'oldest', 'name'];
  const MAX_PAGES = 20;
  let urlReady = $state(false);

  function filterParams(pageNo: number): URLSearchParams {
    const sp = new URLSearchParams();
    const q = searchQuery.trim();
    if (q) sp.set('q', q.slice(0, 80));
    if (PUBLIC_STATUS.includes(mainFilter)) sp.set('status', mainFilter);
    if (yearFilter !== 'all') sp.set('year', yearFilter);
    if (techniqueFilter !== 'all') sp.set('technique', techniqueFilter);
    if (seriesFilter !== 'all') sp.set('series', seriesFilter);
    if (materialFilter !== 'all') sp.set('material', materialFilter);
    if (sortMode !== 'curated') sp.set('sort', sortMode);
    if (pageNo > 1) sp.set('page', String(pageNo));
    return sp;
  }

  function readUrlFilters() {
    const sp = page.url.searchParams;
    searchQuery = sp.get('q') ?? '';
    const status = sp.get('status');
    if (status && PUBLIC_STATUS.includes(status)) mainFilter = status as MainFilter;
    const year = sp.get('year');
    if (year && availableYears.includes(Number(year))) yearFilter = year;
    // Слова техники, серии и материала — на языке читателя; чужое слово ничего не найдёт.
    const technique = sp.get('technique');
    if (technique && availableTechniques.includes(technique)) techniqueFilter = technique;
    const series = sp.get('series');
    if (series && availableSeries.includes(series)) seriesFilter = series;
    const material = sp.get('material');
    if (material && availableMaterials.includes(material)) materialFilter = material;
    const sort = sp.get('sort');
    if (sort && (SORTS as string[]).includes(sort)) sortMode = sort as SortMode;
  }

  // Адрес следует за отбором; запись истории не добавляется.
  $effect(() => {
    if (!urlReady) return;
    const sp = filterParams(Math.ceil(displayLimit / PAGE_SIZE));
    const next = sp.toString();
    if (next === page.url.searchParams.toString()) return;
    const url = new URL(page.url);
    url.search = next;
    replaceState(url, page.state);
  });

  let nextHref = $derived('?' + filterParams(Math.ceil(displayLimit / PAGE_SIZE) + 1).toString());

  // ── Card actions ─────────────────────────────────────────────────────────
  let buyFig = $state<FigurineListItem | null>(null);
  let shareCopiedId = $state('');

  onMount(() => {
    savedFigurines.load();
    houseClock.start();
    showingRooms.load();
    // `page.url.searchParams` is forbidden during prerender — seed `?q=` only
    // in the browser, after the static archive HTML has been written.
    // `?status=available` — ссылка «Смотреть работы в наличии» со страницы «Как приобрести».
    readUrlFilters();
    try {
      const viewed: string[] = JSON.parse(localStorage.getItem('gotiga_viewed') ?? '[]');
      viewedIds = new Set(viewed);
    } catch {}
    // Отбор из адреса сбрасывает порцию до первой; число порций из адреса ставится после этого.
    const pages = Math.min(Math.max(parseInt(page.url.searchParams.get('page') ?? '1', 10) || 1, 1), MAX_PAGES);
    void tick().then(() => {
      displayLimit = pages * PAGE_SIZE;
      urlReady = true;
    });
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

  function markViewed(id: string) {
    if (viewedIds.has(id)) return;
    const next = new Set(viewedIds);
    next.add(id);
    viewedIds = next;
    try { localStorage.setItem('gotiga_viewed', JSON.stringify([...next])); } catch {}
  }

  // Нажатие на фото или название: панель просмотра; с Ctrl/⌘/Shift и средней кнопкой — обычная ссылка.
  function onCardClick(e: MouseEvent, fig: FigurineListItem, i: number) {
    if (doorShut(fig)) { e.preventDefault(); return; }
    interact('work_click', `archive:${i + 1}`, i + 1, false);
    if (e.button === 0 && !e.metaKey && !e.ctrlKey && !e.shiftKey && !e.altKey) {
      e.preventDefault();
      openPeek(fig);
    } else markViewed(fig.id);
  }

  // Точки под фото показывают, какой из двух снимков на экране (только там, где листают пальцем).
  function slideDots(node: HTMLElement) {
    const onScroll = () => {
      const i = Math.round(node.scrollLeft / Math.max(node.clientWidth, 1));
      node.parentElement?.setAttribute('data-i', String(i));
    };
    node.addEventListener('scroll', onScroll, { passive: true });
    return { destroy() { node.removeEventListener('scroll', onScroll); } };
  }

  async function handleShare(e: Event | null, fig: FigurineListItem) {
    e?.preventDefault();
    e?.stopPropagation();
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

  // Pinterest "Pin it": opens the pin-create dialog pre-filled with this work's
  // detail URL, its face image (absolute — Pinterest fetches the media server-side,
  // so a relative path would fail) and a keyword-bearing description. Every visitor
  // who pins becomes a distributor, which is the entire reason the archive is worth
  // being on Pinterest. Opened in a small popup so the archive stays put behind it.
  function pinterestPin(e: Event | null, fig: FigurineListItem) {
    e?.preventDefault();
    e?.stopPropagation();
    const pageUrl = `${window.location.origin}${figurineHref(fig)}`;
    const media = toAbsoluteUrl(fig.faceImageUrl ?? fig.detailImageUrl);
    const description = `${fig.name} — ${$t('archivePinSuffix')}`;
    const share = `https://www.pinterest.com/pin/create/button/?url=${encodeURIComponent(pageUrl)}`
      + (media ? `&media=${encodeURIComponent(media)}` : '')
      + `&description=${encodeURIComponent(description)}`;
    window.open(share, 'pinterest', 'noopener,width=750,height=650');
  }

  function openOrder(e: MouseEvent, fig: FigurineListItem) {
    e.preventDefault();
    e.stopPropagation();
    buyFig = fig;
  }

  function openSimilarCommission(e: MouseEvent, fig: FigurineListItem) {
    e.preventDefault();
    e.stopPropagation();
    markViewed(fig.id);
    goto(`/commission?source=${encodeURIComponent(fig.id)}`);
  }

  // ── Просмотр работы без ухода со страницы ──────────────────────────────────
  // Запись истории без перехода (`pushState`): «назад» закрывает панель, а
  // перезагрузка открывает обычный архив. Листают по тому списку, что на экране.
  let peekWrap = $state<HTMLElement | null>(null);
  let peekStart = $state(0);
  let peekAsk = $state(false);
  let peekId = $derived(page.state.peek ?? null);
  let peekIndex = $derived(peekId ? resultList.findIndex((f) => f.id === peekId) : -1);
  let peekWork = $derived(peekId ? (figById.get(peekId) ?? null) : null);
  let peekSimilar = $derived(
    (peekWork?.similarIds ?? []).map((id) => figById.get(id)).filter((f): f is FigItem => !!f && f.id !== peekWork?.id),
  );

  function openPeek(fig: FigurineListItem, photo = 0, ask = false) {
    peekStart = photo;
    peekAsk = ask;
    markViewed(fig.id);
    if (peekId) replaceState('', { peek: fig.id });
    else {
      interact('peek_open', fig.id);
      pushState('', { peek: fig.id });
      // Панель прилипает под шапкой, но стоит там, где начинается сетка: если сетка ниже
      // экрана, подводим страницу к ней. `scrollIntoView` на прилипающем блоке не прокручивает.
      void tick().then(() => {
        if (!peekWrap || window.innerWidth < 1024) return;
        const top = peekWrap.getBoundingClientRect().top;
        if (top > 90) window.scrollTo({ top: window.scrollY + top - 90, behavior: 'smooth' });
      });
    }
  }

  function closePeek() {
    if (!page.state.peek) return;
    history.back();
    // «Назад» не вернул на запись без панели (например, запись истории не создалась): закрыть самим.
    setTimeout(() => { if (page.state.peek) replaceState('', {}); }, 300);
  }

  function stepPeek(dir: 1 | -1) {
    const n = resultList.length;
    if (n < 2 || peekIndex < 0) return;
    const next = resultList[(peekIndex + dir + n) % n];
    if (displayLimit < resultList.indexOf(next) + 1) displayLimit = resultList.indexOf(next) + 1;
    openPeek(next);
    requestAnimationFrame(() => {
      document.getElementById(`fig-${next.id}`)?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    });
  }

  function onPeekKey(e: KeyboardEvent) {
    if (!peekId || buyFig || e.defaultPrevented) return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    const el = e.target as HTMLElement | null;
    if (el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName))) return;
    if (e.key === 'ArrowRight') { e.preventDefault(); stepPeek(1); }
    else if (e.key === 'ArrowLeft') { e.preventDefault(); stepPeek(-1); }
    else if (e.key === 'Escape') { e.preventDefault(); closePeek(); }
  }

  // ── Scroll + filter state restoration ────────────────────────────────────
  const SCROLL_KEY  = 'figurines-scroll';
  const FILTER_KEY  = 'figurines-filters';

  beforeNavigate(({ to }) => {
    if (stripLang(to?.url.pathname ?? '').startsWith('/figurines/')) {
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
        requestAnimationFrame(() => window.scrollTo({ top: parseInt(savedScroll, 10), behavior: 'instant' }));
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
  <title>{$t('archiveMetaTitle')} — {$brandName}</title>
  <meta name="description" content={$t('archiveMetaDescription')} />
  <meta property="og:site_name" content={$brandName} />
  <meta property="og:locale" content={LANG_META[$lang].og} />
  <meta property="og:type" content="website" />
  <meta property="og:title" content="{$t('archiveMetaTitle')} — {$brandName}" />
  <meta property="og:description" content={$t('archiveMetaDescription')} />
  <meta property="og:image" content="{SITE_URL}/images/cabinet-bg.jpeg" />
  <meta property="og:url" content="{SITE_URL}/figurines" />
  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:title" content="{$t('archiveMetaTitle')} — {$brandName}" />
  <meta name="twitter:image" content="{SITE_URL}/images/cabinet-bg.jpeg" />
  {@html `<script type="application/ld+json">${ldJson({ '@context': 'https://schema.org', '@type': 'BreadcrumbList', itemListElement: [ { '@type': 'ListItem', position: 1, name: $brandName, item: SITE_URL }, { '@type': 'ListItem', position: 2, name: $t('archivePageTitle'), item: `${SITE_URL}/figurines` } ] })}<\/script>`}
  {@html `<script type="application/ld+json">${ldJson(archiveList)}<\/script>`}
  <!-- Fonts loaded once globally in app.html -->
</svelte:head>

<svelte:window onkeydown={onPeekKey} />

<div class="fixed inset-0 bg-[#f8f1e7] -z-50"></div>
<div class="fixed inset-0 pointer-events-none z-0 bg-noise opacity-[0.07] mix-blend-overlay"></div>
<div class="fixed inset-0 pointer-events-none z-0 bg-[radial-gradient(circle_at_50%_50%,transparent_0%,#f8f1e7_90%)]"></div>

<div class="min-h-screen relative z-10 overflow-x-clip font-['Instrument Sans'] text-[#34251c]">

    <div class="container mx-auto px-6 sm:px-12 pt-8 pb-12">
      <!-- ── FILTER BAR ─────────────────────────────────────────────── -->
      <div class="filter-bar" in:fade={{ duration: 250 }}>

        <!-- Row 1: Title -->
        <div class="filter-bar__head">
          <h1 class="filter-bar__title">{$t('archivePageTitle')}</h1>
        </div>

        <!-- Row 2: Search + Sort -->
        <div class="filter-bar__search-row">
          <label class="filter-bar__search-wrap">
            {#if keeperLoading}
              <span class="filter-bar__search-spin" aria-hidden="true"></span>
            {:else}
              <svg class="filter-bar__search-icon" width="13" height="13" viewBox="0 0 13 13" fill="none" aria-hidden="true">
                <circle cx="5.5" cy="5.5" r="4" stroke="currentColor" stroke-width="1"/>
                <path d="M9 9L12 12" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
              </svg>
            {/if}
            <input
              type="search"
              bind:value={searchQuery}
              id={ARCHIVE_KEEPER_INPUT_ID}
              name="q"
              placeholder={$t('archiveAskPlaceholder')}
              class="filter-bar__search-input"
              aria-label={$t('archiveAskPlaceholder')}
              onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); clearTimeout(searchDebounce); if (searchQuery.trim().length >= 2) runKeeper(searchQuery.trim()); } }}
            />
            {#if searchQuery}
              <button
                type="button"
                onclick={() => { searchQuery = ''; }}
                class="filter-bar__search-clear"
                aria-label={$t('archiveClearFilters')}
              >
                <svg width="9" height="9" viewBox="0 0 9 9" fill="none" aria-hidden="true">
                  <path d="M1 1l7 7M8 1L1 8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                </svg>
              </button>
            {/if}
          </label>

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

        </div>

        <!-- Серии: вход по теме, а не по полю фильтра -->
        {#if seriesCards.length > 1}
        <div class="series-strip" role="group" aria-label={$t('archiveSeriesLabel')}>
          {#each seriesCards as sc (sc.value)}
            <button
              type="button"
              class="series-card {seriesFilter === sc.value ? 'series-card--on' : ''}"
              aria-pressed={seriesFilter === sc.value}
              onclick={() => { seriesFilter = seriesFilter === sc.value ? 'all' : sc.value; }}
            >
              <span class="series-card__stack" aria-hidden="true">
                {#each sc.thumbs as th, k (th)}
                  <span class="series-card__thumb" style="--k:{k}">
                    <AppImage src={th} alt="" class="w-full h-full object-cover" loading="lazy" sizes="48px" />
                  </span>
                {/each}
              </span>
              <span class="series-card__name">{sc.value}</span>
              <span class="series-card__n">{sc.count}</span>
            </button>
          {/each}
        </div>
        {/if}

        <!-- Row 4: Refine — secondary axes folded as marginalia, calm by default -->
        {#if true}
        <div class="filter-refine">
          <div class="filter-refine__head">
            {#if true}
            <button
              type="button"
              class="refine-toggle {refineOpen ? 'refine-toggle--open' : ''}"
              onclick={() => refineOpen = !refineOpen}
              aria-expanded={refineOpen}
            >
              <span class="refine-toggle__mark">{refineOpen ? '–' : '+'}</span>
              {$t('archiveMoreFilters')}
            </button>
            {/if}

            <span class="filter-count" aria-live="polite">{#if isNarrowed}{resultList.length} / {(figurines as FigItem[]).length}{/if}</span>

            {#if hasActiveFilters}
            <button type="button" onclick={clearFilters} class="filter-clear">
              <svg width="9" height="9" viewBox="0 0 10 10" fill="none" aria-hidden="true">
                <path d="M1 5h8M5 1v8" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" transform="rotate(45 5 5)"/>
              </svg>
              {$t('archiveClearFilters')}
            </button>
            {/if}
          </div>

          {#if refineOpen}
          <div class="filter-secondary" transition:slide={{ duration: 260 }}>

            <div class="filter-chips filter-chips--inner">
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
            title={savedCount === 0 ? $t('archiveSaveHint') : undefined}
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
            title={viewedCount === 0 ? $t('archiveViewedHint') : undefined}
          >
            <svg width="12" height="11" viewBox="0 0 14 14" fill="none" aria-hidden="true">
              <path d="M1 7C1 7 3.5 3 7 3C10.5 3 13 7 13 7C13 7 10.5 11 7 11C3.5 11 1 7 1 7Z" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round"/>
              <circle cx="7" cy="7" r="2" stroke="currentColor" stroke-width="1.1"/>
            </svg>
            {$t('archiveFilterViewed')}
            <span class="fchip__count">{viewedCount}</span>
          </button>
            </div>
            <span class="filter-secondary__sep" aria-hidden="true"></span>

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

            <FilterPopover
              label={$t('archiveSortLabel')}
              bind:value={sortMode}
              options={sortOptions}
              allValue="curated"
              allLabel={$t('archiveSortCurated')}
            />
          </div>
          {/if}
        </div>
        {/if}

      </div>

      {#if resultList.length > 0}
        <div bind:this={peekWrap} class={peekWork ? 'lg:grid lg:grid-cols-[minmax(0,1fr)_480px] lg:gap-8 lg:items-start' : ''}>
        <div class="min-w-0">
        <ul class="fig-grid {peekWork ? 'fig-grid--peek' : ''}">
          {#each visible as figurine, i (figurine.id)}
            {@const wide = wideIds.has(figurine.id)}
            {@const hasAlt = !!figurine.detailImageUrl && !!figurine.faceImageUrl}
            {@const focus = `${Math.round((figurine.focalX ?? 0.5) * 100)}% ${Math.round((figurine.focalY ?? 0.12) * 100)}%`}
            <li class="fig-tile group {wide ? 'fig-tile--wide' : ''}"
                class:fig-picked={peekId === figurine.id}
                id="fig-{figurine.id}"
                use:observeWork={figurine.id}>
              <div class="fig-card">
                <div
                  class="fig-media"
                  style={doorShut(figurine) ? '' : `view-transition-name: figurine-${figurine.id}`}
                >
                  <a
                    href={doorShut(figurine) ? undefined : figurineHref(figurine)}
                    class="fig-photolink"
                    aria-label={figurine.name}
                    data-sveltekit-preload-data="hover"
                    onclick={(e) => onCardClick(e, figurine, i)}
                  >
                    {#if doorShut(figurine)}
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
                      <!-- Два снимка: на телефоне листаются пальцем, на компьютере второй
                           проявляется при наведении. Точка фокуса берётся из работы, чтобы
                           не резать голову. -->
                      <div class="fig-photo" use:slideDots style="--op: {focus}">
                        {#if figurine.faceImageUrl}
                          <AppImage
                            src={wide ? (figurine.faceImageLargeUrl ?? figurine.faceImageUrl) : figurine.faceImageUrl}
                            thumbUrl={figurine.thumbUrl}
                            alt={figurine.name}
                            class="fig-img-main"
                            loading={i < 4 ? 'eager' : 'lazy'}
                            fetchpriority={i < 2 ? 'high' : undefined}
                            sizes={wide ? '(max-width: 680px) 100vw, 50vw' : '(max-width: 680px) 50vw, (max-width: 1024px) 33vw, 25vw'}
                          />
                          {#if figurine.detailImageUrl}
                            <AppImage
                              src={figurine.detailImageUrl}
                              alt=""
                              class="fig-img-alt"
                              loading="lazy"
                              sizes={wide ? '(max-width: 680px) 100vw, 50vw' : '(max-width: 680px) 50vw, (max-width: 1024px) 33vw, 25vw'}
                            />
                          {/if}
                        {:else}
                          <div class="fig-noimg"><span>?</span></div>
                        {/if}
                      </div>
                      {#if hasAlt}
                        <span class="fig-dots" aria-hidden="true"><i></i><i></i></span>
                      {/if}
                    {/if}
                  </a>

                  <div class="fig-badges">
                    {#if figurine.houseFavorite}
                      <span class="fig-favorite" title={$t('houseFavoriteBadge')}>{$t('houseFavoriteBadge')}</span>
                    {/if}
                    {#if isNewArrival(figurine)}
                      <span class="fig-seal">{$t('archiveCardNew')}</span>
                    {/if}
                  </div>

                  {#if !doorShut(figurine)}
                    <button
                      type="button"
                      class="fig-heart fig-share"
                      onclick={(e) => handleShare(e, figurine)}
                      aria-label={shareCopiedId === figurine.id ? $t('cardLinkCopied') : $t('cardShare')}
                      title={shareCopiedId === figurine.id ? $t('cardLinkCopied') : $t('cardShare')}
                    >
                      {#if shareCopiedId === figurine.id}
                        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true"><path d="M2 7L5.5 10.5L12 3.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
                      {:else}
                        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true"><path d="M9 2H12V5" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round"/><path d="M12 2L7 7" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/><path d="M6 3H3C2.4 3 2 3.4 2 4V11C2 11.6 2.4 12 3 12H10C10.6 12 11 11.6 11 11V8" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/></svg>
                      {/if}
                    </button>
                  {/if}
                  <button
                    class="fig-heart {savedFigurines.has(figurine.id) ? 'fig-heart--on' : ''}"
                    class:fig-just-saved={justSavedId === figurine.id}
                    onclick={(e) => toggleLike(e, figurine.id)}
                    aria-label={savedFigurines.has(figurine.id) ? $t('cardSaved') : $t('cardSave')}
                    aria-pressed={savedFigurines.has(figurine.id)}
                    title={savedFigurines.has(figurine.id) ? $t('cardSaved') : $t('cardSave')}
                  >
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                      <path
                        d="M7 12.5C7 12.5 1 8.5 1 4.5C1 2.5 2.5 1 4.5 1C5.5 1 6.5 1.8 7 3C7.5 1.8 8.5 1 9.5 1C11.5 1 13 2.5 13 4.5C13 8.5 7 12.5 7 12.5Z"
                        fill={savedFigurines.has(figurine.id) ? 'currentColor' : 'none'}
                        stroke="currentColor" stroke-width="1.1" stroke-linejoin="round"
                      />
                    </svg>
                  </button>
                </div>

                <div class="fig-cap">
                  <h2 class="fig-name">
                    <a
                      href={doorShut(figurine) ? undefined : figurineHref(figurine)}
                      onclick={(e) => onCardClick(e, figurine, i)}
                    >{figurine.name}</a>
                  </h2>
                  {#if doorShut(figurine)}
                    <p class="fig-meta"><span>{doorHeadlineOf(figurine)}</span></p>
                  {:else}
                    <p class="fig-meta">
                      <span class="fig-status fig-status--{figurine.status}">
                        {figurine.status === 'available' ? $t('archiveStatusAvailableLabel') : figurine.status === 'reserved' ? $t('archiveStatusReservedLabel') : figurine.status === 'in_progress' ? $t('profileWishInProgress') : $t('archiveStatusSoldLabel')}
                      </span>
                      {#if figurine.year}<span>{figurine.year}</span>{/if}
                      {#if figurine.dimensions}<span>{figurine.dimensions}</span>{/if}
                    </p>
                    {#if wide}
                      {#if figurine.shortText}<p class="fig-lead">{figurine.shortText}</p>{/if}
                      {#if angleShots[figurine.id] || hasAlt}
                        <div class="fig-angles">
                          {#each angleShots[figurine.id] ?? [{ url: figurine.faceImageUrl ?? '', thumb: figurine.thumbUrl ?? figurine.faceImageUrl ?? '' }, { url: figurine.detailImageUrl ?? '', thumb: figurine.detailImageUrl ?? '' }] as shot, i (i)}
                            <button type="button" class="fig-angle" onclick={() => openPeek(figurine, i)} aria-label={figurine.name}>
                              <AppImage src={shot.url} thumbUrl={shot.thumb} alt="" class="fig-angle-img" loading="lazy" sizes="64px" />
                            </button>
                          {/each}
                        </div>
                      {/if}
                    {/if}
                    {#if wide}
                    <div class="fig-actions fig-actions--row">
                      {#if figurine.status === 'available'}
                        <button type="button" class="fig-buy" onclick={(e) => openOrder(e, figurine)}>{$t('buyAction')}</button>
                      {:else}
                        <button type="button" class="fig-similar" onclick={(e) => openSimilarCommission(e, figurine)}>{$t('commissionCreateSimilarShort')}</button>
                      {/if}
                        <button type="button" class="fig-ask" onclick={() => openPeek(figurine, 0, true)} aria-label={$t('dockAsk')} title={$t('dockAsk')}>
                          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M2.5 3.5h11v7h-6l-3 2.5v-2.5h-2z" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round"/></svg>
                        </button>
                    </div>
                    {/if}
                  {/if}
                </div>
              </div>
            </li>
          {/each}
        </ul>

        {#if hasMore}
          <div class="mt-16 flex flex-col items-center gap-3" in:fade>
            <p class="text-xs tracking-wide text-[#7c6554] tabular-nums">{visible.length} / {resultList.length}</p>
            <a
              href={nextHref}
              onclick={(e) => { e.preventDefault(); loadMore(); }}
              class="group flex items-center gap-4 px-10 py-4 border border-[#34251c]/20 hover:border-[#34251c]/50 text-[#5f4636] hover:text-[#34251c] font-['Instrument Sans'] text-xs tracking-[0.08em] uppercase transition-all duration-500"
            >
              <span>{$t('archiveLoadMore')}</span>
              <span class="text-[#34251c]/30 group-hover:text-[#34251c]/85 transition-colors">{Math.min(PAGE_SIZE, resultList.length - displayLimit)}</span>
              <span class="transition-transform group-hover:translate-y-0.5">↓</span>
            </a>
          </div>
        {/if}
        </div>

        {#if peekWork}
          <FigurinePeek
            work={peekWork}
            position={peekIndex + 1}
            total={resultList.length}
            saved={savedFigurines.has(peekWork.id)}
            onClose={closePeek}
            onPrev={() => stepPeek(-1)}
            onNext={() => stepPeek(1)}
            onBuy={() => { if (peekWork) buyFig = peekWork; }}
            onSimilar={() => { if (peekWork) goto(`/commission?source=${encodeURIComponent(peekWork.id)}`); }}
            onToggleSaved={() => { if (peekWork) savedFigurines.toggle(peekWork.id); }}
            shareCopied={shareCopiedId === peekWork.id}
            onShare={() => { if (peekWork) handleShare(null, peekWork); }}
            onPin={() => { if (peekWork) pinterestPin(null, peekWork); }}
            startPhoto={peekStart}
            startAsk={peekAsk}
            similar={peekSimilar}
            onOpenWork={(id) => { const f = figById.get(id); if (f) openPeek(f); }}
          />
        {/if}
        </div>

        <!--
          Соседняя комната, названная её собственными словами: карта — это
          работа дома, увиденная с другой стороны. В шапке её нет и не будет —
          а здесь человек и так смотрит на все работы разом, и приглашение
          стоит ровно там, где оно что-то значит. Тише кнопки «ещё»: это не
          призыв, а приписка под полкой.
        -->
        <div class="mt-24 flex justify-center" in:fade>
          <a
            href="/battles"
            class="group inline-flex items-center gap-3 text-[#7c6554] hover:text-[#34251c] font-['Instrument Sans'] text-[11px] tracking-[0.14em] uppercase transition-colors duration-500"
          >
            <span class="w-10 h-px bg-[#34251c]/20 group-hover:bg-[#34251c]/45 transition-colors"></span>
            <span>{$t('archiveToBattles')}</span>
            <span class="transition-transform group-hover:translate-x-0.5">→</span>
          </a>
        </div>
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
      {:else if keeperLoading}
        <div class="flex flex-col items-center justify-center py-32 border border-dashed border-[#34251c]/10 rounded-lg" in:fade>
          <span class="filter-bar__search-spin filter-bar__search-spin--lg" aria-hidden="true"></span>
          <p class="text-xs tracking-wide text-[#7c6554] uppercase mt-5">{$t('archiveKeeperThinking')}</p>
        </div>
      {:else if keeperError}
        <div class="flex flex-col items-center justify-center py-32 border border-dashed border-[#c65f3c]/25 rounded-lg" in:fade>
          <p class="font-['Fraunces'] text-3xl text-[#6f3b24] mb-2 opacity-85">{$t('archiveKeeperErrorTitle')}</p>
          <p class="text-xs tracking-wide text-[#7c6554] uppercase max-w-md text-center">{$t('archiveKeeperErrorHint')}</p>
        </div>
      {:else if searchQuery}
        <div class="flex flex-col items-center justify-center py-32 border border-dashed border-[#34251c]/10 rounded-lg" in:fade>
          <p class="font-['Fraunces'] text-3xl text-[#5f4636] mb-2 opacity-75">{$t('archiveNotFound')}</p>
          <p class="text-xs tracking-wide text-[#7c6554] uppercase">{$t('archiveNoEntry')} {quote($lang, searchQuery)}</p>
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



<EtsyBuyDialog
  isOpen={!!buyFig}
  work={buyFig}
  onClose={() => { buyFig = null; }}
  onSimilar={() => { if (buyFig) goto(`/commission?source=${encodeURIComponent(buyFig.id)}`); }}
/>

<style>
  @keyframes shimmer {
    100% { transform: translateX(200%); }
  }

  /* Cards render immediately — no scroll-reveal rise/rotate/scale effect. */
  .fig-tile { opacity: 1; }

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


  /* ── Журнальная сетка: две колонки на телефоне, крупная плитка 2×2 для новинок ── */
  .fig-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-auto-flow: dense;
    align-items: stretch;
    gap: 26px 12px;
    list-style: none;
    margin: 0;
    padding: 0;
  }
  @media (min-width: 640px) {
    .fig-grid { grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 36px 20px; }
  }
  @media (min-width: 1024px) {
    .fig-grid--peek { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
  @media (min-width: 1280px) {
    .fig-grid { grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 40px 24px; }
    .fig-grid--peek { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  }
  .fig-tile { min-width: 0; }
  .fig-tile--wide { grid-column: span 2; grid-row: span 2; }
  .fig-card { display: flex; flex-direction: column; height: 100%; }

  .fig-media {
    position: relative;
    aspect-ratio: 4 / 5;
    overflow: hidden;
    border-radius: 3px;
    background: linear-gradient(160deg, #f1e8da 0%, #e6d9c6 100%);
  }
  .fig-tile--wide .fig-media { flex: 1 1 auto; min-height: 0; }
  .fig-photolink { position: absolute; inset: 0; display: block; }
  .fig-photo { position: absolute; inset: 0; }
  .fig-photo :global(.fig-img-main) { width: 100%; height: 100%; }
  .fig-photo :global(.app-image-main),
  .fig-photo :global(.app-image-thumb) { object-position: var(--op, 50% 12%); }
  .fig-noimg { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; opacity: 0.25; font-family: 'Fraunces', serif; font-size: 28px; }

  .fig-heart {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: 999px;
    border: 1px solid rgba(52, 37, 28, 0.13);
    background: rgba(255, 249, 240, 0.82);
    color: rgba(95, 70, 54, 0.7);
    cursor: pointer;
    backdrop-filter: blur(6px);
    transition: color 0.2s, background 0.2s, border-color 0.2s;
  }
  .fig-share { right: 52px; }
  /* Видимый круг 34 px, нажимаемая область 44 px */
  .fig-heart::after { content: ''; position: absolute; inset: -5px; border-radius: 50%; }
  .fig-heart:hover { color: #c65f3c; background: rgba(255, 249, 240, 0.96); }
  .fig-heart--on { color: #c65f3c; border-color: rgba(198, 95, 60, 0.4); background: rgba(255, 240, 230, 0.95); }

  /* Точки под фото: только там, где снимки листают пальцем */
  .fig-dots { display: none; position: absolute; left: 0; right: 0; bottom: 8px; z-index: 3; justify-content: center; gap: 5px; pointer-events: none; }
  .fig-dots i { width: 6px; height: 6px; border-radius: 50%; background: rgba(255, 249, 240, 0.55); box-shadow: 0 0 0 1px rgba(44, 23, 16, 0.18); }
  .fig-dots i:first-child { background: #fff9f0; }
  .fig-photolink[data-i='1'] .fig-dots i:first-child { background: rgba(255, 249, 240, 0.55); }
  .fig-photolink[data-i='1'] .fig-dots i:nth-child(2) { background: #fff9f0; }
  @media (hover: none) {
    .fig-dots { display: flex; }
    .fig-photo { display: flex; overflow-x: auto; scroll-snap-type: x mandatory; scrollbar-width: none; overscroll-behavior-x: contain; }
    .fig-photo::-webkit-scrollbar { display: none; }
    .fig-photo > :global(.app-image-wrap) { flex: 0 0 100%; scroll-snap-align: start; }
    .fig-media :global(.fig-img-alt) { position: relative; inset: auto; opacity: 1; pointer-events: auto; }
  }

  /* ── Подпись рядом с фото ── */
  .fig-cap { flex: 1 1 auto; display: flex; flex-direction: column; gap: 4px; padding: 10px 2px 0; }
  .fig-name {
    margin: 0;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 16px;
    font-weight: 400;
    line-height: 1.25;
    color: #34251c;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .fig-name a { color: inherit; text-decoration: none; }
  .fig-name a:hover { color: #6f3b24; }
  .fig-tile--wide .fig-name { font-size: 22px; }
  .fig-meta { display: flex; flex-wrap: wrap; align-items: center; gap: 2px 8px; margin: 0; font-size: 12px; color: #7c6554; font-variant-numeric: tabular-nums; }
  .fig-status { display: inline-flex; align-items: center; gap: 6px; font-weight: 500; letter-spacing: 0.04em; }
  .fig-status::before { content: ''; width: 7px; height: 7px; border-radius: 50%; background: currentColor; }
  .fig-status--available { color: #4d6b3a; }
  .fig-status--reserved { color: #a2742a; }
  .fig-status--sold,
  .fig-status--in_progress { color: #8a7566; }
  .fig-lead {
    margin: 4px 0 0;
    font-size: 14px;
    line-height: 1.5;
    color: #4a392e;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .fig-angles { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 8px; height: 70px; overflow: hidden; }
  .fig-angle { width: 56px; height: 70px; padding: 0; border: 1px solid transparent; border-radius: 2px; overflow: hidden; background: #efe5d6; cursor: pointer; transition: border-color 0.2s; }
  .fig-angle:hover { border-color: #c65f3c; }
  .fig-angle :global(.fig-angle-img) { width: 100%; height: 100%; }
  .fig-angle :global(.app-image-main), .fig-angle :global(.app-image-thumb) { object-fit: cover; object-position: 50% 12%; }
  .fig-actions { margin-top: auto; padding-top: 6px; }
  .fig-actions--row { display: flex; gap: 8px; }
  .fig-actions--row .fig-buy, .fig-actions--row .fig-similar { flex: 1; width: auto; }
  .fig-ask { flex: none; width: 40px; min-height: 40px; display: flex; align-items: center; justify-content: center; border: 1px solid rgba(52, 37, 28, 0.2); border-radius: 2px; background: transparent; color: #5f4636; cursor: pointer; transition: background 0.2s, border-color 0.2s; }
  .fig-ask:hover { background: rgba(52, 37, 28, 0.06); border-color: rgba(52, 37, 28, 0.4); }
  .fig-buy,
  .fig-similar {
    width: 100%;
    min-height: 40px;
    padding: 0 12px;
    border-radius: 2px;
    font-size: 12px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    cursor: pointer;
    transition: background 0.2s, border-color 0.2s, color 0.2s;
  }
  @media (max-width: 639px) {
    .fig-buy, .fig-similar { font-size: 11px; letter-spacing: 0.05em; padding: 0 6px; }
  }
  .fig-buy { background: #c65f3c; border: 1px solid #c65f3c; color: #fff9f0; }
  .fig-buy:hover { background: #a94d2d; border-color: #a94d2d; }
  .fig-similar { background: transparent; border: 1px solid rgba(111, 59, 36, 0.3); color: #6f3b24; }
  .fig-similar:hover { background: rgba(198, 95, 60, 0.08); border-color: #c65f3c; }

  /* Работа, открытая в панели просмотра */
  .fig-picked .fig-media { outline: 2px solid #c65f3c; outline-offset: 3px; }

  .filter-chips.filter-chips--inner { width: 100%; flex-wrap: wrap; overflow: visible; }

  /* ── Серии обложками ── */
  .series-strip {
    display: flex;
    gap: 10px;
    margin-top: 14px;
    padding: 2px 2px 8px;
    overflow-x: auto;
    scrollbar-width: thin;
  }
  .series-card {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 14px 8px 10px;
    border: 1px solid rgba(52, 37, 28, 0.14);
    border-radius: 3px;
    background: rgba(255, 249, 240, 0.55);
    color: #5f4636;
    cursor: pointer;
    transition: border-color 0.2s, background 0.2s, color 0.2s;
  }
  .series-card:hover { border-color: rgba(52, 37, 28, 0.4); color: #34251c; }
  .series-card--on { border-color: #c65f3c; background: rgba(198, 95, 60, 0.1); color: #6f3b24; }
  .series-card__stack { position: relative; display: block; width: 58px; height: 44px; }
  .series-card__thumb {
    position: absolute;
    top: 0;
    left: calc(var(--k) * 9px);
    width: 36px;
    height: 44px;
    overflow: hidden;
    border-radius: 2px;
    border: 1px solid #f8f1e7;
    box-shadow: 0 2px 6px rgba(44, 23, 16, 0.22);
    transform: rotate(calc((var(--k) - 1) * 4deg));
    z-index: calc(3 - var(--k));
  }
  .series-card__name { font-family: 'Fraunces', Georgia, serif; font-size: 15px; white-space: nowrap; }
  .series-card__n { font-size: 11px; opacity: 0.6; font-variant-numeric: tabular-nums; }

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
    .fig-just-saved {
      animation: none !important;
      transition: opacity 0.2s ease, color 0.2s ease, background 0.2s ease, border-color 0.2s ease !important;
    }
    .fig-tile { opacity: 1 !important; transform: none !important; }
  }

  @media (pointer: coarse) {
    /* iOS Safari auto-zooms on any input with font-size < 16px */
    .filter-bar__search-input {
      font-size: 16px;
      letter-spacing: 0.04em;
      text-transform: none;
    }
    /* Larger filter chip tap targets */
    .fchip { min-height: 44px; padding: 0 14px; }
    .fchip--sm { min-height: 40px; padding: 0 12px; }
    .refine-toggle, .filter-clear { min-height: 44px; }
  }

  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }

  /* ── FILTER BAR LAYOUT ─────────────────────────────────────────── */
  .filter-bar {
    margin-bottom: clamp(20px, 3vw, 36px);
    border-bottom: 1px solid rgba(52,37,28,0.10);
    padding-bottom: 16px;
  }

  .filter-bar__head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    margin-bottom: 18px;
  }

  .filter-bar__title {
    font-family: 'Fraunces', serif;
    font-size: clamp(32px, 4.4vw, 52px);
    color: rgba(111,59,36,0.90);
    letter-spacing: 0.02em;
    line-height: 1;
    margin: 0;
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
    max-width: 640px;
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
    font-family: 'Instrument Sans', sans-serif;
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

  /* Spinner shown while the semantic layer searches the descriptions. In the
     search box it replaces the magnifier (same left slot); the --lg variant is
     the centered spinner in the "searching…" empty state. */
  .filter-bar__search-spin {
    position: absolute;
    left: 9px;
    top: 50%;
    width: 12px;
    height: 12px;
    margin-top: -6px;
    border: 1.5px solid rgba(198,95,60,0.25);
    border-top-color: #c65f3c;
    border-radius: 50%;
    animation: keeper-spin 0.7s linear infinite;
    pointer-events: none;
  }
  .filter-bar__search-spin--lg {
    position: static;
    width: 22px;
    height: 22px;
    margin: 0;
    border-width: 2px;
  }
  @keyframes keeper-spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) {
    .filter-bar__search-spin { animation-duration: 2s; }
  }

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
    font-family: 'Instrument Sans', sans-serif;
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
    font-family: 'Instrument Sans', sans-serif;
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
  .filter-count { font-size: 11px; letter-spacing: 0.06em; color: #7c6554; font-variant-numeric: tabular-nums; }
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
    font-family: 'Instrument Sans', sans-serif;
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
