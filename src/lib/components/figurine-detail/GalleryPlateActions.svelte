<script lang="ts">
  import { getContext } from 'svelte';
  import { t } from '$lib/i18n';

  const ctx = getContext<App.FigurineDetailContext>('figurine-detail');

  let {
    quiet = false,
  }: {
    quiet?: boolean;
  } = $props();
</script>

<div class="gallery-actions">
  {#if !quiet && ctx.showRakingButton}
    <button
      type="button"
      class="gallery-rake"
      class:gallery-rake--active={ctx.isRakingEnabled}
      onclick={ctx.toggleRaking}
      aria-label={ctx.isRakingEnabled ? $t('detailImageRakeOff') : $t('detailImageRakeOn')}
      title={ctx.isRakingEnabled ? $t('detailImageRakeOff') : $t('detailImageRakeOn')}
      aria-pressed={ctx.isRakingEnabled}
    >
      <svg width="15" height="15" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="3.2" cy="3.2" r="1.5" />
        <path d="M4.3 4.3L11 11" />
        <path d="M2 11.4h10" />
        <path d="M5.2 11.4l1.1-2M7.7 11.4l1.1-2" />
      </svg>
    </button>
  {/if}

  <button
    type="button"
    class="gallery-lens"
    class:gallery-lens--active={ctx.isLensEnabled}
    onclick={ctx.toggleLens}
    aria-label={ctx.isLensEnabled ? $t('detailImageLensOff') : $t('detailImageLensOn')}
    title={ctx.isLensEnabled ? $t('detailImageLensOff') : $t('detailImageLensOn')}
    aria-pressed={ctx.isLensEnabled}
  >
    <svg width="15" height="15" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" aria-hidden="true">
      <circle cx="6" cy="6" r="3.7" />
      <path d="M8.8 8.8L12 12" />
    </svg>
  </button>

  {#if ctx.canOpenLightbox}
    <button
      type="button"
      class="gallery-expand"
      onclick={() => ctx.openLightbox(ctx.activeImageIndex)}
      aria-label={$t('figurineFullscreen')}
      title={$t('figurineFullscreen')}
    >
      <svg width="13" height="13" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
        <path d="M1 4V1h3M6 1h3v3M9 6v3H6M4 9H1V6"/>
      </svg>
    </button>
  {/if}
</div>
