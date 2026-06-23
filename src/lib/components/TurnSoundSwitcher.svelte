<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t } from '$lib/i18n';
  import type { TranslationKey } from '$lib/i18n/en';
  import { turnSound, type TurnSoundPref } from '$lib/stores/page-turn-sound.svelte';
  import { playTurnSound } from '$lib/audio/page-turn-sounds';

  // Reader's choice of page-turn sound. Silent by default; selecting a variant
  // previews it so the choice is audible, not abstract.
  const options: { id: TurnSoundPref; key: TranslationKey }[] = [
    { id: 'off', key: 'turnSoundOff' },
    { id: 'parchment', key: 'turnSoundParchment' },
    { id: 'thin', key: 'turnSoundThin' },
    { id: 'tome', key: 'turnSoundTome' },
    { id: 'cloth', key: 'turnSoundCloth' },
    { id: 'riffle', key: 'turnSoundRiffle' },
  ];

  let open = $state(false);
  let wrap = $state<HTMLElement>();

  let current = $derived(turnSound.value);
  let isOn = $derived(current !== 'off');

  onMount(() => turnSound.load());

  function choose(id: TurnSoundPref) {
    turnSound.set(id);
    if (id !== 'off') playTurnSound(id, 'forward');
  }

  function onPointerDown(e: PointerEvent) {
    if (wrap && !wrap.contains(e.target as Node)) open = false;
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false;
  }

  $effect(() => {
    if (!open) return;
    window.addEventListener('pointerdown', onPointerDown, true);
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('pointerdown', onPointerDown, true);
      window.removeEventListener('keydown', onKey);
    };
  });
</script>

<div class="turn-sound" bind:this={wrap}>
  <button
    type="button"
    class="control-btn control-btn--utility {isOn ? 'control-btn--active' : ''}"
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={() => (open = !open)}
    title={$t('turnSoundLabel')}
    aria-label={$t('turnSoundLabel')}
  >
    <svg class="control-svg" width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
      <!-- a leaf curling at the spine + two faint sound arcs -->
      <path d="M2 2.2h4.2c.7 0 1.3.5 1.3 1.2v7.2c0-.6-.6-1.1-1.3-1.1H2z" stroke-linejoin="round" />
      <path d="M7.5 3.4c.6.1 1.1.6 1.1 1.2v7.2" stroke-linecap="round" />
      <path d="M10.4 4.7c.7.7.7 2 0 2.8M11.9 3.6c1.2 1.3 1.2 3.6 0 5" stroke-linecap="round" opacity="0.8" />
    </svg>
    <span class="btn-label">{$t('turnSoundLabel')}</span>
  </button>

  {#if open}
    <div class="turn-sound-panel" role="menu" aria-label={$t('turnSoundLabel')} transition:fade={{ duration: 120 }}>
      {#each options as opt (opt.id)}
        <button
          type="button"
          role="menuitemradio"
          aria-checked={current === opt.id}
          class="turn-sound-opt {current === opt.id ? 'turn-sound-opt--on' : ''}"
          onclick={() => choose(opt.id)}
        >
          <span class="turn-sound-tick" aria-hidden="true"></span>
          <span class="turn-sound-name">{$t(opt.key)}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .turn-sound {
    position: relative;
    display: inline-flex;
  }

  .turn-sound-panel {
    position: absolute;
    top: calc(100% + 0.5rem);
    right: 0;
    z-index: 60;
    min-width: 11rem;
    padding: 0.3rem;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 22%, transparent);
    border-radius: 5px;
    background: var(--color-canvas-raised, #f8f1e7);
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.7) inset,
      0 16px 40px color-mix(in srgb, var(--color-ink-primary, #34251c) 22%, transparent);
  }
  /* double-rule frame, the house motif */
  .turn-sound-panel::before {
    content: '';
    position: absolute;
    inset: 3px;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 10%, transparent);
    border-radius: 3px;
    pointer-events: none;
  }

  .turn-sound-opt {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.42rem 0.55rem;
    border: 0;
    border-radius: 3px;
    background: transparent;
    color: color-mix(in srgb, var(--color-ink-primary, #34251c) 78%, transparent);
    font-family: var(--font-body, Georgia, serif);
    font-size: 0.72rem;
    letter-spacing: 0.02em;
    text-align: left;
    cursor: pointer;
    transition: background var(--duration-fast, 160ms) ease, color var(--duration-fast, 160ms) ease;
  }
  .turn-sound-opt:hover {
    background: color-mix(in srgb, var(--color-ink-primary, #34251c) 7%, transparent);
    color: var(--color-ink-primary, #34251c);
  }
  .turn-sound-opt--on {
    color: var(--color-ink-primary, #34251c);
  }

  .turn-sound-tick {
    flex: none;
    width: 5px;
    height: 5px;
    border-radius: 999px;
    background: transparent;
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-ink-primary, #34251c) 35%, transparent);
    transition: background var(--duration-fast, 160ms) ease;
  }
  .turn-sound-opt--on .turn-sound-tick {
    background: var(--color-ember, #c65f3c);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-ember, #c65f3c) 60%, transparent);
  }

  .turn-sound-opt:focus-visible {
    outline: 1px solid color-mix(in srgb, var(--color-ember, #c65f3c) 70%, transparent);
    outline-offset: 1px;
  }
</style>
