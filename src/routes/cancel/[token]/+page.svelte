<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/state';
  import { fade } from 'svelte/transition';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import DateRangePicker from '$lib/components/DateRangePicker.svelte';
  import type { BookingCancelInfo, FigurineSchedule, BookingRules } from '$lib/types/api';

  let token = $derived(page.params.token ?? '');

  type Phase = 'loading' | 'found' | 'not_found' | 'done' | 'error';

  let phase = $state<Phase>('loading');
  let info  = $state<BookingCancelInfo | null>(null);
  let cancelling = $state(false);

  // Reschedule state
  let showReschedule = $state(false);
  let rescheduleStartsAt = $state('');
  let rescheduleEndsAt   = $state('');
  let rescheduling       = $state(false);
  let rescheduleError    = $state('');
  let rescheduleSuccess  = $state(false);
  let dateError          = $state('');
  let figurineSchedule   = $state<FigurineSchedule>({ entries: [] });
  let bookingRules       = $state<BookingRules | null>(null);

  let todayStr = new Date().toISOString().split('T')[0];
  let rescheduleMinDate = $derived.by(() => {
    if (!bookingRules || bookingRules.advanceDays <= 0) return todayStr;
    const d = new Date();
    d.setDate(d.getDate() + bookingRules.advanceDays);
    return d.toISOString().split('T')[0];
  });

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
      case 'completed': return $t('cancelStatusCompleted');
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

  async function handleReschedule() {
    if (!rescheduleStartsAt || !rescheduleEndsAt || dateError) return;
    rescheduling = true;
    rescheduleError = '';
    try {
      const updated = await api.rescheduleBookingByToken(token, {
        startsAt: rescheduleStartsAt,
        endsAt: rescheduleEndsAt,
      });
      info = updated;
      rescheduleSuccess = true;
      showReschedule = false;
    } catch (err) {
      const msg = err instanceof Error ? err.message : '';
      if (msg.includes('409') || msg.toLowerCase().includes('conflict')) {
        rescheduleError = $t('rescheduleConflict');
      } else if (msg.includes('400')) {
        const match = msg.match(/API \d+: (.+)$/s);
        try { rescheduleError = match ? (JSON.parse(match[1]).error ?? $t('rescheduleError')) : $t('rescheduleError'); }
        catch { rescheduleError = $t('rescheduleError'); }
      } else {
        rescheduleError = $t('rescheduleError');
      }
    } finally {
      rescheduling = false;
    }
  }

  onMount(async () => {
    await fetchInfo();
    if (info?.status === 'pending') {
      pollTimer = setInterval(fetchInfo, 30_000);
    }
    bookingRules = await api.getBookingRules().catch(() => null);
    if (info?.figurineId) {
      figurineSchedule = await api.getFigurineSchedule(info.figurineId).catch(() => ({ entries: [] }));
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
          {:else if info.status === 'completed'}
            <p class="hint">{$t('cancelAlreadyCompleted')}</p>
          {:else if info.status === 'confirmed'}
            <p class="hint">{$t('cancelAlreadyConfirmed')}</p>
          {:else if info.status === 'rejected'}
            {#if info.adminNotes}
              <div class="admin-notes">
                <span class="admin-notes-label">{$t('cancelAdminNotes')}</span>
                <p class="admin-notes-text">{info.adminNotes}</p>
              </div>
            {/if}
            <a href="/figurines/{info.figurineId}" class="action-link try-again">{$t('cancelTryAgain')}</a>
          {:else if info.status === 'pending'}
            {#if !showReschedule}
              <button class="cancel-btn" onclick={() => { showReschedule = true; rescheduleSuccess = false; rescheduleError = ''; }}>{$t('rescheduleBtn')}</button>
              <button class="cancel-btn cancel-btn--destructive" onclick={handleCancel} disabled={cancelling}>
                {cancelling ? $t('cancelCancelling') : $t('cancelBtn')}
              </button>
            {:else}
              <div class="reschedule-block" transition:fade={{ duration: 200 }}>
                <p class="reschedule-title">{$t('rescheduleTitle')}</p>
                <div class="reschedule-picker">
                  <DateRangePicker
                    schedule={figurineSchedule}
                    bind:startsAt={rescheduleStartsAt}
                    bind:endsAt={rescheduleEndsAt}
                    minDate={rescheduleMinDate}
                    {bookingRules}
                    onError={(msg) => { dateError = msg; }}
                  />
                </div>
                {#if dateError}
                  <p class="reschedule-err">{dateError}</p>
                {/if}
                {#if rescheduleError}
                  <p class="reschedule-err">{rescheduleError}</p>
                {/if}
                <div class="reschedule-actions">
                  <button class="cancel-btn" onclick={handleReschedule} disabled={rescheduling || !rescheduleStartsAt || !rescheduleEndsAt || !!dateError}>
                    {rescheduling ? $t('rescheduling') : $t('rescheduleConfirm')}
                  </button>
                  <button class="action-link" onclick={() => { showReschedule = false; rescheduleError = ''; dateError = ''; }}>{$t('rescheduleAbort')}</button>
                </div>
              </div>
            {/if}
          {/if}
          {#if rescheduleSuccess}
            <p class="reschedule-ok" transition:fade={{ duration: 300 }}>{$t('rescheduleSuccess')}</p>
          {/if}
          <a href="/figurines/{info.figurineId}" class="action-link">{$t('cancelGoToFigurine')}</a>
          <button class="print-btn" onclick={() => window.print()}>
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.2" aria-hidden="true">
              <path d="M3 1h6v3H3zM1 4h10v5H1zM3 9v2h6V9"/>
              <rect x="3" y="5.5" width="1.2" height="1.2" fill="currentColor" stroke="none"/>
            </svg>
            {$t('cancelPrint')}
          </button>
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
  .status-completed { background: #dce8e0; color: #2a5040; }

  .admin-notes {
    margin: 0 0 1rem;
    padding: 0.75rem 1rem;
    background: rgba(52,37,28,0.04);
    border: 1px solid #d8c6b1;
    text-align: left;
  }

  .admin-notes-label {
    display: block;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: rgba(95,70,54,0.55);
    margin-bottom: 0.35rem;
    font-weight: 700;
  }

  .admin-notes-text {
    margin: 0;
    font-size: 0.85rem;
    color: #34251c;
    font-style: italic;
    line-height: 1.5;
  }

  .try-again {
    color: #c65f3c;
    border-bottom-color: rgba(198,95,60,0.35);
  }
  .try-again:hover { color: #9e452d; border-bottom-color: #9e452d; }

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

  .cancel-btn--destructive {
    background: transparent;
    color: rgba(160,72,48,0.6);
    border: 1px solid rgba(160,72,48,0.22);
    font-size: 0.72rem;
    letter-spacing: 0.07em;
    padding: 0.5rem 1.5rem;
  }
  .cancel-btn--destructive:hover:not(:disabled) {
    background: rgba(160,72,48,0.05);
    color: #a04830;
    border-color: rgba(160,72,48,0.38);
  }

  .reschedule-block {
    width: 100%;
    max-width: 340px;
    text-align: left;
  }
  .reschedule-title {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1rem;
    color: #34251c;
    margin: 0 0 0.75rem;
    text-align: center;
  }
  .reschedule-picker {
    border: 1px solid #d8c6b1;
    padding: 0.75rem;
    background: #fdf8f2;
    margin-bottom: 0.5rem;
  }
  .reschedule-actions {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    margin-top: 0.75rem;
  }
  .reschedule-err {
    font-size: 0.78rem;
    color: #a03020;
    text-align: center;
    margin: 0.25rem 0 0;
    font-style: italic;
  }
  .reschedule-ok {
    font-size: 0.82rem;
    color: #3a6020;
    margin: 0;
    font-style: italic;
  }

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
    margin-top: 0.25rem;
  }
  .print-btn:hover { color: #34251c; border-color: #b0a090; }

  @media print {
    .cancel-wrap { min-height: unset; padding: 0; background: white; }
    .cancel-frame { outline: none; border: 1px solid #aaa; transform: none; box-shadow: none; background: white; }
    .action-link, .cancel-btn, .seal-icon, .wax-seal, .print-btn { display: none; }
    .info-list { font-size: 11pt; }
    dd, dt { color: black !important; }
    .cancel-heading { font-size: 16pt; }
  }
</style>
