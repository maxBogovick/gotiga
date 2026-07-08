<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t, type TranslationKey } from '$lib/i18n';
  import type { AdminImpressionDto, AdminImpressionsPage } from '$lib/types/api';

  const MOOD_LABEL_KEYS: Record<string, TranslationKey> = {
    quiet: 'impressionsMood_quiet',
    haunting: 'impressionsMood_haunting',
    nostalgic: 'impressionsMood_nostalgic',
    meticulous: 'impressionsMood_meticulous',
    uneasy: 'impressionsMood_uneasy',
    moved: 'impressionsMood_moved',
  };
  function moodLabel(mood: string): string {
    const key = MOOD_LABEL_KEYS[mood];
    return key ? $t(key) : mood;
  }

  let { onPendingCount }: { onPendingCount?: (n: number) => void } = $props();

  let pageData = $state<AdminImpressionsPage | null>(null);
  let loading = $state(true);
  let onlyPending = $state(true);
  let newestFirst = $state(true);
  let page = $state(1);
  const perPage = 20;

  async function load() {
    loading = true;
    try {
      pageData = await api.adminListImpressions({
        pending: onlyPending,
        sort: newestFirst ? 'newest' : 'oldest',
        page,
        perPage,
      });
      onPendingCount?.(pageData.pendingCount);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  async function approve(im: AdminImpressionDto) {
    // Approve and publish are one action here: this admin panel is the only
    // moderation step for impressions, so there's no separate "approved but
    // hidden" state to manage — approving puts it straight on the site.
    await api.adminModerateImpression(im.id, { isApproved: true, isFeatured: true });
    await load();
  }

  async function reject(im: AdminImpressionDto) {
    await api.adminModerateImpression(im.id, { isApproved: false, isFeatured: false });
    await load();
  }

  async function toggleFeatured(im: AdminImpressionDto) {
    await api.adminModerateImpression(im.id, { isApproved: im.isApproved, isFeatured: !im.isFeatured });
    await load();
  }

  async function del(id: string) {
    if (!confirm($t('adminImpressionsDelete') + '?')) return;
    await api.adminDeleteImpression(id);
    await load();
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  }
</script>

<div class="panel">
  <div class="panel-header">
    <h2 class="panel-title">{$t('adminImpressionsTitle')}</h2>
    <div class="header-controls">
      <label class="pending-toggle">
        <input type="checkbox" bind:checked={onlyPending} onchange={() => { page = 1; load(); }} />
        {$t('adminImpressionsPending')}
        {#if pageData && pageData.pendingCount > 0}
          <span class="badge">{pageData.pendingCount}</span>
        {/if}
      </label>
      <select class="sort-select" bind:value={newestFirst} onchange={() => { page = 1; load(); }}>
        <option value={true}>{$t('adminCommentsSortNewest')}</option>
        <option value={false}>{$t('adminCommentsSortOldest')}</option>
      </select>
    </div>
  </div>

  {#if loading}
    <div class="loading-rows">
      {#each Array(5) as _}
        <div class="skel-row">
          <div class="skel skel-wide"></div>
          <div class="skel skel-narrow"></div>
        </div>
      {/each}
    </div>
  {:else if !pageData || pageData.items.length === 0}
    <p class="empty">{$t('adminImpressionsEmpty')}</p>
  {:else}
    <ul class="impression-list">
      {#each pageData.items as im (im.id)}
        <li class="impression-row" class:approved={im.isApproved} class:pending={!im.isApproved}>
          <div class="impression-meta">
            <strong class="author">{im.authorName || $t('impressionsAnonymous')}</strong>
            {#if im.mood}
              <span class="sep">·</span>
              <span class="tag tag--mood">{moodLabel(im.mood)}</span>
            {/if}
            <span class="sep">·</span>
            <time class="date">{formatDate(im.createdAt)}</time>
            {#if im.isApproved}
              <span class="tag tag--approved">{$t('adminCommentsApproved')}</span>
            {:else}
              <span class="tag tag--pending">{$t('adminCommentsPendingBadge')}</span>
            {/if}
            {#if im.isFeatured}
              <span class="tag tag--featured">{$t('adminImpressionsFeatured')}</span>
            {/if}
          </div>

          <p class="impression-body">{im.message}</p>

          <div class="impression-actions">
            {#if !im.isApproved}
              <button class="btn btn--sm btn--accent" onclick={() => approve(im)}>{$t('adminCommentsApprove')}</button>
            {/if}
            {#if im.isApproved}
              <button class="btn btn--sm btn--muted" onclick={() => reject(im)}>{$t('adminCommentsReject')}</button>
              <button class="btn btn--sm btn--ghost" onclick={() => toggleFeatured(im)}>
                {im.isFeatured ? $t('adminImpressionsUnfeature') : $t('adminImpressionsFeature')}
              </button>
            {/if}
            <button class="btn btn--sm btn--danger" onclick={() => del(im.id)}>{$t('adminCommentsDelete')}</button>
          </div>
        </li>
      {/each}
    </ul>

    {#if pageData.total > perPage}
      <div class="pagination">
        <button class="page-btn" disabled={page <= 1} onclick={() => { page--; load(); }}>←</button>
        <span class="page-info">{page} / {Math.ceil(pageData.total / perPage)}</span>
        <button class="page-btn" disabled={page * perPage >= pageData.total} onclick={() => { page++; load(); }}>→</button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .panel { padding: 2rem; max-width: 900px; }

  .panel-header {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
    justify-content: space-between;
  }

  .header-controls { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; }

  .sort-select {
    font-size: 0.75rem;
    color: #5f4636;
    background: transparent;
    border: 1px solid #d8c6b1;
    padding: 0.25rem 0.5rem;
    cursor: pointer;
  }

  .panel-title {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.1rem;
    font-weight: 500;
    color: #34251c;
  }

  .pending-toggle { display: flex; align-items: center; gap: 0.5rem; font-size: 0.8rem; color: #5f4636; cursor: pointer; }

  .badge { background: #c65f3c; color: #fff; font-size: 0.7rem; padding: 1px 6px; border-radius: 99px; }

  .loading-rows { display: flex; flex-direction: column; gap: 1.25rem; }
  .skel-row { display: flex; flex-direction: column; gap: 0.4rem; }
  .skel { background: #34251c10; border-radius: 3px; animation: pulse 1.6s ease-in-out infinite; height: 10px; }
  .skel-wide { width: 70%; }
  .skel-narrow { width: 40%; }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }

  .empty { font-size: 0.85rem; color: #8a7060; font-style: italic; }

  .impression-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0; }

  .impression-row { padding: 1.25rem 0; border-bottom: 1px solid #d8c6b130; }
  .impression-row.pending { border-left: 3px solid #c65f3c; padding-left: 0.75rem; }
  .impression-row.approved { border-left: 3px solid #d8c6b1; padding-left: 0.75rem; }

  .impression-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.5rem;
    font-size: 0.78rem;
    color: #5f4636;
  }

  .sep { color: #d8c6b1; }
  .author { color: #34251c; }
  .date { color: #8a7060; }

  .tag { font-size: 0.65rem; padding: 1px 6px; border-radius: 2px; text-transform: uppercase; letter-spacing: 0.04em; }
  .tag--mood      { background: #6f3b2420; color: #6f3b24; }
  .tag--approved  { background: #2d6a4f20; color: #2d6a4f; }
  .tag--pending   { background: #c65f3c20; color: #c65f3c; }
  .tag--featured  { background: #34251c15; color: #34251c; }

  .impression-body { font-size: 0.875rem; line-height: 1.65; color: #34251c; white-space: pre-wrap; word-break: break-word; margin-bottom: 0.5rem; }

  .impression-actions { display: flex; gap: 0.5rem; flex-wrap: wrap; margin-top: 0.5rem; }

  .btn { border: none; cursor: pointer; font-family: inherit; transition: opacity 0.15s; }
  .btn:disabled { opacity: 0.4; cursor: default; }
  .btn--sm { font-size: 0.72rem; padding: 0.3rem 0.75rem; text-transform: uppercase; letter-spacing: 0.06em; }
  .btn--accent  { background: #34251c; color: #f8f1e7; }
  .btn--accent:hover:not(:disabled) { background: #6f3b24; }
  .btn--muted   { background: #d8c6b1; color: #34251c; }
  .btn--muted:hover:not(:disabled) { background: #c9b49d; }
  .btn--ghost   { background: transparent; border: 1px solid #d8c6b1; color: #5f4636; }
  .btn--ghost:hover:not(:disabled) { border-color: #8a7060; }
  .btn--danger  { background: transparent; border: 1px solid #c65f3c40; color: #c65f3c; }
  .btn--danger:hover:not(:disabled) { background: #c65f3c10; }

  .pagination { display: flex; align-items: center; gap: 1rem; margin-top: 1.5rem; }
  .page-btn { background: none; border: 1px solid #d8c6b1; padding: 0.3rem 0.75rem; font-size: 0.85rem; cursor: pointer; color: #34251c; }
  .page-btn:disabled { opacity: 0.35; cursor: default; }
  .page-info { font-size: 0.8rem; color: #5f4636; }
</style>
