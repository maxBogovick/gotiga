<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { UserBookingDto, UserOrderDto } from '$lib/types/api';

  type Tab = 'bookings' | 'orders' | 'wishlist';
  let activeTab = $state<Tab>('bookings');

  let bookings = $state<UserBookingDto[]>([]);
  let orders = $state<UserOrderDto[]>([]);
  let wishlistIds = $state<string[]>([]);
  let loading = $state(true);
  let error = $state('');

  onMount(async () => {
    if (!authStore.isLoggedIn && !authStore.token) {
      goto('/login?from=/profile');
      return;
    }

    // Try to restore session if token exists but user not loaded
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

      // wishlist from localStorage
      if (typeof localStorage !== 'undefined') {
        wishlistIds = JSON.parse(localStorage.getItem('gotiga_wishlist') ?? '[]');
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

  async function logout() {
    const token = authStore.token;
    if (token) {
      try { await api.userLogout(token); } catch { /* ok */ }
    }
    authStore.clearSession();
    goto('/');
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

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString('ru-RU', { day: 'numeric', month: 'long', year: 'numeric' });
  }
</script>

<svelte:head>
  <title>{$t('profileTitle')} — Gotiga</title>
</svelte:head>

<div class="page">
  <div class="frame">

    <div class="header">
      <div>
        <p class="greeting">{authStore.user?.displayName ?? ''}</p>
        <h1 class="title">{$t('profileTitle')}</h1>
      </div>
      <button class="logout" onclick={logout}>{$t('profileLogout')}</button>
    </div>

    <!-- Tabs -->
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

    <div class="content">
      {#if loading}
        <p class="empty">…</p>
      {:else if error}
        <p class="error">{error}</p>
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
                <p class="item-meta">{b.startsAt} — {b.endsAt}</p>
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
                  <span class="mode">{orderModeLabel(o.mode)}</span>
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
              <li class="item">
                <a href="/figurines/{id}" class="item-name">{id}</a>
              </li>
            {/each}
          </ul>
        {/if}
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
    max-width: 560px;
    background: #fdf8f2;
    border: 1px solid #d8c6b1;
    outline: 3px solid #f8f1e7;
    outline-offset: -6px;
    padding: 2rem;
    font-family: Georgia, serif;
    color: #34251c;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1.5rem;
    border-bottom: 1px solid #d8c6b1;
    padding-bottom: 1.25rem;
  }

  .greeting {
    font-size: 0.75rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    margin: 0 0 0.2rem;
  }

  .title {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.5rem;
    font-weight: 400;
    margin: 0;
    letter-spacing: 0.02em;
  }

  .logout {
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #9a7c5c;
    padding: 0.4rem 0.8rem;
    font-size: 0.78rem;
    font-family: Inter, sans-serif;
    cursor: pointer;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    transition: all 0.2s;
  }
  .logout:hover { border-color: #c65f3c; color: #c65f3c; }

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
    font-size: 0.8rem;
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
    font-size: 0.65rem;
    border-radius: 10px;
    padding: 0 5px;
    min-width: 16px;
    text-align: center;
  }

  .content { min-height: 200px; }

  .empty {
    color: #9a7c5c;
    font-style: italic;
    font-size: 0.9rem;
    padding: 2rem 0;
    text-align: center;
  }

  .error {
    color: #c65f3c;
    font-size: 0.85rem;
    font-family: Inter, sans-serif;
  }

  .list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .item {
    padding: 0.9rem 0;
    border-bottom: 1px solid #eee3d6;
  }
  .item:last-child { border-bottom: none; }

  .item-main {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.25rem;
  }

  .item-name {
    font-family: Georgia, serif;
    font-size: 0.95rem;
    color: #34251c;
    text-decoration: none;
  }
  .item-name:hover { color: #c65f3c; }

  .status {
    font-size: 0.7rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-family: Inter, sans-serif;
    padding: 2px 8px;
    border-radius: 2px;
  }
  .status--pending   { background: #f4ead8; color: #6f3b24; }
  .status--confirmed { background: #e8f4e8; color: #2d6a3f; }
  .status--rejected  { background: #fde8e8; color: #9b2020; }
  .status--cancelled { background: #ebebeb; color: #666; }

  .mode {
    font-size: 0.7rem;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .item-meta {
    font-size: 0.8rem;
    color: #9a7c5c;
    margin: 0;
    font-family: Inter, sans-serif;
  }

  .item-date {
    font-size: 0.75rem;
    color: #b5a090;
    margin: 0.15rem 0 0;
    font-family: Inter, sans-serif;
  }
</style>
