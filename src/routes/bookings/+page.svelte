<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';
  import { goto } from '$app/navigation';
  import { t , brandName } from '$lib/i18n';
  import { allClaims } from '$lib/stores/all-claims.svelte';
  import { authStore } from '$lib/stores/auth.svelte';

  function fmtDate(iso: string) {
    return new Date(iso).toLocaleDateString(undefined, {
      day: '2-digit', month: 'long', year: 'numeric',
    });
  }

  function statusLabel(s?: string) {
    switch (s) {
      case 'confirmed': return $t('bookingsConfirmed');
      case 'rejected':  return $t('bookingsRejected');
      case 'cancelled': return $t('bookingsCancelDone');
      case 'completed': return $t('bookingsCompleted');
      default:          return $t('bookingsPending');
    }
  }

  onMount(() => {
    if (authStore.isLoggedIn) {
      goto('/profile');
      return;
    }
    allClaims.reload();
    allClaims.verify();
    allClaims.startPolling();
  });

  onDestroy(() => allClaims.stopPolling());
</script>

<svelte:head>
  <title>{$t('bookingsPageTitle')} — {$brandName}</title>
</svelte:head>

<div class="bookings-page">
  <div class="page-top">
    <a href="/" class="back-link">{$t('bookingsBack')}</a>
    {#if allClaims.claims.length > 0}
      <button class="print-btn" onclick={() => window.print()}>
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.2" aria-hidden="true">
          <path d="M3 1h6v3H3zM1 4h10v5H1zM3 9v2h6V9"/>
          <rect x="3" y="5.5" width="1.2" height="1.2" fill="currentColor" stroke="none"/>
        </svg>
        {$t('bookingsPrint')}
      </button>
    {/if}
  </div>

  <h1 class="page-title">{$t('bookingsPageTitle')}</h1>

  {#if allClaims.claims.length === 0}
    <div class="empty-state" transition:fade={{ duration: 200 }}>
      <div class="empty-seal">✦</div>
      <p class="empty-text">{$t('bookingsEmpty')}</p>
      <p class="empty-hint">{$t('bookingsEmptyHint')}</p>
      <a href="/figurines" class="browse-link">{$t('archiveBackLink')}</a>
    </div>
  {:else}
    <ul class="claims-list" transition:fade={{ duration: 200 }}>
      {#each allClaims.claims as c (c.token)}
        <li class="claim-card" class:claim-card--confirmed={c.status === 'confirmed'}>
          <div class="claim-card-inner">

            <div class="claim-top">
              <a href="/figurines/{c.figurineId}" class="claim-name">{c.figurineName}</a>
              <span class="claim-status claim-status--{c.status ?? 'pending'}">
                {statusLabel(c.status)}
              </span>
            </div>

            <p class="claim-period">
              {fmtDate(c.startsAt)} — {fmtDate(c.endsAt)}
            </p>

            <div class="claim-meta">
              <span class="claim-token-label">Code:</span>
              <span class="claim-token">{c.token}</span>
            </div>

            <div class="claim-actions">
              <a href="/figurines/{c.figurineId}" class="action-secondary">
                {$t('bookingsViewFigurine')} →
              </a>
              {#if !c.status || c.status === 'pending'}
                <button
                  class="action-cancel"
                  onclick={() => allClaims.cancel(c)}
                  disabled={allClaims.cancellingToken === c.token}
                >
                  {allClaims.cancellingToken === c.token ? $t('claimCancelling') : $t('claimCancelBtn')}
                </button>
              {/if}
              <a href="/cancel/{c.token}" target="_blank" rel="noopener" class="action-secondary">
                {$t('bookingClaimPageLink')} →
              </a>
            </div>

            {#if allClaims.errors[c.token]}
              <p class="claim-err">{allClaims.errors[c.token]}</p>
            {/if}

          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .bookings-page {
    max-width: 640px;
    margin: 0 auto;
    padding: 3rem 1.5rem 6rem;
    font-family: Georgia, serif;
    color: #34251c;
  }

  .page-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 2rem;
  }

  .back-link {
    display: inline-block;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 9px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.55);
    text-decoration: none;
    transition: color 0.2s;
  }
  .back-link:hover { color: #c65f3c; }

  .print-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    background: none;
    border: 1px solid #d8c6b1;
    color: rgba(95,70,54,0.55);
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 8px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    padding: 5px 12px;
    cursor: pointer;
    transition: color 0.2s, border-color 0.2s;
  }
  .print-btn:hover { color: #34251c; border-color: #b0a090; }

  .page-title {
    font-family: 'Fraunces', Georgia, serif;
    font-size: clamp(1.6rem, 4vw, 2.2rem);
    font-weight: 400;
    color: #34251c;
    margin: 0 0 2.5rem;
    letter-spacing: 0.02em;
  }

  /* ── Empty state ── */
  .empty-state {
    text-align: center;
    padding: 4rem 0;
  }

  .empty-seal {
    font-size: 2rem;
    color: rgba(198,95,60,0.3);
    margin-bottom: 1.2rem;
  }

  .empty-text {
    font-size: 1.1rem;
    color: rgba(52,37,28,0.5);
    font-style: italic;
    margin: 0 0 0.5rem;
  }

  .empty-hint {
    font-size: 0.82rem;
    color: rgba(95,70,54,0.5);
    margin: 0 0 2rem;
    line-height: 1.6;
  }

  .browse-link {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 9px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #c65f3c;
    text-decoration: none;
    border-bottom: 1px solid rgba(198,95,60,0.4);
    padding-bottom: 1px;
    transition: border-color 0.2s;
  }
  .browse-link:hover { border-color: #c65f3c; }

  /* ── Claim cards ── */
  .claims-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .claim-card {
    background: #f2e8d9;
    border: 1px solid #d8c6b1;
    position: relative;
  }

  .claim-card--confirmed {
    border-color: #b8d0a0;
    background: #f0f5ea;
  }

  .claim-card-inner {
    padding: 1.25rem 1.5rem;
  }

  .claim-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 0.4rem;
  }

  .claim-name {
    font-size: 1rem;
    font-weight: 500;
    color: #34251c;
    text-decoration: none;
    line-height: 1.3;
    transition: color 0.2s;
  }
  .claim-name:hover { color: #c65f3c; }

  .claim-status {
    flex-shrink: 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 7.5px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 3px 8px;
    border-radius: 2px;
    margin-top: 2px;
  }
  .claim-status--pending   { background: #f5e6c8; color: #7a5520; }
  .claim-status--confirmed { background: #d4e8c8; color: #3a6020; }
  .claim-status--cancelled { background: #e8e0d4; color: #6a5040; }
  .claim-status--rejected  { background: #f0d0c8; color: #8a3020; }
  .claim-status--completed { background: #dce8e0; color: #2a5040; }

  .claim-period {
    margin: 0 0 0.5rem;
    font-size: 0.88rem;
    color: rgba(95,70,54,0.7);
    font-style: italic;
  }

  .claim-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 1rem;
  }

  .claim-token-label {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 8px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.45);
  }

  .claim-token {
    font-family: 'Courier New', monospace;
    font-size: 0.78rem;
    color: rgba(95,70,54,0.6);
    letter-spacing: 0.08em;
    background: rgba(52,37,28,0.05);
    padding: 1px 6px;
    border-radius: 2px;
  }

  .claim-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
  }

  .action-secondary {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 8.5px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.5);
    text-decoration: none;
    transition: color 0.2s;
  }
  .action-secondary:hover { color: #c65f3c; }

  .action-cancel {
    background: none;
    border: 1px solid rgba(198,95,60,0.45);
    color: #c65f3c;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 8.5px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 4px 12px;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }
  .action-cancel:hover:not(:disabled) { background: rgba(198,95,60,0.07); border-color: #c65f3c; }
  .action-cancel:disabled { opacity: 0.5; cursor: not-allowed; }

  .claim-err {
    margin: 8px 0 0;
    font-size: 0.78rem;
    color: #a03020;
    font-style: italic;
  }

  @media (max-width: 520px) {
    .claim-card-inner { padding: 1rem 1.1rem; }
    .claim-top { flex-direction: column; gap: 6px; }
  }

  @media print {
    .bookings-page { padding: 1cm; max-width: none; }
    .page-top { display: none; }
    .page-title { font-size: 18pt; margin-bottom: 1cm; }
    .claim-card { break-inside: avoid; border: 1px solid #aaa; background: white; }
    .claim-card-inner { padding: 0.5cm; }
    .claim-name { color: black !important; text-decoration: none; }
    .claim-name::after { content: ' (/figurines/' attr(href) ')'; font-size: 8pt; color: #555; }
    .action-cancel, .action-secondary { display: none; }
    .claim-status { border: 1px solid #aaa !important; background: white !important; color: black !important; }
    .claim-token { background: #eee; }
    .claims-list { gap: 0.5cm; }
  }
</style>
