<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t, type TranslationKey } from '$lib/i18n';
  import { fillTemplate } from '$lib/gazette';
  import type {
    FigurineListItem,
    GazetteCutting,
    GazetteFeed,
    GazetteKind,
    GazetteLeaf,
    GazetteStatus,
    SaveGazetteLeafRequest,
  } from '$lib/types/api';

  type Tab = 'leaves' | 'desk';

  let tab = $state<Tab>('leaves');
  let leaves = $state<GazetteLeaf[]>([]);
  let figurines = $state<FigurineListItem[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let message = $state('');
  let selectedId = $state<string | null>(null);

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
  let sourceName = $state('');
  let sourceUrl = $state('');
  let imageUrl = $state('');
  let slug = $state('');
  let pinned = $state(false);
  let scheduledAt = $state('');

  let feeds = $state<GazetteFeed[]>([]);
  let cuttings = $state<GazetteCutting[]>([]);
  let showDismissed = $state(false);
  let refreshing = $state(false);
  let newFeedTitle = $state('');
  let newFeedUrl = $state('');

  const KINDS: GazetteKind[] = ['arrival', 'collage', 'showing', 'guest_story', 'tale', 'note', 'world'];
  const STATUSES: GazetteStatus[] = ['draft', 'scheduled', 'published', 'archived'];
  const TPL: { kind: GazetteKind; key: TranslationKey }[] = [
    { kind: 'arrival', key: 'adminGazetteTplArrival' },
    { kind: 'collage', key: 'adminGazetteTplCollage' },
    { kind: 'showing', key: 'adminGazetteTplShowing' },
    { kind: 'guest_story', key: 'adminGazetteTplGuest' },
    { kind: 'tale', key: 'adminGazetteTplTale' },
    { kind: 'note', key: 'adminGazetteTplNote' },
    { kind: 'world', key: 'adminGazetteTplWorld' },
  ];

  function kindLabel(k: GazetteKind): string {
    return $t(`gazetteKind_${k}` as TranslationKey);
  }
  function statusLabel(s: GazetteStatus): string {
    return $t(`adminGazetteStatus_${s}` as TranslationKey);
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
    sourceName = '';
    sourceUrl = '';
    imageUrl = '';
    slug = '';
    pinned = false;
    scheduledAt = '';
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
    sourceName = leaf.sourceName ?? '';
    sourceUrl = leaf.sourceUrl ?? '';
    imageUrl = leaf.imageUrl ?? '';
    slug = leaf.slug;
    pinned = leaf.pinned;
    scheduledAt = leaf.scheduledAt ? leaf.scheduledAt.slice(0, 16) : '';
  }

  function applyTemplate(k: GazetteKind) {
    kind = k;
    const fig = figurines.find((f) => f.id === figurineId);
    const fill = fillTemplate(k, fig?.name ?? '');
    titleEn = fill.titleEn;
    titleRu = fill.titleRu;
    dekEn = fill.dekEn;
    dekRu = fill.dekRu;
    if (fig) {
      if (!imageUrl) imageUrl = fig.faceImageUrl ?? '';
      if (!href) href = `/figurines/${fig.slug ?? fig.id}`;
    }
  }

  function onFigurineChange() {
    const fig = figurines.find((f) => f.id === figurineId);
    if (!fig) return;
    if (!imageUrl) imageUrl = fig.faceImageUrl ?? '';
    if (!href || href.startsWith('/figurines/')) href = `/figurines/${fig.slug ?? fig.id}`;
  }

  function toIsoScheduled(local: string): string | null {
    if (!local.trim()) return null;
    const d = new Date(local);
    if (Number.isNaN(d.getTime())) return local;
    return d.toISOString();
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
      sourceName: sourceName.trim() || null,
      sourceUrl: sourceUrl.trim() || null,
      imageUrl: imageUrl.trim() || null,
      pinned,
      scheduledAt: toIsoScheduled(scheduledAt),
    };
  }

  async function loadLeaves() {
    const page = await api.adminListGazetteLeaves({ perPage: 80 });
    leaves = page.items;
  }

  async function save() {
    saving = true;
    message = '';
    try {
      const saved = await api.adminSaveGazetteLeaf(payload(), selectedId ?? undefined);
      message = $t('adminGazetteSaved');
      await loadLeaves();
      loadLeaf(saved);
    } catch (e) {
      message = String(e);
    } finally {
      saving = false;
      setTimeout(() => (message = ''), 4000);
    }
  }

  async function destroy() {
    if (!selectedId) return;
    if (!confirm($t('adminGazetteDelete') + '?')) return;
    await api.adminDeleteGazetteLeaf(selectedId);
    blank();
    await loadLeaves();
  }

  async function loadDesk() {
    const [f, c] = await Promise.all([
      api.adminListGazetteFeeds(),
      api.adminListGazetteCuttings({ dismissed: showDismissed, perPage: 40 }),
    ]);
    feeds = f;
    cuttings = c.items;
  }

  async function refreshDesk() {
    refreshing = true;
    try {
      const r = await api.adminRefreshGazetteDesk();
      message = `${$t('adminGazetteRefreshed')} (${r.imported})`;
      await loadDesk();
    } catch (e) {
      message = String(e);
    } finally {
      refreshing = false;
      setTimeout(() => (message = ''), 5000);
    }
  }

  async function addFeed() {
    if (!newFeedTitle.trim() || !newFeedUrl.trim()) return;
    await api.adminSaveGazetteFeed({ title: newFeedTitle.trim(), url: newFeedUrl.trim(), enabled: true });
    newFeedTitle = '';
    newFeedUrl = '';
    await loadDesk();
  }

  async function toggleFeed(feed: GazetteFeed) {
    await api.adminSaveGazetteFeed(
      { title: feed.title, url: feed.url, enabled: !feed.enabled },
      feed.id,
    );
    await loadDesk();
  }

  async function removeFeed(id: string) {
    if (!confirm('?')) return;
    await api.adminDeleteGazetteFeed(id);
    await loadDesk();
  }

  async function dismiss(id: string) {
    await api.adminDismissGazetteCutting(id);
    await loadDesk();
  }
  async function restore(id: string) {
    await api.adminRestoreGazetteCutting(id);
    await loadDesk();
  }
  async function pinCut(id: string, pin: boolean) {
    await api.adminPinGazetteCutting(id, pin);
    await loadDesk();
  }
  async function promote(id: string) {
    const leaf = await api.adminPromoteGazetteCutting(id);
    tab = 'leaves';
    await loadLeaves();
    loadLeaf(leaf);
  }

  onMount(async () => {
    try {
      const [figs] = await Promise.all([
        api.getAllFigurinesAdmin().catch(() => api.getAllFigurines()),
        loadLeaves(),
      ]);
      figurines = figs;
    } finally {
      loading = false;
    }
  });

  $effect(() => {
    if (tab === 'desk') void loadDesk();
  });
</script>

<div class="gz-admin">
  <div class="gz-admin-head">
    <div>
      <p class="hint">{$t('adminGazetteHint')}</p>
      {#if message}<span class="msg">{message}</span>{/if}
    </div>
    <div class="tabs">
      <button class:on={tab === 'leaves'} onclick={() => (tab = 'leaves')}>{$t('adminGazetteLeaves')}</button>
      <button class:on={tab === 'desk'} onclick={() => (tab = 'desk')}>{$t('adminGazetteDesk')}</button>
    </div>
  </div>

  {#if loading}
    <p class="muted">{$t('adminLoading')}</p>
  {:else if tab === 'leaves'}
    <div class="split">
      <aside class="list">
        <button class="new" onclick={blank}>{$t('adminGazetteNew')}</button>
        {#if leaves.length === 0}
          <p class="muted">{$t('adminGazetteEmptyLeaves')}</p>
        {:else}
          {#each leaves as leaf (leaf.id)}
            <button class="row" class:sel={selectedId === leaf.id} onclick={() => loadLeaf(leaf)}>
              <span class="row-kind">{kindLabel(leaf.kind)}</span>
              <span class="row-title">{leaf.titleRu || leaf.titleEn}</span>
              <span class="row-st">{statusLabel(leaf.status)}</span>
            </button>
          {/each}
        {/if}
      </aside>

      <form class="editor" onsubmit={(e) => { e.preventDefault(); void save(); }}>
        <p class="lbl">{$t('adminGazetteTemplates')}</p>
        <div class="tpls">
          {#each TPL as t0}
            <button type="button" class="chip" class:on={kind === t0.kind} onclick={() => applyTemplate(t0.kind)}>
              {$t(t0.key)}
            </button>
          {/each}
        </div>

        <label class="lbl" for="gz-fig">{$t('adminGazetteFigurine')}</label>
        <select id="gz-fig" bind:value={figurineId} onchange={onFigurineChange}>
          <option value="">{$t('adminGazetteFigurineNone')}</option>
          {#each figurines as fig}
            <option value={fig.id}>{fig.name}</option>
          {/each}
        </select>

        <div class="grid2">
          <div>
            <label class="lbl" for="gz-kind">{$t('adminGazetteKind')}</label>
            <select id="gz-kind" bind:value={kind}>
              {#each KINDS as k}<option value={k}>{kindLabel(k)}</option>{/each}
            </select>
          </div>
          <div>
            <label class="lbl" for="gz-st">{$t('adminGazetteStatus')}</label>
            <select id="gz-st" bind:value={status}>
              {#each STATUSES as s}<option value={s}>{statusLabel(s)}</option>{/each}
            </select>
          </div>
        </div>

        <div class="grid2">
          <div>
            <label class="lbl" for="gz-te">{$t('adminGazetteTitleEn')}</label>
            <input id="gz-te" bind:value={titleEn} required />
          </div>
          <div>
            <label class="lbl" for="gz-tr">{$t('adminGazetteTitleRu')}</label>
            <input id="gz-tr" bind:value={titleRu} />
          </div>
        </div>
        <div class="grid2">
          <div>
            <label class="lbl" for="gz-de">{$t('adminGazetteDekEn')}</label>
            <textarea id="gz-de" rows="3" bind:value={dekEn}></textarea>
          </div>
          <div>
            <label class="lbl" for="gz-dr">{$t('adminGazetteDekRu')}</label>
            <textarea id="gz-dr" rows="3" bind:value={dekRu}></textarea>
          </div>
        </div>
        <div class="grid2">
          <div>
            <label class="lbl" for="gz-be">{$t('adminGazetteBodyEn')}</label>
            <textarea id="gz-be" rows="6" bind:value={bodyEn}></textarea>
          </div>
          <div>
            <label class="lbl" for="gz-br">{$t('adminGazetteBodyRu')}</label>
            <textarea id="gz-br" rows="6" bind:value={bodyRu}></textarea>
          </div>
        </div>

        <label class="lbl" for="gz-href">{$t('adminGazetteHref')}</label>
        <input id="gz-href" bind:value={href} />
        <div class="grid2">
          <div>
            <label class="lbl" for="gz-sn">{$t('adminGazetteSourceName')}</label>
            <input id="gz-sn" bind:value={sourceName} />
          </div>
          <div>
            <label class="lbl" for="gz-su">{$t('adminGazetteSourceUrl')}</label>
            <input id="gz-su" bind:value={sourceUrl} />
          </div>
        </div>
        <label class="lbl" for="gz-img">{$t('adminGazetteImage')}</label>
        <input id="gz-img" bind:value={imageUrl} />
        <div class="grid2">
          <div>
            <label class="lbl" for="gz-slug">{$t('adminGazetteSlug')}</label>
            <input id="gz-slug" bind:value={slug} />
          </div>
          <div>
            <label class="lbl" for="gz-when">{$t('adminGazetteScheduledAt')}</label>
            <input id="gz-when" type="datetime-local" bind:value={scheduledAt} />
          </div>
        </div>
        <label class="check">
          <input type="checkbox" bind:checked={pinned} />
          {$t('adminGazettePin')}
        </label>

        <div class="actions">
          <button type="submit" class="btn" disabled={saving}>{$t('adminGazetteSave')}</button>
          {#if selectedId}
            <button type="button" class="btn ghost" onclick={destroy}>{$t('adminGazetteDelete')}</button>
          {/if}
        </div>
      </form>
    </div>
  {:else}
    <div class="desk">
      <div class="desk-bar">
        <button class="btn" onclick={refreshDesk} disabled={refreshing}>
          {refreshing ? $t('adminGazetteRefreshing') : $t('adminGazetteRefresh')}
        </button>
        <label class="check">
          <input type="checkbox" bind:checked={showDismissed} />
          {$t('adminGazetteShowDismissed')}
        </label>
      </div>

      <h3 class="sub">{$t('adminGazetteFeeds')}</h3>
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
            {#if feed.lastError}<span class="err">{feed.lastError}</span>{/if}
            <button class="tiny" onclick={() => removeFeed(feed.id)}>×</button>
          </li>
        {/each}
      </ul>

      {#if cuttings.length === 0}
        <p class="muted">{$t('adminGazetteEmptyCuttings')}</p>
      {:else}
        <ul class="cuts">
          {#each cuttings as cut (cut.id)}
            <li class:dim={cut.dismissed}>
              <a href={cut.url} target="_blank" rel="noopener noreferrer">{cut.title}</a>
              <span class="muted">{cut.sourceName}</span>
              {#if cut.summary}<p>{cut.summary}</p>{/if}
              <div class="cut-act">
                {#if cut.dismissed}
                  <button class="tiny" onclick={() => restore(cut.id)}>{$t('adminGazetteRestore')}</button>
                {:else}
                  <button class="tiny" onclick={() => dismiss(cut.id)}>{$t('adminGazetteDismiss')}</button>
                {/if}
                <button class="tiny" onclick={() => pinCut(cut.id, !cut.pinned)}>
                  {cut.pinned ? $t('adminGazetteUnpinCutting') : $t('adminGazettePinCutting')}
                </button>
                <button class="tiny" onclick={() => promote(cut.id)}>{$t('adminGazettePromote')}</button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</div>

<style>
  .gz-admin { height: 100%; display: flex; flex-direction: column; color: #34251c; }
  .gz-admin-head { display: flex; justify-content: space-between; gap: 16px; align-items: flex-start; margin-bottom: 16px; }
  .hint { font-size: 12px; color: #5f4636; max-width: 52em; margin: 0; line-height: 1.5; }
  .msg { font-size: 12px; color: #6f3b24; margin-left: 8px; }
  .tabs { display: flex; gap: 6px; }
  .tabs button, .chip, .new, .btn, .tiny {
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    border: 1px solid #d8c6b1;
    background: #fff9f0;
    color: #34251c;
    padding: 6px 10px;
    cursor: pointer;
  }
  .tabs button.on, .chip.on { background: #34251c; color: #f8f1e7; }
  .split { display: grid; grid-template-columns: 260px 1fr; gap: 20px; min-height: 0; flex: 1; overflow: hidden; }
  .list { overflow: auto; border-right: 1px solid #d8c6b1; padding-right: 12px; display: flex; flex-direction: column; gap: 6px; }
  .row { display: grid; text-align: left; gap: 2px; padding: 8px; background: transparent; border: 1px solid transparent; }
  .row.sel { border-color: #c65f3c; background: #fff9f0; }
  .row-kind, .row-st { font-size: 9px; letter-spacing: 0.1em; text-transform: uppercase; color: #6f3b24; }
  .row-title { font-size: 13px; }
  .editor { overflow: auto; display: flex; flex-direction: column; gap: 8px; padding-bottom: 24px; }
  .lbl { font-size: 10px; text-transform: uppercase; letter-spacing: 0.08em; color: #5f4636; font-weight: 700; }
  .tpls { display: flex; flex-wrap: wrap; gap: 6px; }
  input, select, textarea {
    width: 100%;
    background: #f8f1e7;
    border: 1px solid rgba(198, 95, 60, 0.25);
    padding: 0.55rem 0.7rem;
    color: #34251c;
    font-size: 13px;
  }
  .grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .check { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .actions { display: flex; gap: 10px; margin-top: 8px; }
  .btn { background: #6f3b24; color: #f8f1e7; border-color: #6f3b24; }
  .btn.ghost { background: transparent; color: #6f3b24; }
  .muted { font-size: 12px; color: #5f4636; }
  .desk { overflow: auto; }
  .desk-bar, .feed-add { display: flex; gap: 10px; align-items: center; margin-bottom: 16px; flex-wrap: wrap; }
  .sub { font-size: 12px; letter-spacing: 0.1em; text-transform: uppercase; margin: 8px 0; }
  .feeds, .cuts { list-style: none; padding: 0; margin: 0 0 24px; display: flex; flex-direction: column; gap: 10px; }
  .feeds li, .cuts li { border: 1px solid #d8c6b1; padding: 10px 12px; background: #fff9f0; }
  .cuts li.dim { opacity: 0.5; }
  .cut-act { display: flex; gap: 8px; margin-top: 8px; }
  .err { color: #8a2a2a; font-size: 11px; display: block; }
  @media (max-width: 900px) {
    .split, .grid2 { grid-template-columns: 1fr; }
  }
</style>
