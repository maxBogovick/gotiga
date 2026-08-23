<script lang="ts">
  /**
   * Desktop-only Like and Remember, sitting on the plate above the photograph.
   */
  import { getContext } from 'svelte';
  import { t } from '$lib/i18n';
  import MarkIcon from '$lib/components/figurine-detail/MarkIcon.svelte';
  import LikeDeed from './LikeDeed.svelte';

  const ctx = getContext<App.FigurineDetailContext>('figurine-detail');
  let chosen = $derived(ctx.markToneOptions.find((opt) => opt.tone === ctx.markTone));
</script>

<div class="catalog-plate-marks">
  <div class="catalog-plate-marks-row">
    <LikeDeed extraClass="catalog-plate-mark" />

    <button
      type="button"
      class="deed catalog-plate-mark"
      class:deed--on={ctx.markTone}
      class:deed--open={ctx.markPickerOpen}
      onclick={ctx.toggleMarkPicker}
      aria-expanded={ctx.markPickerOpen}
      aria-haspopup="listbox"
      aria-label={ctx.markLabel}
    >
      <span class="deed-ico mark-seal" class:mark-seal--pressing={ctx.markPressing} aria-hidden="true">
        <MarkIcon tone={ctx.markTone ?? 'bookmark'} active={!!ctx.markTone} />
      </span>
      <span class="deed-label">{chosen?.label ?? $t('figurineMarkNone')}</span>
    </button>
  </div>

  {#if ctx.markPickerOpen}
    <div class="catalog-plate-marks-tones" role="listbox" aria-label={$t('figurineMarkNone')}>
      {#each ctx.markToneOptions as opt (opt.tone)}
        <button
          type="button"
          class="deed deed--tone catalog-plate-mark"
          class:deed--on={ctx.markTone === opt.tone}
          role="option"
          aria-selected={ctx.markTone === opt.tone}
          onclick={() => ctx.setMarkTone(opt.tone)}
        >
          <span class="deed-ico" aria-hidden="true">
            <MarkIcon tone={opt.tone} active={ctx.markTone === opt.tone} />
          </span>
          <span class="deed-label">{opt.label}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if ctx.markThanksVisible}
    <p class="plate-gestures-thanks">{$t('figurineMarkThanks')}</p>
  {/if}
</div>
