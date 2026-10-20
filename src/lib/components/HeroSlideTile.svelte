<script module lang="ts">
  export interface TileSlide {
    id: string;
    href: string;
    title: string;
    imageUrl?: string | null;
    external?: boolean;
    markKey?: string;
    markUrl?: string | null;
    letter?: string;
  }
</script>

<script lang="ts">
  /**
   * Квадратная плитка входа с листаемым фоном: на весь квадрат — фото
   * текущей новости или байки, текст прямо на фото поверх затемнения к низу.
   * Таблички нет: она закрывала лицо фигурки. Раз в несколько секунд сменяются и фото, и текст; наведение и
   * фокус ставят смену на паузу. Плитка одна на вестник и на байки: два
   * отдельных отрисовщика одной и той же вещи разошлись бы на первой правке.
   */
  import { fade } from 'svelte/transition';
  import { browser } from '$app/environment';
  import { t } from '$lib/i18n';
  import AppImage from '$lib/components/AppImage.svelte';
  import GazetteMark from '$lib/components/GazetteMark.svelte';

  const INTERVAL_MS = 6000;
  const FADE_MS = 800;

  let {
    slides = [],
    kicker,
    startDelayMs = 0,
  }: {
    slides?: TileSlide[];
    kicker: string;
    /** Сдвиг первой смены: две плитки рядом не должны листать в один миг. */
    startDelayMs?: number;
  } = $props();

  let tick = $state(0);
  let paused = $state(false);
  let reduced = $state(false);
  let current = $derived(slides.length ? slides[tick % slides.length] : null);
  let cycling = $derived(slides.length > 1);
  let dur = $derived(reduced ? 0 : FADE_MS);

  $effect(() => {
    if (!browser) return;
    reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  });

  $effect(() => {
    if (!browser || !cycling || reduced) return;
    let interval: ReturnType<typeof setInterval> | undefined;
    const advance = () => {
      if (paused || document.hidden) return;
      tick += 1;
    };
    // Первая смена приходит на startDelayMs позже, дальше — обычным шагом.
    const first = setTimeout(() => {
      advance();
      interval = setInterval(advance, INTERVAL_MS);
    }, startDelayMs + INTERVAL_MS);
    return () => {
      clearTimeout(first);
      clearInterval(interval);
    };
  });
</script>

{#if current}
  <div
    class="tile"
    class:paused
    role="group"
    onmouseenter={() => (paused = true)}
    onmouseleave={() => (paused = false)}
  >
    <a
      class="hit"
      href={current.href}
      target={current.external ? '_blank' : undefined}
      rel={current.external ? 'noopener noreferrer' : undefined}
      aria-label={$t('homeGazettePlateAria').replace('{title}', current.title)}
      onfocus={() => (paused = true)}
      onblur={() => (paused = false)}
    >
      <span class="bg" aria-hidden="true">
        {#key current.id}
          <span class="slide" in:fade={{ duration: dur }} out:fade={{ duration: 0, delay: dur }}>
            {#if current.imageUrl}
              <AppImage
                src={current.imageUrl}
                alt=""
                class="tile-img"
                loading="eager"
                sizes="(max-width: 440px) 100vw, 260px"
              />
            {:else}
              <span class="blank">
                {#if current.markKey || current.markUrl || current.letter}
                  <GazetteMark
                    markKey={current.markKey}
                    markUrl={current.markUrl}
                    letter={current.letter}
                    size={40}
                  />
                {:else}
                  <span class="glyph">✦</span>
                {/if}
              </span>
            {/if}
          </span>
        {/key}
      </span>

      <span class="panel">
        <span class="copy">
          <span class="kicker">{kicker}</span>
          {#key current.id}
            <b in:fade={{ duration: dur }}>{current.title}</b>
          {/key}
        </span>
        <svg class="go" width="22" height="12" viewBox="0 0 24 14" fill="none" aria-hidden="true">
          <path d="M1 7h21m-5-5.5L22.5 7 17 12.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        {#if cycling}
          <span class="progress" aria-hidden="true">
            {#key tick}
              <span class="bar" style="animation-duration:{INTERVAL_MS}ms"></span>
            {/key}
          </span>
        {/if}
      </span>
    </a>
  </div>
{/if}

<style>
  .tile {
    position: relative;
    width: 100%;
    min-width: 0;
    aspect-ratio: 1 / 1;
    overflow: hidden;
    border-radius: 14px;
    background: #2a1a12;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 10%, transparent);
    box-shadow: 0 10px 24px -16px rgba(52, 37, 28, 0.5);
    transition: box-shadow 0.22s ease, transform 0.18s ease;
  }
  .tile:hover {
    box-shadow: 0 14px 28px -16px rgba(52, 37, 28, 0.6);
  }
  .tile:active {
    transform: translateY(1px);
  }

  .hit {
    position: absolute;
    inset: 0;
    display: block;
    color: inherit;
    text-decoration: none;
  }
  .hit:focus-visible {
    outline: 2px solid rgba(198, 95, 60, 0.7);
    outline-offset: -3px;
    border-radius: 14px;
  }

  .bg,
  .slide {
    position: absolute;
    inset: 0;
    display: block;
  }
  .slide :global(.app-image-wrap),
  .slide :global(.tile-img),
  .slide :global(img) {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 30%;
  }
  /* Затемнение к низу: на нём читается текст и на светлом кадре. Верх и
     середина кадра чистые, там лицо. */
  .bg::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      to top,
      rgba(24, 13, 8, 0.86) 0%,
      rgba(24, 13, 8, 0.5) 32%,
      rgba(24, 13, 8, 0) 58%
    );
    pointer-events: none;
  }
  .blank {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: radial-gradient(circle at 50% 38%, #4a2f20, #231510 72%);
  }
  .glyph {
    font-size: 30px;
    color: #c65f3c;
    opacity: 0.7;
  }

  .panel {
    position: absolute;
    left: 14px;
    right: 14px;
    bottom: 12px;
    z-index: 2;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: end;
    gap: 8px;
  }
  .copy {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .kicker {
    font-family: Inter, system-ui, sans-serif;
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #f0b896;
  }
  b {
    font-family: 'Cormorant Garamond', serif;
    font-size: 20px;
    font-weight: 500;
    line-height: 1.12;
    color: #fbf3e4;
    text-shadow: 0 1px 8px rgba(20, 10, 5, 0.45);
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    overflow: hidden;
    transition: color 0.18s ease;
  }
  .tile:hover b {
    color: #fff;
  }
  .go {
    align-self: end;
    margin-bottom: 3px;
    color: #f0b896;
    opacity: 0.85;
    transition: transform 0.22s ease, opacity 0.22s ease;
  }
  .tile:hover .go {
    opacity: 1;
    transform: translateX(3px);
  }

  .progress {
    grid-column: 1 / -1;
    display: block;
    height: 1px;
    margin-top: 2px;
    background: rgba(251, 243, 228, 0.22);
    overflow: hidden;
  }
  .bar {
    display: block;
    height: 100%;
    width: 0;
    background: #f0b896;
    animation-name: tile-fill;
    animation-timing-function: linear;
    animation-fill-mode: forwards;
  }
  .tile.paused .bar {
    animation-play-state: paused;
  }
  @keyframes tile-fill {
    to { width: 100%; }
  }

  /* Узкая колонка: квадрат на всю ширину телефона был бы выше экрана. */
  @container (max-width: 440px) {
    .tile {
      aspect-ratio: 16 / 10;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tile,
    .go {
      transition: none;
    }
    .bar {
      animation: none;
      width: 100%;
    }
  }
</style>
