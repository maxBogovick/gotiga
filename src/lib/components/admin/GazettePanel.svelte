<script lang="ts">
  import { onMount } from 'svelte';
  import { api, resolveMediaUrl } from '$lib/api';
  import { t, lang, type TranslationKey } from '$lib/i18n';
  import { DEK_MAX, TITLE_MAX, decodeEntities, fillTemplate, quietDate } from '$lib/gazette';
  import type {
    FigurineListItem,
    GazetteCutting,
    GazetteFeed,
    GazetteKind,
    GazetteLeaf,
    GazetteStatus,
    SaveGazetteFeedRequest,
    SaveGazetteLeafRequest,
  } from '$lib/types/api';

  type Tab = 'notes' | 'external';
  const NOTE_TYPES: GazetteKind[] = ['arrival', 'showing', 'note'];

  let {
    seed = null,
    onSeedConsumed,
  }: {
    seed?: { figurineId: string; kind?: GazetteKind } | null;
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
  let russianOpen = $state(false);
  let figQuery = $state('');
  let figOpen = $state(false);

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
  let imageUrl = $state('');
  let slug = $state('');

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
  let newFeedTitle = $state('');
  let newFeedUrl = $state('');
  let uploadingLeaf = $state(false);

  const TYPE_KEY: Record<GazetteKind, TranslationKey> = {
    arrival: 'adminGazetteTypeArrival',
    showing: 'adminGazetteTypeShowing',
    note: 'adminGazetteTypeNote',
    collage: 'adminGazetteTypeNote',
    guest_story: 'adminGazetteTypeNote',
    tale: 'adminGazetteTypeNote',
    world: 'adminGazetteTypeExternal',
  };

  let dirty = $derived(JSON.stringify(payload()) !== snapshot);
  let selectedFig = $derived(figurines.find((f) => f.id === figurineId) ?? null);
  let figMatches = $derived.by(() => {
    const q = figQuery.trim().toLowerCase();
    const list = q
      ? figurines.filter((f) => f.name.toLowerCase().includes(q))
      : figurines;
    return list.slice(0, 14);
  });
  let isPublished = $derived(status === 'published');
  let visibleTypes = $derived.by(() => {
    if (NOTE_TYPES.includes(kind)) return NOTE_TYPES;
    return [kind, ...NOTE_TYPES.filter((k) => k !== kind)];
  });

  function typeLabel(k: GazetteKind): string {
    return $t(TYPE_KEY[k]);
  }

  function publishedLabel(leaf: GazetteLeaf): string {
    return leaf.status === 'published'
      ? $t('adminGazettePublished')
      : $t('adminGazetteUnpublished');
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
    imageUrl = '';
    slug = '';
    russianOpen = false;
    figQuery = '';
    snapshot = JSON.stringify(payload());
  }

  function loadLeaf(leaf: GazetteLeaf) {
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
    imageUrl = leaf.imageUrl ?? '';
    slug = leaf.slug;
    russianOpen = !!(leaf.titleRu.trim() && leaf.titleRu !== leaf.titleEn);
    figQuery = '';
    snapshot = JSON.stringify(payload());
  }

  function fillEmptyFromTemplate() {
    const fill = fillTemplate(kind, selectedFig?.name ?? '');
    if (!titleEn.trim()) titleEn = fill.titleEn;
    if (!titleRu.trim()) titleRu = fill.titleRu;
    if (!dekEn.trim()) dekEn = fill.dekEn;
    if (!dekRu.trim()) dekRu = fill.dekRu;
  }

  function setKind(k: GazetteKind) {
    kind = k;
    fillEmptyFromTemplate();
  }

  function pickFigurine(id: string) {
    figurineId = id;
    figQuery = '';
    figOpen = false;
    const fig = figurines.find((f) => f.id === id);
    if (!fig) return;
    if (!imageUrl) imageUrl = fig.faceImageUrl ?? '';
    if (!href || href.startsWith('/figurines/')) href = `/figurines/${fig.slug ?? fig.id}`;
    fillEmptyFromTemplate();
  }

  function clearFigurine() {
    figurineId = '';
    figQuery = '';
  }

  function payload(): SaveGazetteLeafRequest {
    return {
      slug: slug.trim() || null,
      kind,
      status,
      titleEn: titleEn.trim(),
      titleRu: titleRu.trim(),
      dekEn: dekEn.trim() || null,
      dekRu: dekRu.trim() || null,
      bodyEn: bodyEn.trim() || null,
      bodyRu: bodyRu.trim() || null,
      figurineId: figurineId || null,
      href: href.trim() || null,
      sourceName: null,
      sourceUrl: null,
      imageUrl: imageUrl.trim() || null,
      pinned: false,
      scheduledAt: null,
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

  async function pickImage(): Promise<File> {
    return new Promise((resolve, reject) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = 'image/jpeg,image/png,image/webp';
      input.onchange = () => {
        const f = input.files?.[0];
        f ? resolve(f) : reject(new Error('cancelled'));
      };
      input.click();
    });
  }

  async function uploadLeafImage() {
    uploadingLeaf = true;
    try {
      const file = await pickImage();
      const imported = await api.importMediaWithVariants(
        file,
        'images',
        titleEn || titleRu || 'gazette',
      );
      imageUrl = imported.url;
    } catch {
      // cancelled
    } finally {
      uploadingLeaf = false;
    }
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

  function applySeed() {
    if (!seed?.figurineId) return;
    blank();
    kind = seed.kind && NOTE_TYPES.includes(seed.kind) ? seed.kind : 'arrival';
    pickFigurine(seed.figurineId);
    snapshot = JSON.stringify(payload());
    onSeedConsumed?.();
  }

  function goExternal() {
    if (tab === 'notes' && !confirmLeave()) return;
    tab = 'external';
    void loadExternal();
  }

  function selectRow(leaf: GazetteLeaf) {
    if (selectedId === leaf.id) return;
    if (!confirmLeave()) return;
    loadLeaf(leaf);
  }

  function newNote() {
    if (!confirmLeave()) return;
    blank();
  }

  onMount(async () => {
    try {
      const [figs] = await Promise.all([
        api.getAllFigurinesAdmin().catch(() => api.getAllFigurines()),
        loadLeaves(),
      ]);
      figurines = figs;
      if (seed?.figurineId) applySeed();
      else blank();
    } finally {
      loading = false;
    }
  });
</script>

<div class="gz-admin">
  <div class="gz-admin-head">
    <div class="tabs">
      <button class:on={tab === 'notes'} onclick={() => (tab = 'notes')}>{$t('adminGazetteLeaves')}</button>
      <button class:on={tab === 'external'} onclick={goExternal}>{$t('adminGazetteDesk')}</button>
    </div>
    {#if message}<span class="msg">{message}</span>{/if}
    {#if dirty && tab === 'notes'}<span class="dirty">{$t('adminGazetteUnsaved')}</span>{/if}
  </div>

  {#if loading}
    <p class="muted">{$t('adminLoading')}</p>
  {:else if tab === 'notes'}
    <div class="split">
      <aside class="list">
        <button class="new" onclick={newNote}>{$t('adminGazetteNew')}</button>
        {#if leaves.length === 0}
          <p class="muted">{$t('adminGazetteEmptyLeaves')}</p>
        {:else}
          {#each leaves as leaf (leaf.id)}
            <button class="row" class:sel={selectedId === leaf.id} onclick={() => selectRow(leaf)}>
              <span class="row-title">{leaf.titleEn || leaf.titleRu}</span>
              <span class="row-st">
                {typeLabel(leaf.kind)}
                · {publishedLabel(leaf)}
                {#if quietDate(leaf.publishedAt ?? leaf.updatedAt, $lang)}
                  · {quietDate(leaf.publishedAt ?? leaf.updatedAt, $lang)}
                {/if}
              </span>
            </button>
          {/each}
        {/if}
      </aside>

      <form
        class="editor"
        onsubmit={(e) => {
          e.preventDefault();
          void save(isPublished ? 'published' : 'draft');
        }}
      >
        <p class="lbl">{$t('adminGazetteType')}</p>
        <div class="tpls">
          {#each visibleTypes as k}
            <button type="button" class="chip" class:on={kind === k} onclick={() => setKind(k)}>
              {typeLabel(k)}
            </button>
          {/each}
        </div>

        <label class="lbl" for="gz-fig-search">{$t('adminGazetteWork')}</label>
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
                  <button type="button" onclick={() => pickFigurine(fig.id)}>{fig.name}</button>
                </li>
              {/each}
            </ul>
          {/if}
        {/if}

        <label class="lbl" for="gz-te">
          {$t('adminGazetteTitleEn')}
          <span class="count">{titleEn.trim().length}/{TITLE_MAX}</span>
        </label>
        <input id="gz-te" bind:value={titleEn} maxlength={TITLE_MAX} />

        <label class="lbl" for="gz-de">
          {$t('adminGazetteDekEn')}
          <span class="count">{dekEn.trim().length}/{DEK_MAX}</span>
        </label>
        <textarea id="gz-de" class="summary" rows="5" bind:value={dekEn} maxlength={DEK_MAX}></textarea>

        <label class="lbl">{$t('adminGazetteImage')}</label>
        <div class="pic">
          {#if imageUrl}
            <img src={resolveMediaUrl(imageUrl) ?? imageUrl} alt="" class="pic-preview" />
          {/if}
          <div class="pic-act">
            <button type="button" class="tiny" onclick={uploadLeafImage} disabled={uploadingLeaf}>
              {uploadingLeaf ? '…' : imageUrl ? $t('adminGazetteImageChange') : $t('adminGazetteImageUpload')}
            </button>
            {#if imageUrl}
              <button type="button" class="tiny" onclick={() => (imageUrl = '')}>{$t('adminGazetteImageClear')}</button>
            {/if}
          </div>
        </div>

        <div class="actions">
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

        <details bind:open={russianOpen}>
          <summary>{$t('adminGazetteLangRu')}</summary>
          <div class="nested">
            <label class="lbl" for="gz-tr">
              {$t('adminGazetteTitleRu')}
              <span class="count">{titleRu.trim().length}/{TITLE_MAX}</span>
            </label>
            <input id="gz-tr" bind:value={titleRu} maxlength={TITLE_MAX} />
            <label class="lbl" for="gz-dr">
              {$t('adminGazetteDekRu')}
              <span class="count">{dekRu.trim().length}/{DEK_MAX}</span>
            </label>
            <textarea id="gz-dr" class="summary" rows="5" bind:value={dekRu} maxlength={DEK_MAX}></textarea>
          </div>
        </details>

        <details>
          <summary>{$t('adminGazettePageText')}</summary>
          <div class="nested">
            <label class="lbl" for="gz-be">{$t('adminGazetteBodyEn')}</label>
            <textarea id="gz-be" class="body" rows="8" bind:value={bodyEn}></textarea>
            <label class="lbl" for="gz-br">{$t('adminGazetteBodyRu')}</label>
            <textarea id="gz-br" class="body" rows="8" bind:value={bodyRu}></textarea>
          </div>
        </details>
      </form>
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
            <li>
              <a href={cut.url} target="_blank" rel="noopener noreferrer">{decodeEntities(cut.title)}</a>
              <span class="muted">
                {cut.sourceName}
                {#if quietDate(cut.publishedAt ?? cut.createdAt, $lang)}
                  · {quietDate(cut.publishedAt ?? cut.createdAt, $lang)}
                {/if}
              </span>
              {#if cut.summary}<p>{decodeEntities(cut.summary)}</p>{/if}
              <div class="cut-act">
                <button class="tiny" onclick={() => publishCut(cut.id)}>{$t('adminGazettePublish')}</button>
                <button class="tiny" onclick={() => dismissCut(cut.id)}>{$t('adminGazetteDontNeed')}</button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}

      <h3 class="sub">{$t('adminGazettePublishedExternal')} {#if publishedCutsTotal}({publishedCutsTotal}){/if}</h3>
      {#if publishedCuts.length === 0}
        <p class="muted">{$t('adminGazetteNoPublishedExternal')}</p>
      {:else}
        <ul class="cuts">
          {#each publishedCuts as cut (cut.id)}
            <li class="on-site">
              <a href={cut.url} target="_blank" rel="noopener noreferrer">{decodeEntities(cut.title)}</a>
              <span class="muted">{cut.sourceName}</span>
              <div class="cut-act">
                <button class="tiny" onclick={() => unpublishCut(cut.id)}>{$t('adminGazetteUnpublish')}</button>
              </div>
            </li>
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
              <li>
                <a href={cut.url} target="_blank" rel="noopener noreferrer">{decodeEntities(cut.title)}</a>
                <span class="muted">{cut.sourceName}</span>
                <div class="cut-act">
                  <button class="tiny" onclick={() => restoreCut(cut.id)}>{$t('adminGazetteRestore')}</button>
                  <button class="tiny" onclick={() => publishCut(cut.id)}>{$t('adminGazettePublish')}</button>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </div>
  {/if}
</div>

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
  .lbl { font-size: 10px; text-transform: uppercase; letter-spacing: 0.08em; color: #5f4636; font-weight: 700; display: flex; justify-content: space-between; align-items: baseline; gap: 8px; }
  .count { font-weight: 500; letter-spacing: 0; text-transform: none; color: #8a6a55; flex-shrink: 0; }
  .tpls { display: flex; flex-wrap: wrap; gap: 6px; }
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
  .actions { display: flex; gap: 10px; margin-top: 4px; flex-wrap: wrap; align-items: center; }
  .btn { background: #6f3b24; color: #f8f1e7; border-color: #6f3b24; }
  .btn.ghost { background: transparent; color: #6f3b24; }
  .muted { font-size: 12px; color: #5f4636; }
  .tiny { flex-shrink: 0; }
  .tiny.linkish { border: none; background: none; padding: 0; text-decoration: underline; text-underline-offset: 3px; }
  a.tiny.linkish { display: inline-flex; align-items: center; }
  .picked { display: flex; align-items: center; justify-content: space-between; gap: 8px; border: 1px solid #d8c6b1; padding: 8px 10px; background: #fff9f0; font-size: 13px; min-height: 2.6rem; }
  .picked-name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .suggest { list-style: none; margin: 0; padding: 0; border: 1px solid #d8c6b1; background: #fff9f0; max-height: 220px; overflow: auto; }
  .suggest button { width: 100%; text-align: left; background: none; border: none; padding: 8px 10px; cursor: pointer; font-size: 13px; }
  .suggest button:hover { background: #f8f1e7; }
  details { border-top: 1px solid #d8c6b1; padding-top: 10px; display: block; }
  summary { cursor: pointer; font-size: 12px; color: #5f4636; margin-bottom: 8px; }
  .nested { display: flex; flex-direction: column; gap: 10px; }
  .nested > * { flex: 0 0 auto; }
  .pic { display: flex; align-items: flex-start; gap: 12px; }
  .pic-preview { width: 72px; height: 72px; object-fit: cover; border: 1px solid #d8c6b1; background: #1a120e; flex-shrink: 0; }
  .pic-act { display: flex; gap: 8px; flex-wrap: wrap; }
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
  @media (max-width: 900px) {
    .split { grid-template-columns: 1fr; overflow: auto; }
    .list { border-right: none; border-bottom: 1px solid #d8c6b1; padding-right: 0; padding-bottom: 12px; max-height: 220px; }
  }
</style>
