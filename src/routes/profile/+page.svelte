<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { UserBookingDto, UserOrderDto, FigurineListItem } from '$lib/types/api';

  type Tab = 'bookings' | 'orders' | 'wishlist';
  let activeTab = $state<Tab>('bookings');

  let bookings = $state<UserBookingDto[]>([]);
  let orders = $state<UserOrderDto[]>([]);
  let wishlistIds = $state<string[]>([]);
  let wishlistItems = $state<Map<string, { name: string; status: string } | null>>(new Map());
  let loading = $state(true);
  let error = $state('');

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
      const [b, o] = await Promise.all([
        api.userProfileBookings(token),
        api.userProfileOrders(token),
      ]);
      bookings = b;
      orders = o;

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

    <!-- ── Account panel ── -->
    <div class="account">
      <div class="avatar-wrap">
        <button
          class="avatar-btn"
          onclick={() => avatarInput?.click()}
          title={$t('profileUploadPhoto')}
          disabled={uploadingAvatar}
        >
          {#if resolveAvatarUrl(authStore.user?.avatarUrl)}
            <img src={resolveAvatarUrl(authStore.user?.avatarUrl)} alt="" class="avatar-img" />
          {:else}
            <span class="avatar-initials">{(authStore.user?.displayName ?? '?')[0].toUpperCase()}</span>
          {/if}
          <span class="avatar-overlay">{uploadingAvatar ? $t('profileUploadingPhoto') : $t('profileUploadPhoto')}</span>
        </button>
        <input
          bind:this={avatarInput}
          type="file"
          accept="image/jpeg,image/png,image/webp"
          class="avatar-file-input"
          onchange={handleAvatarChange}
        />
      </div>

      <div class="account-info">
        {#if editingName}
          <div class="name-edit">
            <input
              class="name-input"
              bind:value={editNameValue}
              onkeydown={(e) => { if (e.key === 'Enter') saveName(); if (e.key === 'Escape') cancelEditName(); }}
              autofocus
            />
            <button class="name-save" onclick={saveName} disabled={savingName}>
              {savingName ? '…' : $t('profileSaveName')}
            </button>
            <button class="name-cancel" onclick={cancelEditName}>✕</button>
          </div>
        {:else}
          <div class="name-row">
            <span class="display-name">{authStore.user?.displayName ?? ''}</span>
            <button class="edit-btn" onclick={startEditName} title={$t('profileEditName')}>
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
                <path d="M8.5 1.5L10.5 3.5L4 10H2V8L8.5 1.5Z"/>
              </svg>
            </button>
            {#if nameSaved}
              <span class="name-saved">{$t('profileNameSaved')}</span>
            {/if}
          </div>
        {/if}
        <p class="account-email">{authStore.user?.email ?? ''}</p>
        {#if authStore.user?.createdAt}
          <p class="account-since">{$t('profileMemberSince')} {formatDate(authStore.user.createdAt)}</p>
        {/if}
      </div>

      <button class="logout" onclick={logout}>{$t('profileLogout')}</button>
    </div>

    <!-- ── Tabs ── -->
    <div class="tabs">
      <button class="tab" class:active={activeTab === 'bookings'} onclick={() => activeTab = 'bookings'}>
        {$t('profileBookings')}
        {#if bookings.length > 0}
          <span class="badge">{bookings.length}</span>
        {/if}
      </button>
      <button class="tab" class:active={activeTab === 'orders'} onclick={() => activeTab = 'orders'}>
        {$t('profileOrders')}
        {#if orders.length > 0}
          <span class="badge">{orders.length}</span>
        {/if}
      </button>
      <button class="tab" class:active={activeTab === 'wishlist'} onclick={() => activeTab = 'wishlist'}>
        {$t('profileWishlist')}
        {#if wishlistIds.length > 0}
          <span class="badge">{wishlistIds.length}</span>
        {/if}
      </button>
    </div>

    <!-- ── Tab content ── -->
    <div class="content">
      {#if loading}
        <p class="empty">…</p>
      {:else if error}
        <p class="error-msg">{error}</p>
      {:else if activeTab === 'bookings'}
        {#if bookings.length === 0}
          <p class="empty">{$t('profileEmpty')}</p>
        {:else}
          <ul class="list">
            {#each bookings as b}
              <li class="item">
                <div class="item-main">
                  <a href="/figurines/{b.figurineId}" class="item-name">{b.figurineName}</a>
                  <span class="status status--{b.status}">{bookingStatusLabel(b.status)}</span>
                </div>
                <p class="item-meta">{formatDateRange(b.startsAt, b.endsAt)}</p>
                <p class="item-date">{formatDate(b.createdAt)}</p>
              </li>
            {/each}
          </ul>
        {/if}
      {:else if activeTab === 'orders'}
        {#if orders.length === 0}
          <p class="empty">{$t('profileEmpty')}</p>
        {:else}
          <ul class="list">
            {#each orders as o}
              <li class="item">
                <div class="item-main">
                  <a href="/figurines/{o.figurineId}" class="item-name">{o.figurineName}</a>
                  <div class="order-badges">
                    <span class="mode">{orderModeLabel(o.mode)}</span>
                    <span class="order-status order-status--{o.status}">{orderStatusLabel(o.status)}</span>
                  </div>
                </div>
                <p class="item-date">{formatDate(o.createdAt)}</p>
              </li>
            {/each}
          </ul>
        {/if}
      {:else if activeTab === 'wishlist'}
        {#if wishlistIds.length === 0}
          <p class="empty">{$t('profileEmpty')}</p>
        {:else}
          <ul class="list">
            {#each wishlistIds as id}
              {@const item = wishlistItems.get(id)}
              <li class="item">
                <div class="item-main">
                  {#if item}
                    <a href="/figurines/{id}" class="item-name">{item.name}</a>
                    <span class="wishlist-status wishlist-status--{item.status}">{wishlistStatusLabel(item.status)}</span>
                  {:else if item === undefined}
                    <span class="item-name item-name--loading">…</span>
                  {:else}
                    <a href="/figurines/{id}" class="item-name item-name--missing">{id}</a>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </div>

    <!-- ── Delete account ── -->
    <div class="danger-zone">
      {#if !showDeleteConfirm}
        <button class="delete-btn" onclick={() => showDeleteConfirm = true}>
          {$t('profileDeleteAccount')}
        </button>
      {:else}
        <div class="delete-confirm">
          <p class="delete-warning">{$t('profileDeleteWarning')}</p>
          <div class="delete-actions">
            <button class="delete-confirm-btn" onclick={deleteAccount} disabled={deleting}>
              {deleting ? $t('profileDeleting') : $t('profileDeleteConfirm')}
            </button>
            <button class="delete-cancel-btn" onclick={() => showDeleteConfirm = false}>
              {$t('profileDeleteCancel')}
            </button>
          </div>
        </div>
      {/if}
    </div>

  </div>
</div>

<style>
  .page {
    min-height: 100vh;
    background: #f8f1e7;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding: 4rem 1rem 3rem;
  }

  .frame {
    width: 100%;
    max-width: 580px;
    background: #fdf8f2;
    border: 1px solid #d8c6b1;
    outline: 3px solid #f8f1e7;
    outline-offset: -6px;
    padding: 2rem;
    font-family: Georgia, serif;
    color: #34251c;
  }

  /* ── Account panel ── */

  .account {
    display: flex;
    align-items: flex-start;
    gap: 1.1rem;
    margin-bottom: 1.75rem;
    padding-bottom: 1.5rem;
    border-bottom: 1px solid #d8c6b1;
  }

  .avatar-wrap {
    flex-shrink: 0;
    position: relative;
  }

  .avatar-btn {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    border: 1.5px solid #d8c6b1;
    background: #efe6d6;
    cursor: pointer;
    overflow: hidden;
    position: relative;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: border-color 0.2s;
  }
  .avatar-btn:hover { border-color: #c65f3c; }
  .avatar-btn:disabled { cursor: default; opacity: 0.7; }

  .avatar-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .avatar-initials {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.3rem;
    color: #9a7c5c;
    line-height: 1;
    pointer-events: none;
  }

  .avatar-overlay {
    position: absolute;
    inset: 0;
    background: rgba(52, 37, 28, 0.55);
    color: #f8f1e7;
    font-family: Inter, sans-serif;
    font-size: 0.58rem;
    letter-spacing: 0.05em;
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
  .avatar-btn:hover .avatar-overlay { opacity: 1; }
  .avatar-btn:disabled .avatar-overlay { opacity: 1; }

  .avatar-file-input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }

  .account-info {
    flex: 1;
    min-width: 0;
  }

  .name-row {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    margin-bottom: 0.2rem;
  }

  .display-name {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.1rem;
    color: #34251c;
    line-height: 1.3;
  }

  .edit-btn {
    background: transparent;
    border: none;
    color: #b5a090;
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
    transition: color 0.15s;
    flex-shrink: 0;
  }
  .edit-btn:hover { color: #c65f3c; }

  .name-saved {
    font-family: Inter, sans-serif;
    font-size: 0.7rem;
    color: #6a9e5a;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .name-edit {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.2rem;
  }

  .name-input {
    font-family: Georgia, serif;
    font-size: 1rem;
    background: transparent;
    border: none;
    border-bottom: 1.5px solid #c65f3c;
    color: #34251c;
    padding: 2px 0;
    outline: none;
    min-width: 0;
    flex: 1;
  }

  .name-save {
    background: transparent;
    border: 1px solid #c65f3c;
    color: #c65f3c;
    font-family: Inter, sans-serif;
    font-size: 0.7rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 2px 8px;
    cursor: pointer;
    transition: background 0.15s;
    flex-shrink: 0;
  }
  .name-save:hover:not(:disabled) { background: rgba(198,95,60,0.08); }
  .name-save:disabled { opacity: 0.5; }

  .name-cancel {
    background: transparent;
    border: none;
    color: #b5a090;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 2px 4px;
    line-height: 1;
    flex-shrink: 0;
    transition: color 0.15s;
  }
  .name-cancel:hover { color: #34251c; }

  .account-email {
    font-family: Inter, sans-serif;
    font-size: 0.8rem;
    color: #9a7c5c;
    margin: 0 0 0.15rem;
  }

  .account-since {
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    color: #b5a090;
    margin: 0;
    letter-spacing: 0.02em;
  }

  .logout {
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #9a7c5c;
    padding: 0.35rem 0.75rem;
    font-size: 0.73rem;
    font-family: Inter, sans-serif;
    cursor: pointer;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    transition: all 0.2s;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .logout:hover { border-color: #c65f3c; color: #c65f3c; }

  /* ── Tabs ── */

  .tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid #d8c6b1;
    margin-bottom: 1.5rem;
  }

  .tab {
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 0.6rem 1rem;
    font-family: Inter, sans-serif;
    font-size: 0.78rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: #9a7c5c;
    cursor: pointer;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: -1px;
  }
  .tab:hover { color: #6f3b24; }
  .tab.active { color: #34251c; border-bottom-color: #c65f3c; }

  .badge {
    background: #d8c6b1;
    color: #6f3b24;
    font-size: 0.62rem;
    border-radius: 10px;
    padding: 0 5px;
    min-width: 16px;
    text-align: center;
  }

  /* ── Content ── */

  .content { min-height: 180px; }

  .empty {
    color: #9a7c5c;
    font-style: italic;
    font-size: 0.9rem;
    padding: 2rem 0;
    text-align: center;
  }

  .error-msg {
    color: #c65f3c;
    font-size: 0.85rem;
    font-family: Inter, sans-serif;
    padding: 1rem 0;
  }

  .list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
  }

  .item {
    padding: 0.85rem 0;
    border-bottom: 1px solid #eee3d6;
  }
  .item:last-child { border-bottom: none; }

  .item-main {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.5rem;
    margin-bottom: 0.2rem;
  }

  .item-name {
    font-family: Georgia, serif;
    font-size: 0.95rem;
    color: #34251c;
    text-decoration: none;
  }
  .item-name:hover { color: #c65f3c; }
  .item-name--loading { color: #b5a090; font-style: italic; }
  .item-name--missing { color: #b5a090; font-size: 0.78rem; font-family: 'Courier New', monospace; }

  .status {
    font-size: 0.68rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-family: Inter, sans-serif;
    padding: 2px 7px;
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
    gap: 0.35rem;
    flex-shrink: 0;
  }

  .mode {
    font-size: 0.68rem;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .order-status {
    font-size: 0.63rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    font-family: Inter, sans-serif;
    padding: 2px 6px;
    border-radius: 2px;
    white-space: nowrap;
  }
  .order-status--new     { background: #f4ead8; color: #6f3b24; }
  .order-status--seen    { background: #e8eef4; color: #2d4a6a; }
  .order-status--replied { background: #e8f4e8; color: #2d6a3f; }

  .wishlist-status {
    font-size: 0.68rem;
    letter-spacing: 0.05em;
    font-family: Inter, sans-serif;
    color: #9a7c5c;
    font-style: italic;
    flex-shrink: 0;
  }
  .wishlist-status--sold     { color: #b5a090; }
  .wishlist-status--available { color: #4a7a3a; }

  .item-meta {
    font-size: 0.78rem;
    color: #9a7c5c;
    margin: 0;
    font-family: Inter, sans-serif;
    font-style: italic;
  }

  .item-date {
    font-size: 0.72rem;
    color: #b5a090;
    margin: 0.1rem 0 0;
    font-family: Inter, sans-serif;
  }

  /* ── Danger zone ── */

  .danger-zone {
    margin-top: 2rem;
    padding-top: 1.25rem;
    border-top: 1px solid #eee3d6;
  }

  .delete-btn {
    background: transparent;
    border: none;
    color: #b5a090;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    cursor: pointer;
    padding: 0;
    transition: color 0.2s;
  }
  .delete-btn:hover { color: #c65f3c; }

  .delete-confirm {
    background: #fdf3f3;
    border: 1px solid #f0d0c8;
    padding: 1rem 1.1rem;
  }

  .delete-warning {
    font-family: Inter, sans-serif;
    font-size: 0.8rem;
    color: #7a3020;
    margin: 0 0 0.85rem;
    line-height: 1.5;
  }

  .delete-actions {
    display: flex;
    gap: 0.6rem;
    align-items: center;
  }

  .delete-confirm-btn {
    background: #9b2020;
    border: none;
    color: #fff;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.4rem 0.9rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .delete-confirm-btn:hover:not(:disabled) { background: #7a1818; }
  .delete-confirm-btn:disabled { opacity: 0.6; cursor: not-allowed; }

  .delete-cancel-btn {
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.4rem 0.9rem;
    cursor: pointer;
    transition: all 0.15s;
  }
  .delete-cancel-btn:hover { border-color: #9a7c5c; color: #34251c; }
</style>
