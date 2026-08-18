<script lang="ts">
  /**
   * A square plate beside the workshop lockets. One slip at a time; the
   * type steps up and the next line rises. The plate opens /gazette; a
   * small arrow at the foot turns the next leaf.
   */
  import { t, lang } from '$lib/i18n';
  import type { GazetteCutting, GazetteLeaf } from '$lib/types/api';
  import { plateSlips } from '$lib/gazette';
  import GazetteMark from '$lib/components/GazetteMark.svelte';

  const INTERVAL_MS = 5200;
  const FADE_MS = 620;

  let {
    leaves = [],
    cuttings = [],
  }: {
    leaves?: GazetteLeaf[];
    cuttings?: GazetteCutting[];
  } = $props();

  let slips = $derived(plateSlips(leaves, cuttings, $lang));
  let tick = $state(0);
  let paused = $state(false);
  let leaving = $state<number | null>(null);
  let shown = $derived(slips.length ? tick % slips.length : 0);
  let cycling = $derived(slips.length > 1);
  let fadeTimer: ReturnType<typeof setTimeout> | undefined;

  function advance() {
    const count = slips.length;
    if (count <= 1) return;
    leaving = tick % count;
    tick += 1;
    clearTimeout(fadeTimer);
    fadeTimer = setTimeout(() => {
      leaving = null;
    }, FADE_MS);
  }

  $effect(() => {
    const count = slips.length;
    if (count <= 1) return;
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;

    const id = window.setInterval(() => {
      if (paused || document.hidden) return;
      advance();
    }, INTERVAL_MS);

    return () => clearInterval(id);
  });
</script>

{#if slips.length > 0}
  <div
    class="gz-plate"
    class:paused
    class:cycling
    onmouseenter={() => (paused = true)}
    onmouseleave={() => (paused = false)}
  >
    <a
      class="gz-plate-hit"
      href="/gazette?src=home_plate"
      aria-label={$t('homeGazettePlateAria')}
      onfocus={() => (paused = true)}
      onblur={() => (paused = false)}
    >
      <span class="gz-plate-top">
        <span class="gz-plate-kicker">{$t('homeGazettePlateLabel')}</span>
        {#if cycling}
          <span class="gz-plate-idx">{String(shown + 1).padStart(2, '0')}</span>
        {/if}
      </span>
      {#if cycling}
        <span class="gz-progress" aria-hidden="true">
          {#key tick}
            <span class="gz-progress-bar" style="animation-duration:{INTERVAL_MS}ms"></span>
          {/key}
        </span>
      {/if}
      <span class="gz-plate-stage">
        {#each slips as slip, i (slip.id)}
          <span
            class="gz-plate-slip"
            class:on={i === shown}
            class:out={i === leaving}
            aria-hidden={i !== shown}
          >
            {#if slip.markKey || slip.markUrl}
              <span class="gz-plate-mark">
                <GazetteMark
                  markKey={slip.markKey}
                  markUrl={slip.markUrl}
                  letter={slip.letter}
                  size={22}
                />
              </span>
            {/if}{slip.title}
          </span>
        {/each}
      </span>
    </a>
    {#if cycling}
      <button
        type="button"
        class="gz-next"
        aria-label={$t('homeGazettePlateNext')}
        onclick={(e) => {
          e.stopPropagation();
          advance();
        }}
      >
        <svg width="12" height="7" viewBox="0 0 12 7" fill="none" aria-hidden="true">
          <path d="M1 1.2L6 5.8L11 1.2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
    {/if}
  </div>
{/if}

<style>
  .gz-plate {
    --gz-ink: #34251c;
    --gz-ember: #c65f3c;
    position: relative;
    display: flex;
    flex-direction: column;
    width: 142px;
    height: 142px;
    flex-shrink: 0;
    margin: 5px 1px 5px 2px;
    overflow: hidden;
    color: var(--gz-ink);
    border-radius: 6px;
    background:
      linear-gradient(165deg, #fbf4e8 0%, #f3e6d0 72%, #ead9bc 100%);
    border: 1px solid rgba(52, 37, 28, 0.14);
    border-left: 2px solid var(--gz-ember);
    box-shadow:
      inset 0 1px 0 rgba(255, 247, 234, 0.72),
      0 12px 28px -18px rgba(52, 37, 28, 0.48);
    transition: border-color 0.22s ease, box-shadow 0.22s ease, transform 0.22s ease;
  }

  .gz-plate::after {
    content: '';
    position: absolute;
    inset: -5px;
    border-radius: 9px;
    border: 1px solid rgba(198, 95, 60, 0.28);
    pointer-events: none;
  }

  .gz-plate:hover {
    border-color: rgba(198, 95, 60, 0.45);
    box-shadow:
      inset 0 1px 0 rgba(255, 247, 234, 0.8),
      0 16px 32px -18px rgba(52, 37, 28, 0.52);
    transform: translateY(-1px);
  }

  .gz-plate-hit {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    padding: 8px 8px 18px 10px;
    text-decoration: none;
    color: inherit;
  }
  .gz-plate-hit:focus-visible {
    outline: none;
  }

  .gz-plate-top {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    flex-shrink: 0;
    margin-bottom: 4px;
  }

  .gz-plate-kicker {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 8px;
    font-weight: 600;
    letter-spacing: 0.12em;
    line-height: 1;
    text-transform: uppercase;
    color: var(--gz-ember);
  }

  .gz-plate-idx {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 9px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.06em;
    line-height: 1;
    color: rgba(52, 37, 28, 0.42);
  }

  .gz-progress {
    display: block;
    height: 1px;
    margin: 0 0 5px;
    background: rgba(198, 95, 60, 0.16);
    overflow: hidden;
    flex-shrink: 0;
  }

  .gz-progress-bar {
    display: block;
    height: 100%;
    width: 0;
    background: var(--gz-ember);
    animation-name: gz-fill;
    animation-timing-function: linear;
    animation-fill-mode: forwards;
  }

  .gz-plate.paused .gz-progress-bar {
    animation-play-state: paused;
  }

  @keyframes gz-fill {
    to { width: 100%; }
  }

  .gz-plate-stage {
    position: relative;
    flex: 1;
    min-height: 0;
  }

  .gz-plate-slip {
    position: absolute;
    inset: 0;
    margin: 0;
    overflow: hidden;
    font-family: Georgia, 'Times New Roman', serif;
    font-size: 12.5px;
    font-weight: 400;
    line-height: 1.32;
    letter-spacing: 0.01em;
    color: var(--gz-ink);
    opacity: 0;
    transform: translateY(11px);
    transition:
      opacity 0.55s ease,
      transform 0.55s cubic-bezier(0.22, 1, 0.36, 1);
    pointer-events: none;
  }

  .gz-plate-mark {
    float: left;
    margin: 1px 6px 3px 0;
    line-height: 0;
  }

  .gz-plate-slip.on {
    opacity: 1;
    transform: translateY(0);
    z-index: 1;
  }

  .gz-plate-slip.out {
    opacity: 0;
    transform: translateY(-9px);
    z-index: 0;
  }

  .gz-next {
    position: absolute;
    right: 5px;
    bottom: 3px;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 18px;
    padding: 0;
    border: none;
    background: none;
    color: #6f3b24;
    cursor: pointer;
    opacity: 0.72;
    transition: color 0.18s ease, opacity 0.18s ease, transform 0.18s ease;
  }
  .gz-next:hover,
  .gz-next:focus-visible {
    color: var(--gz-ember);
    opacity: 1;
    outline: none;
  }
  .gz-next:active {
    transform: translateY(1px);
  }

  @media (prefers-reduced-motion: reduce) {
    .gz-plate,
    .gz-plate-slip,
    .gz-next {
      transition: none;
    }
    .gz-plate:hover {
      transform: none;
    }
    .gz-progress-bar {
      animation: none;
      width: 100%;
    }
  }
</style>
