<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { AdminCommentDto, AdminCommentsPage } from '$lib/types/api';

  let { onPendingCount }: { onPendingCount?: (n: number) => void } = $props();

  let pageData = $state<AdminCommentsPage | null>(null);
  let loading = $state(true);
  let onlyPending = $state(true);
  let newestFirst = $state(true);
  let figurineFilter = $state('');
  let page = $state(1);
  const perPage = 20;

  let replyDrafts = $state<Record<string, string>>({});
  let savingReply = $state<string | null>(null);
  let expandedReply = $state<string | null>(null);

  async function load() {
    loading = true;
    try {
      pageData = await api.adminListComments({
        pending: onlyPending,
        sort: newestFirst ? 'newest' : 'oldest',
        figurineId: figurineFilter.trim() || undefined,
        page,
        perPage,
      });
      onPendingCount?.(pageData.pendingCount);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  async function approve(id: string) {
    await api.adminModerateComment(id, { isApproved: true, adminReply: replyDrafts[id]?.trim() || null });
    await load();
  }

  async function reject(id: string) {
    await api.adminModerateComment(id, { isApproved: false, adminReply: null });
    await load();
  }

  async function saveReply(c: AdminCommentDto) {
    savingReply = c.id;
    try {
      await api.adminModerateComment(c.id, {
        isApproved: c.isApproved,
        adminReply: replyDrafts[c.id]?.trim() || null,
      });
      delete replyDrafts[c.id];
      expandedReply = null;
      await load();
    } finally {
      savingReply = null;
    }
  }

  async function del(id: string) {
    if (!confirm($t('adminCommentsDelete') + '?')) return;
    await api.adminDeleteComment(id);
    await load();
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  }

  function toggleReply(id: string, current: string | null) {
    if (expandedReply === id) {
      expandedReply = null;
    } else {
      expandedReply = id;
      // always reset to saved value so edits are always based on current reply
      replyDrafts[id] = current ?? '';
    }
  }
</script>

<div class="panel">
  <div class="panel-header">
    <h2 class="panel-title">{$t('adminCommentsTitle')}</h2>
    <div class="header-controls">
      <label class="pending-toggle">
        <input type="checkbox" bind:checked={onlyPending} onchange={() => { page = 1; load(); }} />
        {$t('adminCommentsPending')}
        {#if pageData && pageData.pendingCount > 0}
          <span class="badge">{pageData.pendingCount}</span>
        {/if}
      </label>
      <select class="sort-select" bind:value={newestFirst} onchange={() => { page = 1; load(); }}>
        <option value={true}>{$t('adminCommentsSortNewest')}</option>
        <option value={false}>{$t('adminCommentsSortOldest')}</option>
      </select>
      <input
        class="filter-input"
        type="text"
        placeholder={$t('adminCommentsFilterFigurine')}
        bind:value={figurineFilter}
        oninput={() => { page = 1; load(); }}
      />
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
    <p class="empty">{$t('adminCommentsEmpty')}</p>
  {:else}
    <ul class="comment-list">
      {#each pageData.items as c (c.id)}
        <li class="comment-row" class:approved={c.isApproved} class:pending={!c.isApproved}>
          <div class="comment-meta">
            <a href="/figurines/{c.figurineId}" target="_blank" rel="noopener" class="figurine-link">{c.figurineName}</a>
            <span class="sep">·</span>
            <strong class="author">{c.authorName}</strong>
            {#if c.authorEmail}
              <span class="email">({c.authorEmail})</span>
            {/if}
            {#if c.userId}
              <span class="tag tag--user">{$t('adminCommentsUser')}</span>
            {:else}
              <span class="tag tag--anon">{$t('adminCommentsAnon')}</span>
            {/if}
            <span class="sep">·</span>
            <time class="date">{formatDate(c.createdAt)}</time>
            {#if c.isApproved}
              <span class="tag tag--approved">{$t('adminCommentsApproved')}</span>
            {:else}
              <span class="tag tag--pending">{$t('adminCommentsPendingBadge')}</span>
            {/if}
          </div>

          <p class="comment-body">{c.body}</p>

          {#if c.adminReply}
            <p class="existing-reply">↳ {c.adminReply}</p>
          {/if}

          {#if expandedReply === c.id}
            <div class="reply-box">
              <textarea
                class="reply-textarea"
                rows="3"
                placeholder={$t('adminCommentsReply') + '…'}
                bind:value={replyDrafts[c.id]}
              ></textarea>
              <div class="reply-actions">
                <button class="btn btn--sm btn--accent" onclick={() => saveReply(c)} disabled={savingReply === c.id}>
                  {$t('adminCommentsReplySave')}
                </button>
                <button class="btn btn--sm btn--ghost" onclick={() => expandedReply = null}>✕</button>
              </div>
            </div>
          {/if}

          <div class="comment-actions">
            {#if !c.isApproved}
              <button class="btn btn--sm btn--accent" onclick={() => approve(c.id)}>{$t('adminCommentsApprove')}</button>
            {/if}
            {#if c.isApproved}
              <button class="btn btn--sm btn--muted" onclick={() => reject(c.id)}>{$t('adminCommentsReject')}</button>
            {/if}
            <button class="btn btn--sm btn--ghost" onclick={() => toggleReply(c.id, c.adminReply)}>
              {$t('adminCommentsReply')}
            </button>
            <button class="btn btn--sm btn--danger" onclick={() => del(c.id)}>{$t('adminCommentsDelete')}</button>
          </div>
        </li>
      {/each}
    </ul>

    <!-- Pagination -->
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
  .panel {
    padding: 2rem;
    max-width: 900px;
  }

  .panel-header {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
    justify-content: space-between;
  }

  .header-controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .sort-select {
    font-size: 0.75rem;
    color: #5f4636;
    background: transparent;
    border: 1px solid #d8c6b1;
    padding: 0.25rem 0.5rem;
    cursor: pointer;
  }

  .filter-input {
    font-size: 0.75rem;
    color: #34251c;
    background: #fdfaf5;
    border: 1px solid #d8c6b1;
    padding: 0.25rem 0.6rem;
    width: 220px;
    outline: none;
  }

  .filter-input:focus { border-color: #c65f3c; }

  .panel-title {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.1rem;
    font-weight: 500;
    color: #34251c;
  }

  .pending-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8rem;
    color: #5f4636;
    cursor: pointer;
  }

  .badge {
    background: #c65f3c;
    color: #fff;
    font-size: 0.7rem;
    padding: 1px 6px;
    border-radius: 99px;
  }

  .loading-rows {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .skel-row {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .skel {
    background: #34251c10;
    border-radius: 3px;
    animation: pulse 1.6s ease-in-out infinite;
    height: 10px;
  }

  .skel-wide   { width: 70%; }
  .skel-narrow { width: 40%; }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50%       { opacity: 0.4; }
  }

  .empty {
    font-size: 0.85rem;
    color: #8a7060;
    font-style: italic;
  }

  .comment-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .comment-row {
    padding: 1.25rem 0;
    border-bottom: 1px solid #d8c6b130;
  }

  .comment-row.pending {
    border-left: 3px solid #c65f3c;
    padding-left: 0.75rem;
  }

  .comment-row.approved {
    border-left: 3px solid #d8c6b1;
    padding-left: 0.75rem;
  }

  .comment-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.5rem;
    font-size: 0.78rem;
    color: #5f4636;
  }

  .figurine-link {
    color: #6f3b24;
    font-weight: 500;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .sep { color: #d8c6b1; }

  .author { color: #34251c; }

  .email { color: #8a7060; font-size: 0.72rem; }

  .date { color: #8a7060; }

  .tag {
    font-size: 0.65rem;
    padding: 1px 6px;
    border-radius: 2px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .tag--user     { background: #6f3b2420; color: #6f3b24; }
  .tag--anon     { background: #34251c10; color: #8a7060; }
  .tag--approved { background: #2d6a4f20; color: #2d6a4f; }
  .tag--pending  { background: #c65f3c20; color: #c65f3c; }

  .comment-body {
    font-size: 0.875rem;
    line-height: 1.65;
    color: #34251c;
    white-space: pre-wrap;
    word-break: break-word;
    margin-bottom: 0.5rem;
  }

  .existing-reply {
    font-size: 0.8rem;
    font-style: italic;
    color: #6f3b24;
    margin-bottom: 0.5rem;
    padding-left: 0.5rem;
    border-left: 2px solid #c65f3c50;
  }

  .reply-box {
    margin: 0.5rem 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .reply-textarea {
    width: 100%;
    border: 1px solid #d8c6b1;
    background: #fff9f2;
    padding: 0.5rem 0.75rem;
    font-size: 0.85rem;
    font-family: inherit;
    color: #34251c;
    resize: vertical;
    outline: none;
  }

  .reply-textarea:focus { border-color: #c65f3c; }

  .reply-actions {
    display: flex;
    gap: 0.5rem;
  }

  .comment-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-top: 0.5rem;
  }

  .btn {
    border: none;
    cursor: pointer;
    font-family: inherit;
    transition: opacity 0.15s;
  }

  .btn:disabled { opacity: 0.4; cursor: default; }

  .btn--sm {
    font-size: 0.72rem;
    padding: 0.3rem 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .btn--accent  { background: #34251c; color: #f8f1e7; }
  .btn--accent:hover:not(:disabled) { background: #6f3b24; }
  .btn--muted   { background: #d8c6b1; color: #34251c; }
  .btn--muted:hover:not(:disabled) { background: #c9b49d; }
  .btn--ghost   { background: transparent; border: 1px solid #d8c6b1; color: #5f4636; }
  .btn--ghost:hover:not(:disabled) { border-color: #8a7060; }
  .btn--danger  { background: transparent; border: 1px solid #c65f3c40; color: #c65f3c; }
  .btn--danger:hover:not(:disabled) { background: #c65f3c10; }

  .pagination {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-top: 1.5rem;
  }

  .page-btn {
    background: none;
    border: 1px solid #d8c6b1;
    padding: 0.3rem 0.75rem;
    font-size: 0.85rem;
    cursor: pointer;
    color: #34251c;
  }

  .page-btn:disabled { opacity: 0.35; cursor: default; }

  .page-info {
    font-size: 0.8rem;
    color: #5f4636;
  }
</style>
