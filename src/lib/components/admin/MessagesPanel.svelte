<script lang="ts">
  import { onMount } from 'svelte';
  import { api, resolveMediaUrl } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { MessageThreadDto, ThreadDetailDto, AttachmentInput } from '$lib/types/api';
  import MessageAttachments from '$lib/components/MessageAttachments.svelte';

  type StatusFilter = 'all' | 'open' | 'resolved';
  type CategoryFilter = 'all' | 'booking' | 'waitlist' | 'order' | 'commission' | 'general' | 'system';

  let statusFilter = $state<StatusFilter>('open');
  let categoryFilter = $state<CategoryFilter>('all');
  let page = $state(1);
  const perPage = 30;

  let items = $state<Array<{ thread: MessageThreadDto; user: { id: string; displayName: string; email: string } }>>([]);
  let total = $state(0);
  let loading = $state(true);

  let selectedDetail = $state<ThreadDetailDto | null>(null);
  let detailLoading = $state(false);

  let replyBody = $state('');
  let replySending = $state(false);
  let replySent = $state(false);
  let replyAttachments = $state<AttachmentInput[]>([]);
  let replyUploading = $state(false);

  async function handleAdminFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files) return;
    for (const file of Array.from(input.files)) {
      if (replyAttachments.length >= 5) break;
      replyUploading = true;
      try {
        const media = await api.importMediaWithVariants(file, 'images');
        replyAttachments = [...replyAttachments, { url: media.url, thumbUrl: media.thumbUrl ?? null }];
      } catch { /* ignore */ }
      finally { replyUploading = false; }
    }
    input.value = '';
  }

  async function load() {
    loading = true;
    try {
      const result = await api.adminListThreads({
        status: statusFilter === 'all' ? undefined : statusFilter,
        category: categoryFilter === 'all' ? undefined : categoryFilter,
        page,
        perPage,
      });
      items = result.items;
      total = result.total;
    } finally {
      loading = false;
    }
  }

  onMount(load);

  async function selectThread(id: string) {
    if (selectedDetail?.thread.id === id) return;
    detailLoading = true;
    selectedDetail = null;
    replyBody = '';
    replyAttachments = [];
    replySent = false;
    try {
      selectedDetail = await api.adminGetThread(id);
    } finally {
      detailLoading = false;
    }
  }

  async function sendReply() {
    if ((!replyBody.trim() && replyAttachments.length === 0) || replySending || !selectedDetail) return;
    replySending = true;
    try {
      const msg = await api.adminReplyToThread(selectedDetail.thread.id, replyBody.trim(), replyAttachments);
      selectedDetail = {
        ...selectedDetail,
        messages: [...selectedDetail.messages, msg],
      };
      // update preview in list
      items = items.map(it =>
        it.thread.id === selectedDetail!.thread.id
          ? { ...it, thread: { ...it.thread, lastMessageAt: msg.createdAt } }
          : it
      );
      replyBody = '';
      replyAttachments = [];
      replySent = true;
      setTimeout(() => { replySent = false; }, 2000);
    } finally {
      replySending = false;
    }
  }

  async function resolveThread() {
    if (!selectedDetail) return;
    await api.adminResolveThread(selectedDetail.thread.id);
    selectedDetail = { ...selectedDetail, thread: { ...selectedDetail.thread, status: 'resolved' } };
    items = items.map(it =>
      it.thread.id === selectedDetail!.thread.id
        ? { ...it, thread: { ...it.thread, status: 'resolved' } }
        : it
    );
  }

  async function reopenThread() {
    if (!selectedDetail) return;
    await api.adminReopenThread(selectedDetail.thread.id);
    selectedDetail = { ...selectedDetail, thread: { ...selectedDetail.thread, status: 'open' } };
    items = items.map(it =>
      it.thread.id === selectedDetail!.thread.id
        ? { ...it, thread: { ...it.thread, status: 'open' } }
        : it
    );
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  }

  function categoryLabel(cat: string): string {
    const map: Record<string, string> = {
      booking: $t('adminMessagesBooking'),
      order: $t('adminMessagesOrder'),
      commission: 'Commissions',
      waitlist: $t('adminMessagesWaitlist'),
      general: $t('adminMessagesGeneral'),
      system: $t('adminMessagesSystem'),
    };
    return map[cat] ?? cat;
  }
</script>

<div class="messages-panel">
  <!-- Left pane: thread list -->
  <div class="thread-list-pane">
    <div class="pane-filters">
      <div class="filter-row">
        {#each (['all', 'open', 'resolved'] as StatusFilter[]) as s}
          <button
            class="filter-btn"
            class:active={statusFilter === s}
            onclick={() => { statusFilter = s; page = 1; load(); }}
          >
            {s === 'all' ? $t('adminMessagesAll') : s === 'open' ? $t('adminMessagesOpen') : $t('adminMessagesClosed')}
          </button>
        {/each}
      </div>
      <div class="filter-row">
        {#each (['all', 'booking', 'order', 'commission', 'waitlist', 'general', 'system'] as CategoryFilter[]) as c}
          <button
            class="filter-btn filter-btn--sm"
            class:active={categoryFilter === c}
            onclick={() => { categoryFilter = c; page = 1; load(); }}
          >
            {c === 'all' ? $t('adminMessagesAll') : categoryLabel(c)}
          </button>
        {/each}
      </div>
    </div>

    {#if loading}
      <p class="pane-empty">…</p>
    {:else if items.length === 0}
      <p class="pane-empty">{$t('adminMessagesEmpty')}</p>
    {:else}
      <ul class="thread-list">
        {#each items as item}
          {@const isSelected = selectedDetail?.thread.id === item.thread.id}
          <li
            class="thread-item"
            class:selected={isSelected}
            class:has-unread={item.thread.unread > 0}
          >
            <button
              class="thread-item-hit"
              onclick={() => selectThread(item.thread.id)}
              aria-label={item.thread.subject}
            ></button>
            <div class="thread-item-top">
              <span class="thread-item-user">{item.user.displayName}</span>
              <span class="thread-item-cat">{categoryLabel(item.thread.category)}</span>
            </div>
            <div class="thread-item-subject">{item.thread.subject}</div>
            <div class="thread-item-bottom">
              <span class="thread-item-date">{formatDate(item.thread.lastMessageAt)}</span>
              {#if item.thread.unread > 0}
                <span class="thread-unread-dot"></span>
              {/if}
              {#if item.thread.status === 'resolved'}
                <span class="thread-item-resolved">{$t('adminMessagesClosed')}</span>
              {/if}
            </div>
          </li>
        {/each}
      </ul>

      {#if total > perPage}
        <div class="pagination">
          <button class="page-btn" onclick={() => { page = Math.max(1, page - 1); load(); }} disabled={page === 1}>←</button>
          <span class="page-info">{page} / {Math.ceil(total / perPage)}</span>
          <button class="page-btn" onclick={() => { page = page + 1; load(); }} disabled={page * perPage >= total}>→</button>
        </div>
      {/if}
    {/if}
  </div>

  <!-- Right pane: thread detail -->
  <div class="thread-detail-pane">
    {#if detailLoading}
      <p class="pane-empty">…</p>
    {:else if !selectedDetail}
      <p class="pane-empty pane-empty--center">{$t('adminMessagesSelectPrompt')}</p>
    {:else}
      <div class="detail-header">
        <div class="detail-meta">
          <span class="detail-subject">{selectedDetail.thread.subject}</span>
          <span class="detail-cat">{categoryLabel(selectedDetail.thread.category)}</span>
        </div>
        {#if selectedDetail.user}
          <span class="detail-user">{selectedDetail.user.displayName} &lt;{selectedDetail.user.email}&gt;</span>
        {/if}
        <div class="detail-actions">
          {#if selectedDetail.thread.status === 'open'}
            <button class="action-btn action-btn--muted" onclick={resolveThread}>{$t('adminMessagesResolve')}</button>
          {:else}
            <button class="action-btn" onclick={reopenThread}>{$t('adminMessagesReopen')}</button>
          {/if}
        </div>
      </div>

      <div class="message-chain">
        {#each selectedDetail.messages as msg}
          <div class="msg-bubble" class:msg-bubble--admin={msg.fromAdmin} class:msg-bubble--user={!msg.fromAdmin}>
            <p class="msg-bubble-from">
              {msg.fromAdmin ? $t('adminMessagesReply') : (selectedDetail.user?.displayName ?? 'User')}
            </p>
            {#if msg.body}<p class="msg-bubble-body">{msg.body}</p>{/if}
            {#if msg.attachments && msg.attachments.length > 0}
              <MessageAttachments attachments={msg.attachments} />
            {/if}
            <p class="msg-bubble-date">{formatDate(msg.createdAt)}</p>
          </div>
        {/each}
      </div>

      {#if selectedDetail.thread.status === 'open'}
        <div class="reply-area">
          <textarea
            class="reply-input"
            bind:value={replyBody}
            rows="4"
            placeholder={$t('adminMessagesReply') + '…'}
          ></textarea>
          {#if replyAttachments.length > 0}
            <div class="reply-atts">
              {#each replyAttachments as att, i (att.url)}
                <div class="reply-att">
                  <img src={resolveMediaUrl(att.thumbUrl ?? att.url)} alt="" />
                  <button type="button" onclick={() => replyAttachments = replyAttachments.filter((_, idx) => idx !== i)} aria-label="×">×</button>
                </div>
              {/each}
            </div>
          {/if}
          <div class="reply-controls">
            <label class="reply-attach" title="Прикрепить изображение">
              <input type="file" accept="image/*" multiple hidden onchange={handleAdminFiles} />
              {replyUploading ? '…' : '📎'}
            </label>
            <button
              class="reply-btn"
              onclick={sendReply}
              disabled={replySending || (!replyBody.trim() && replyAttachments.length === 0)}
            >
              {replySending ? $t('adminMessagesReplying') : replySent ? '✓' : $t('adminMessagesReply')}
            </button>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .messages-panel {
    display: grid;
    grid-template-columns: 280px 1fr;
    min-height: 500px;
    border: 1px solid #d8c6b1;
    background: #fdf8f2;
  }

  /* ── Left pane ── */

  .thread-list-pane {
    border-right: 1px solid #d8c6b1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .pane-filters {
    padding: 0.7rem;
    border-bottom: 1px solid #eee3d6;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .filter-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .filter-btn {
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    font-size: 0.65rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.15rem 0.45rem;
    cursor: pointer;
    transition: all 0.15s;
  }
  .filter-btn:hover { border-color: #9a7c5c; color: #34251c; }
  .filter-btn.active { border-color: #c65f3c; color: #c65f3c; background: rgba(198,95,60,0.06); }
  .filter-btn--sm { font-size: 0.58rem; }

  .pane-empty {
    padding: 2rem 1rem;
    text-align: center;
    color: #b5a090;
    font-family: Inter, sans-serif;
    font-size: 0.8rem;
    font-style: italic;
  }
  .pane-empty--center {
    margin: auto;
    padding: 4rem 2rem;
  }

  .thread-list {
    list-style: none;
    padding: 0;
    margin: 0;
    overflow-y: auto;
    flex: 1;
  }

  .thread-item {
    padding: 0.6rem 0.75rem;
    border-bottom: 1px solid #eee3d6;
    cursor: pointer;
    transition: background 0.12s;
    position: relative;
  }
  /* Invisible full-area button keeps the row keyboard-accessible without an
     interactive role on the <li>. Content above is non-interactive. */
  .thread-item-hit {
    position: absolute;
    inset: 0;
    width: 100%;
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    cursor: pointer;
  }
  .thread-item:hover { background: rgba(216,198,177,0.2); }
  .thread-item.selected { background: rgba(198,95,60,0.08); border-left: 2px solid #c65f3c; }
  .thread-item.has-unread { font-weight: 500; }

  .thread-item-top {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 0.4rem;
    margin-bottom: 0.15rem;
  }

  .thread-item-user {
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    color: #34251c;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .thread-item-cat {
    font-family: Inter, sans-serif;
    font-size: 0.6rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: #9a7c5c;
    flex-shrink: 0;
  }

  .thread-item-subject {
    font-family: Georgia, serif;
    font-size: 0.82rem;
    color: #34251c;
    margin-bottom: 0.2rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .thread-item-bottom {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .thread-item-date {
    font-family: Inter, sans-serif;
    font-size: 0.62rem;
    color: #b5a090;
  }

  .thread-unread-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #c65f3c;
    flex-shrink: 0;
  }

  .thread-item-resolved {
    font-family: Inter, sans-serif;
    font-size: 0.58rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: #b5a090;
    margin-left: auto;
  }

  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.6rem;
    border-top: 1px solid #eee3d6;
  }

  .page-btn {
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    padding: 0.2rem 0.5rem;
    cursor: pointer;
    transition: all 0.15s;
  }
  .page-btn:hover:not(:disabled) { border-color: #9a7c5c; color: #34251c; }
  .page-btn:disabled { opacity: 0.35; cursor: default; }

  .page-info {
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    color: #9a7c5c;
  }

  /* ── Right pane ── */

  .thread-detail-pane {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .detail-header {
    padding: 0.85rem 1rem;
    border-bottom: 1px solid #d8c6b1;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .detail-meta {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
  }

  .detail-subject {
    font-family: Georgia, serif;
    font-size: 1rem;
    color: #34251c;
    flex: 1;
    min-width: 0;
  }

  .detail-cat {
    font-family: Inter, sans-serif;
    font-size: 0.62rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #9a7c5c;
    flex-shrink: 0;
  }

  .detail-user {
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    color: #9a7c5c;
  }

  .detail-actions {
    display: flex;
    gap: 0.4rem;
    margin-top: 0.2rem;
  }

  .action-btn {
    background: transparent;
    border: 1px solid #c65f3c;
    color: #c65f3c;
    font-family: Inter, sans-serif;
    font-size: 0.65rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.2rem 0.6rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .action-btn:hover { background: rgba(198,95,60,0.08); }
  .action-btn--muted {
    border-color: #d8c6b1;
    color: #9a7c5c;
  }
  .action-btn--muted:hover { border-color: #9a7c5c; color: #34251c; background: transparent; }

  .message-chain {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
  }

  .msg-bubble {
    padding: 0.65rem 0.85rem;
    border: 1px solid #eee3d6;
    max-width: 90%;
  }

  .msg-bubble--admin {
    background: rgba(248,241,231,0.7);
    border-color: #d8c6b1;
    align-self: flex-start;
  }

  .msg-bubble--user {
    align-self: flex-end;
    background: #fff;
  }

  .msg-bubble-from {
    font-family: Inter, sans-serif;
    font-size: 0.62rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #9a7c5c;
    margin: 0 0 0.25rem;
  }

  .msg-bubble-body {
    font-family: Inter, sans-serif;
    font-size: 0.85rem;
    color: #34251c;
    margin: 0 0 0.25rem;
    line-height: 1.6;
    white-space: pre-wrap;
  }

  .msg-bubble-date {
    font-family: Inter, sans-serif;
    font-size: 0.62rem;
    color: #b5a090;
    margin: 0;
  }

  .reply-area {
    padding: 0.85rem 1rem;
    border-top: 1px solid #d8c6b1;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .reply-input {
    font-family: Inter, sans-serif;
    font-size: 0.85rem;
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #34251c;
    padding: 0.5rem;
    outline: none;
    resize: vertical;
    line-height: 1.5;
    width: 100%;
    box-sizing: border-box;
  }
  .reply-input::placeholder { color: #b5a090; }
  .reply-input:focus { border-color: #c65f3c; }

  .reply-btn {
    align-self: flex-end;
    background: #c65f3c;
    border: none;
    color: #fff;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    padding: 0.4rem 1rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .reply-btn:hover:not(:disabled) { background: #a84e30; }
  .reply-btn:disabled { opacity: 0.45; cursor: default; }
  .reply-controls { display: flex; align-items: center; gap: 0.5rem; }
  .reply-attach { display: inline-grid; place-items: center; width: 2.2rem; height: 2.2rem; border: 1px solid #d8c6b1; cursor: pointer; font-size: 0.95rem; }
  .reply-attach:hover { border-color: #c65f3c; }
  .reply-atts { display: flex; flex-wrap: wrap; gap: 0.4rem; margin: 0.4rem 0; }
  .reply-att { position: relative; width: 52px; height: 52px; border: 1px solid #d8c6b1; overflow: hidden; }
  .reply-att img { width: 100%; height: 100%; object-fit: cover; }
  .reply-att button { position: absolute; top: 0; right: 0; width: 16px; height: 16px; background: rgba(52,37,28,0.8); color: #fff; border: none; cursor: pointer; line-height: 1; font-size: 0.7rem; }
</style>
