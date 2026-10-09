<script lang="ts">
  /**
   * Где стоит знак числа: нет его, рядом с цифрой или отдельно (§ 9.10).
   *
   * Один на стол значка и стол метки: знак у стоимости и знак у маны
   * устроены одинаково, и два набора кнопок разошлись бы на первой правке.
   * Отдельный знак таскают по самой карте; здесь — только положение и
   * величина.
   */
  import { t } from '$lib/i18n';
  import {
    freeGlyphAt,
    glyphMode,
    setGlyphMode,
    setFreeMark,
    FREE_MARK_SIZE_MIN,
    FREE_MARK_SIZE_MAX,
    type FrameOverride,
    type GlyphMode,
    type GlyphSlot,
  } from '$lib/battles';
  import type { BattleFrame } from '$lib/types/api';

  let {
    frame,
    slot,
    write,
    from,
  }: {
    frame: BattleFrame;
    slot: GlyphSlot;
    write: () => BattleFrame | FrameOverride;
    /** Где число стоит сейчас — от него отделённый знак отходит. */
    from: { x: number; y: number };
  } = $props();

  let mode = $derived(glyphMode(frame, slot));
  let apart = $derived(freeGlyphAt(frame, slot));

  const MODES: { id: GlyphMode; key: 'adminBattlesGlyphOff' | 'adminBattlesGlyphBeside' | 'adminBattlesGlyphApart' }[] = [
    { id: 'off', key: 'adminBattlesGlyphOff' },
    { id: 'beside', key: 'adminBattlesGlyphBeside' },
    { id: 'apart', key: 'adminBattlesGlyphApart' },
  ];
</script>

<section class="gc">
  <span class="gc-label">{$t('adminBattlesReadyGlyphLong')}</span>
  <div class="gc-modes" role="radiogroup" aria-label={$t('adminBattlesReadyGlyphLong')}>
    {#each MODES as one (one.id)}
      <button
        type="button"
        class="gc-mode"
        class:active={mode === one.id}
        role="radio"
        aria-checked={mode === one.id}
        onclick={() => setGlyphMode(write(), slot, one.id, from)}
      >{$t(one.key)}</button>
    {/each}
  </div>
  {#if apart}
    <div class="gc-row">
      <input
        type="range"
        min={FREE_MARK_SIZE_MIN}
        max={FREE_MARK_SIZE_MAX}
        step="0.05"
        aria-label={$t('adminBattlesReadySize')}
        value={apart.size}
        oninput={(e) => setFreeMark(write(), slot, { glyphSize: Number(e.currentTarget.value) })}
      />
      <span class="gc-num">{apart.size.toFixed(2)}×</span>
    </div>
    <p class="gc-hint">{$t('adminBattlesGlyphApartHint')}</p>
  {/if}
</section>

<style>
  .gc {
    display: flex;
    flex-direction: column;
    gap: 0.32em;
  }

  .gc-label {
    font-size: 0.9em;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.65;
  }

  .gc-modes {
    display: flex;
  }

  .gc-mode {
    flex: 1;
    padding: 0.3em 0.2em;
    font: inherit;
    font-size: 0.95em;
    color: var(--ink);
    background: none;
    border: 1px solid color-mix(in oklab, var(--ink) 22%, transparent);
    cursor: pointer;
  }

  .gc-mode + .gc-mode {
    border-left: none;
  }

  .gc-mode.active {
    color: var(--paper);
    background: var(--ink);
  }

  .gc-row {
    display: flex;
    align-items: center;
    gap: 0.35em;
  }

  .gc-row input[type='range'] {
    flex: 1;
    min-width: 0;
    accent-color: var(--ink);
  }

  .gc-num {
    min-width: 3em;
    text-align: right;
    font-variant-numeric: tabular-nums;
    opacity: 0.75;
  }

  .gc-hint {
    margin: 0;
    font-size: 0.88em;
    line-height: 1.35;
    white-space: normal;
    opacity: 0.6;
  }
</style>
