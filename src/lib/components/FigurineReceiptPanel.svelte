<script lang="ts">
  import { t } from '$lib/i18n';

  let {
    title,
    note,
    stale = false,
    position = null,
    positionLabel = '',
    actionLabel,
    actionBusyLabel,
    busy = false,
    variant = 'default',
    onAction,
  }: {
    title: string;
    note: string;
    stale?: boolean;
    position?: number | null;
    positionLabel?: string;
    actionLabel: string;
    actionBusyLabel: string;
    busy?: boolean;
    variant?: 'default' | 'notify';
    onAction: () => void;
  } = $props();
</script>

<div class="queue-receipt {variant === 'notify' ? 'queue-receipt--notify' : ''}">
  {#if position !== null}
    <div class="queue-receipt-head">
      <span class="queue-receipt-title">{title}</span>
      <span class="queue-receipt-pos">
        <span class="queue-receipt-pos-label">{positionLabel}</span>
        <span class="queue-receipt-pos-num">№{position}</span>
      </span>
    </div>
  {:else}
    <span class="queue-receipt-title">{title}</span>
  {/if}

  <p class="queue-receipt-note">{note}</p>
  {#if stale}
    <p class="queue-receipt-note queue-receipt-note--warning">{$t('detailReceiptStale')}</p>
  {/if}
  <button type="button" onclick={onAction} disabled={busy} class="queue-receipt-leave">
    {busy ? actionBusyLabel : actionLabel}
  </button>
</div>
