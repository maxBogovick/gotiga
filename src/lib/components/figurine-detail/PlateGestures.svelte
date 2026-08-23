<script lang="ts">
  /**
   * Remember and share-to-story — labelled buttons under the plate.
   * Idle mark still names the act; a tone icon appears once chosen.
   */
  import { getContext } from 'svelte';
  import { t } from '$lib/i18n';
  import MarkIcon from '$lib/components/figurine-detail/MarkIcon.svelte';

  const ctx = getContext<App.FigurineDetailContext>('figurine-detail');

  let chosen = $derived(ctx.markToneOptions.find((opt) => opt.tone === ctx.markTone));
</script>

<div class="plate-gestures">
  <div class="deed-row">
    <button
      type="button"
      class="deed"
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

    <button
      type="button"
      class="deed"
      onclick={() => ctx.openStoryModal()}
      disabled={ctx.storySaving}
      aria-label={ctx.storySaving ? $t('figurineStorySaving') : $t('figurineStoryShare')}
    >
      <span class="deed-ico" aria-hidden="true">
        {#if ctx.storySaving}
          <span class="deed-spin"></span>
        {:else}
          <svg width="14" height="14" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.35">
            <rect x="2" y="1.5" width="9" height="10" rx="1.2"/>
            <path d="M4.1 8.5 6 6.8l1.3 1.1 1.6-2 1.1 1.4"/>
            <circle cx="4.6" cy="4.1" r="0.65" fill="currentColor" stroke="none"/>
          </svg>
        {/if}
      </span>
      <span class="deed-label">{ctx.storySaving ? $t('figurineStorySaving') : $t('figurineStoryShare')}</span>
    </button>
  </div>

  {#if ctx.markPickerOpen}
    <div class="deed-row plate-gestures-tones" role="listbox" aria-label={$t('figurineMarkNone')}>
      {#each ctx.markToneOptions as opt (opt.tone)}
        <button
          type="button"
          class="deed deed--tone"
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
