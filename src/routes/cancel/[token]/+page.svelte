<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/state';
  import { fade } from 'svelte/transition';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import type { BookingCancelInfo } from '$lib/types/api';

  let token = $derived(page.params.token ?? '');

  type Phase = 'loading' | 'found' | 'not_found' | 'done' | 'error';

  let phase = $state<Phase>('loading');
  let info  = $state<BookingCancelInfo | null>(null);
  let cancelling = $state(false);

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, {
      day: '2-digit', month: 'long', year: 'numeric',
    });
  }

  function statusLabel(s: string): string {
    switch (s) {
      case 'pending':   return $t('cancelStatusPending');
      case 'confirmed': return $t('cancelStatusConfirmed');
      case 'rejected':  return $t('cancelStatusRejected');
      case 'cancelled': return $t('cancelStatusCancelled');
      default:          return s;
    }
  }

  let pollTimer: ReturnType<typeof setInterval> | null = null;

  async function fetchInfo() {
    try {
      const fresh = await api.getBookingByToken(token);
      info = fresh;
      if (phase === 'loading') phase = 'found';
      // Stop polling once the reservation moves out of pending
      if (fresh.status !== 'pending' && pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
    } catch {
      if (phase === 'loading') phase = 'not_found';
    }
  }

  onMount(async () => {
    await fetchInfo();
    if (info?.status === 'pending') {
      pollTimer = setInterval(fetchInfo, 30_000);
    }
  });

  onDestroy(() => {
    if (pollTimer) clearInterval(pollTimer);
  });

  async function handleCancel() {
    if (!info || cancelling) return;
    cancelling = true;
    try {
      await api.cancelBookingByToken(token);
      info  = { ...info, status: 'cancelled' };
      phase = 'done';
    } catch {
      phase = 'error';
    } finally {
      cancelling = false;
    }
  }
</script>

<svelte:head>
  <title>{$t('cancelPageTitle')} — Gotiga</title>
</svelte:head>

<div class="cancel-wrap">
  <div class="cancel-frame">

    {#if phase === 'loading'}
      <p class="status-text" transition:fade={{ duration: 200 }}>{$t('cancelLoading')}</p>

    {:else if phase === 'not_found'}
      <div class="seal-icon" transition:fade={{ duration: 300 }}>✦</div>
      <h1 class="cancel-heading">{$t('cancelNotFound')}</h1>
      <p class="hint">{$t('cancelNotFoundHint')}</p>
      <a href="/" class="action-link">{$t('cancelBackHome')}</a>

    {:else if phase === 'done'}
      <div class="seal-icon done" transition:fade={{ duration: 300 }}>✓</div>
      <h1 class="cancel-heading">{$t('cancelDone')}</h1>
      <a href="/" class="action-link">{$t('cancelBackHome')}</a>

    {:else if phase === 'error'}
      <div class="seal-icon err" transition:fade={{ duration: 300 }}>✕</div>
      <h1 class="cancel-heading">{$t('cancelError')}</h1>
      <button class="cancel-btn" onclick={() => { phase = 'found'; }}>{$t('cancelBtn')}</button>

    {:else if phase === 'found' && info}
      <div transition:fade={{ duration: 300 }} class="found-block">
        <div class="wax-seal">✦</div>
        <h1 class="cancel-heading">{$t('cancelPageTitle')}</h1>

        <dl class="info-list">
          <dt>{$t('cancelArtifact')}</dt>
          <dd>
            <a href="/figurines/{info.figurineId}" class="figurine-link">{info.figurineName}</a>
          </dd>

          <dt>{$t('cancelPeriod')}</dt>
          <dd>{formatDate(info.startsAt)} — {formatDate(info.endsAt)}</dd>

          <dt>{$t('cancelStatusLabel')}</dt>
          <dd class="status-badge status-{info.status}">{statusLabel(info.status)}</dd>
        </dl>

        <div class="actions">
          {#if info.status === 'cancelled'}
            <p class="hint">{$t('cancelAlreadyCancelled')}</p>
          {:else if info.status === 'confirmed'}
            <p class="hint">{$t('cancelAlreadyConfirmed')}</p>
          {:else if info.status === 'pending'}
            <button class="cancel-btn" onclick={handleCancel} disabled={cancelling}>
              {cancelling ? $t('cancelCancelling') : $t('cancelBtn')}
            </button>
          {/if}
          <a href="/figurines/{info.figurineId}" class="action-link">{$t('cancelGoToFigurine')}</a>
        </div>
      </div>
    {/if}

  </div>
</div>

<style>
  .cancel-wrap {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem 1rem;
    background: #f8f1e7;
  }

  .cancel-frame {
    width: 100%;
    max-width: 480px;
    background: #f2e8d9;
    border: 1px solid #d8c6b1;
    outline: 3px solid #d8c6b1;
    outline-offset: 4px;
    padding: 3rem 2.5rem;
    transform: rotate(0.4deg);
    font-family: Georgia, serif;
    color: #34251c;
    text-align: center;
  }

  .wax-seal, .seal-icon {
    font-size: 2.4rem;
    color: #c65f3c;
    margin-bottom: 1rem;
    display: block;
  }

  .seal-icon.done { color: #5a7a4a; }
  .seal-icon.err  { color: #a03020; }

  .cancel-heading {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.5rem;
    font-weight: 600;
    color: #34251c;
    margin: 0 0 1.5rem;
    line-height: 1.3;
  }

  .info-list {
    margin: 0 0 2rem;
    padding: 0;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.5rem 1rem;
    text-align: left;
  }

  dt {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #8b6a52;
    padding-top: 0.15rem;
  }

  dd {
    margin: 0;
    font-size: 0.95rem;
    color: #34251c;
  }

  .figurine-link {
    color: #c65f3c;
    text-decoration: none;
  }
  .figurine-link:hover { text-decoration: underline; }

  .status-badge {
    display: inline-block;
    padding: 0.15rem 0.6rem;
    border-radius: 2px;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .status-pending   { background: #f5e6c8; color: #7a5520; }
  .status-confirmed { background: #d4e8c8; color: #3a6020; }
  .status-rejected  { background: #f0d0c8; color: #8a3020; }
  .status-cancelled { background: #e8e0d4; color: #6a5040; }

  .actions {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .cancel-btn {
    background: #c65f3c;
    color: #f8f1e7;
    border: none;
    padding: 0.7rem 2rem;
    font-family: Georgia, serif;
    font-size: 0.9rem;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: background 0.2s;
  }
  .cancel-btn:hover:not(:disabled) { background: #a04830; }
  .cancel-btn:disabled { opacity: 0.6; cursor: not-allowed; }

  .action-link {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: #8b6a52;
    text-decoration: none;
    border-bottom: 1px solid #d8c6b1;
    padding-bottom: 1px;
    transition: color 0.2s;
  }
  .action-link:hover { color: #c65f3c; }

  .hint {
    font-size: 0.85rem;
    color: #7a6050;
    line-height: 1.5;
    margin: 0;
    font-style: italic;
  }

  .status-text {
    color: #8b6a52;
    font-style: italic;
    font-size: 0.9rem;
  }

  @media (max-width: 520px) {
    .cancel-frame {
      padding: 2rem 1.25rem;
      transform: none;
    }
  }
</style>
