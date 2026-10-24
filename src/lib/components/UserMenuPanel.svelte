<script lang="ts">
  // Меню вошедшего: разделы профиля с числами и выход. Числа берутся из того же
  // стола посетителя, что и рейка профиля: два счётчика одного однажды
  // разошлись бы.
  import { fade } from 'svelte/transition';
  import { authStore } from '$lib/stores/auth.svelte';
  import { userDesk } from '$lib/stores/user-desk.svelte';
  import { resolveMediaUrl } from '$lib/api';
  import { t } from '$lib/i18n';

  let { onclose, onlogout }: { onclose: () => void; onlogout: () => void } = $props();

  let avatarUrl = $derived(resolveMediaUrl(authStore.user?.avatarUrl));

  let rooms = $derived([
    { href: '/profile/dealings', label: $t('profileHubDealings'), count: userDesk.counts.deals, unread: 0 },
    { href: '/profile/messages', label: $t('profileMessages'), count: userDesk.counts.threads, unread: userDesk.counts.unread },
    { href: '/profile/wishlist', label: $t('profileWishlist'), count: userDesk.counts.wishlist, unread: 0 },
    { href: '/profile/watches', label: $t('profileWatches'), count: userDesk.counts.watches, unread: 0 },
  ]);
</script>

<div id="user-panel" class="user-panel" transition:fade={{ duration: 150 }}>
  <a href="/profile" class="user-panel-head" onclick={onclose}>
    {#if avatarUrl}
      <img src={avatarUrl} alt="" class="user-panel-avatar" />
    {:else}
      <span class="user-panel-initial">{(authStore.user?.displayName ?? '?')[0].toUpperCase()}</span>
    {/if}
    <span class="user-panel-info">
      <span class="user-panel-name">{authStore.user?.displayName}</span>
      <span class="user-panel-email">{authStore.handle}</span>
    </span>
  </a>

  <ul class="user-rooms">
    {#each rooms as room (room.href)}
      <li>
        <a href={room.href} class="user-room" onclick={onclose}>
          <span class="user-room-name">{room.label}</span>
          {#if room.unread > 0}
            <span class="user-room-new">{room.unread}</span>
          {:else if room.count > 0}
            <span class="user-room-count">{room.count}</span>
          {/if}
        </a>
      </li>
    {/each}
  </ul>

  <a href="/profile/account" class="user-panel-link" onclick={onclose}>
    {$t('profileTabAccount')}
  </a>
  <button class="user-panel-logout" onclick={onlogout}>
    {$t('profileLogout')}
  </button>
</div>

<style>
  /* ── Меню вошедшего ──────────────────────────────────────────────────
     Краски — только переменными. Хекс в <style> мимо темы не проходит
     (`generateHexBridgeCSS` переписывает лишь Tailwind-классы), поэтому
     прежняя панель на #f2e8d9 / #c65f3c не менялась, когда автор менял
     палитру в админке. Измерено на её бумаге: прежние «ПРОФИЛЬ →» давали
     3.51:1, «ВЫЙТИ» — 2.33:1, почта — 2.57:1 при пороге AA 4.5:1. Теперь
     ink-secondary 8.9:1, ink-tertiary 5.7:1 и ember-deep 5.5:1. */
  .user-panel {
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    width: 244px;
    background: var(--color-canvas-sunken);
    border: 1px solid var(--color-border-default);
    box-shadow: var(--shadow-md);
    z-index: 300;
    font-family: var(--font-serif);
    color: var(--color-ink-primary);
  }

  .user-panel-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px 10px;
    border-bottom: 1px solid var(--color-border-subtle);
    text-decoration: none;
    transition: background 0.15s;
  }
  .user-panel-head:hover { background: var(--color-canvas-raised); }

  .user-panel-avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
    border: 1px solid var(--color-border-subtle);
  }

  .user-panel-initial {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--color-canvas-raised);
    border: 1px solid var(--color-border-subtle);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    font-size: 1rem;
    color: var(--color-ink-tertiary);
  }

  .user-panel-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .user-panel-name {
    font-family: var(--font-display);
    font-size: 0.88rem;
    color: var(--color-ink-primary);
    line-height: 1.2;
  }

  .user-panel-email {
    font-family: var(--font-body);
    font-size: 0.72rem;
    color: var(--color-ink-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .user-rooms {
    list-style: none;
    margin: 0;
    padding: 4px 0;
    border-bottom: 1px solid var(--color-border-subtle);
  }

  .user-room {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    padding: 7px 14px;
    font-family: var(--font-body);
    font-size: 0.82rem;
    color: var(--color-ink-secondary);
    text-decoration: none;
    transition: background 0.15s, color 0.15s;
  }
  .user-room:hover {
    background: var(--color-canvas-raised);
    color: var(--color-ember-deep);
  }

  .user-room-name { white-space: nowrap; }

  .user-room-count {
    font-size: 0.74rem;
    font-variant-numeric: tabular-nums;
    color: var(--color-ink-tertiary);
  }

  .user-room-new {
    font-family: var(--font-body);
    font-size: 0.7rem;
    font-weight: 500;
    line-height: 1;
    background: var(--color-ember);
    color: var(--color-canvas-raised);
    border-radius: 999px;
    padding: 3px 7px;
  }

  .user-panel-link {
    display: block;
    padding: 9px 14px;
    font-family: var(--font-body);
    font-size: 0.82rem;
    color: var(--color-ink-secondary);
    text-decoration: none;
    border-bottom: 1px solid var(--color-border-subtle);
    transition: background 0.15s, color 0.15s;
  }
  .user-panel-link:hover {
    background: var(--color-canvas-raised);
    color: var(--color-ember-deep);
  }

  .user-panel-logout {
    display: block;
    width: 100%;
    padding: 9px 14px;
    background: none;
    border: none;
    text-align: left;
    font-family: var(--font-body);
    font-size: 0.82rem;
    color: var(--color-ink-tertiary);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .user-panel-logout:hover {
    background: var(--color-canvas-raised);
    color: var(--color-ember-deep);
  }
</style>
