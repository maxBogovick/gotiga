<script lang="ts">
  import { onMount } from 'svelte';
  import { api, resolveMediaUrl } from '$lib/api';
  import { t, lang, type TranslationKey, type Lang } from '$lib/i18n';
  import {
    DEK_MAX,
    TITLE_MAX,
    decodeEntities,
    fillTemplate,
    quietDate,
    showingDateline,
    workFrameUrls,
    sketchUrlsFromWork,
    SKETCH_MAX,
    expectedWhen,
  } from '$lib/gazette';
  import type {
    Figurine,
    FigurineListItem,
    GazetteCutting,
    GazetteFeed,
    GazetteKind,
    GazetteLeaf,
    GazetteSeed,
    GazetteStatus,
    SaveGazetteFeedRequest,
    SaveGazetteLeafRequest,
  } from '$lib/types/api';
  import GazetteSlipPreview from '$lib/components/admin/GazetteSlipPreview.svelte';

  type Tab = 'notes' | 'external';
  type StatusFilter = 'all' | 'draft' | 'published';
  const PRIMARY_KINDS: GazetteKind[] = ['arrival', 'sketch', 'showing', 'note'];
  const MORE_KINDS: GazetteKind[] = ['collage', 'guest_story', 'tale'];
  const ALL_KINDS: GazetteKind[] = [...PRIMARY_KINDS, ...MORE_KINDS, 'world'];

  let {
    seed = null,
    onSeedConsumed,
  }: {
    seed?: GazetteSeed | null;
    onSeedConsumed?: () => void;
  } = $props();

  let tab = $state<Tab>('notes');
  let leaves = $state<GazetteLeaf[]>([]);
  let figurines = $state<FigurineListItem[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let message = $state('');
  let selectedId = $state<string | null>(null);
  let snapshot = $state('');
  let figQuery = $state('');
  let figOpen = $state(false);
  let picking = $state(true);
  let moreKinds = $state(false);
  let editLang = $state<Lang>('en');
  let bodyOpen = $state(false);
  let listQuery = $state('');
  let statusFilter = $state<StatusFilter>('all');
  let kindFilter = $state<GazetteKind | 'all'>('all');
  let extraFig = $state<Figurine | null>(null);
  let publishedAt = $state<string | null>(null);
  let pinned = $state(false);
  let scheduledAt = $state<string | null>(null);
  let sourceName = $state('');
  let sourceUrl = $state('');

  let kind = $state<GazetteKind>('arrival');
  let status = $state<GazetteStatus>('draft');
  let titleEn = $state('');
  let titleRu = $state('');
  let dekEn = $state('');
  let dekRu = $state('');
  let bodyEn = $state('');
  let bodyRu = $state('');
  let figurineId = $state('');
  let href = $state('');
  let imageUrls = $state<string[]>([]);
  let slug = $state('');
  type ExpectedMode = 'none' | 'day' | 'range';
  let expectedMode = $state<ExpectedMode>('none');
  let expectedFrom = $state('');
  let expectedTo = $state('');
  let watchCount = $state<number | null>(null);

  let feeds = $state<GazetteFeed[]>([]);
  let incoming = $state<GazetteCutting[]>([]);
  let incomingTotal = $state(0);
  let publishedCuts = $state<GazetteCutting[]>([]);
  let publishedCutsTotal = $state(0);
  let hiddenCuts = $state<GazetteCutting[]>([]);
  let hiddenTotal = $state(0);
  let hiddenOpen = $state(false);
  let feedsOpen = $state(false);
  let refreshing = $state(false);
  let promoting = $state(false);
  let newFeedTitle = $state('');
  let newFeedUrl = $state('');
  let uploadingLeaf = $state(false);

  const TYPE_KEY: Record<GazetteKind, TranslationKey> = {
    arrival: 'adminGazetteTplArrival',
    sketch: 'adminGazetteTplSketch',
    showing: 'adminGazetteTplShowing',
    note: 'adminGazetteTplNote',
    collage: 'adminGazetteTplCollage',
    guest_story: 'adminGazetteTplGuest',
    tale: 'adminGazetteTplTale',
    world: 'adminGazetteTplWorld',
  };

  let dirty = $derived(fieldsKey() !== snapshot);
  let selectedFig = $derived(figurines.find((f) => f.id === figurineId) ?? null);
  let figMatches = $derived.by(() => {
    const q = figQuery.trim().toLowerCase();
    let list = q
      ? figurines.filter((f) => f.name.toLowerCase().includes(q))
      : [...figurines];
    if (kind === 'sketch') {
      list = [...list].sort(
        (a, b) => Number(b.status === 'in_progress') - Number(a.status === 'in_progress'),
      );
    }
    return list.slice(0, 14);
  });
  let isPublished = $derived(status === 'published');
  let bodyMain = $derived(kind === 'tale' || kind === 'guest_story');
  let imageFirst = $derived(kind === 'collage' || kind === 'sketch');
  let multiImages = $derived(kind === 'sketch' || kind === 'collage');
  let workFirst = $derived(kind === 'arrival' || kind === 'showing' || kind === 'sketch');
  let imageUrl = $derived(imageUrls[0] ?? '');
  let frames = $derived.by(() => {
    if (kind === 'sketch') {
      if (extraFig) return sketchUrlsFromWork(extraFig);
      if (selectedFig) return sketchUrlsFromWork(selectedFig);
      return [];
    }
    return selectedFig ? workFrameUrls(selectedFig, extraFig) : [];
  });
  let composerKinds = $derived.by(() => {
    const extra = moreKinds || MORE_KINDS.includes(kind) ? MORE_KINDS : [];
    const world = kind === 'world' ? (['world'] as GazetteKind[]) : [];
    return [...PRIMARY_KINDS, ...extra, ...world];
  });
  let titleNow = $derived(editLang === 'ru' ? titleRu : titleEn);
  let dekNow = $derived(editLang === 'ru' ? dekRu : dekEn);
  let bodyNow = $derived(editLang === 'ru' ? bodyRu : bodyEn);
  let previewDate = $derived(
    quietDate(publishedAt ?? (picking ? '' : new Date().toISOString()), $lang),
  );
  let previewExpected = $derived(
    kind === 'sketch' && expectedMode !== 'none'
      ? expectedWhen(
          expectedFrom,
          expectedMode === 'range' ? expectedTo : expectedFrom,
          $lang,
          (d) => $t('gazetteExpectedAround').replace('{date}', d),
          (a, b) => $t('gazetteExpectedRange').replace('{from}', a).replace('{to}', b),
        )
      : '',
  );
  let expectedPast = $derived.by(() => {
    if (kind !== 'sketch' || expectedMode === 'none') return false;
    const end = (expectedMode === 'range' ? expectedTo : expectedFrom).slice(0, 10);
    if (!end) return false;
    const now = new Date();
    const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
    return end < today;
  });
  let existingArrival = $derived.by(() => {
    if (kind !== 'arrival' || !figurineId) return null;
    return (
      leaves.find(
        (l) =>
          l.kind === 'arrival' &&
          l.figurineId === figurineId &&
          l.status === 'published' &&
          l.id !== selectedId,
      ) ?? null
    );
  });
  let existingSketch = $derived.by(() => {
    if (kind !== 'sketch' || !figurineId) return null;
    return (
      leaves.find(
        (l) =>
          l.kind === 'sketch' &&
          l.figurineId === figurineId &&
          l.status === 'published' &&
          l.id !== selectedId,
      ) ?? null
    );
  });
  let visibleLeaves = $derived.by(() => {
    const q = listQuery.trim().toLowerCase();
    return leaves.filter((l) => {
      if (statusFilter === 'published' && l.status !== 'published') return false;
      if (statusFilter === 'draft' && l.status === 'published') return false;
      if (kindFilter !== 'all' && l.kind !== kindFilter) return false;
      if (!q) return true;
      const hay = `${l.titleEn} ${l.titleRu} ${l.figurineName ?? ''}`.toLowerCase();
      return hay.includes(q);
    });
  });

  function typeLabel(k: GazetteKind): string {
    return $t(TYPE_KEY[k]);
  }

  function publishedLabel(leaf: GazetteLeaf): string {
    return leaf.status === 'published'
      ? $t('adminGazettePublished')
      : $t('adminGazetteUnpublished');
  }

  function fieldsKey(): string {
    return JSON.stringify({
      kind,
      status,
      titleEn,
      titleRu,
      dekEn,
      dekRu,
      bodyEn,
      bodyRu,
      figurineId,
      href,
      imageUrls,
      sourceName,
      sourceUrl,
      slug,
      pinned,
      scheduledAt,
      expectedMode,
      expectedFrom,
      expectedTo,
    });
  }

  function setTitle(v: string) {
    if (editLang === 'ru') titleRu = v;
    else titleEn = v;
  }
  function setDek(v: string) {
    if (editLang === 'ru') dekRu = v;
    else dekEn = v;
  }
  function setBody(v: string) {
    if (editLang === 'ru') bodyRu = v;
    else bodyEn = v;
  }

  function flash(text: string, ms = 4000) {
    message = text;
    setTimeout(() => {
      if (message === text) message = '';
    }, ms);
  }

  function confirmLeave(): boolean {
    return !dirty || confirm($t('adminMsgUnsavedLeave'));
  }

  function blank() {
    selectedId = null;
    kind = 'arrival';
    status = 'draft';
    titleEn = '';
    titleRu = '';
    dekEn = '';
    dekRu = '';
    bodyEn = '';
    bodyRu = '';
    figurineId = '';
    href = '';
    imageUrls = [];
    slug = '';
    sourceName = '';
    sourceUrl = '';
    publishedAt = null;
    pinned = false;
    scheduledAt = null;
    expectedMode = 'none';
    expectedFrom = '';
    expectedTo = '';
    watchCount = null;
    figQuery = '';
    extraFig = null;
    bodyOpen = false;
    editLang = $lang;
    snapshot = fieldsKey();
  }

  function loadLeaf(leaf: GazetteLeaf) {
    picking = false;
    selectedId = leaf.id;
    kind = leaf.kind;
    status = leaf.status;
    titleEn = leaf.titleEn;
    titleRu = leaf.titleRu;
    dekEn = leaf.dekEn ?? '';
    dekRu = leaf.dekRu ?? '';
    bodyEn = leaf.bodyEn ?? '';
    bodyRu = leaf.bodyRu ?? '';
    figurineId = leaf.figurineId ?? '';
    href = leaf.href ?? '';
    imageUrls = leaf.imageUrls?.length
      ? leaf.imageUrls.filter(Boolean)
      : leaf.imageUrl
        ? [leaf.imageUrl]
        : [];
    slug = leaf.slug;
    sourceName = leaf.sourceName ?? '';
    sourceUrl = leaf.sourceUrl ?? '';
    publishedAt = leaf.publishedAt;
    pinned = leaf.pinned;
    scheduledAt = leaf.scheduledAt;
    const from = (leaf.expectedFrom ?? '').slice(0, 10);
    const to = (leaf.expectedTo ?? '').slice(0, 10);
    expectedFrom = from;
    expectedTo = to;
    if (!from && !to) expectedMode = 'none';
    else if (from && to && from !== to) expectedMode = 'range';
    else expectedMode = 'day';
    watchCount = leaf.watchCount ?? null;
    figQuery = '';
    bodyOpen =
      leaf.kind === 'tale' ||
      leaf.kind === 'guest_story' ||
      !!(leaf.bodyEn?.trim() || leaf.bodyRu?.trim());
    moreKinds = MORE_KINDS.includes(leaf.kind);
    editLang = $lang;
    extraFig = null;
    if (leaf.figurineId) void loadFrames(leaf.figurineId);
    snapshot = fieldsKey();
  }

  function fillEmptyFromTemplate() {
    const generic = fillTemplate(kind, '');
    const fill = fillTemplate(kind, selectedFig?.name ?? '');
    if (!titleEn.trim() || titleEn === generic.titleEn) titleEn = fill.titleEn;
    if (!titleRu.trim() || titleRu === generic.titleRu) titleRu = fill.titleRu;
    if (!dekEn.trim() || dekEn === generic.dekEn) dekEn = fill.dekEn;
    if (!dekRu.trim() || dekRu === generic.dekRu) dekRu = fill.dekRu;
  }

  function setKind(k: GazetteKind) {
    kind = k;
    if (k === 'tale' || k === 'guest_story') bodyOpen = true;
    fillEmptyFromTemplate();
  }

  function startKind(k: GazetteKind) {
    if (!picking && !confirmLeave()) return;
    blank();
    picking = false;
    kind = k;
    moreKinds = MORE_KINDS.includes(k);
    bodyOpen = k === 'tale' || k === 'guest_story';
    fillEmptyFromTemplate();
    snapshot = fieldsKey();
  }

  function addImageUrl(url: string) {
    const v = url.trim();
    if (!v || imageUrls.includes(v) || imageUrls.length >= SKETCH_MAX) return;
    imageUrls = [...imageUrls, v];
  }

  function removeImageUrl(url: string) {
    imageUrls = imageUrls.filter((u) => u !== url);
  }

  function makeCover(url: string) {
    if (!imageUrls.includes(url)) return;
    imageUrls = [url, ...imageUrls.filter((u) => u !== url)];
  }

  function pickFrame(url: string) {
    if (multiImages) addImageUrl(url);
    else imageUrls = [url];
  }

  async function loadFrames(id: string) {
    extraFig = null;
    try {
      extraFig = await api.getFigurine(id);
    } catch {
      extraFig = null;
    }
    if (kind === 'sketch' && extraFig && imageUrls.length === 0) {
      imageUrls = sketchUrlsFromWork(extraFig);
    }
  }

  function pickFigurine(id: string) {
    figurineId = id;
    figQuery = '';
    figOpen = false;
    const fig = figurines.find((f) => f.id === id);
    if (!fig) return;
    if (!href || href.startsWith('/figurines/')) href = `/figurines/${fig.slug ?? fig.id}`;
    fillEmptyFromTemplate();
    if (kind === 'sketch') {
      void loadFrames(id);
      return;
    }
    if (imageUrls.length === 0 && fig.faceImageUrl) imageUrls = [fig.faceImageUrl];
    void loadFrames(id);
  }

  function clearFigurine() {
    figurineId = '';
    figQuery = '';
    extraFig = null;
  }

  function copyOr(a: string, b: string): string {
    const left = a.trim();
    const right = b.trim();
    return left || right;
  }

  function payload(): SaveGazetteLeafRequest {
    return {
      slug: slug.trim() || null,
      kind,
      status,
      titleEn: copyOr(titleEn, titleRu),
      titleRu: copyOr(titleRu, titleEn),
      dekEn: copyOr(dekEn, dekRu) || null,
      dekRu: copyOr(dekRu, dekEn) || null,
      bodyEn: copyOr(bodyEn, bodyRu) || null,
      bodyRu: copyOr(bodyRu, bodyEn) || null,
      figurineId: figurineId || null,
      href: href.trim() || null,
      sourceName: sourceName.trim() || null,
      sourceUrl: sourceUrl.trim() || null,
      imageUrl: imageUrls[0] ?? null,
      imageUrls: imageUrls.length ? imageUrls : null,
      pinned,
      scheduledAt,
      expectedFrom:
        kind === 'sketch' && expectedMode !== 'none'
          ? expectedFrom.trim() || null
          : null,
      expectedTo:
        kind === 'sketch' && expectedMode === 'range'
          ? expectedTo.trim() || null
          : kind === 'sketch' && expectedMode === 'day'
            ? expectedFrom.trim() || null
            : null,
    };
  }

  async function loadLeaves() {
    const page = await api.adminListGazetteLeaves({ perPage: 80 });
    leaves = page.items;
  }

  async function save(next?: GazetteStatus) {
    if (!titleEn.trim() && !titleRu.trim()) {
      flash($t('adminGazetteNeedTitle'));
      return;
    }
    if (next) status = next;
    saving = true;
    message = '';
    try {
      const saved = await api.adminSaveGazetteLeaf(payload(), selectedId ?? undefined);
      flash($t('adminGazetteSaved'));
      await loadLeaves();
      loadLeaf(saved);
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      saving = false;
    }
  }

  async function destroy() {
    if (!selectedId) return;
    if (!confirm($t('adminGazetteDeleteConfirm'))) return;
    await api.adminDeleteGazetteLeaf(selectedId);
    blank();
    picking = true;
    await loadLeaves();
  }

  async function loadExternal() {
    const [f, inc, pub] = await Promise.all([
      api.adminListGazetteFeeds(),
      api.adminListGazetteCuttings({ bucket: 'inbox', perPage: 40 }),
      api.adminListGazetteCuttings({ bucket: 'table', perPage: 40 }),
    ]);
    feeds = f;
    incoming = inc.items;
    incomingTotal = inc.total;
    publishedCuts = pub.items;
    publishedCutsTotal = pub.total;
  }

  async function loadHidden() {
    const h = await api.adminListGazetteCuttings({ bucket: 'aside', perPage: 40 });
    hiddenCuts = h.items;
    hiddenTotal = h.total;
  }

  async function toggleHidden() {
    hiddenOpen = !hiddenOpen;
    if (hiddenOpen) await loadHidden();
  }

  async function refreshExternal() {
    refreshing = true;
    try {
      const r = await api.adminRefreshGazetteDesk();
      const err = r.errors?.length ? ` — ${r.errors.join('; ')}` : '';
      flash(`${$t('adminGazetteFetchedRss')}: ${r.imported}${err}`, err ? 8000 : 5000);
      await loadExternal();
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      refreshing = false;
    }
  }

  async function addFeed() {
    if (!newFeedTitle.trim() || !newFeedUrl.trim()) return;
    await api.adminSaveGazetteFeed({
      title: newFeedTitle.trim(),
      url: newFeedUrl.trim(),
      enabled: true,
    });
    newFeedTitle = '';
    newFeedUrl = '';
    await loadExternal();
  }

  async function pickImages(multiple: boolean): Promise<File[]> {
    return new Promise((resolve, reject) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = 'image/jpeg,image/png,image/webp';
      input.multiple = multiple;
      input.onchange = () => {
        const files = [...(input.files ?? [])];
        files.length ? resolve(files) : reject(new Error('cancelled'));
      };
      input.click();
    });
  }

  async function addSketchFiles(files: File[]) {
    uploadingLeaf = true;
    try {
      for (const file of files) {
        if (imageUrls.length >= SKETCH_MAX) break;
        const imported = await api.importMediaWithVariants(
          file,
          'images',
          titleEn || titleRu || 'gazette',
        );
        addImageUrl(imported.url);
      }
    } finally {
      uploadingLeaf = false;
    }
  }

  async function uploadLeafImage() {
    try {
      const files = await pickImages(multiImages);
      if (multiImages) {
        await addSketchFiles(files);
      } else {
        uploadingLeaf = true;
        try {
          const imported = await api.importMediaWithVariants(
            files[0],
            'images',
            titleEn || titleRu || 'gazette',
          );
          imageUrls = [imported.url];
        } finally {
          uploadingLeaf = false;
        }
      }
    } catch {
      // cancelled
    }
  }

  function onSketchDrop(e: DragEvent) {
    e.preventDefault();
    const files = [...(e.dataTransfer?.files ?? [])].filter((f) => f.type.startsWith('image/'));
    if (files.length) void addSketchFiles(files);
  }

  function feedBody(feed: GazetteFeed, patch: Partial<SaveGazetteFeedRequest> = {}): SaveGazetteFeedRequest {
    return {
      title: feed.title,
      url: feed.url,
      enabled: feed.enabled,
      markKey: feed.markKey ?? 'letter',
      markUrl: feed.markUrl ?? null,
      ...patch,
    };
  }

  async function toggleFeed(feed: GazetteFeed) {
    await api.adminSaveGazetteFeed(feedBody(feed, { enabled: !feed.enabled }), feed.id);
    await loadExternal();
  }

  async function removeFeed(feed: GazetteFeed) {
    if (!confirm($t('adminGazetteFeedDelete'))) return;
    await api.adminDeleteGazetteFeed(feed.id);
    await loadExternal();
  }

  async function publishCut(id: string) {
    await api.adminPinGazetteCutting(id, true);
    await loadExternal();
    if (hiddenOpen) await loadHidden();
  }

  async function unpublishCut(id: string) {
    await api.adminDismissGazetteCutting(id);
    await loadExternal();
    if (hiddenOpen) await loadHidden();
  }

  async function dismissCut(id: string) {
    await api.adminDismissGazetteCutting(id);
    await loadExternal();
  }

  async function restoreCut(id: string) {
    await api.adminRestoreGazetteCutting(id);
    await loadExternal();
    await loadHidden();
  }

  async function dismissIncoming() {
    const ids = incoming.map((c) => c.id);
    if (ids.length === 0) return;
    if (!confirm(`${$t('adminGazetteDismissAll')} (${ids.length})?`)) return;
    await Promise.all(ids.map((id) => api.adminDismissGazetteCutting(id)));
    await loadExternal();
  }

  async function promoteCut(id: string) {
    promoting = true;
    try {
      const leaf = await api.adminPromoteGazetteCutting(id);
      flash($t('adminGazetteSaved'));
      tab = 'notes';
      picking = false;
      await loadLeaves();
      loadLeaf(leaf);
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      promoting = false;
    }
  }

  async function applySeed(s: GazetteSeed) {
    tab = 'notes';
    picking = false;
    if (s.leafId) {
      const found = leaves.find((l) => l.id === s.leafId);
      if (found) {
        loadLeaf(found);
      } else {
        try {
          loadLeaf(await api.adminGetGazetteLeaf(s.leafId));
        } catch (e) {
          flash(String(e), 6000);
        }
      }
      onSeedConsumed?.();
      return;
    }
    blank();
    picking = false;
    kind = s.kind ?? 'arrival';
    moreKinds = MORE_KINDS.includes(kind);
    bodyOpen = kind === 'tale' || kind === 'guest_story';
    if (s.figurineId) pickFigurine(s.figurineId);
    else fillEmptyFromTemplate();
    if (s.imageUrls?.length) {
      imageUrls = [...new Set(s.imageUrls.map((u) => u.trim()).filter(Boolean))].slice(0, SKETCH_MAX);
    }
    if (kind === 'showing') {
      const enLine = showingDateline(s.startsAt, s.endsAt, s.venue, 'en');
      const ruLine = showingDateline(s.startsAt, s.endsAt, s.venue, 'ru');
      if (enLine) dekEn = enLine;
      if (ruLine) dekRu = ruLine;
    }
    snapshot = fieldsKey();
    onSeedConsumed?.();
  }

  function goExternal() {
    if (tab === 'notes' && !confirmLeave()) return;
    tab = 'external';
    void loadExternal();
  }

  function selectRow(leaf: GazetteLeaf) {
    if (selectedId === leaf.id && !picking) return;
    if (!confirmLeave()) return;
    loadLeaf(leaf);
  }

  function newNote() {
    if (!confirmLeave()) return;
    blank();
    picking = true;
    moreKinds = false;
  }

  function onHotkey(e: KeyboardEvent) {
    if (tab !== 'notes' || picking) return;
    if (!(e.metaKey || e.ctrlKey)) return;
    if (e.key === 's') {
      e.preventDefault();
      void save(isPublished ? 'published' : 'draft');
    } else if (e.key === 'Enter') {
      e.preventDefault();
      void save('published');
    }
  }

  function onPaste(e: ClipboardEvent) {
    if (tab !== 'notes' || picking || !multiImages) return;
    const el = e.target as HTMLElement | null;
    if (el?.closest('input, textarea, [contenteditable]')) return;
    const files = [...(e.clipboardData?.files ?? [])].filter((f) => f.type.startsWith('image/'));
    if (!files.length) return;
    e.preventDefault();
    void addSketchFiles(files);
  }

  onMount(async () => {
    try {
      const [figs] = await Promise.all([
        api.getAllFigurinesAdmin().catch(() => api.getAllFigurines()),
        loadLeaves(),
      ]);
      figurines = figs;
      if (seed && (seed.figurineId || seed.leafId || seed.kind)) {
        await applySeed(seed);
      } else {
        blank();
        picking = true;
      }
    } finally {
      loading = false;
    }
  });
</script>

<svelte:window onkeydown={onHotkey} onpaste={onPaste} />

<div class="gz-admin">
  <div class="gz-admin-head">
    <div class="tabs">
      <button class:on={tab === 'notes'} onclick={() => (tab = 'notes')}>{$t('adminGazetteLeaves')}</button>
      <button class:on={tab === 'external'} onclick={goExternal}>{$t('adminGazetteDesk')}</button>
    </div>
    {#if message}<span class="msg">{message}</span>{/if}
    {#if dirty && tab === 'notes' && !picking}<span class="dirty">{$t('adminGazetteUnsaved')}</span>{/if}
  </div>

  {#if loading}
    <p class="muted">{$t('adminLoading')}</p>
  {:else if tab === 'notes'}
    <div class="split">
      <aside class="list">
        <button class="new" onclick={newNote}>{$t('adminGazetteNew')}</button>
        <input
          class="search"
          placeholder={$t('adminGazetteSearchLeaves')}
          bind:value={listQuery}
        />
        <div class="filters">
          <button
            type="button"
            class="chip tiny-chip"
            class:on={statusFilter === 'all'}
            onclick={() => (statusFilter = 'all')}
          >{$t('adminGazetteFilterAll')}</button>
          <button
            type="button"
            class="chip tiny-chip"
            class:on={statusFilter === 'draft'}
            onclick={() => (statusFilter = 'draft')}
          >{$t('adminGazetteStatus_draft')}</button>
          <button
            type="button"
            class="chip tiny-chip"
            class:on={statusFilter === 'published'}
            onclick={() => (statusFilter = 'published')}
          >{$t('adminGazetteStatus_published')}</button>
        </div>
        <label class="lbl sr" for="gz-kind-filter">{$t('adminGazetteFilterKind')}</label>
        <select id="gz-kind-filter" bind:value={kindFilter}>
          <option value="all">{$t('adminGazetteFilterAll')}</option>
          {#each ALL_KINDS as k}
            <option value={k}>{typeLabel(k)}</option>
          {/each}
        </select>
        {#if visibleLeaves.length === 0}
          {#if leaves.length === 0}
            <div class="tpls col">
              {#each PRIMARY_KINDS as k}
                <button type="button" class="chip start" onclick={() => startKind(k)}>
                  {typeLabel(k)}
                </button>
              {/each}
            </div>
          {:else}
            <p class="muted">{$t('adminGazetteEmptyLeaves')}</p>
          {/if}
        {:else}
          {#each visibleLeaves as leaf (leaf.id)}
            <button class="row" class:sel={selectedId === leaf.id && !picking} onclick={() => selectRow(leaf)}>
              <span class="row-title">{leaf.titleEn || leaf.titleRu}</span>
              <span class="row-st">
                {typeLabel(leaf.kind)}
                · {publishedLabel(leaf)}
                {#if leaf.figurineName}
                  · {leaf.figurineName}
                {/if}
                {#if quietDate(leaf.publishedAt ?? leaf.updatedAt, $lang)}
                  · {quietDate(leaf.publishedAt ?? leaf.updatedAt, $lang)}
                {/if}
              </span>
            </button>
          {/each}
        {/if}
      </aside>

      {#if picking}
        <div class="starters">
          <p class="lbl">{$t('adminGazettePickKind')}</p>
          <p class="muted">{ $t('adminGazettePickKindHint') }</p>
          <div class="tpls">
            {#each PRIMARY_KINDS as k}
              <button type="button" class="chip start" onclick={() => startKind(k)}>{typeLabel(k)}</button>
            {/each}
            <button type="button" class="chip" class:on={moreKinds} onclick={() => (moreKinds = !moreKinds)}>
              {$t('adminGazetteMore')}
            </button>
          </div>
          {#if moreKinds}
            <div class="tpls">
              {#each MORE_KINDS as k}
                <button type="button" class="chip start" onclick={() => startKind(k)}>{typeLabel(k)}</button>
              {/each}
            </div>
          {/if}
        </div>
      {:else}
        <form
          class="editor"
          onsubmit={(e) => {
            e.preventDefault();
            void save(isPublished ? 'published' : 'draft');
          }}
        >
          <div class="actions top">
            {#if isPublished}
              <button type="submit" class="btn" disabled={saving}>{$t('adminGazetteSave')}</button>
              <button type="button" class="btn ghost" disabled={saving} onclick={() => save('archived')}>
                {$t('adminGazetteUnpublish')}
              </button>
            {:else}
              <button type="button" class="btn" disabled={saving} onclick={() => save('published')}>
                {$t('adminGazettePublish')}
              </button>
              <button type="button" class="btn ghost" disabled={saving} onclick={() => save('draft')}>
                {$t('adminGazetteSaveUnpublished')}
              </button>
            {/if}
            {#if selectedId && slug && isPublished}
              <a class="tiny linkish" href="/gazette/{slug}" target="_blank" rel="noopener noreferrer">
                {$t('adminGazetteOpenSite')}
              </a>
            {/if}
            {#if selectedId}
              <button type="button" class="btn ghost" onclick={destroy}>{$t('adminGazetteDelete')}</button>
            {/if}
          </div>

          <div class="write">
            <div class="compose">
              <p class="lbl">{$t('adminGazetteType')}</p>
              <div class="tpls">
                {#each composerKinds as k}
                  <button type="button" class="chip" class:on={kind === k} onclick={() => setKind(k)}>
                    {typeLabel(k)}
                  </button>
                {/each}
                {#if kind !== 'world'}
                  <button type="button" class="chip" class:on={moreKinds} onclick={() => (moreKinds = !moreKinds)}>
                    {$t('adminGazetteMore')}
                  </button>
                {/if}
              </div>

              <div class="tabs lang-tabs">
                <button type="button" class:on={editLang === 'en'} onclick={() => (editLang = 'en')}>
                  {$t('adminGazetteLangEn')}
                </button>
                <button type="button" class:on={editLang === 'ru'} onclick={() => (editLang = 'ru')}>
                  {$t('adminGazetteLangRu')}
                </button>
              </div>

              {#if existingArrival}
                <p class="warn">
                  {$t('adminGazetteAlreadyArrival')}
                  <button type="button" class="tiny linkish" onclick={() => loadLeaf(existingArrival)}>
                    {$t('adminGazetteOpenExisting')}
                  </button>
                </p>
              {/if}
              {#if existingSketch}
                <p class="warn">
                  {$t('adminGazetteAlreadySketch')}
                  <button type="button" class="tiny linkish" onclick={() => loadLeaf(existingSketch)}>
                    {$t('adminGazetteOpenExisting')}
                  </button>
                </p>
              {/if}

              {#if imageFirst}
                {@render imageField()}
              {/if}
              {#if workFirst}
                {@render workField()}
              {/if}
              {#if kind === 'sketch'}
                {@render expectedField()}
                {#if watchCount != null && watchCount > 0}
                  <p class="muted tight">{$t('adminGazetteWatchCount').replace('{n}', String(watchCount))}</p>
                {/if}
              {/if}

              <label class="lbl" for="gz-title">
                {$t('adminGazetteTitle')}
                <span class="count">{titleNow.trim().length}/{TITLE_MAX}</span>
              </label>
              <input
                id="gz-title"
                value={titleNow}
                maxlength={TITLE_MAX}
                oninput={(e) => setTitle(e.currentTarget.value)}
              />

              {#if bodyMain}
                {@render bodyField()}
              {/if}

              <label class="lbl" for="gz-dek">
                {$t('adminGazetteSummary')}
                <span class="count">{dekNow.trim().length}/{DEK_MAX}</span>
              </label>
              <textarea
                id="gz-dek"
                class="summary"
                rows="5"
                maxlength={DEK_MAX}
                value={dekNow}
                oninput={(e) => setDek(e.currentTarget.value)}
              ></textarea>

              {#if kind === 'world'}
                <label class="lbl" for="gz-href">{$t('adminGazetteHref')}</label>
                <input id="gz-href" bind:value={href} />
                <label class="lbl" for="gz-src">{$t('adminGazetteSourceName')}</label>
                <input id="gz-src" bind:value={sourceName} />
              {/if}

              {#if !workFirst}
                {@render workField()}
              {/if}
              {#if !imageFirst}
                {@render imageField()}
              {/if}

              {#if !bodyMain}
                <details bind:open={bodyOpen}>
                  <summary>{$t('adminGazettePageText')}</summary>
                  <div class="nested">
                    {@render bodyField()}
                  </div>
                </details>
              {/if}
            </div>

            <GazetteSlipPreview
              {kind}
              title={titleNow}
              dek={dekNow}
              {imageUrl}
              extraUrls={imageUrls.slice(1, 4)}
              dateLabel={previewExpected || previewDate}
            />
          </div>
        </form>
      {/if}
    </div>
  {:else}
    <div class="desk">
      <div class="desk-bar">
        <button class="btn" onclick={refreshExternal} disabled={refreshing}>
          {refreshing ? $t('adminGazetteFetchingRss') : $t('adminGazetteFetchRss')}
        </button>
        {#if incoming.length > 0}
          <button class="tiny" onclick={dismissIncoming}>{$t('adminGazetteDismissAll')}</button>
        {/if}
        <button class="tiny" onclick={() => (feedsOpen = !feedsOpen)}>
          {$t('adminGazetteRssSourcesToggle')}
        </button>
      </div>

      {#if feedsOpen}
        <section class="feeds-box">
          <h3 class="sub">{$t('adminGazetteRssSources')}</h3>
          <div class="feed-add">
            <input placeholder={$t('adminGazetteFeedTitle')} bind:value={newFeedTitle} />
            <input placeholder={$t('adminGazetteFeedUrl')} bind:value={newFeedUrl} />
            <button class="btn" type="button" onclick={addFeed}>{$t('adminGazetteFeedAdd')}</button>
          </div>
          <ul class="feeds">
            {#each feeds as feed (feed.id)}
              <li>
                <label class="check">
                  <input type="checkbox" checked={feed.enabled} onchange={() => toggleFeed(feed)} />
                  <strong>{feed.title}</strong>
                </label>
                <span class="muted">{feed.url}</span>
                {#if feed.lastFetchedAt}
                  <span class="muted">{$t('adminGazetteLastHeard')}: {quietDate(feed.lastFetchedAt, $lang)}</span>
                {/if}
                {#if feed.lastError}<span class="err">{feed.lastError}</span>{/if}
                <button class="tiny" onclick={() => removeFeed(feed)}>{$t('adminGazetteDelete')}</button>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      <h3 class="sub">{$t('adminGazetteIncoming')} {#if incomingTotal}({incomingTotal}){/if}</h3>
      {#if incoming.length === 0}
        <p class="muted">{$t('adminGazetteNoIncoming')}</p>
      {:else}
        <ul class="cuts">
          {#each incoming as cut (cut.id)}
            {@render cutRow(cut, 'inbox')}
          {/each}
        </ul>
      {/if}

      <h3 class="sub">{$t('adminGazettePublishedExternal')} {#if publishedCutsTotal}({publishedCutsTotal}){/if}</h3>
      {#if publishedCuts.length === 0}
        <p class="muted">{$t('adminGazetteNoPublishedExternal')}</p>
      {:else}
        <ul class="cuts">
          {#each publishedCuts as cut (cut.id)}
            {@render cutRow(cut, 'table')}
          {/each}
        </ul>
      {/if}

      <button class="tiny linkish" onclick={toggleHidden}>
        {$t('adminGazetteHidden')}{hiddenOpen && hiddenTotal ? ` (${hiddenTotal})` : ''}
      </button>
      {#if hiddenOpen}
        {#if hiddenCuts.length === 0}
          <p class="muted">{$t('adminGazetteEmptyHidden')}</p>
        {:else}
          <ul class="cuts dim-list">
            {#each hiddenCuts as cut (cut.id)}
              {@render cutRow(cut, 'aside')}
            {/each}
          </ul>
        {/if}
      {/if}
    </div>
  {/if}
</div>

{#snippet expectedField()}
  <p class="lbl">{$t('adminGazetteExpected')}</p>
  <div class="tpls">
    <button type="button" class="chip" class:on={expectedMode === 'none'} onclick={() => (expectedMode = 'none')}>
      {$t('adminGazetteExpectedNone')}
    </button>
    <button type="button" class="chip" class:on={expectedMode === 'day'} onclick={() => (expectedMode = 'day')}>
      {$t('adminGazetteExpectedDay')}
    </button>
    <button type="button" class="chip" class:on={expectedMode === 'range'} onclick={() => (expectedMode = 'range')}>
      {$t('adminGazetteExpectedSpan')}
    </button>
  </div>
  {#if expectedMode === 'day'}
    <label class="lbl" for="gz-exp-day">{$t('adminGazetteExpectedDay')}</label>
    <input id="gz-exp-day" type="date" bind:value={expectedFrom} />
  {:else if expectedMode === 'range'}
    <div class="span-row">
      <div>
        <label class="lbl" for="gz-exp-from">{$t('adminGazetteExpectedFrom')}</label>
        <input id="gz-exp-from" type="date" bind:value={expectedFrom} />
      </div>
      <div>
        <label class="lbl" for="gz-exp-to">{$t('adminGazetteExpectedTo')}</label>
        <input id="gz-exp-to" type="date" bind:value={expectedTo} />
      </div>
    </div>
  {/if}
  {#if expectedPast}
    <p class="warn">{$t('adminGazetteExpectedPast')}</p>
  {/if}
{/snippet}

{#snippet workField()}
  <label class="lbl" for="gz-fig-search">{$t('adminGazetteWork')}</label>
  {#if kind === 'sketch'}
    <p class="muted tight">{$t('adminGazetteWorkOptional')}</p>
  {/if}
  {#if selectedFig}
    <div class="picked">
      <span class="picked-name">{selectedFig.name}</span>
      <button type="button" class="tiny" onclick={clearFigurine}>{$t('adminGazetteWorkNone')}</button>
    </div>
  {:else}
    <input
      id="gz-fig-search"
      placeholder={$t('adminGazetteWorkSearch')}
      bind:value={figQuery}
      onfocus={() => (figOpen = true)}
      oninput={() => (figOpen = true)}
    />
    {#if figOpen && figMatches.length > 0}
      <ul class="suggest">
        {#each figMatches as fig (fig.id)}
          <li>
            <button type="button" onclick={() => pickFigurine(fig.id)}>
              {#if fig.faceImageUrl}
                <img src={resolveMediaUrl(fig.faceImageUrl) ?? fig.faceImageUrl} alt="" />
              {/if}
              <span class="suggest-copy">
                {fig.name}
                {#if kind === 'sketch' && fig.status === 'in_progress'}
                  <span class="suggest-st">{$t('figurineStatusInProgress')}</span>
                {/if}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
{/snippet}

{#snippet imageField()}
  <p class="lbl">{multiImages ? $t('adminGazetteSketches') : $t('adminGazetteImage')}</p>
  {#if multiImages}
    <p class="muted tight">{$t('adminGazetteSketchesHint')}</p>
    {#if imageUrls.length === 0}
      <button
        type="button"
        class="drop empty"
        class:busy={uploadingLeaf}
        disabled={uploadingLeaf}
        ondragover={(e) => e.preventDefault()}
        ondrop={onSketchDrop}
        onclick={() => void uploadLeafImage()}
      >
        <span class="muted tight">{$t('adminGazetteSketchesDrop')}</span>
      </button>
    {:else}
      <div
        class="drop"
        ondragover={(e) => e.preventDefault()}
        ondrop={onSketchDrop}
      >
        <div class="frames">
          {#each imageUrls as url, i (url)}
            <div class="frame-wrap">
              <button
                type="button"
                class="frame"
                class:on={i === 0}
                onclick={() => makeCover(url)}
                title={$t('adminGazetteSketchCover')}
              >
                <img src={resolveMediaUrl(url) ?? url} alt="" />
              </button>
              <button type="button" class="tiny frame-x" onclick={() => removeImageUrl(url)}>×</button>
            </div>
          {/each}
        </div>
        <div class="pic-act">
          <button type="button" class="tiny" onclick={uploadLeafImage} disabled={uploadingLeaf || imageUrls.length >= SKETCH_MAX}>
            {uploadingLeaf ? '…' : $t('adminGazetteSketchesAdd')}
          </button>
          {#if imageUrls.length >= SKETCH_MAX}
            <span class="muted tight">{$t('adminGazetteSketchesFull')}</span>
          {/if}
        </div>
      </div>
    {/if}
  {/if}
  {#if frames.length > 0}
    <p class="muted tight">{$t('adminGazetteWorkFrames')}</p>
    <div class="frames">
      {#each frames as url (url)}
        <button
          type="button"
          class="frame"
          class:on={imageUrls.includes(url)}
          onclick={() => pickFrame(url)}
        >
          <img src={resolveMediaUrl(url) ?? url} alt="" />
        </button>
      {/each}
    </div>
  {/if}
  {#if !multiImages}
    <div class="pic">
      {#if imageUrl}
        <img src={resolveMediaUrl(imageUrl) ?? imageUrl} alt="" class="pic-preview" />
      {/if}
      <div class="pic-act">
        <button type="button" class="tiny" onclick={uploadLeafImage} disabled={uploadingLeaf}>
          {uploadingLeaf ? '…' : imageUrl ? $t('adminGazetteImageChange') : $t('adminGazetteImageUpload')}
        </button>
        {#if imageUrl}
          <button type="button" class="tiny" onclick={() => (imageUrls = [])}>{$t('adminGazetteImageClear')}</button>
        {/if}
      </div>
    </div>
  {/if}
{/snippet}

{#snippet bodyField()}
  <label class="lbl" for="gz-body">
    {$t('adminGazetteBody')}
  </label>
  <textarea
    id="gz-body"
    class="body"
    rows={bodyMain ? 12 : 8}
    value={bodyNow}
    oninput={(e) => setBody(e.currentTarget.value)}
  ></textarea>
  <p class="muted tight">{$t('adminGazetteBodyHint')}</p>
{/snippet}

{#snippet cutRow(cut: GazetteCutting, bucket: 'inbox' | 'table' | 'aside')}
  <li class:on-site={bucket === 'table'}>
    <a href={cut.url} target="_blank" rel="noopener noreferrer">{decodeEntities(cut.title)}</a>
    <span class="muted">
      {cut.sourceName}
      {#if quietDate(cut.publishedAt ?? cut.createdAt, $lang)}
        · {quietDate(cut.publishedAt ?? cut.createdAt, $lang)}
      {/if}
    </span>
    {#if cut.summary && bucket === 'inbox'}<p>{decodeEntities(cut.summary)}</p>{/if}
    <div class="cut-act">
      {#if bucket === 'inbox'}
        <button class="tiny" onclick={() => publishCut(cut.id)}>{$t('adminGazettePublish')}</button>
        <button class="tiny" onclick={() => dismissCut(cut.id)}>{$t('adminGazetteDontNeed')}</button>
      {:else if bucket === 'table'}
        <button class="tiny" onclick={() => unpublishCut(cut.id)}>{$t('adminGazetteUnpublish')}</button>
      {:else}
        <button class="tiny" onclick={() => restoreCut(cut.id)}>{$t('adminGazetteRestore')}</button>
        <button class="tiny" onclick={() => publishCut(cut.id)}>{$t('adminGazettePublish')}</button>
      {/if}
      <button class="tiny" disabled={promoting} onclick={() => promoteCut(cut.id)}>
        {$t('adminGazetteAsHouseLeaf')}
      </button>
    </div>
  </li>
{/snippet}

<style>
  .gz-admin,
  .gz-admin * { box-sizing: border-box; }
  .gz-admin { height: 100%; min-height: 0; display: flex; flex-direction: column; color: #34251c; }
  .gz-admin-head { display: flex; gap: 16px; align-items: center; margin-bottom: 16px; flex-wrap: wrap; flex-shrink: 0; }
  .msg { font-size: 12px; color: #6f3b24; }
  .dirty { font-size: 11px; color: #c65f3c; }
  .tabs { display: flex; gap: 6px; }
  .tabs button, .chip, .new, .btn, .tiny {
    font-size: 11px;
    letter-spacing: 0.04em;
    border: 1px solid #d8c6b1;
    background: #fff9f0;
    color: #34251c;
    padding: 6px 10px;
    cursor: pointer;
  }
  .tabs button.on, .chip.on { background: #34251c; color: #f8f1e7; }
  .chip.start { min-height: 2.4rem; }
  .tiny-chip { padding: 4px 8px; font-size: 10px; }
  .split {
    display: grid;
    grid-template-columns: minmax(180px, 240px) minmax(0, 1fr);
    gap: 20px;
    min-height: 0;
    min-width: 0;
    flex: 1;
    overflow: hidden;
  }
  .list { overflow: auto; border-right: 1px solid #d8c6b1; padding-right: 12px; display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .row { display: grid; text-align: left; gap: 2px; padding: 8px; background: transparent; border: 1px solid transparent; cursor: pointer; }
  .row.sel { border-color: #c65f3c; background: #fff9f0; }
  .row-st { font-size: 10px; color: #6f3b24; }
  .row-title { font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .search { margin: 0; }
  .filters { display: flex; flex-wrap: wrap; gap: 4px; }
  .starters {
    min-width: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 8px 12px 32px 0;
  }
  .editor {
    min-width: 0;
    overflow-x: hidden;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
    padding: 0 12px 32px 0;
  }
  .editor > * { flex: 0 0 auto; max-width: 100%; min-width: 0; }
  .write {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(180px, 240px);
    gap: 20px;
    align-items: start;
  }
  .compose { display: flex; flex-direction: column; gap: 10px; min-width: 0; }
  .lbl { font-size: 10px; text-transform: uppercase; letter-spacing: 0.08em; color: #5f4636; font-weight: 700; display: flex; justify-content: space-between; align-items: baseline; gap: 8px; }
  .count { font-weight: 500; letter-spacing: 0; text-transform: none; color: #8a6a55; flex-shrink: 0; }
  .tpls { display: flex; flex-wrap: wrap; gap: 6px; }
  .tpls.col { flex-direction: column; align-items: stretch; }
  input, select, textarea {
    display: block;
    width: 100%;
    max-width: 100%;
    min-width: 0;
    flex-shrink: 0;
    background: #f8f1e7;
    border: 1px solid rgba(198, 95, 60, 0.25);
    padding: 0.65rem 0.75rem;
    color: #34251c;
    font-size: 13px;
    line-height: 1.45;
    font-family: inherit;
  }
  input { min-height: 2.6rem; }
  textarea.summary {
    min-height: 7.5rem;
    height: 7.5rem;
    resize: vertical;
  }
  textarea.body {
    min-height: 10rem;
    height: 10rem;
    resize: vertical;
  }
  .check { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .check input { width: auto; min-height: 0; }
  .actions { display: flex; gap: 10px; flex-wrap: wrap; align-items: center; }
  .actions.top { position: sticky; top: 0; z-index: 2; background: #fff9f0; padding: 4px 0 8px; }
  .btn { background: #6f3b24; color: #f8f1e7; border-color: #6f3b24; }
  .btn.ghost { background: transparent; color: #6f3b24; }
  .muted { font-size: 12px; color: #5f4636; }
  .muted.tight { margin: 0; }
  .tiny { flex-shrink: 0; }
  .tiny.linkish { border: none; background: none; padding: 0; text-decoration: underline; text-underline-offset: 3px; }
  a.tiny.linkish { display: inline-flex; align-items: center; }
  .warn {
    font-size: 12px;
    color: #6f3b24;
    border: 1px solid #d8c6b1;
    background: #fff9f0;
    padding: 8px 10px;
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
    margin: 0;
  }
  .picked { display: flex; align-items: center; justify-content: space-between; gap: 8px; border: 1px solid #d8c6b1; padding: 8px 10px; background: #fff9f0; font-size: 13px; min-height: 2.6rem; }
  .picked-name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .suggest { list-style: none; margin: 0; padding: 0; border: 1px solid #d8c6b1; background: #fff9f0; max-height: 220px; overflow: auto; }
  .suggest button { width: 100%; text-align: left; background: none; border: none; padding: 8px 10px; cursor: pointer; font-size: 13px; display: flex; align-items: center; gap: 8px; }
  .suggest button:hover { background: #f8f1e7; }
  .suggest img { width: 28px; height: 36px; object-fit: cover; border: 1px solid #d8c6b1; background: #1a120e; flex-shrink: 0; }
  .suggest-copy { display: grid; gap: 1px; min-width: 0; }
  .suggest-st { font-size: 10px; letter-spacing: 0.06em; text-transform: uppercase; color: #8a6a55; }
  details { border-top: 1px solid #d8c6b1; padding-top: 10px; display: block; }
  summary { cursor: pointer; font-size: 12px; color: #5f4636; margin-bottom: 8px; }
  .nested { display: flex; flex-direction: column; gap: 10px; }
  .nested > * { flex: 0 0 auto; }
  .pic { display: flex; align-items: flex-start; gap: 12px; }
  .pic-preview { width: 72px; height: 72px; object-fit: cover; border: 1px solid #d8c6b1; background: #1a120e; flex-shrink: 0; }
  .pic-act { display: flex; gap: 8px; flex-wrap: wrap; }
  .frames { display: flex; gap: 8px; flex-wrap: wrap; }
  .frame {
    width: 48px;
    height: 60px;
    padding: 0;
    border: 1px solid #d8c6b1;
    background: #1a120e;
    cursor: pointer;
  }
  .frame.on { border-color: #c65f3c; outline: 1px solid #c65f3c; }
  .frame img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .frame-wrap { position: relative; }
  .frame-x {
    position: absolute;
    top: -6px;
    right: -6px;
    padding: 0 5px;
    line-height: 1.3;
    background: #fff9f0;
  }
  .drop {
    border: 1px dashed #d8c6b1;
    background: #fff9f0;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    text-align: left;
    font: inherit;
    color: inherit;
  }
  .drop.empty {
    min-height: 7.5rem;
    justify-content: center;
    cursor: pointer;
  }
  .drop.busy { cursor: wait; }
  .span-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .lang-tabs { margin: 2px 0; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0, 0, 0, 0); }
  .desk { overflow: auto; min-width: 0; }
  .desk-bar, .feed-add { display: flex; gap: 10px; align-items: center; margin-bottom: 16px; flex-wrap: wrap; }
  .sub { font-size: 12px; letter-spacing: 0.06em; text-transform: uppercase; margin: 16px 0 8px; }
  .feeds-box { border: 1px solid #d8c6b1; padding: 12px; margin-bottom: 16px; background: #fff9f0; }
  .feeds, .cuts { list-style: none; padding: 0; margin: 0 0 16px; display: flex; flex-direction: column; gap: 10px; }
  .feeds li, .cuts li { border: 1px solid #d8c6b1; padding: 10px 12px; background: #fff9f0; display: flex; flex-direction: column; gap: 4px; }
  .cuts li.on-site { border-color: #c65f3c; }
  .cut-act { display: flex; gap: 8px; margin-top: 6px; flex-wrap: wrap; }
  .dim-list li { opacity: 0.7; }
  .err { color: #8a2a2a; font-size: 11px; display: block; }
  @media (max-width: 1100px) {
    .write { grid-template-columns: 1fr; }
  }
  @media (max-width: 900px) {
    .split { grid-template-columns: 1fr; overflow: auto; }
    .list { border-right: none; border-bottom: 1px solid #d8c6b1; padding-right: 0; padding-bottom: 12px; max-height: 220px; }
  }
</style>
