<script lang="ts">
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { AdminUserListItem, AdminUserDetail } from '$lib/types/api';

  const PER_PAGE = 20;

  // List state
  let items       = $state<AdminUserListItem[]>([]);
  let total       = $state(0);
  let page        = $state(1);
  let loading     = $state(true);
  let error       = $state('');
  let search      = $state('');
  let searchTimer = $state<ReturnType<typeof setTimeout> | null>(null);

  // Detail state
  let detail        = $state<AdminUserDetail | null>(null);
  let detailLoading = $state(false);
  let notesText     = $state('');
  let notesSaving   = $state(false);
  let notesSaved    = $state(false);
  let revoking      = $state(false);
  let blocking      = $state(false);

  // Reset-link state
  let resetLink      = $state('');
  let resetExpiry    = $state('');
  let resetCopied    = $state(false);
  let resetGenerating = $state(false);

  // Message compose state
  let msgSubject  = $state('');
  let msgBody     = $state('');
  let msgSending  = $state(false);
  let msgSent     = $state(false);

  async function load() {
    loading = true;
    error = '';
    try {
      const res = await api.adminListUsers({ search: search || undefined, page, perPage: PER_PAGE });
      items = res.items;
      total = res.total;
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function onSearchInput() {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => { page = 1; load(); }, 350);
  }

  async function openDetail(id: string) {
    detailLoading = true;
    detail = null;
    resetLink = '';
    resetExpiry = '';
    msgSubject = '';
    msgBody = '';
    msgSent = false;
    try {
      detail = await api.adminGetUser(id);
      notesText = detail.adminNotes ?? '';
      notesSaved = false;
    } catch { /* ignore */ } finally {
      detailLoading = false;
    }
  }

  function closeDetail() {
    detail = null;
    resetLink = '';
  }

  async function saveNotes() {
    if (!detail) return;
    notesSaving = true;
    notesSaved = false;
    try {
      await api.adminUpdateUserNotes(detail.id, notesText.trim() || null);
      const notes = notesText.trim() || null;
      detail = { ...detail, adminNotes: notes };
      items = items.map(u => u.id === detail!.id ? { ...u, adminNotes: notes } : u);
      notesSaved = true;
      setTimeout(() => { notesSaved = false; }, 2000);
    } catch { /* ignore */ } finally {
      notesSaving = false;
    }
  }

  async function revokeSessions() {
    if (!detail) return;
    if (!confirm($t('adminUsersRevokeConfirm'))) return;
    revoking = true;
    try {
      await api.adminRevokeUserSessions(detail.id);
      detail = { ...detail, sessions: detail.sessions.map(s => ({ ...s, isActive: false })) };
    } catch { /* ignore */ } finally {
      revoking = false;
    }
  }

  async function toggleBlock() {
    if (!detail) return;
    const willBlock = !detail.isBlocked;
    const msg = willBlock ? $t('adminUsersBlockConfirm') : $t('adminUsersUnblockConfirm');
    if (!confirm(msg)) return;
    blocking = true;
    try {
      await api.adminSetUserBlocked(detail.id, willBlock);
      detail = {
        ...detail,
        isBlocked: willBlock,
        // sessions are revoked server-side on block
        sessions: willBlock ? detail.sessions.map(s => ({ ...s, isActive: false })) : detail.sessions,
      };
      items = items.map(u => u.id === detail!.id ? { ...u, isBlocked: willBlock } : u);
    } catch { /* ignore */ } finally {
      blocking = false;
    }
  }

  async function generateResetLink() {
    if (!detail) return;
    if (!confirm($t('adminUsersResetConfirm'))) return;
    resetGenerating = true;
    resetCopied = false;
    try {
      const res = await api.adminGenerateResetToken(detail.id);
      const origin = typeof window !== 'undefined' ? window.location.origin : '';
      resetLink = `${origin}/set-password?token=${res.token}`;
      resetExpiry = new Date(res.expiresAt).toLocaleString();
    } catch { /* ignore */ } finally {
      resetGenerating = false;
    }
  }

  async function copyResetLink() {
    if (!resetLink) return;
    await navigator.clipboard.writeText(resetLink);
    resetCopied = true;
    setTimeout(() => { resetCopied = false; }, 2500);
  }

  async function sendMessage() {
    if (!detail || !msgSubject.trim() || !msgBody.trim() || msgSending) return;
    msgSending = true;
    try {
      const msg = await api.adminSendMessage(detail.id, msgSubject.trim(), msgBody.trim());
      detail = { ...detail, messages: [msg, ...detail.messages] };
      msgSubject = '';
      msgBody = '';
      msgSent = true;
      setTimeout(() => { msgSent = false; }, 2500);
    } catch { /* ignore */ } finally {
      msgSending = false;
    }
  }

  let totalPages    = $derived(Math.max(1, Math.ceil(total / PER_PAGE)));
  let activeSessions = $derived(detail?.sessions.filter(s => s.isActive) ?? []);

  $effect(() => { load(); });
</script>

{#if detail || detailLoading}
  <!-- ── DETAIL VIEW ── -->
  <div class="users-detail">
    <button class="back-btn" onclick={closeDetail}>{$t('adminUsersBack')}</button>

    {#if detailLoading}
      <p class="loading-text">…</p>
    {:else if detail}
      <div class="detail-header">
        <div class="detail-name-row">
          <h2 class="detail-name">{detail.displayName}</h2>
          {#if detail.isBlocked}
            <span class="blocked-badge">{$t('adminUsersBlocked')}</span>
          {/if}
        </div>
        <span class="detail-email">{detail.email}</span>
        <span class="detail-date">{$t('adminUsersRegistered')}: {new Date(detail.createdAt).toLocaleDateString()}</span>
      </div>

      <!-- Block / Password reset actions -->
      <section class="detail-section actions-section">
        <div class="action-row">
          <button
            class="action-btn"
            class:danger={!detail.isBlocked}
            class:safe={detail.isBlocked}
            onclick={toggleBlock}
            disabled={blocking}
          >
            {blocking ? '…' : detail.isBlocked ? $t('adminUsersUnblock') : $t('adminUsersBlock')}
          </button>

          <div class="reset-group">
            <button class="action-btn" onclick={generateResetLink} disabled={resetGenerating}>
              {resetGenerating ? '…' : $t('adminUsersResetPassword')}
            </button>
            {#if resetLink}
              <div class="reset-link-box">
                <input class="reset-link-input" type="text" readonly value={resetLink} onclick={(e) => (e.target as HTMLInputElement).select()} />
                <button class="copy-btn" onclick={copyResetLink}>
                  {resetCopied ? $t('adminUsersResetCopied') + ' ✓' : '⎘'}
                </button>
              </div>
              <p class="reset-hint">{$t('adminUsersResetExpiry')} ({resetExpiry})</p>
            {/if}
          </div>
        </div>
      </section>

      <!-- Notes -->
      <section class="detail-section">
        <h3 class="section-title">{$t('adminUsersNotesLabel')}</h3>
        <textarea
          class="notes-input"
          bind:value={notesText}
          rows="3"
          placeholder="…"
        ></textarea>
        <div class="notes-actions">
          <button class="save-btn" onclick={saveNotes} disabled={notesSaving}>
            {notesSaving ? '…' : notesSaved ? $t('adminUsersSaved') + ' ✓' : $t('adminUsersNotesSave')}
          </button>
        </div>
      </section>

      <!-- Security -->
      <section class="detail-section">
        <h3 class="section-title">{$t('adminUsersSessionsTitle')}</h3>
        <div class="sessions-meta">
          <span class="failures-badge" class:danger={detail.recentFailures >= 3}>
            {$t('adminUsersFailures')}: <strong>{detail.recentFailures}</strong>
          </span>
          {#if activeSessions.length > 0}
            <button class="revoke-btn" onclick={revokeSessions} disabled={revoking}>
              {revoking ? '…' : $t('adminUsersRevokeAll')} ({activeSessions.length})
            </button>
          {/if}
        </div>
        {#if detail.sessions.length > 0}
          <ul class="sessions-list">
            {#each detail.sessions as s}
              <li class="session-item" class:active={s.isActive} class:expired={!s.isActive}>
                <span class="session-status">{s.isActive ? $t('adminUsersActive') : $t('adminUsersExpired')}</span>
                <span class="session-dates">
                  {new Date(s.createdAt).toLocaleString()} → {new Date(s.expiresAt).toLocaleDateString()}
                </span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="empty-text">{$t('adminUsersEmpty')}</p>
        {/if}
      </section>

      <!-- Bookings -->
      <section class="detail-section">
        <h3 class="section-title">{$t('adminUsersBookingsTitle')} ({detail.bookings.length})</h3>
        {#if detail.bookings.length > 0}
          <ul class="history-list">
            {#each detail.bookings as b}
              <li class="history-item">
                <span class="history-name">{b.figurineName}</span>
                <span class="history-dates">{b.startsAt} — {b.endsAt}</span>
                <span class="history-status status-{b.status}">{b.status}</span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="empty-text">{$t('adminUsersEmpty')}</p>
        {/if}
      </section>

      <!-- Orders -->
      <section class="detail-section">
        <h3 class="section-title">{$t('adminUsersOrdersTitle')} ({detail.orders.length})</h3>
        {#if detail.orders.length > 0}
          <ul class="history-list">
            {#each detail.orders as o}
              <li class="history-item">
                <span class="history-name">{o.figurineName}</span>
                <span class="history-dates">{o.mode}</span>
                <span class="history-status status-{o.status}">{o.status}</span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="empty-text">{$t('adminUsersEmpty')}</p>
        {/if}
      </section>

      <!-- Messages -->
      <section class="detail-section">
        <h3 class="section-title">{$t('adminUsersMessagesTitle')}</h3>

        <div class="msg-compose">
          <input
            class="msg-input"
            bind:value={msgSubject}
            placeholder={$t('adminUsersMessagesSubject')}
          />
          <textarea
            class="msg-textarea"
            bind:value={msgBody}
            rows="3"
            placeholder={$t('adminUsersMessagesBody')}
          ></textarea>
          <button
            class="msg-send-btn"
            onclick={sendMessage}
            disabled={msgSending || !msgSubject.trim() || !msgBody.trim()}
          >
            {msgSending ? $t('adminUsersMessagesSending') : msgSent ? $t('adminUsersMessagesSent') : $t('adminUsersMessagesSend')}
          </button>
        </div>

        {#if detail.messages.length === 0}
          <p class="empty-text">{$t('adminUsersMessagesEmpty')}</p>
        {:else}
          <ul class="msg-list">
            {#each detail.messages as msg}
              <li class="msg-item" class:msg-unread={!msg.readAt}>
                <div class="msg-meta">
                  <span class="msg-from">{msg.fromAdmin ? '→ пользователю' : $t('adminUsersMessagesFromUser')}</span>
                  {#if !msg.readAt && !msg.fromAdmin}
                    <span class="msg-badge">new</span>
                  {/if}
                  <span class="msg-date">{new Date(msg.createdAt).toLocaleDateString()}</span>
                </div>
                {#if msg.subject}
                  <p class="msg-subject">{msg.subject}</p>
                {/if}
                <p class="msg-body">{msg.body}</p>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}
  </div>

{:else}
  <!-- ── LIST VIEW ── -->
  <div class="users-panel">
    <div class="toolbar">
      <input
        class="search-input"
        type="search"
        placeholder={$t('adminUsersSearch')}
        bind:value={search}
        oninput={onSearchInput}
      />
      <span class="total-badge">{$t('adminUsersTotal')}: {total}</span>
    </div>

    {#if loading}
      <p class="loading-text">…</p>
    {:else if error}
      <p class="error-text">{error}</p>
    {:else if items.length === 0}
      <p class="empty-text">{$t('adminUsersNoResults')}</p>
    {:else}
      <table class="users-table">
        <thead>
          <tr>
            <th>{$t('adminUsersEmail')}</th>
            <th>{$t('adminUsersName')}</th>
            <th class="num-col">{$t('adminUsersBookings')}</th>
            <th class="num-col">{$t('adminUsersOrders')}</th>
            <th>{$t('adminUsersRegistered')}</th>
            <th class="status-col"></th>
          </tr>
        </thead>
        <tbody>
          {#each items as u}
            <tr class="user-row" class:blocked-row={u.isBlocked} onclick={() => openDetail(u.id)}>
              <td class="email-cell">{u.email}</td>
              <td>{u.displayName}</td>
              <td class="num-col">{u.bookingCount}</td>
              <td class="num-col">{u.orderCount}</td>
              <td class="date-cell">{new Date(u.createdAt).toLocaleDateString()}</td>
              <td class="status-col">
                {#if u.isBlocked}
                  <span class="list-blocked-badge">{$t('adminUsersBlocked')}</span>
                {:else if u.adminNotes}
                  <span class="notes-dot" title={u.adminNotes}>●</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>

      {#if totalPages > 1}
        <div class="pagination">
          <button disabled={page <= 1} onclick={() => { page--; load(); }}>‹</button>
          <span>{page} / {totalPages}</span>
          <button disabled={page >= totalPages} onclick={() => { page++; load(); }}>›</button>
        </div>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .users-panel, .users-detail {
    padding: 1rem 0;
    font-family: Inter, sans-serif;
    color: #34251c;
  }

  /* Toolbar */
  .toolbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1rem;
  }
  .search-input {
    flex: 1;
    max-width: 360px;
    padding: .45rem .75rem;
    border: 1px solid #d8c6b1;
    background: #fdf8f2;
    color: #34251c;
    font-family: Inter, sans-serif;
    font-size: .85rem;
    outline: none;
  }
  .search-input:focus { border-color: #c65f3c; }
  .total-badge { font-size: .8rem; color: #6f3b24; opacity: .7; }

  /* Table */
  .users-table {
    width: 100%;
    border-collapse: collapse;
    font-size: .82rem;
  }
  .users-table th {
    text-align: left;
    padding: .4rem .6rem;
    border-bottom: 1px solid #d8c6b1;
    color: #6f3b24;
    font-weight: 600;
    font-size: .75rem;
    text-transform: uppercase;
    letter-spacing: .04em;
  }
  .users-table td {
    padding: .5rem .6rem;
    border-bottom: 1px solid #ede3d7;
    vertical-align: middle;
  }
  .user-row { cursor: pointer; transition: background .12s; }
  .user-row:hover td { background: #fdf4ec; }
  .blocked-row td { opacity: .55; }
  .num-col   { text-align: center; width: 64px; }
  .status-col { width: 80px; text-align: right; }
  .email-cell { font-weight: 500; color: #34251c; }
  .date-cell  { color: #888; font-size: .78rem; }
  .notes-dot  { color: #c65f3c; font-size: .9rem; cursor: default; }
  .list-blocked-badge {
    font-size: .68rem;
    text-transform: uppercase;
    letter-spacing: .05em;
    color: #c65f3c;
    border: 1px solid #c65f3c55;
    padding: .1rem .35rem;
  }

  /* Pagination */
  .pagination {
    display: flex;
    align-items: center;
    gap: .75rem;
    margin-top: .75rem;
    font-size: .82rem;
    color: #6f3b24;
  }
  .pagination button {
    background: none;
    border: 1px solid #d8c6b1;
    color: #6f3b24;
    padding: .25rem .6rem;
    cursor: pointer;
    font-size: .9rem;
  }
  .pagination button:disabled { opacity: .35; cursor: default; }

  /* Detail */
  .back-btn {
    background: none;
    border: none;
    color: #6f3b24;
    cursor: pointer;
    font-size: .82rem;
    padding: 0;
    margin-bottom: 1rem;
    text-decoration: underline dotted;
  }
  .detail-header {
    margin-bottom: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: .25rem;
  }
  .detail-name-row {
    display: flex;
    align-items: center;
    gap: .75rem;
  }
  .detail-name {
    margin: 0;
    font-family: Fraunces, Georgia, serif;
    font-size: 1.3rem;
    color: #34251c;
  }
  .blocked-badge {
    font-size: .7rem;
    text-transform: uppercase;
    letter-spacing: .06em;
    color: #fff;
    background: #c65f3c;
    padding: .15rem .5rem;
  }
  .detail-email { font-size: .85rem; color: #6f3b24; }
  .detail-date  { font-size: .78rem; color: #999; }

  /* Sections */
  .detail-section {
    margin-bottom: 1.75rem;
    border-top: 1px solid #d8c6b1;
    padding-top: 1rem;
  }
  .section-title {
    margin: 0 0 .75rem;
    font-size: .78rem;
    text-transform: uppercase;
    letter-spacing: .05em;
    color: #6f3b24;
  }

  /* Actions section */
  .actions-section { border-top: none; padding-top: 0; margin-bottom: 1.5rem; }
  .action-row {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: .75rem;
  }
  .action-btn {
    padding: .35rem .9rem;
    border: 1px solid #d8c6b1;
    background: none;
    color: #34251c;
    font-size: .8rem;
    cursor: pointer;
    white-space: nowrap;
    transition: background .12s, color .12s, border-color .12s;
  }
  .action-btn:hover:not(:disabled) { border-color: #6f3b24; }
  .action-btn.danger:hover:not(:disabled) { border-color: #c65f3c; color: #c65f3c; }
  .action-btn.safe:hover:not(:disabled)   { border-color: #4a7c59; color: #4a7c59; }
  .action-btn:disabled { opacity: .5; cursor: default; }

  /* Reset link */
  .reset-group {
    display: flex;
    flex-direction: column;
    gap: .4rem;
    flex: 1;
    min-width: 0;
  }
  .reset-link-box {
    display: flex;
    gap: .4rem;
    align-items: center;
  }
  .reset-link-input {
    flex: 1;
    min-width: 0;
    padding: .3rem .5rem;
    border: 1px solid #d8c6b1;
    background: #fdf8f2;
    color: #34251c;
    font-size: .75rem;
    font-family: monospace;
    outline: none;
    cursor: text;
  }
  .copy-btn {
    padding: .3rem .65rem;
    border: 1px solid #d8c6b1;
    background: none;
    color: #6f3b24;
    font-size: .8rem;
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .copy-btn:hover { border-color: #6f3b24; }
  .reset-hint { margin: 0; font-size: .73rem; color: #888; }

  /* Notes */
  .notes-input {
    width: 100%;
    max-width: 520px;
    padding: .5rem .6rem;
    border: 1px solid #d8c6b1;
    background: #fdf8f2;
    color: #34251c;
    font-family: Inter, sans-serif;
    font-size: .82rem;
    resize: vertical;
    outline: none;
    box-sizing: border-box;
  }
  .notes-input:focus { border-color: #c65f3c; }
  .notes-actions { margin-top: .5rem; }
  .save-btn {
    padding: .35rem .9rem;
    border: 1px solid #c65f3c;
    background: none;
    color: #c65f3c;
    font-size: .8rem;
    cursor: pointer;
    transition: background .12s, color .12s;
  }
  .save-btn:hover:not(:disabled) { background: #c65f3c; color: #fff; }
  .save-btn:disabled { opacity: .5; cursor: default; }

  /* Sessions */
  .sessions-meta {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: .6rem;
  }
  .failures-badge { font-size: .8rem; color: #6f3b24; }
  .failures-badge.danger { color: #c65f3c; font-weight: 600; }
  .revoke-btn {
    padding: .3rem .75rem;
    border: 1px solid #c65f3c;
    background: none;
    color: #c65f3c;
    font-size: .78rem;
    cursor: pointer;
  }
  .revoke-btn:hover:not(:disabled) { background: #c65f3c; color: #fff; }
  .revoke-btn:disabled { opacity: .5; cursor: default; }

  .sessions-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: .35rem;
  }
  .session-item {
    display: flex;
    gap: .75rem;
    align-items: center;
    font-size: .78rem;
  }
  .session-status { min-width: 52px; font-weight: 600; }
  .session-item.active .session-status  { color: #4a7c59; }
  .session-item.expired .session-status { color: #aaa; }
  .session-dates { color: #888; }

  /* History lists */
  .history-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: .3rem;
  }
  .history-item {
    display: flex;
    gap: .75rem;
    align-items: baseline;
    font-size: .8rem;
    padding: .35rem 0;
    border-bottom: 1px dotted #e8dcd0;
  }
  .history-name   { flex: 1; font-weight: 500; }
  .history-dates  { color: #888; font-size: .76rem; min-width: 130px; }
  .history-status {
    font-size: .72rem;
    text-transform: uppercase;
    letter-spacing: .04em;
    padding: .15rem .4rem;
    border: 1px solid currentColor;
  }
  .status-pending, .status-new        { color: #c65f3c; }
  .status-confirmed, .status-replied  { color: #4a7c59; }
  .status-cancelled, .status-rejected { color: #aaa; }
  .status-seen { color: #6f3b24; }

  /* Misc */
  .loading-text, .empty-text, .error-text {
    font-size: .82rem;
    color: #aaa;
    padding: .5rem 0;
  }
  .error-text { color: #c65f3c; }

  /* Messages */
  .msg-compose {
    display: flex;
    flex-direction: column;
    gap: .4rem;
    margin-bottom: .75rem;
  }
  .msg-input, .msg-textarea {
    font-family: inherit;
    font-size: .82rem;
    background: #fdf8f2;
    border: 1px solid #d8c6b1;
    color: #34251c;
    padding: .35rem .5rem;
    outline: none;
    width: 100%;
    box-sizing: border-box;
  }
  .msg-textarea { resize: vertical; line-height: 1.45; }
  .msg-input:focus, .msg-textarea:focus { border-color: #c65f3c; }
  .msg-send-btn {
    align-self: flex-end;
    background: transparent;
    border: 1px solid #c65f3c;
    color: #c65f3c;
    font-family: Inter, sans-serif;
    font-size: .7rem;
    letter-spacing: .07em;
    text-transform: uppercase;
    padding: .3rem .75rem;
    cursor: pointer;
  }
  .msg-send-btn:hover:not(:disabled) { background: rgba(198,95,60,.08); }
  .msg-send-btn:disabled { opacity: .45; cursor: default; }
  .msg-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: .5rem;
  }
  .msg-item {
    border: 1px solid #e8ddd0;
    padding: .5rem .65rem;
    background: #fdf8f2;
  }
  .msg-unread { border-left: 2px solid #c65f3c; }
  .msg-meta {
    display: flex;
    align-items: center;
    gap: .4rem;
    margin-bottom: .2rem;
  }
  .msg-from {
    font-size: .68rem;
    letter-spacing: .06em;
    text-transform: uppercase;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
  }
  .msg-badge {
    font-size: .6rem;
    background: #c65f3c;
    color: #fff;
    padding: 1px 4px;
    border-radius: 2px;
    letter-spacing: .05em;
    text-transform: uppercase;
  }
  .msg-date {
    font-size: .68rem;
    color: #b5a090;
    font-family: Inter, sans-serif;
    margin-left: auto;
  }
  .msg-subject {
    font-family: Georgia, serif;
    font-size: .88rem;
    color: #34251c;
    margin: 0 0 .15rem;
  }
  .msg-body {
    font-family: Inter, sans-serif;
    font-size: .8rem;
    color: #6f4e37;
    margin: 0;
    white-space: pre-wrap;
    line-height: 1.45;
  }
</style>
