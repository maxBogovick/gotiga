<script lang="ts">
  /**
   * Одно дело на работе: бронь, заказ, прошение или место в очереди.
   *
   * Всё подробное печатается здесь же — сроки, условия хранителя, ступени
   * прошения, паспорт, — ничего не спрятано за ещё одним нажатием: дальше
   * этого листа идти уже некуда.
   */
  import { t, lang } from '$lib/i18n';
  import { api, resolveMediaUrl } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import AppImage from '$lib/components/AppImage.svelte';
  import { userDesk, COMMISSION_STAGES, type FeedItem } from '$lib/stores/user-desk.svelte';
  import {
    kindLabel, feedStatusLabel, orderModeLabel, reserveStatusLabel,
    commissionStatusLabel, wishStatusLabel, formatDate, formatDateRange,
  } from '$lib/profile-labels';
  import type { CommissionDto } from '$lib/types/api';

  let { item, onedit }: { item: FeedItem; onedit: (c: CommissionDto) => void } = $props();

  let booking = $derived(item.kind === 'booking' ? userDesk.bookingById.get(item.id) : undefined);
  let order = $derived(item.kind === 'order' ? userDesk.orderById.get(item.id) : undefined);
  let commission = $derived(item.kind === 'commission' ? userDesk.commissionById.get(item.id) : undefined);

  let confirmDelete = $state(false);
  let deleting = $state(false);
  let deleteError = $state('');

  function stageState(status: string, stage: string): 'done' | 'current' | 'todo' {
    if (status === 'declined') return 'todo';
    const stages: readonly string[] = COMMISSION_STAGES;
    const cur = stages.indexOf(status);
    const idx = stages.indexOf(stage);
    if (cur < 0) return 'todo';
    return idx < cur ? 'done' : idx === cur ? 'current' : 'todo';
  }

  async function removeCommission(c: CommissionDto) {
    if (deleting) return;
    deleting = true;
    deleteError = '';
    try {
      await api.deleteCommission(authStore.token!, c.id);
      userDesk.dropCommission(c.id);
      confirmDelete = false;
    } catch {
      deleteError = $t('profileActionError');
    } finally {
      deleting = false;
    }
  }
</script>

<li class="entry" class:unread={item.unread > 0}>
  <div class="entry-head">
    <span class="entry-date">{formatDate($lang, item.date)}</span>
    <span class="entry-kind">{kindLabel($t, item.kind)}</span>
    <span class="pf-tone pf-tone--{item.tone} entry-tone">{feedStatusLabel($t, item)}</span>
  </div>

  {#if booking}
    <div class="detail">
      <p class="line">{formatDateRange($lang, booking.startsAt, booking.endsAt)}</p>
      {#if booking.curatorConditions}
        <div class="said">
          <span class="said-label">{$t('profileBookingCuratorConditions')}</span>
          <p class="said-text">{booking.curatorConditions}</p>
        </div>
      {/if}
      <div class="acts">
        <a class="pf-link" href="/cancel/{booking.cancelToken}">{$t('profileBookingManage')}</a>
      </div>
    </div>
  {:else if order}
    <div class="detail">
      <p class="line">{orderModeLabel($t, order.mode)}</p>
      {#if order.mode === 'reserve'}
        <p class="line">{reserveStatusLabel($t, order.reserveStatus)}</p>
        {#if order.reserveExpiresAt}
          <p class="line">{$t('profileReserveExpires')} {formatDate($lang, order.reserveExpiresAt)}</p>
        {/if}
        {#if order.adminTermsNote}
          <div class="said">
            <span class="said-label">{$t('profileReserveTerms')}</span>
            <p class="said-text">{order.adminTermsNote}</p>
          </div>
        {/if}
        {#if order.invoiceNote}
          <div class="said">
            <span class="said-label">{$t('profileReserveInvoice')}</span>
            <p class="said-text">{order.invoiceNote}</p>
          </div>
        {/if}
        {#if order.adminNotes}<p class="line">{order.adminNotes}</p>{/if}
        {#if order.certificate}
          <div class="said">
            <span class="said-label">{$t('profileCertificateTitle')}</span>
            <p class="said-text">
              {order.certificate.certificateNumber}
              {#if order.certificate.revokedAt} · {$t('profileCertificateRevoked')}{/if}
            </p>
            <div class="acts">
              <a class="pf-link" href="/certificate/{order.certificate.token}">{$t('profileCertificateOpen')}</a>
            </div>
          </div>
        {/if}
      {/if}
    </div>
  {:else if commission}
    {@const c = commission}
    <div class="detail">
      {#if c.status === 'declined'}
        <p class="line">{commissionStatusLabel($t, 'declined')}</p>
      {:else}
        <ol class="stages">
          {#each COMMISSION_STAGES as stage (stage)}
            {@const st = stageState(c.status, stage)}
            <li class="stage stage--{st}">
              <span class="stage-dot" aria-hidden="true">{st === 'done' ? '✓' : ''}</span>
              <span class="stage-word">{commissionStatusLabel($t, stage)}</span>
            </li>
          {/each}
        </ol>
      {/if}

      {#if c.adminNotes}
        <div class="said">
          <span class="said-label">{$t('profileCommissionMasterNote')}</span>
          <p class="said-text">{c.adminNotes}</p>
        </div>
      {/if}

      {#if c.certificate}
        <div class="said">
          <span class="said-label">{$t('profileCertificateTitle')}</span>
          <p class="said-text">
            {c.certificate.certificateNumber}
            {#if c.certificate.revokedAt} · {$t('profileCertificateRevoked')}{/if}
          </p>
          <div class="acts">
            <a class="pf-link" href="/certificate/{c.certificate.token}">{$t('profileCertificateOpen')}</a>
          </div>
        </div>
      {/if}

      {#if c.sourceFigurineId}
        {@const source = userDesk.figurineById.get(c.sourceFigurineId)}
        <a class="source" href="/figurines/{c.sourceFigurineId}">
          <span class="source-thumb">
            {#if source?.faceImageUrl}
              <AppImage src={source.faceImageUrl} thumbUrl={source.thumbUrl} alt={source.name} class="source-img" loading="lazy" />
            {:else}
              <span class="source-ph" aria-hidden="true">✦</span>
            {/if}
          </span>
          <span class="source-copy">
            <span class="source-label">{$t('profileCommissionSource')}</span>
            <span class="source-name">{source?.name ?? c.sourceFigurineId}</span>
            <span class="source-meta">
              {source ? wishStatusLabel($t, source.status) : $t('profileCommissionSourceMissing')}
            </span>
          </span>
        </a>
      {/if}

      {#if c.description}<p class="line">{c.description}</p>{/if}

      {#if c.similarKeepNote}
        <p class="line"><span class="said-label">{$t('profileCommissionKeep')}</span> {c.similarKeepNote}</p>
      {/if}
      {#if c.similarChangeNote}
        <p class="line"><span class="said-label">{$t('profileCommissionChange')}</span> {c.similarChangeNote}</p>
      {/if}

      {#if c.attachments.length > 0}
        <div class="thumbs">
          {#each c.attachments as att (att.id)}
            <img src={resolveMediaUrl(att.thumbUrl ?? att.url)} alt="" />
          {/each}
        </div>
      {/if}

      <div class="acts">
        {#if c.threadId}
          <a class="pf-link" href="/profile/messages/{c.threadId}">{$t('profileCommissionsOpenChat')} →</a>
        {/if}
        {#if c.started}
          <span class="pf-quiet">{$t('profileCommissionsLocked')}</span>
        {:else if confirmDelete}
          <span class="pf-quiet">{$t('profileCommissionsDeleteConfirm')}</span>
          <button class="pf-link pf-link--danger" onclick={() => removeCommission(c)} disabled={deleting}>
            {deleting ? '…' : $t('profileCommissionsDeleteYes')}
          </button>
          <button class="pf-link" onclick={() => (confirmDelete = false)}>{$t('profileDeleteCancel')}</button>
        {:else}
          <button class="pf-link" onclick={() => onedit(c)}>{$t('profileCommissionsEdit')}</button>
          <button class="pf-link pf-link--danger" onclick={() => (confirmDelete = true)}>{$t('profileCommissionsDelete')}</button>
        {/if}
      </div>
      {#if deleteError}<p class="pf-error" role="alert">{deleteError}</p>{/if}
    </div>
  {/if}

  {#if item.unread > 0 && item.threadId}
    <a class="reply" href="/profile/messages/{item.threadId}">
      <span class="pf-count pf-count--new">{item.unread}</span>
      {$t('profileOverviewNewReply')} →
    </a>
  {/if}
</li>

<style>
  .entry {
    padding: 1.1rem 0;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 8%, transparent);
  }
  .entry.unread { border-left: 2px solid var(--color-ember); padding-left: 0.8rem; }

  .entry-head {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 0.3rem 0.8rem;
  }

  .entry-date {
    font-family: var(--font-body);
    font-size: 0.76rem;
    color: var(--color-ink-tertiary);
  }

  .entry-kind {
    font-family: var(--font-body);
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-ink-secondary);
  }

  .entry-tone { margin-left: auto; }

  .detail { margin-top: 0.7rem; display: flex; flex-direction: column; gap: 0.6rem; }

  .line {
    font-family: var(--font-body);
    font-size: 0.86rem;
    line-height: 1.55;
    color: var(--color-ink-secondary);
    margin: 0;
    overflow-wrap: anywhere;
  }

  .said {
    background: var(--color-canvas-sunken);
    border-left: 2px solid var(--color-border-default);
    padding: 0.55rem 0.8rem;
  }

  .said-label {
    display: block;
    font-family: var(--font-body);
    font-size: 0.72rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
    margin-bottom: 0.2rem;
  }

  .said-text {
    font-family: var(--font-body);
    font-size: 0.86rem;
    line-height: 1.55;
    color: var(--color-ink-secondary);
    margin: 0;
    white-space: pre-wrap;
  }

  .acts { display: flex; flex-wrap: wrap; align-items: center; gap: 0.9rem; }

  /* ── Ступени прошения ── */
  .stages {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem 0.9rem;
  }

  .stage { display: flex; align-items: center; gap: 0.35rem; }

  .stage-dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 1px solid var(--color-border-default);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.6rem;
    line-height: 1;
    color: var(--color-canvas-raised);
  }

  .stage-word {
    font-family: var(--font-body);
    font-size: 0.76rem;
    color: var(--color-ink-tertiary);
  }

  .stage--done .stage-dot { background: var(--color-sage); border-color: var(--color-sage); }
  .stage--done .stage-word { color: var(--color-ink-secondary); }
  .stage--current .stage-dot { background: var(--color-ember); border-color: var(--color-ember); }
  .stage--current .stage-word { color: var(--color-ember-ink); }

  /* ── Работа-источник ── */
  .source {
    display: flex;
    gap: 0.7rem;
    align-items: center;
    text-decoration: none;
    border: 1px solid var(--color-border-subtle);
    background: var(--color-canvas-raised);
    padding: 0.5rem;
    transition: border-color 0.15s;
  }
  .source:hover { border-color: var(--color-border-ember); }

  .source-thumb {
    width: 44px;
    height: 54px;
    flex-shrink: 0;
    overflow: hidden;
    background: var(--color-canvas-sunken);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .source-thumb :global(.source-img) { width: 100%; height: 100%; object-fit: cover; }
  .source-ph { color: var(--color-ink-tertiary); }

  .source-copy { display: flex; flex-direction: column; gap: 0.12rem; min-width: 0; }

  .source-label {
    font-family: var(--font-body);
    font-size: 0.7rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
  }

  .source-name {
    font-family: var(--font-serif);
    font-size: 0.95rem;
    color: var(--color-ink-primary);
  }

  .source-meta {
    font-family: var(--font-body);
    font-size: 0.74rem;
    color: var(--color-ink-tertiary);
  }

  .thumbs { display: flex; flex-wrap: wrap; gap: 0.4rem; }
  .thumbs img {
    width: 62px;
    height: 62px;
    object-fit: cover;
    border: 1px solid var(--color-border-subtle);
  }

  .reply {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    margin-top: 0.8rem;
    font-family: var(--font-body);
    font-size: 0.82rem;
    color: var(--color-ember);
    text-decoration: none;
  }
  .reply:hover { color: var(--color-ember-deep); text-decoration: underline; }
</style>
