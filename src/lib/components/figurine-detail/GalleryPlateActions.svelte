<script lang="ts">
  import { getContext } from 'svelte';
  import { t } from '$lib/i18n';
  import MarkIcon from '$lib/components/figurine-detail/MarkIcon.svelte';

  const ctx = getContext<App.FigurineDetailContext>('figurine-detail');

  let {
    quiet = false,
  }: {
    quiet?: boolean;
  } = $props();
</script>

<div class="gallery-actions">
  <div class="gallery-mark-wrap">
    <button
      type="button"
      class="gallery-mark"
      class:gallery-mark--marked={ctx.markTone}
      onclick={ctx.toggleMarkPicker}
      aria-label={ctx.markLabel}
      title={ctx.markLabel}
      aria-expanded={ctx.markPickerOpen}
    >
      <span class="mark-seal {ctx.markPressing ? 'mark-seal--pressing' : ''}" aria-hidden="true">
        <MarkIcon tone={ctx.markIconTone} active={!!ctx.markTone} />
      </span>
    </button>

    {#if ctx.markPickerOpen}
      <div class="gallery-mark-menu">
        {#each ctx.markToneOptions as opt (opt.tone)}
          <button
            type="button"
            class="gallery-mark-option"
            class:gallery-mark-option--active={ctx.markTone === opt.tone}
            onclick={() => ctx.setMarkTone(opt.tone)}
            aria-label={opt.label}
            title={opt.label}
          >
            <MarkIcon tone={opt.tone} active={ctx.markTone === opt.tone} />
            <span>{opt.label}</span>
          </button>
        {/each}
      </div>
    {/if}

    {#if ctx.markThanksVisible}
      <p class="mark-thanks">{$t('figurineMarkThanks')}</p>
    {/if}
  </div>

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

  <button
    type="button"
    class="gallery-heart"
    class:gallery-heart--saved={ctx.isSaved}
    onclick={ctx.toggleSaved}
    aria-label={ctx.isSaved ? $t('cardSaved') : $t('cardSave')}
    title={ctx.isSaved ? $t('cardSaved') : $t('cardSave')}
    aria-pressed={ctx.isSaved}
  >
    <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
      <path
        d="M7 12.5C7 12.5 1 8.5 1 4.5C1 2.5 2.5 1 4.5 1C5.5 1 6.5 1.8 7 3C7.5 1.8 8.5 1 9.5 1C11.5 1 13 2.5 13 4.5C13 8.5 7 12.5 7 12.5Z"
        fill={ctx.isSaved ? 'currentColor' : 'none'}
        stroke="currentColor"
        stroke-width="1.15"
        stroke-linejoin="round"
      />
    </svg>
  </button>
</div>
