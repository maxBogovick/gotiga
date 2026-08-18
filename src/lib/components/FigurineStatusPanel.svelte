<script lang="ts">
  import { onMount } from 'svelte';
  import type { Figurine, FigurineStatus } from '$lib/types/api';
  import FigurineReceiptPanel from '$lib/components/FigurineReceiptPanel.svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import { createFigurineAnalytics } from '$lib/analytics';

  type RequestIntent = 'request' | 'reserve' | 'waitlist' | 'viewing' | 'similar' | 'question' | 'notify';

  let {
    figurine,
    id,
    hasActiveShowing,
    nextAvailableDate,
    scheduleLoadFailed,
    onOpenModal,
    analyticsClient,
    queueJoin = null,
    notifyJoin = null,
    omitLead = false,
  }: {
    figurine: Figurine;
    id: string;
    hasActiveShowing: boolean;
    nextAvailableDate: Date | null;
    scheduleLoadFailed: boolean;
    onOpenModal: (intent: RequestIntent) => void;
    analyticsClient: ReturnType<typeof createFigurineAnalytics> | null;
    queueJoin?: { token: string; position: number } | null;
    notifyJoin?: string | null;
    omitLead?: boolean;
  } = $props();

  function readStoredToken(key: string): string | null {
    try { return localStorage.getItem(key); } catch { return null; }
  }
  function writeStoredToken(key: string, token: string) {
    try { localStorage.setItem(key, token); } catch {}
  }
  function removeStoredToken(key: string) {
    try { localStorage.removeItem(key); } catch {}
  }

  let queueKey = $derived(`gotiga_queue_${id}`);
  let queuePosition = $state(0);
  let queueLeaving = $state(false);
  let queueLeft = $state(false);
  let queueLookupStale = $state(false);
  let queueLeaveError = $state(false);

  async function loadQueue() {
    const token = readStoredToken(queueKey);
    if (!token) return;
    try {
      const info = await api.getWaitlistByToken(token);
      queueLookupStale = false;
      if (info) {
        queuePosition = info.position;
        queueLeft = false;
      } else {
        removeStoredToken(queueKey);
        queuePosition = 0;
      }
    } catch {
      queueLookupStale = true;
    }
  }

  $effect(() => {
    if (!queueJoin) return;
    writeStoredToken(queueKey, queueJoin.token);
    queuePosition = queueJoin.position;
    queueLeft = false;
    queueLookupStale = false;
    queueLeaveError = false;
  });

  async function leaveQueue() {
    const token = readStoredToken(queueKey);
    if (!token || queueLeaving) return;
    queueLeaving = true;
    queueLeaveError = false;
    try {
      await api.leaveWaitlistByToken(token);
      removeStoredToken(queueKey);
      queuePosition = 0;
      queueLeft = true;
      queueLookupStale = false;
    } catch {
      queueLeaveError = true;
    } finally {
      queueLeaving = false;
    }
  }

  let notifyKey = $derived(`gotiga_notify_${id}`);
  let notifyActive = $state(false);
  let notifyStopping = $state(false);
  let notifyStopped = $state(false);
  let notifyLookupStale = $state(false);
  let notifyStopError = $state(false);

  async function loadNotify() {
    const token = readStoredToken(notifyKey);
    if (!token) return;
    try {
      const info = await api.getNotifyByToken(token);
      notifyLookupStale = false;
      if (info) {
        notifyActive = true;
        notifyStopped = false;
      } else {
        removeStoredToken(notifyKey);
        notifyActive = false;
      }
    } catch {
      notifyLookupStale = true;
    }
  }

  $effect(() => {
    if (!notifyJoin) return;
    writeStoredToken(notifyKey, notifyJoin);
    notifyActive = true;
    notifyStopped = false;
    notifyLookupStale = false;
    notifyStopError = false;
  });

  async function stopNotify() {
    const token = readStoredToken(notifyKey);
    if (!token || notifyStopping) return;
    notifyStopping = true;
    notifyStopError = false;
    try {
      await api.cancelNotifyByToken(token);
      removeStoredToken(notifyKey);
      notifyActive = false;
      notifyStopped = true;
      notifyLookupStale = false;
    } catch {
      notifyStopError = true;
    } finally {
      notifyStopping = false;
    }
  }

  function statusLabel(status: FigurineStatus): string {
    switch (status) {
      case 'available':   return $t('figurineStatusAvailable');
      case 'reserved':    return $t('figurineStatusReserved');
      case 'in_progress': return $t('figurineStatusInProgress');
      case 'sold':        return $t('figurineStatusSold');
      default:            return '';
    }
  }

  let registryCode = $derived(id.slice(0, 3).toUpperCase());
  let formattedDate = $derived(
    nextAvailableDate?.toLocaleDateString(undefined, { day: 'numeric', month: 'long', year: 'numeric' }) ?? null
  );

  let statusTitle = $derived.by(() => {
    const isAvailable = figurine.status === 'available';
    return isAvailable
      ? (hasActiveShowing ? $t('detailRegistryViewingTitle') : $t('detailRegistryAvailableTitle'))
      : figurine.status === 'reserved'
        ? $t('detailRegistryReservedTitle')
        : figurine.status === 'in_progress'
          ? $t('detailRegistryProgressTitle')
          : $t('detailRegistrySoldTitle');
  });

  onMount(() => {
    void loadQueue();
    void loadNotify();
  });
</script>

<div class="entry-status entry-status--{figurine.status}" class:entry-status--follow={omitLead}>

  {#if !omitLead}
  <div class="entry-status-grain" aria-hidden="true"></div>

  <div class="entry-status-head">
    <span class="entry-status-marque">
      <span class="entry-wax" aria-hidden="true">
        <span class="entry-wax-ring"></span>
        <span class="entry-wax-glyph">GT</span>
      </span>
      <span class="entry-status-kind">
        <span class="entry-status-dot" aria-hidden="true"></span>
        {statusLabel(figurine.status)}
      </span>
    </span>
    <span class="entry-registry">
      <span class="entry-registry-label">{$t('detailRegistryNo')}</span>
      <span class="entry-registry-code">{registryCode}</span>
    </span>
  </div>

  <div class="entry-divider" aria-hidden="true"></div>
  {/if}

  <div class="entry-status-body">
    {#if !omitLead}
    <div class="entry-status-copy">
      <h2 class="entry-status-title">
        {statusTitle}
      </h2>

      <p class="entry-status-line">
        <span class="entry-price">{$t('figurinePriceOnRequest')}</span>
        {#if figurine.status === 'available'}
          {#if hasActiveShowing}
            <span class="entry-sep" aria-hidden="true"></span>{$t('detailPresenceOnExhibition')}{#if formattedDate}<span class="entry-sep" aria-hidden="true"></span>{$t('figurineAvailableFrom')} {formattedDate}{/if}
          {:else if formattedDate}
            <span class="entry-sep" aria-hidden="true"></span>{$t('figurineAvailableFrom')} {formattedDate}
          {:else}
            <span class="entry-sep" aria-hidden="true"></span>{$t('detailPresenceAvailableNow')}
          {/if}
        {:else if figurine.status === 'reserved'}
          <span class="entry-sep" aria-hidden="true"></span>{#if formattedDate}{$t('detailPresenceMayFree')} {formattedDate}{:else}{$t('figurineReserved')}{/if}
        {/if}
      </p>
    </div>
    {/if}

    <button
      type="button"
      class="commission-similar-btn"
      onclick={() => onOpenModal('similar')}
    >
      <span class="commission-similar-btn-label">{$t('commissionCreateSimilarCta')}</span>
      <span class="commission-similar-btn-icon" aria-hidden="true">
        <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
          <path d="M1.5 6h9M7 2.5L10.5 6 7 9.5"/>
        </svg>
      </span>
    </button>
  </div>

  {#if !omitLead}
  <div class="entry-status-facts" aria-label={$t('detailRegistryFacts')}>
    <span class="entry-status-facts-item">{$t('detailReplyWindow')}</span>
    <span class="entry-status-facts-item">{$t('detailNoObligation')}</span>
    <span class="entry-status-facts-item">{$t('detailPersonalTransfer')}</span>
  </div>

  <section class="trust-ledger" aria-label={$t('detailTrustBlockLabel')}>
    <p class="trust-ledger-mark">
      <span class="trust-ledger-lozenge" aria-hidden="true"></span>
      {$t('detailTrustUnique')}
    </p>
    <div class="trust-ledger-next">
      <p class="trust-ledger-desc">
        {#if figurine.status === 'available'}
          {$t('detailTrustNextAvailable')}
        {:else}
          {figurine.status === 'reserved'
            ? $t('detailTrustNextReserved')
            : figurine.status === 'in_progress'
              ? $t('detailTrustNextProgress')
              : $t('detailTrustNextSold')}
        {/if}
      </p>
      <a class="trust-ledger-link" href="/figurines/{id}/passport" onclick={() => analyticsClient?.cta('passport')}>
        <span>{$t('detailOpenPassport')}</span>
        <svg width="9" height="9" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M1 5h8M5.5 1.5L9 5l-3.5 3.5"/>
        </svg>
      </a>
    </div>
  </section>
  {/if}

  {#if scheduleLoadFailed}
    <p class="queue-receipt-left queue-receipt-left--warning">
      <span class="queue-receipt-mark" aria-hidden="true">!</span>
      {$t('detailScheduleLoadStale')}
    </p>
  {/if}

  {#if figurine.status === 'reserved'}
    {#if queuePosition > 0}
      <FigurineReceiptPanel
        title={$t('detailQueuePanelTitle')}
        note={$t('detailQueueNote')}
        stale={queueLookupStale}
        position={queuePosition}
        positionLabel={$t('detailQueuePositionLabel')}
        actionLabel={$t('detailQueueLeave')}
        actionBusyLabel={$t('detailQueueLeaving')}
        busy={queueLeaving}
        onAction={leaveQueue}
      />
      {#if queueLeaveError}
        <p class="queue-receipt-left queue-receipt-left--warning">
          <span class="queue-receipt-mark" aria-hidden="true">!</span>
          {$t('detailQueueLeaveError')}
        </p>
      {/if}
    {:else if queueLookupStale}
      <p class="queue-receipt-left queue-receipt-left--warning">
        <span class="queue-receipt-mark" aria-hidden="true">!</span>
        {$t('detailReceiptStale')}
      </p>
    {:else if queueLeft}
      <p class="queue-receipt-left">{$t('detailQueueLeft')}</p>
    {/if}
  {:else if figurine.status === 'in_progress' || figurine.status === 'sold'}
    {#if notifyActive}
      <FigurineReceiptPanel
        title={$t('detailNotifyPanelTitle')}
        note={$t('detailNotifyNote')}
        stale={notifyLookupStale}
        actionLabel={$t('detailNotifyStop')}
        actionBusyLabel={$t('detailNotifyStopping')}
        busy={notifyStopping}
        variant="notify"
        onAction={stopNotify}
      />
      {#if notifyStopError}
        <p class="queue-receipt-left queue-receipt-left--warning">
          <span class="queue-receipt-mark" aria-hidden="true">!</span>
          {$t('detailNotifyStopError')}
        </p>
      {/if}
    {:else if notifyLookupStale}
      <p class="queue-receipt-left queue-receipt-left--warning">
        <span class="queue-receipt-mark" aria-hidden="true">!</span>
        {$t('detailReceiptStale')}
      </p>
    {:else if notifyStopped}
      <p class="queue-receipt-left">{$t('detailNotifyStopped')}</p>
    {/if}
  {/if}

</div>

<style>
  /* ══════════════════════════════════════════════════════════════════════
     Shell — layered like a lacquered archive box: deep inset shadow,
     a faint paper-grain field, and a hairline ember seam at the crown.
     ══════════════════════════════════════════════════════════════════════ */
  .entry-status {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 1rem;
    margin: 1.6rem 0 1.1rem;
    padding: 0;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 13%, transparent);
    border-radius: 6px;
    background:
      radial-gradient(
        120% 160% at 12% -10%,
        color-mix(in srgb, var(--color-canvas-raised) 30%, transparent) 0%,
        transparent 55%
      ),
      var(--color-canvas-base);
    overflow: hidden;
    isolation: isolate;

    box-shadow:
      0 0 0 1px color-mix(in srgb, var(--color-canvas-raised) 20%, transparent) inset,
      0 1px 0 color-mix(in srgb, white 14%, transparent) inset,
      0 -1px 0 color-mix(in srgb, var(--color-ink-primary) 8%, transparent) inset,
      0 22px 56px -16px color-mix(in srgb, var(--color-ink-primary) 42%, transparent) inset,
      0 6px 20px -4px color-mix(in srgb, var(--color-ink-primary) 16%, transparent),
      0 1px 2px color-mix(in srgb, var(--color-ink-primary) 10%, transparent);

    text-align: left;
  }

  /* Faint archival grain — a quiet textile field behind everything */
  .entry-status-grain {
    position: absolute;
    inset: 0;
    z-index: 0;
    pointer-events: none;
    opacity: 0.5;
    background-image:
      radial-gradient(color-mix(in srgb, var(--color-ink-primary) 7%, transparent) 0.5px, transparent 0.5px);
    background-size: 11px 11px;
    background-position: 0 0;
    mask-image: linear-gradient(180deg, black 0%, transparent 78%);
  }

  /* Ember seam along the top edge, brighter at the seal */
  .entry-status::before {
    content: '';
    position: absolute;
    inset: 0 0 auto 0;
    height: 2px;
    z-index: 1;
    background: linear-gradient(
      90deg,
      transparent 0%,
      color-mix(in srgb, var(--color-ember) 38%, transparent) 18%,
      color-mix(in srgb, var(--color-ember) 85%, transparent) 32%,
      color-mix(in srgb, var(--color-ember) 38%, transparent) 46%,
      transparent 60%
    );
    pointer-events: none;
  }

  /* ══════════════════════════════════════════════════════════════════════
     Head
     ══════════════════════════════════════════════════════════════════════ */
  .entry-status-head {
    position: relative;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    min-width: 0;
    padding: 1.15rem 1.3rem 0;
  }

  .entry-status-marque {
    display: inline-flex;
    align-items: center;
    gap: 0.7rem;
    min-width: 0;
  }

  /* Wax seal — double ring, deeper carve, embossed glyph */
  .entry-wax {
    position: relative;
    display: inline-grid;
    place-items: center;
    width: 1.9rem;
    height: 1.9rem;
    flex: none;
    border-radius: 50%;
    transform: rotate(-7deg);
  }

  .entry-wax-ring {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      radial-gradient(
        circle at 36% 26%,
        color-mix(in srgb, var(--color-ember-deep) 50%, var(--color-accent, #c65f3c)) 0%,
        var(--color-ember-deep) 50%,
        color-mix(in srgb, var(--color-ember-deep) 72%, black) 100%
      );
    box-shadow:
      inset 0 1.5px 2px color-mix(in srgb, white 40%, transparent),
      inset 0 -3px 5px color-mix(in srgb, black 48%, transparent),
      inset 0 0 0 3px color-mix(in srgb, black 14%, transparent),
      0 0 0 1.5px color-mix(in srgb, var(--color-ember-deep) 42%, transparent),
      0 3px 7px color-mix(in srgb, var(--color-ink-primary) 38%, transparent);
  }

  .entry-wax-glyph {
    position: relative;
    z-index: 1;
    color: color-mix(in srgb, var(--color-canvas-raised) 94%, white);
    font-family: var(--font-display);
    font-size: 0.56rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-shadow: 0 1px 0 color-mix(in srgb, black 45%, transparent);
  }

  .entry-status-kind {
    display: inline-flex;
    align-items: center;
    gap: 0.38rem;
    width: fit-content;
    padding: 0.26rem 0.6rem 0.26rem 0.5rem;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--color-sage-ink) 24%, transparent);
    color: var(--color-sage-ink);
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--color-sage-subtle) 88%, transparent) 0%,
        color-mix(in srgb, var(--color-sage-subtle) 68%, transparent) 100%
      );
    font-family: var(--font-body);
    font-size: 0.57rem;
    font-weight: 800;
    letter-spacing: 0.12em;
    line-height: 1.2;
    text-transform: uppercase;
    box-shadow: inset 0 1px 0 color-mix(in srgb, white 22%, transparent);
  }

  .entry-status-dot {
    width: 0.32rem;
    height: 0.32rem;
    flex: none;
    border-radius: 50%;
    background: currentColor;
    box-shadow: 0 0 0 2.5px color-mix(in srgb, currentColor 20%, transparent);
  }

  .entry-status--reserved .entry-status-kind {
    border-color: color-mix(in srgb, var(--color-ochre-ink) 24%, transparent);
    color: var(--color-ochre-ink);
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--color-ochre-subtle) 88%, transparent) 0%,
        color-mix(in srgb, var(--color-ochre-subtle) 68%, transparent) 100%
      );
  }

  .entry-status--sold .entry-status-kind {
    border-color: color-mix(in srgb, var(--color-ink-primary) 14%, transparent);
    color: color-mix(in srgb, var(--color-ink-primary) 58%, transparent);
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--color-ink-primary) 8%, transparent) 0%,
        color-mix(in srgb, var(--color-ink-primary) 4%, transparent) 100%
      );
  }

  .entry-status--in_progress .entry-status-kind {
    border-color: color-mix(in srgb, var(--color-ember-deep) 30%, transparent);
    color: var(--color-ember-deep);
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--color-ember-subtle) 78%, transparent) 0%,
        color-mix(in srgb, var(--color-ember-subtle) 58%, transparent) 100%
      );
  }

  .entry-registry {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.1rem;
    min-width: 0;
    flex: none;
  }

  .entry-registry-label {
    color: color-mix(in srgb, var(--color-ink-secondary) 36%, transparent);
    font-family: var(--font-body);
    font-size: 0.46rem;
    font-weight: 800;
    letter-spacing: 0.16em;
    line-height: 1;
    text-transform: uppercase;
  }

  .entry-registry-code {
    color: color-mix(in srgb, var(--color-ink-secondary) 56%, transparent);
    font-family: var(--font-display);
    font-size: 0.78rem;
    font-style: italic;
    font-weight: 600;
    letter-spacing: 0.06em;
    line-height: 1.1;
  }

  /* ══════════════════════════════════════════════════════════════════════
     Hairline divider — etched, not flat
     ══════════════════════════════════════════════════════════════════════ */
  .entry-divider {
    position: relative;
    z-index: 2;
    height: 1px;
    margin: 0 1.3rem;
    background: linear-gradient(
      90deg,
      transparent 0%,
      color-mix(in srgb, var(--color-ink-primary) 11%, transparent) 16%,
      color-mix(in srgb, var(--color-ink-primary) 11%, transparent) 84%,
      transparent 100%
    );
  }

  .entry-divider::after {
    content: '';
    position: absolute;
    inset: 1px 0 auto 0;
    height: 1px;
    background: linear-gradient(
      90deg,
      transparent 0%,
      color-mix(in srgb, white 30%, transparent) 16%,
      color-mix(in srgb, white 30%, transparent) 84%,
      transparent 100%
    );
  }

  /* ══════════════════════════════════════════════════════════════════════
     Body
     ══════════════════════════════════════════════════════════════════════ */
  .entry-status-body {
    position: relative;
    z-index: 2;
    display: flex;
    flex-direction: column;
    gap: 1.15rem;
    padding: 0 1.3rem;
  }

  .entry-status-copy {
    display: grid;
    gap: 0.55rem;
    min-width: 0;
  }

  .entry-status-title {
    margin: 0;
    color: var(--color-ink-primary);
    font-family: var(--font-display);
    font-size: 1.78rem;
    font-weight: 440;
    letter-spacing: -0.022em;
    line-height: 1.04;
    text-wrap: balance;
  }

  .entry-status-line {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.25rem 0.55rem;
    color: color-mix(in srgb, var(--color-ink-primary) 62%, transparent);
    font-family: var(--font-body);
    font-size: 0.81rem;
    line-height: 1.5;
    margin: 0;
  }

  .entry-price {
    position: relative;
    color: var(--color-ink-primary);
    font-weight: 700;
    letter-spacing: 0.01em;
    padding-bottom: 0.05rem;
    border-bottom: 1.5px solid color-mix(in srgb, var(--color-ember) 35%, transparent);
  }

  .entry-status-line .entry-sep {
    display: inline-block;
    width: 0.18rem;
    height: 0.18rem;
    margin: 0 0.05rem;
    border-radius: 50%;
    background: color-mix(in srgb, var(--color-ink-primary) 28%, transparent);
    align-self: center;
  }

  /* ── Similar CTA — quieter card, livelier hover, sliding glyph ──────────── */
  .commission-similar-btn {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    align-self: stretch;
    min-height: 2.9rem;
    padding: 0 0.4rem 0 1.15rem;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 17%, transparent);
    border-radius: 4px;
    color: var(--color-ink-primary);
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--color-canvas-raised) 44%, transparent) 0%,
        color-mix(in srgb, var(--color-canvas-raised) 16%, transparent) 100%
      );
    box-shadow:
      inset 0 1px 0 color-mix(in srgb, white 20%, transparent),
      0 1px 3px color-mix(in srgb, var(--color-ink-primary) 7%, transparent);
    cursor: pointer;
    overflow: hidden;
    transition:
      border-color 220ms ease,
      color 220ms ease,
      background 220ms ease,
      box-shadow 220ms ease;
  }

  .commission-similar-btn-label {
    font-family: var(--font-body);
    font-size: 0.63rem;
    font-weight: 800;
    letter-spacing: 0.11em;
    line-height: 1.1;
    text-transform: uppercase;
  }

  .commission-similar-btn-icon {
    display: inline-grid;
    place-items: center;
    width: 2.1rem;
    height: 2.1rem;
    flex: none;
    border-radius: 50%;
    background: color-mix(in srgb, var(--color-ink-primary) 6%, transparent);
    transition:
      background 220ms ease,
      transform 220ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .commission-similar-btn:hover {
    border-color: color-mix(in srgb, var(--color-ember-deep) 45%, transparent);
    color: var(--color-ember-deep);
    background: color-mix(in srgb, var(--color-ember) 7%, color-mix(in srgb, var(--color-canvas-raised) 38%, transparent));
    box-shadow:
      inset 0 1px 0 color-mix(in srgb, white 16%, transparent),
      0 2px 6px color-mix(in srgb, var(--color-ember-deep) 14%, transparent);
  }

  .commission-similar-btn:hover .commission-similar-btn-icon {
    background: color-mix(in srgb, var(--color-ember) 16%, transparent);
    transform: translateX(2px);
  }

  .commission-similar-btn:active {
    transform: translateY(1px);
    box-shadow: inset 0 1px 0 color-mix(in srgb, white 10%, transparent);
  }

  @media (prefers-reduced-motion: reduce) {
    .commission-similar-btn,
    .commission-similar-btn-icon {
      transition: none;
    }
  }

  /* ══════════════════════════════════════════════════════════════════════
     Facts strip — recessed ledger band
     ══════════════════════════════════════════════════════════════════════ */
  .entry-status-facts {
    position: relative;
    z-index: 2;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0;
    padding: 0.75rem 1.3rem;
    border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 7%, transparent);
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 7%, transparent);
    background: color-mix(in srgb, var(--color-ink-primary) 3%, transparent);
    box-shadow:
      inset 0 1px 2px color-mix(in srgb, var(--color-ink-primary) 5%, transparent),
      inset 0 -1px 0 color-mix(in srgb, white 8%, transparent);
    color: color-mix(in srgb, var(--color-ink-secondary) 44%, transparent);
    font-family: var(--font-body);
    font-size: 0.52rem;
    font-weight: 800;
    letter-spacing: 0.12em;
    line-height: 1.6;
    text-transform: uppercase;
  }

  .entry-status-facts-item {
    position: relative;
    min-width: 0;
    padding: 0 0.75rem;
  }

  .entry-status-facts-item:first-child {
    padding-left: 0;
  }

  .entry-status-facts-item:last-child {
    padding-right: 0;
  }

  .entry-status-facts-item + .entry-status-facts-item::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0.2em;
    bottom: 0.2em;
    width: 1px;
    background: color-mix(in srgb, var(--color-ink-primary) 16%, transparent);
  }

  /* ══════════════════════════════════════════════════════════════════════
     Trust ledger — italic display mark + recessed passport link
     ══════════════════════════════════════════════════════════════════════ */
  .trust-ledger {
    position: relative;
    z-index: 2;
    display: grid;
    gap: 0.95rem;
    padding: 1.05rem 1.3rem 1.15rem;
  }

  .trust-ledger-mark {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    margin: 0;
    color: var(--color-ink-primary);
    font-family: var(--font-display);
    font-size: 1.14rem;
    font-style: italic;
    font-weight: 490;
    line-height: 1.2;
  }

  .trust-ledger-lozenge {
    position: relative;
    flex: none;
    width: 0.4rem;
    height: 0.4rem;
    transform: rotate(45deg);
    background: var(--color-ember);
    box-shadow:
      0 0 0 3px color-mix(in srgb, var(--color-ember) 16%, transparent),
      0 0 10px 1px color-mix(in srgb, var(--color-ember) 32%, transparent);
  }

  .trust-ledger-lozenge::after {
    content: '';
    position: absolute;
    inset: -5px;
    border: 1px solid color-mix(in srgb, var(--color-ember) 22%, transparent);
  }

  .trust-ledger-next {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.9rem;
    padding-top: 0.15rem;
    border-top: 1px dashed color-mix(in srgb, var(--color-ink-primary) 10%, transparent);
  }

  .trust-ledger-desc {
    margin: 0;
    padding-top: 0.7rem;
    color: color-mix(in srgb, var(--color-ink-secondary) 74%, transparent);
    font-family: var(--font-body);
    font-size: 0.76rem;
    line-height: 1.55;
  }

  .trust-ledger-link {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    flex: 0 0 auto;
    margin-top: 0.7rem;
    padding: 0.34rem 0.8rem;
    border: 1px solid color-mix(in srgb, var(--color-ember) 38%, transparent);
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-ember) 7%, transparent);
    color: var(--color-ember-deep);
    cursor: pointer;
    font-family: var(--font-body);
    font-size: 0.59rem;
    font-weight: 800;
    letter-spacing: 0.1em;
    line-height: 1.2;
    text-decoration: none;
    text-transform: uppercase;
    transition:
      border-color 200ms ease,
      background 200ms ease,
      color 200ms ease;
  }

  .trust-ledger-link svg {
    transition: transform 200ms ease;
  }

  .trust-ledger-link:hover {
    border-color: color-mix(in srgb, var(--color-ember) 70%, transparent);
    background: color-mix(in srgb, var(--color-ember) 14%, transparent);
    color: var(--color-ink-primary);
  }

  .trust-ledger-link:hover svg {
    transform: translateX(2px);
  }

  @media (prefers-reduced-motion: reduce) {
    .trust-ledger-link,
    .trust-ledger-link svg {
      transition: none;
    }
  }

  /* ══════════════════════════════════════════════════════════════════════
     Receipt states
     ══════════════════════════════════════════════════════════════════════ */
  .queue-receipt-left {
    position: relative;
    z-index: 2;
    display: flex;
    align-items: baseline;
    gap: 0.45rem;
    margin: 0;
    padding: 0 1.3rem 1.1rem;
    color: color-mix(in srgb, var(--color-ink-secondary) 74%, transparent);
    font-family: var(--font-body);
    font-size: 0.73rem;
    font-style: italic;
    line-height: 1.5;
  }

  .queue-receipt-left--warning {
    color: var(--color-ember-deep);
  }

  .queue-receipt-mark {
    display: inline-grid;
    place-items: center;
    flex: none;
    width: 0.95rem;
    height: 0.95rem;
    border-radius: 50%;
    background: color-mix(in srgb, var(--color-ember-deep) 14%, transparent);
    color: var(--color-ember-deep);
    font-family: var(--font-body);
    font-size: 0.58rem;
    font-style: normal;
    font-weight: 800;
    line-height: 1;
    transform: translateY(0.1rem);
  }

  /* ══════════════════════════════════════════════════════════════════════
     Mobile
     ══════════════════════════════════════════════════════════════════════ */
  @media (max-width: 767px) {
    .entry-status-head {
      align-items: flex-start;
      padding: 1rem 1.05rem 0;
    }

    .entry-registry {
      align-items: flex-end;
    }

    .entry-divider {
      margin: 0 1.05rem;
    }

    .entry-status-body {
      padding: 0 1.05rem;
    }

    .entry-status-title {
      font-size: 1.36rem;
    }

    .commission-similar-btn {
      justify-content: space-between;
    }

    .entry-status-facts {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.3rem;
      padding: 0.7rem 1.05rem;
    }

    .entry-status-facts-item {
      padding: 0 !important;
    }

    .entry-status-facts-item + .entry-status-facts-item::before {
      content: none;
    }

    .trust-ledger {
      padding: 0.95rem 1.05rem 1.05rem;
    }

    .trust-ledger-next {
      align-items: flex-start;
      flex-direction: column;
      gap: 0;
    }

    .queue-receipt-left {
      padding: 0 1.05rem 1rem;
    }
  }

  .entry-status--follow {
    margin: 0.2rem 0 0.85rem;
    padding: 0;
    border: none;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    overflow: visible;
  }

  .entry-status--follow::before,
  .entry-status--follow .entry-status-grain {
    display: none;
  }

  .entry-status--follow .entry-status-body {
    padding: 0;
  }
</style>