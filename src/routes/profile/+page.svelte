<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { UserBookingDto, UserOrderDto, MessageThreadDto, ThreadDetailDto } from '$lib/types/api';

  type Tab = 'bookings' | 'orders' | 'wishlist' | 'messages';
  let activeTab = $state<Tab>('bookings');

  let bookings = $state<UserBookingDto[]>([]);
  let orders = $state<UserOrderDto[]>([]);
  let wishlistIds = $state<string[]>([]);
  let wishlistItems = $state<Map<string, { name: string; status: string } | null>>(new Map());
  let threads = $state<MessageThreadDto[]>([]);
  let unreadCount = $state(0);
  let loading = $state(true);
  let error = $state('');

  // Thread view
  type MsgFilter = 'all' | 'booking' | 'order' | 'waitlist' | 'general';
  let msgFilter = $state<MsgFilter>('all');
  let openThread = $state<ThreadDetailDto | null>(null);
  let threadLoading = $state(false);
  let messagesEl: HTMLElement;

  // Reply
  let replyBody = $state('');
  let replySending = $state(false);
  let replySent = $state(false);

  // Compose new thread
  let showCompose = $state(false);
  let composeSubject = $state('');
  let composeBody = $state('');
  let composeSending = $state(false);
  let composeSent = $state(false);


  // Account editing
  let editingName = $state(false);
  let editNameValue = $state('');
  let savingName = $state(false);
  let nameSaved = $state(false);

  // Avatar upload
  let avatarInput: HTMLInputElement;
  let uploadingAvatar = $state(false);

  // Delete account
  let showDeleteConfirm = $state(false);
  let deleting = $state(false);

  onMount(async () => {
    if (!authStore.isLoggedIn && !authStore.token) {
      goto('/login?from=/profile');
      return;
    }
    if (!authStore.isLoggedIn && authStore.token) {
      try {
        const user = await api.userMe(authStore.token!);
        authStore.user = user;
      } catch {
        authStore.clearSession();
        goto('/login?from=/profile');
        return;
      }
    }
    await loadData();
  });

  async function loadData() {
    loading = true;
    error = '';
    try {
      const token = authStore.token!;
      const [b, o, th] = await Promise.all([
        api.userProfileBookings(token),
        api.userProfileOrders(token),
        api.getUserThreads(token),
      ]);
      bookings = b;
      orders = o;
      threads = th.threads;
      unreadCount = th.unread;

      if (typeof localStorage !== 'undefined') {
        wishlistIds = JSON.parse(localStorage.getItem('gotiga_wishlist') ?? '[]');
      }

      if (wishlistIds.length > 0) {
        loadWishlistDetails();
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : '';
      if (msg.includes('401')) {
        authStore.clearSession();
        goto('/login?from=/profile');
        return;
      }
      error = $t('authErrorServer');
    } finally {
      loading = false;
    }
  }

  async function loadWishlistDetails() {
    const results = await Promise.all(
      wishlistIds.map(id => api.getFigurine(id).catch(() => null))
    );
    const map = new Map<string, { name: string; status: string } | null>();
    results.forEach((fig, i) => {
      map.set(wishlistIds[i], fig ? { name: fig.name, status: fig.status } : null);
    });
    wishlistItems = map;
  }

  async function logout() {
    const token = authStore.token;
    if (token) {
      try { await api.userLogout(token); } catch { /* ok */ }
    }
    authStore.clearSession();
    goto('/');
  }

  function startEditName() {
    editNameValue = authStore.user?.displayName ?? '';
    editingName = true;
    nameSaved = false;
  }

  async function saveName() {
    if (!editNameValue.trim() || savingName) return;
    savingName = true;
    try {
      const updated = await api.updateProfile(authStore.token!, editNameValue.trim());
      authStore.user = updated;
      editingName = false;
      nameSaved = true;
      setTimeout(() => nameSaved = false, 2000);
    } catch {
      // keep editing open
    } finally {
      savingName = false;
    }
  }

  function cancelEditName() {
    editingName = false;
  }

  async function handleAvatarChange(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    uploadingAvatar = true;
    try {
      const updated = await api.uploadAvatar(authStore.token!, file);
      authStore.user = updated;
    } catch {
      // silent
    } finally {
      uploadingAvatar = false;
      if (avatarInput) avatarInput.value = '';
    }
  }

  async function deleteAccount() {
    if (deleting) return;
    deleting = true;
    try {
      await api.deleteAccount(authStore.token!);
      authStore.clearSession();
      goto('/');
    } catch {
      deleting = false;
      showDeleteConfirm = false;
    }
  }

  function scrollToBottom() {
    if (messagesEl) messagesEl.scrollTop = messagesEl.scrollHeight;
  }

  async function openThreadDetail(id: string) {
    threadLoading = true;
    openThread = null;
    replyBody = '';
    replySent = false;
    try {
      const detail = await api.getThread(authStore.token!, id);
      openThread = detail;
      const prev = threads.find(t => t.id === id);
      if (prev && prev.unread > 0) {
        unreadCount = Math.max(0, unreadCount - prev.unread);
        threads = threads.map(t => t.id === id ? { ...t, unread: 0 } : t);
      }
      await tick();
      scrollToBottom();
    } catch { /* silent */ } finally {
      threadLoading = false;
    }
  }

  function closeThread() {
    openThread = null;
    replyBody = '';
    replySent = false;
  }

  async function sendReply() {
    if (!replyBody.trim() || replySending || !openThread) return;
    replySending = true;
    try {
      const msg = await api.replyToThread(authStore.token!, openThread.thread.id, replyBody.trim());
      openThread = { ...openThread, messages: [...openThread.messages, msg] };
      replyBody = '';
      replySent = true;
      setTimeout(() => { replySent = false; }, 2000);
      await tick();
      scrollToBottom();
    } catch { /* silent */ } finally {
      replySending = false;
    }
  }

  async function sendCompose() {
    if (!composeSubject.trim() || !composeBody.trim() || composeSending) return;
    composeSending = true;
    try {
      const detail = await api.createThread(authStore.token!, composeSubject.trim(), composeBody.trim(), 'general');
      threads = [detail.thread, ...threads];
      composeSubject = '';
      composeBody = '';
      composeSent = true;
      setTimeout(() => {
        composeSent = false;
        showCompose = false;
      }, 1800);
    } catch { /* silent */ } finally {
      composeSending = false;
    }
  }

  let filteredThreads = $derived(
    msgFilter === 'all' ? threads : threads.filter(t => t.category === msgFilter)
  );

  function bookingStatusLabel(status: string): string {
    const map: Record<string, string> = {
      pending: 'Ожидает',
      confirmed: 'Подтверждено',
      rejected: 'Отклонено',
      cancelled: 'Отменено',
    };
    return map[status] ?? status;
  }

  function orderModeLabel(mode: string): string {
    const map: Record<string, string> = { request: 'Запрос', question: 'Вопрос', notify: 'Уведомление' };
    return map[mode] ?? mode;
  }

  function orderStatusLabel(status: string): string {
    if (status === 'replied') return $t('profileOrderReplied');
    if (status === 'seen') return $t('profileOrderSeen');
    return $t('profileOrderNew');
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString('ru-RU', { day: 'numeric', month: 'long', year: 'numeric' });
  }

  function formatDateRange(start: string, end: string): string {
    const s = new Date(start);
    const e = new Date(end);
    const opts: Intl.DateTimeFormatOptions = { day: 'numeric', month: 'short' };
    return `${s.toLocaleDateString('ru-RU', opts)} — ${e.toLocaleDateString('ru-RU', opts)}`;
  }

  function wishlistStatusLabel(status: string): string {
    const map: Record<string, string> = {
      available: 'доступна',
      sold: 'продана',
      reserved: 'зарезервирована',
      in_progress: 'в работе',
    };
    return map[status] ?? status;
  }

  function resolveAvatarUrl(url: string | null | undefined): string | null {
    if (!url) return null;
    if (url.startsWith('http')) return url;
    if (url.startsWith('/static/')) {
      if (typeof localStorage !== 'undefined') {
        const serverUrl = localStorage.getItem('gotiga_server_url') ?? '';
        return serverUrl ? `${serverUrl}${url}` : url;
      }
    }
    return url;
  }
</script>

<svelte:head>
  <title>{$t('profileTitle')} — Gotiga</title>
</svelte:head>

<div class="page">
  <div class="frame">
    <div class="body">

      <!-- ── Sidebar ── -->
      <aside class="sidebar">

        <!-- Profile (hero) -->
        <div class="sidebar-profile">
          <div class="hero-avatar-zone">
            <button
              class="hero-avatar-btn"
              onclick={() => avatarInput?.click()}
              title={$t('profileUploadPhoto')}
              disabled={uploadingAvatar}
            >
              {#if resolveAvatarUrl(authStore.user?.avatarUrl)}
                <img src={resolveAvatarUrl(authStore.user?.avatarUrl)} alt="" class="hero-avatar-img" />
              {:else}
                <span class="hero-avatar-initials">{(authStore.user?.displayName ?? '?')[0].toUpperCase()}</span>
              {/if}
              <span class="hero-avatar-overlay">{uploadingAvatar ? $t('profileUploadingPhoto') : $t('profileUploadPhoto')}</span>
            </button>
            <input
              bind:this={avatarInput}
              type="file"
              accept="image/jpeg,image/png,image/webp"
              class="sr-only"
              onchange={handleAvatarChange}
            />
          </div>

          <div class="hero-identity">
            {#if editingName}
              <div class="hero-name-edit">
                <input
                  class="hero-name-input"
                  bind:value={editNameValue}
                  onkeydown={(e) => { if (e.key === 'Enter') saveName(); if (e.key === 'Escape') cancelEditName(); }}
                  autofocus
                />
                <button class="hero-name-save" onclick={saveName} disabled={savingName}>
                  {savingName ? '…' : $t('profileSaveName')}
                </button>
                <button class="hero-name-cancel" onclick={cancelEditName}>✕</button>
              </div>
            {:else}
              <div class="hero-name-row">
                <h1 class="hero-name">{authStore.user?.displayName ?? ''}</h1>
                <button class="hero-edit-btn" onclick={startEditName} title={$t('profileEditName')}>
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
                    <path d="M8.5 1.5L10.5 3.5L4 10H2V8L8.5 1.5Z"/>
                  </svg>
                </button>
              </div>
              {#if nameSaved}<span class="hero-name-saved">{$t('profileNameSaved')}</span>{/if}
            {/if}
            <p class="hero-email">{authStore.user?.email ?? ''}</p>
            {#if authStore.user?.createdAt}
              <p class="hero-since">{$t('profileMemberSince')} {formatDate(authStore.user.createdAt)}</p>
            {/if}
          </div>
        </div>
        <nav class="sidebar-nav">
          <button class="nav-item" class:active={activeTab === 'bookings'} onclick={() => activeTab = 'bookings'}>
            <span>{$t('profileBookings')}</span>
            {#if bookings.length > 0}<span class="nav-badge">{bookings.length}</span>{/if}
          </button>
          <button class="nav-item" class:active={activeTab === 'orders'} onclick={() => activeTab = 'orders'}>
            <span>{$t('profileOrders')}</span>
            {#if orders.length > 0}<span class="nav-badge">{orders.length}</span>{/if}
          </button>
          <button class="nav-item" class:active={activeTab === 'wishlist'} onclick={() => activeTab = 'wishlist'}>
            <span>{$t('profileWishlist')}</span>
            {#if wishlistIds.length > 0}<span class="nav-badge">{wishlistIds.length}</span>{/if}
          </button>
          <button class="nav-item" class:active={activeTab === 'messages'} onclick={() => { activeTab = 'messages'; closeThread(); }}>
            <span>{$t('profileMessages')}</span>
            {#if unreadCount > 0}<span class="nav-badge nav-badge--unread">{unreadCount}</span>{/if}
          </button>
        </nav>

        <div class="sidebar-footer">
          <button class="sidebar-logout" onclick={logout}>{$t('profileLogout')}</button>
          {#if !showDeleteConfirm}
            <button class="sidebar-delete-btn" onclick={() => showDeleteConfirm = true}>{$t('profileDeleteAccount')}</button>
          {:else}
            <div class="sidebar-delete-confirm">
              <p class="sidebar-delete-warning">{$t('profileDeleteWarning')}</p>
              <div class="sidebar-delete-actions">
                <button class="sidebar-delete-yes" onclick={deleteAccount} disabled={deleting}>
                  {deleting ? $t('profileDeleting') : $t('profileDeleteConfirm')}
                </button>
                <button class="sidebar-delete-cancel" onclick={() => showDeleteConfirm = false}>
                  {$t('profileDeleteCancel')}
                </button>
              </div>
            </div>
          {/if}
        </div>
      </aside>

      <!-- Content -->
      <div class="content">
        {#if loading}
          <p class="empty">…</p>
        {:else if error}
          <p class="error-msg">{error}</p>
        {:else if activeTab === 'bookings'}
          {#if bookings.length === 0}
            <p class="empty">{$t('profileEmpty')}</p>
          {:else}
            <div class="cards-grid">
              {#each bookings as b}
                <a href="/figurines/{b.figurineId}" class="card">
                  <div class="card-head">
                    <span class="card-name">{b.figurineName}</span>
                    <span class="status status--{b.status}">{bookingStatusLabel(b.status)}</span>
                  </div>
                  <p class="card-range">{formatDateRange(b.startsAt, b.endsAt)}</p>
                  <p class="card-date">{formatDate(b.createdAt)}</p>
                </a>
              {/each}
            </div>
          {/if}
        {:else if activeTab === 'orders'}
          {#if orders.length === 0}
            <p class="empty">{$t('profileEmpty')}</p>
          {:else}
            <div class="cards-grid">
              {#each orders as o}
                <a href="/figurines/{o.figurineId}" class="card">
                  <div class="card-head">
                    <span class="card-name">{o.figurineName}</span>
                    <div class="order-badges">
                      <span class="mode">{orderModeLabel(o.mode)}</span>
                      <span class="order-status order-status--{o.status}">{orderStatusLabel(o.status)}</span>
                    </div>
                  </div>
                  <p class="card-date">{formatDate(o.createdAt)}</p>
                </a>
              {/each}
            </div>
          {/if}
        {:else if activeTab === 'wishlist'}
          {#if wishlistIds.length === 0}
            <p class="empty">{$t('profileEmpty')}</p>
          {:else}
            <div class="cards-grid">
              {#each wishlistIds as id}
                {@const item = wishlistItems.get(id)}
                <div class="card">
                  <div class="card-head">
                    {#if item}
                      <a href="/figurines/{id}" class="card-name card-name--link">{item.name}</a>
                      <span class="wishlist-status wishlist-status--{item.status}">{wishlistStatusLabel(item.status)}</span>
                    {:else if item === undefined}
                      <span class="card-name card-name--loading">…</span>
                    {:else}
                      <a href="/figurines/{id}" class="card-name card-name--missing">{id}</a>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {:else if activeTab === 'messages'}
          <div class="messages-wrap">
            {#if openThread}
              <!-- ── Thread detail ── -->
              <div class="thread-detail">
                <div class="thread-detail-header">
                  <button class="thread-back" onclick={closeThread}>← {$t('profileMessagesBack')}</button>
                  <span class="thread-detail-subject">{openThread.thread.subject}</span>
                  {#if openThread.thread.status === 'resolved'}
                    <span class="thread-resolved-badge">{$t('profileMessagesResolved')}</span>
                  {/if}
                </div>

                <div class="chat-messages" bind:this={messagesEl}>
                  {#if threadLoading}
                    <p class="empty">…</p>
                  {:else}
                    {#each openThread.messages as msg}
                      <div class="chat-row" class:chat-row--user={!msg.fromAdmin} class:chat-row--admin={msg.fromAdmin}>
                        <div class="chat-bubble" class:chat-bubble--user={!msg.fromAdmin} class:chat-bubble--admin={msg.fromAdmin}>
                          <p class="chat-sender">{msg.fromAdmin ? $t('profileMessagesFromAdmin') : $t('profileMessagesFromYou')}</p>
                          <p class="chat-body">{msg.body}</p>
                        </div>
                        <p class="chat-time">{formatDate(msg.createdAt)}</p>
                      </div>
                    {/each}
                  {/if}
                </div>

                {#if openThread.thread.status !== 'resolved'}
                  <div class="chat-reply">
                    <textarea
                      class="chat-reply-input"
                      bind:value={replyBody}
                      rows="2"
                      placeholder={$t('profileMessageWriteBody')}
                      onkeydown={(e) => { if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) sendReply(); }}
                    ></textarea>
                    <div class="chat-reply-footer">
                      <span class="chat-reply-hint">Ctrl+Enter</span>
                      <button
                        class="chat-reply-btn"
                        onclick={sendReply}
                        disabled={replySending || !replyBody.trim()}
                      >
                        {replySending ? $t('profileMessagesReplying') : replySent ? $t('profileMessageWriteSent') : $t('profileMessagesReply')} →
                      </button>
                    </div>
                  </div>
                {:else}
                  <p class="chat-resolved-note">{$t('profileMessagesResolved')}</p>
                {/if}
              </div>

            {:else}
              <!-- ── Thread list ── -->
              <div class="threads-header">
                <div class="thread-filters">
                  {#each (['all', 'booking', 'order', 'waitlist', 'general'] as MsgFilter[]) as f}
                    <button
                      class="thread-filter-btn"
                      class:active={msgFilter === f}
                      onclick={() => msgFilter = f}
                    >
                      {#if f === 'all'}{$t('profileMessagesAll')}
                      {:else if f === 'booking'}{$t('profileMessagesBooking')}
                      {:else if f === 'order'}{$t('profileMessagesOrder')}
                      {:else if f === 'waitlist'}{$t('profileMessagesWaitlist')}
                      {:else}{$t('profileMessagesGeneral')}
                      {/if}
                    </button>
                  {/each}
                </div>
                <button class="compose-toggle-btn" onclick={() => showCompose = !showCompose}>
                  {$t('profileMessagesCompose')}
                </button>
              </div>

              {#if showCompose}
                <div class="msg-compose">
                  <input
                    class="msg-subject"
                    bind:value={composeSubject}
                    placeholder={$t('profileMessageWriteSubject')}
                  />
                  <textarea
                    class="msg-body"
                    bind:value={composeBody}
                    rows="3"
                    placeholder={$t('profileMessageWriteBody')}
                  ></textarea>
                  <button
                    class="msg-send-btn"
                    onclick={sendCompose}
                    disabled={composeSending || !composeSubject.trim() || !composeBody.trim()}
                  >
                    {composeSending ? $t('profileMessagesSending') : composeSent ? $t('profileMessagesSent') : $t('profileMessagesSend')}
                  </button>
                </div>
              {/if}

              {#if filteredThreads.length === 0}
                <p class="empty">{$t('profileMessagesEmpty')}</p>
              {:else}
                <ul class="list">
                  {#each filteredThreads as thread}
                    <li class="item msg-item" class:msg-unread={thread.unread > 0} onclick={() => openThreadDetail(thread.id)}>
                      <div class="item-main">
                        <span class="thread-subject">{thread.subject}</span>
                        <div class="thread-meta-right">
                          {#if thread.unread > 0}
                            <span class="msg-new-badge">{thread.unread}</span>
                          {/if}
                          {#if thread.status === 'resolved'}
                            <span class="thread-resolved-small">{$t('profileMessagesResolved')}</span>
                          {/if}
                        </div>
                      </div>
                      {#if thread.preview}
                        <p class="msg-body-preview">{thread.preview}</p>
                      {/if}
                      <p class="item-date">{formatDate(thread.lastMessageAt)}</p>
                    </li>
                  {/each}
                </ul>
              {/if}
            {/if}
          </div>
        {/if}
      </div>

    </div>
  </div>
</div>

<style>
  /* ── Page shell ── */

  .page {
    min-height: 100vh;
    background: #f8f1e7;
    font-family: Georgia, serif;
    color: #34251c;
  }

  .frame { display: contents; }

  /* ── Body layout ── */

  /* ── Sidebar profile (hero) ── */

  .sidebar-profile {
    background: #2a1a10;
    padding: 1.75rem 1.25rem 1.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    text-align: center;
  }

  .hero-avatar-zone { position: relative; }

  .hero-avatar-btn {
    width: 70px;
    height: 70px;
    border-radius: 50%;
    border: 2px solid rgba(248,241,231,0.2);
    background: rgba(248,241,231,0.06);
    cursor: pointer;
    overflow: hidden;
    position: relative;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: border-color 0.2s;
  }
  .hero-avatar-btn:hover { border-color: rgba(198,95,60,0.6); }
  .hero-avatar-btn:disabled { cursor: default; opacity: 0.7; }

  .hero-avatar-img { width: 100%; height: 100%; object-fit: cover; }

  .hero-avatar-initials {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.6rem;
    color: rgba(248,241,231,0.55);
    line-height: 1;
    pointer-events: none;
  }

  .hero-avatar-overlay {
    position: absolute;
    inset: 0;
    background: rgba(42,26,16,0.72);
    color: #f8f1e7;
    font-family: Inter, sans-serif;
    font-size: 0.55rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 4px;
    opacity: 0;
    transition: opacity 0.18s;
    pointer-events: none;
  }
  .hero-avatar-btn:hover .hero-avatar-overlay { opacity: 1; }
  .hero-avatar-btn:disabled .hero-avatar-overlay { opacity: 1; }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }

  .hero-identity { text-align: center; }

  .hero-name-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    margin-bottom: 0.25rem;
  }

  .hero-name {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.35rem;
    font-weight: 400;
    color: #f8f1e7;
    margin: 0;
    line-height: 1.2;
  }

  .hero-edit-btn {
    background: transparent;
    border: none;
    color: rgba(248,241,231,0.3);
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
    transition: color 0.15s;
    flex-shrink: 0;
  }
  .hero-edit-btn:hover { color: #c65f3c; }

  .hero-name-saved {
    display: block;
    font-family: Inter, sans-serif;
    font-size: 0.62rem;
    color: #6a9e5a;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    text-align: center;
    margin-bottom: 0.2rem;
  }

  .hero-name-edit {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    margin-bottom: 0.25rem;
  }

  .hero-name-input {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.1rem;
    background: transparent;
    border: none;
    border-bottom: 1.5px solid rgba(198,95,60,0.65);
    color: #f8f1e7;
    padding: 2px 0;
    outline: none;
    text-align: center;
    width: 180px;
  }

  .hero-name-save {
    background: transparent;
    border: 1px solid rgba(198,95,60,0.55);
    color: rgba(198,95,60,0.9);
    font-family: Inter, sans-serif;
    font-size: 0.67rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 2px 8px;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.15s;
  }
  .hero-name-save:hover:not(:disabled) { background: rgba(198,95,60,0.12); }
  .hero-name-save:disabled { opacity: 0.5; }

  .hero-name-cancel {
    background: transparent;
    border: none;
    color: rgba(248,241,231,0.35);
    cursor: pointer;
    font-size: 0.85rem;
    padding: 2px 4px;
    line-height: 1;
    flex-shrink: 0;
    transition: color 0.15s;
  }
  .hero-name-cancel:hover { color: #f8f1e7; }

  .hero-email {
    font-family: Inter, sans-serif;
    font-size: 0.79rem;
    color: rgba(200,168,130,0.75);
    margin: 0 0 0.12rem;
  }

  .hero-since {
    font-family: Inter, sans-serif;
    font-size: 0.67rem;
    color: rgba(200,168,130,0.45);
    margin: 0;
    letter-spacing: 0.02em;
  }

  /* ── Body layout ── */

  .body {
    display: flex;
    align-items: flex-start;
    min-height: calc(100vh - 68px);
  }

  /* ── Sidebar ── */

  .sidebar {
    position: fixed;
    left: 0;
    top: 68px;
    height: calc(100vh - 68px);
    width: 220px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    border-right: 1px solid rgba(52,37,28,0.12);
    z-index: 5;
  }

  @media (max-width: 680px) {
    .sidebar {
      position: static;
      width: 100%;
      height: auto;
      z-index: auto;
    }
    .body {
      flex-direction: column;
      height: auto;
      min-height: calc(100vh - 58px);
    }
    .content {
      margin-left: 0;
    }
  }

  .sidebar-nav {
    flex: 1;
    padding: 0.85rem 0;
    display: flex;
    flex-direction: column;
    background: #f0e6d6;
  }

  .nav-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.6rem 1.25rem;
    background: transparent;
    border: none;
    border-left: 2px solid transparent;
    text-align: left;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #9a7c5c;
    cursor: pointer;
    transition: all 0.15s;
  }
  .nav-item:hover { background: rgba(52,37,28,0.05); color: #34251c; }
  .nav-item.active { color: #c65f3c; background: rgba(198,95,60,0.07); border-left-color: #c65f3c; }

  .nav-badge {
    background: #d8c6b1;
    color: #6f3b24;
    font-family: Inter, sans-serif;
    font-size: 0.6rem;
    border-radius: 10px;
    padding: 1px 6px;
    min-width: 18px;
    text-align: center;
    flex-shrink: 0;
  }
  .nav-badge--unread { background: #c65f3c; color: #fff; }

  .sidebar-footer {
    padding: 1rem 1.25rem;
    border-top: 1px solid rgba(52,37,28,0.1);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    background: #f0e6d6;
  }

  .sidebar-logout {
    background: transparent;
    border: none;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    font-size: 0.7rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    cursor: pointer;
    padding: 0.25rem 0;
    text-align: left;
    transition: color 0.15s;
  }
  .sidebar-logout:hover { color: #34251c; }

  .sidebar-delete-btn {
    background: transparent;
    border: none;
    color: #c8b89a;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    cursor: pointer;
    padding: 0;
    text-align: left;
    transition: color 0.15s;
  }
  .sidebar-delete-btn:hover { color: #c65f3c; }

  .sidebar-delete-confirm {
    padding: 0.65rem 0.75rem;
    background: #fdf3f3;
    border: 1px solid #f0d0c8;
  }

  .sidebar-delete-warning {
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    color: #7a3020;
    margin: 0 0 0.6rem;
    line-height: 1.45;
  }

  .sidebar-delete-actions {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .sidebar-delete-yes {
    background: #9b2020;
    border: none;
    color: #fff;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.35rem 0.75rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .sidebar-delete-yes:hover:not(:disabled) { background: #7a1818; }
  .sidebar-delete-yes:disabled { opacity: 0.6; cursor: not-allowed; }

  .sidebar-delete-cancel {
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.3rem 0.75rem;
    cursor: pointer;
    transition: all 0.15s;
    text-align: center;
  }
  .sidebar-delete-cancel:hover { border-color: #9a7c5c; color: #34251c; }

  /* ── Content ── */

  .content {
    flex: 1;
    margin-left: 220px;
    background: #f8f1e7;
    padding: 2rem 2.5rem;
    min-height: 400px;
  }

  .empty {
    color: #9a7c5c;
    font-style: italic;
    font-size: 0.9rem;
    padding: 2.5rem 0;
    text-align: center;
    font-family: Georgia, serif;
  }

  .error-msg {
    color: #c65f3c;
    font-size: 0.85rem;
    font-family: Inter, sans-serif;
    padding: 1rem 0;
  }

  /* ── Cards grid ── */

  .cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.85rem;
    max-width: 1100px;
  }

  .card {
    display: block;
    background: #fff;
    border: 1px solid #e4d8c8;
    padding: 1rem 1.1rem;
    text-decoration: none;
    color: #34251c;
    transition: border-color 0.18s, box-shadow 0.18s, transform 0.15s;
  }
  .card:hover {
    border-color: #c65f3c;
    box-shadow: 0 2px 10px rgba(198,95,60,0.1);
    transform: translateY(-1px);
  }

  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.4rem;
    margin-bottom: 0.4rem;
  }

  .card-name {
    font-family: Georgia, serif;
    font-size: 0.93rem;
    color: #34251c;
    line-height: 1.35;
    flex: 1;
    min-width: 0;
    text-decoration: none;
  }
  .card-name--link:hover { color: #c65f3c; }
  .card-name--loading { color: #b5a090; font-style: italic; }
  .card-name--missing { color: #b5a090; font-size: 0.78rem; font-family: 'Courier New', monospace; }

  .card-range {
    font-size: 0.77rem;
    color: #9a7c5c;
    margin: 0 0 0.2rem;
    font-family: Inter, sans-serif;
    font-style: italic;
  }

  .card-date {
    font-size: 0.68rem;
    color: #b5a090;
    margin: 0;
    font-family: Inter, sans-serif;
  }

  .status {
    font-size: 0.62rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-family: Inter, sans-serif;
    padding: 2px 6px;
    border-radius: 2px;
    flex-shrink: 0;
    white-space: nowrap;
  }
  .status--pending   { background: #f4ead8; color: #6f3b24; }
  .status--confirmed { background: #e8f4e8; color: #2d6a3f; }
  .status--rejected  { background: #fde8e8; color: #9b2020; }
  .status--cancelled { background: #ebebeb; color: #666; }

  .order-badges {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex-shrink: 0;
  }

  .mode {
    font-size: 0.62rem;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .order-status {
    font-size: 0.6rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    font-family: Inter, sans-serif;
    padding: 2px 5px;
    border-radius: 2px;
    white-space: nowrap;
  }
  .order-status--new     { background: #f4ead8; color: #6f3b24; }
  .order-status--seen    { background: #e8eef4; color: #2d4a6a; }
  .order-status--replied { background: #e8f4e8; color: #2d6a3f; }

  .wishlist-status {
    font-size: 0.67rem;
    letter-spacing: 0.05em;
    font-family: Inter, sans-serif;
    color: #9a7c5c;
    font-style: italic;
    flex-shrink: 0;
  }
  .wishlist-status--sold      { color: #b5a090; }
  .wishlist-status--available { color: #4a7a3a; }

  .item-date {
    font-size: 0.7rem;
    color: #b5a090;
    margin: 0.1rem 0 0;
    font-family: Inter, sans-serif;
  }


  /* ── Messages ── */

  .messages-wrap {
    max-width: 720px;
  }

  .badge--unread {
    background: #c65f3c;
    color: #fff;
  }

  .threads-header {
    position: sticky;
    top: 68px;
    z-index: 2;
    background: #f8f1e7;
    padding-top: 0.75rem;
    padding-bottom: 0.85rem;
    margin-bottom: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
    border-bottom: 1px solid rgba(216,198,177,0.5);
    margin-left: -2.5rem;
    margin-right: -2.5rem;
    padding-left: 2.5rem;
    padding-right: 2.5rem;
  }

  @media (max-width: 680px) {
    .threads-header { top: 58px; }
  }

  .thread-filters {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
  }

  .thread-filter-btn {
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.2rem 0.55rem;
    cursor: pointer;
    transition: all 0.15s;
  }
  .thread-filter-btn:hover { border-color: #9a7c5c; color: #34251c; }
  .thread-filter-btn.active { border-color: #c65f3c; color: #c65f3c; background: rgba(198,95,60,0.06); }

  .compose-toggle-btn {
    background: transparent;
    border: 1px solid #c65f3c;
    color: #c65f3c;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.2rem 0.65rem;
    cursor: pointer;
    transition: background 0.15s;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .compose-toggle-btn:hover { background: rgba(198,95,60,0.08); }

  .msg-compose {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 1.25rem;
    padding: 0.85rem;
    border: 1px solid #d8c6b1;
    background: rgba(248,241,231,0.5);
  }

  .msg-subject {
    font-family: Georgia, serif;
    font-size: 0.88rem;
    background: transparent;
    border: none;
    border-bottom: 1px solid #d8c6b1;
    color: #34251c;
    padding: 4px 0;
    outline: none;
  }
  .msg-subject::placeholder { color: #b5a090; }

  .msg-body {
    font-family: Inter, sans-serif;
    font-size: 0.82rem;
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #34251c;
    padding: 0.5rem;
    outline: none;
    resize: vertical;
    line-height: 1.5;
  }
  .msg-body::placeholder { color: #b5a090; }
  .msg-body:focus { border-color: #c65f3c; }

  .msg-send-btn {
    align-self: flex-end;
    background: transparent;
    border: 1px solid #c65f3c;
    color: #c65f3c;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    padding: 0.35rem 0.85rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .msg-send-btn:hover:not(:disabled) { background: rgba(198,95,60,0.08); }
  .msg-send-btn:disabled { opacity: 0.45; cursor: default; }

  .msg-item { cursor: pointer; transition: background 0.12s; }
  .msg-item:hover { background: rgba(216,198,177,0.15); }
  .msg-unread { border-left: 2px solid #c65f3c; padding-left: 0.6rem; }

  .thread-subject {
    font-family: Georgia, serif;
    font-size: 0.92rem;
    color: #34251c;
    line-height: 1.3;
    flex: 1;
    min-width: 0;
  }

  .thread-meta-right {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex-shrink: 0;
  }

  .msg-new-badge {
    font-family: Inter, sans-serif;
    font-size: 0.6rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    background: #c65f3c;
    color: #fff;
    padding: 1px 5px;
  }

  .thread-resolved-small {
    font-family: Inter, sans-serif;
    font-size: 0.6rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #b5a090;
    font-style: italic;
  }

  .msg-body-preview {
    font-family: Inter, sans-serif;
    font-size: 0.82rem;
    color: #6f4e37;
    margin: 0.15rem 0 0;
    line-height: 1.5;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ── Thread detail ── */

  .thread-detail {
    display: flex;
    flex-direction: column;
  }

  .thread-detail-header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid #d8c6b1;
    margin-bottom: 0.85rem;
    flex-wrap: wrap;
  }

  .thread-back {
    background: transparent;
    border: none;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    font-size: 0.75rem;
    letter-spacing: 0.04em;
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
    transition: color 0.15s;
  }
  .thread-back:hover { color: #c65f3c; }

  .thread-detail-subject {
    font-family: Georgia, serif;
    font-size: 0.95rem;
    color: #34251c;
    flex: 1;
    min-width: 0;
  }

  .thread-resolved-badge {
    font-family: Inter, sans-serif;
    font-size: 0.62rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #9a7c5c;
    border: 1px solid #d8c6b1;
    padding: 1px 6px;
    flex-shrink: 0;
  }

  /* ── Chat bubbles ── */

  .chat-messages {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    max-height: 360px;
    overflow-y: auto;
    padding: 0.25rem 0 0.75rem;
    scroll-behavior: smooth;
  }

  .chat-row {
    display: flex;
    flex-direction: column;
    max-width: 82%;
  }
  .chat-row--user {
    align-self: flex-end;
    align-items: flex-end;
  }
  .chat-row--admin {
    align-self: flex-start;
    align-items: flex-start;
  }

  .chat-bubble {
    padding: 0.55rem 0.8rem;
    line-height: 1.55;
  }
  .chat-bubble--user {
    background: #f4ead8;
    border: 1px solid #d8c6b1;
  }
  .chat-bubble--admin {
    background: #ece0ce;
    border: 1px solid #c8b89a;
  }

  .chat-sender {
    font-family: Inter, sans-serif;
    font-size: 0.6rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #9a7c5c;
    margin: 0 0 0.2rem;
  }

  .chat-body {
    font-family: Inter, sans-serif;
    font-size: 0.85rem;
    color: #34251c;
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .chat-time {
    font-family: Inter, sans-serif;
    font-size: 0.63rem;
    color: #b5a090;
    margin: 0.2rem 0 0;
  }

  /* ── Chat reply ── */

  .chat-reply {
    padding-top: 0.85rem;
    border-top: 1px solid #eee3d6;
    margin-top: 0.25rem;
  }

  .chat-reply-input {
    width: 100%;
    box-sizing: border-box;
    font-family: Inter, sans-serif;
    font-size: 0.85rem;
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #34251c;
    padding: 0.6rem 0.75rem;
    outline: none;
    resize: none;
    line-height: 1.55;
    min-height: 72px;
    transition: border-color 0.15s;
  }
  .chat-reply-input::placeholder { color: #b5a090; }
  .chat-reply-input:focus { border-color: #c65f3c; }

  .chat-reply-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 0.5rem;
  }

  .chat-reply-hint {
    font-family: Inter, sans-serif;
    font-size: 0.63rem;
    color: #c8b89a;
    letter-spacing: 0.03em;
  }

  .chat-reply-btn {
    background: #c65f3c;
    border: none;
    color: #fff;
    font-family: Inter, sans-serif;
    font-size: 0.73rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.4rem 1rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .chat-reply-btn:hover:not(:disabled) { background: #a84e30; }
  .chat-reply-btn:disabled { opacity: 0.45; cursor: default; }

  .chat-resolved-note {
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    color: #b5a090;
    font-style: italic;
    text-align: center;
    padding: 0.75rem 0 0;
    border-top: 1px solid #eee3d6;
    margin-top: 0.25rem;
  }
</style>
