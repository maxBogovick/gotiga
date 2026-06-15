<script lang="ts">
  import { t } from '$lib/i18n';
  import type { ClaimData } from '$lib/stores/figurine-claims.svelte';

  let {
    claim,
    isLoggedIn,
    isCancelling = false,
    error = '',
    formatDate,
    onCancel,
  }: {
    claim: ClaimData;
    isLoggedIn: boolean;
    isCancelling?: boolean;
    error?: string;
    formatDate: (date: string) => string;
    onCancel: (claim: ClaimData) => void;
  } = $props();

  let isConfirmed = $derived(claim.status === 'confirmed');
  let manageHref = $derived(isLoggedIn ? '/profile' : `/cancel/${claim.token}`);
</script>

<div class="cp-row {isConfirmed ? 'cp-row--confirmed' : ''}">
  <div class="cp-row-main">
    {#if isConfirmed}
      <svg width="11" height="11" viewBox="0 0 11 11" fill="none" stroke="currentColor" stroke-width="1.6" class="cp-icon cp-icon--ok" aria-hidden="true">
        <path d="M1 5.5l3 3 6-6"/>
      </svg>
    {:else}
      <svg width="11" height="11" viewBox="0 0 11 11" fill="none" stroke="currentColor" stroke-width="1.3" class="cp-icon" aria-hidden="true">
        <circle cx="5.5" cy="5.5" r="4.5"/>
        <path d="M5.5 3.5v2.2l1.5 1.3"/>
      </svg>
    {/if}
    <span class="cp-dates">{formatDate(claim.startsAt)} - {formatDate(claim.endsAt)}</span>
    {#if isConfirmed}
      <span class="cp-token">{claim.token.trim().length <= 4 ? claim.token.trim() : `${claim.token.trim().slice(0, 2)}...${claim.token.trim().slice(-2)}`}</span>
    {/if}
    <div class="cp-actions">
      <a href={manageHref} class="cp-link {isConfirmed ? '' : 'cp-link--reschedule'}">
        {isConfirmed ? $t('claimManageLink') : $t('ctaRescheduleLink')}
      </a>
      <button type="button" onclick={() => onCancel(claim)} disabled={isCancelling} class="cp-revoke">
        {isCancelling ? (isConfirmed ? '...' : $t('claimCancelling')) : $t('claimCancelBtn')}
      </button>
    </div>
  </div>
  <p class="cp-note {isConfirmed ? '' : 'cp-note--pending'}">
    {isConfirmed ? $t('claimConfirmedNext') : $t('claimPendingNote')}
  </p>
  {#if error}<p class="cp-err">{error}</p>{/if}
</div>
