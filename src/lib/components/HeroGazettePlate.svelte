<script lang="ts">
  /**
   * A square plate beside the workshop lockets. One slip at a time; the
   * type steps up and the next line rises. Each slip opens its own leaf,
   * cutting, or figurine. The newest work is always among the slips, and
   * stands in alone when the blotter is empty.
   */
  import { t, lang } from '$lib/i18n';
  import { browser } from '$app/environment';
  import type { FigurineListItem, GazetteCutting, GazetteLeaf } from '$lib/types/api';
  import { plateSlips } from '$lib/gazette';
  import AppImage from '$lib/components/AppImage.svelte';
  import GazetteMark from '$lib/components/GazetteMark.svelte';

  const INTERVAL_MS = 5200;
  const FADE_MS = 620;

  let {
    leaves = [],
    cuttings = [],
    latestWork = null,
    variant = 'plate',
  }: {
    leaves?: GazetteLeaf[];
    cuttings?: GazetteCutting[];
    latestWork?: FigurineListItem | null;
    variant?: 'plate' | 'door';
  } = $props();

  let slips = $derived(plateSlips(leaves, cuttings, $lang, latestWork));
  let tick = $state(0);
  let paused = $state(false);
  let leaving = $state<number | null>(null);
  let shown = $derived(slips.length ? tick % slips.length : 0);
  let current = $derived(slips[shown] ?? null);
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
    if (!browser || count <= 1) return;
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;

    const id = window.setInterval(() => {
      if (paused || document.hidden) return;
      advance();
    }, INTERVAL_MS);

    return () => clearInterval(id);
  });
</script>

{#if current}
  <div
    class="gz-plate"
    class:gz-door={variant === 'door'}
    class:paused
    class:cycling
    onmouseenter={() => (paused = true)}
    onmouseleave={() => (paused = false)}
  >
    <a
      class="gz-plate-hit"
      href={current.href}
      target={current.external ? '_blank' : undefined}
      rel={current.external ? 'noopener noreferrer' : undefined}
      aria-label={$t('homeGazettePlateAria').replace('{title}', current.title)}
      onfocus={() => (paused = true)}
      onblur={() => (paused = false)}
    >
      {#if variant === 'door'}
        <span class="gz-door-face" aria-hidden="true">
          {#if current.imageUrl}
            <AppImage src={current.imageUrl} alt="" class="gz-work-img" sizes="64px" />
          {:else if current.markKey || current.markUrl || current.letter}
            <GazetteMark
              markKey={current.markKey}
              markUrl={current.markUrl}
              letter={current.letter}
              size={28}
            />
          {/if}
        </span>
        <svg class="gz-door-go" width="22" height="12" viewBox="0 0 24 14" fill="none" aria-hidden="true"><path d="M1 7h21m-5-5.5L22.5 7 17 12.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/></svg>
        <span class="gz-door-copy">
          <b>{$t('homeGazettePlateLabel')}</b>
          <span>{current.title || $t('homeDoorGazetteHint')}</span>
        </span>
        {#if cycling}
          <span class="gz-progress" aria-hidden="true">
            {#key tick}
              <span class="gz-progress-bar" style="animation-duration:{INTERVAL_MS}ms"></span>
            {/key}
          </span>
        {/if}
      {:else}
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
            class:has-face={Boolean(slip.imageUrl) || slip.kind === 'work'}
            aria-hidden={i !== shown}
          >
            {#if slip.kind === 'work' || slip.imageUrl}
              <span class="gz-work-name">{slip.title}</span>
              {#if slip.dateLabel}
                <span class="gz-work-date">{slip.dateLabel}</span>
              {/if}
              {#if slip.imageUrl}
                <span class="gz-work-face">
                  <AppImage src={slip.imageUrl} alt="" class="gz-work-img" sizes="72px" />
                </span>
              {/if}
            {:else}
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
            {/if}
          </span>
        {/each}
      </span>
      {/if}
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

  .gz-plate-slip.has-face {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  .gz-work-name {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    flex-shrink: 0;
    font-family: Georgia, 'Times New Roman', serif;
    font-size: 12px;
    font-weight: 400;
    line-height: 1.25;
    letter-spacing: 0.01em;
  }

  .gz-work-date {
    flex-shrink: 0;
    margin-top: 3px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 8px;
    font-weight: 500;
    letter-spacing: 0.04em;
    line-height: 1.2;
    color: rgba(52, 37, 28, 0.48);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .gz-work-face {
    position: relative;
    flex: 1 1 0;
    min-height: 0;
    margin-top: 6px;
    overflow: hidden;
    border: 1px solid rgba(52, 37, 28, 0.14);
    background: #2a1a12;
  }

  .gz-work-face :global(.gz-work-img) {
    display: block;
    width: 100%;
    height: 100%;
  }

  .gz-work-face :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center top;
    display: block;
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

  /* Плитка входа: тот же фон и скругление, что у соседних входов первого
     экрана. Сверху лицо и стрелка, ниже название и строка текущего листа.
     Ручная кнопка «следующий» в плитке снята: лист и так сменяется сам, а
     полоска внизу показывает, когда. */
  .gz-plate.gz-door {
    width: 100%;
    height: auto;
    min-height: 0;
    margin: 0;
    overflow: hidden;
    flex-direction: column;
    align-items: stretch;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 6%, transparent);
    border-radius: 14px;
    background: color-mix(in srgb, var(--color-canvas-sunken, #f0e9da) 72%, transparent);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.55);
  }

  .gz-plate.gz-door::after {
    display: none;
  }

  .gz-plate.gz-door:hover {
    transform: none;
    background: color-mix(in srgb, var(--color-canvas-sunken, #f0e9da) 92%, transparent);
    border-color: rgba(198, 95, 60, 0.28);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.55);
  }

  .gz-plate.gz-door .gz-plate-hit {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-areas: 'face go' 'copy copy' 'prog prog';
    grid-template-rows: auto 1fr auto;
    gap: 14px 8px;
    align-items: start;
    flex: 1;
    min-height: 0;
    padding: 14px 14px 16px;
  }

  .gz-door-face {
    grid-area: face;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 64px;
    height: 64px;
    overflow: hidden;
    border-radius: 50%;
    background: #2a1a12;
    box-shadow: 0 6px 16px rgba(52, 37, 28, 0.22);
  }

  .gz-door-face :global(.gz-work-img),
  .gz-door-face :global(img) {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center top;
  }

  .gz-door-go {
    grid-area: go;
    align-self: center;
    color: var(--gz-ember);
    opacity: 0.8;
    transition: transform 0.22s ease, opacity 0.22s ease;
  }

  .gz-plate.gz-door:hover .gz-door-go {
    opacity: 1;
    transform: translateX(3px);
  }

  .gz-door-copy {
    grid-area: copy;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .gz-door-copy b {
    font-family: 'Cormorant Garamond', serif;
    font-size: 21px;
    font-weight: 500;
    line-height: 1.12;
    letter-spacing: 0;
    color: var(--gz-ink);
    transition: color 0.18s ease;
  }

  .gz-door-copy span {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 12px;
    font-weight: 400;
    letter-spacing: 0.01em;
    line-height: 1.35;
    color: rgba(52, 37, 28, 0.6);
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    overflow: hidden;
  }

  .gz-plate.gz-door .gz-progress {
    grid-area: prog;
    margin: 0;
  }

  .gz-plate.gz-door .gz-next {
    display: none;
  }

  .gz-plate.gz-door:hover .gz-door-copy b {
    color: var(--gz-ember);
  }

  @container (max-width: 440px) {
    .gz-plate.gz-door .gz-plate-hit {
      grid-template-columns: auto minmax(0, 1fr) auto;
      grid-template-areas: 'face copy go' '. prog .';
      grid-template-rows: auto auto;
      align-items: center;
      gap: 14px;
      row-gap: 8px;
    }
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
