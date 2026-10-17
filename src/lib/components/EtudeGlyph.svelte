<svelte:options namespace="svg" />

<script lang="ts">
  // Знак рода этюда: башня (расстановка), лагерь (своя колода), город (штурм).
  //
  // Один рисунок на три места — знак на карте, легенда карты и дорога на
  // телефоне (`EtudeRoad`). Три копии одних путей однажды разошлись бы, и
  // легенда показывала бы не тот знак, что стоит на карте.
  //
  // Рисуется в начале координат: основание знака — на нуле, рисунок уходит
  // вверх до −58. Обёртку (`<svg>` или `<g>` со сдвигом) даёт вызывающий.
  import type { EtudeKind } from './EtudeMap.svelte';

  let { kind, flag = true }: { kind: EtudeKind; flag?: boolean } = $props();
</script>

{#if kind === 'city'}
  <path class="g-fill" d="M-20 0V-18h40V0ZM-24 0V-30h10V0ZM14 0V-30h10V0ZM-7 -18V-36h14V-18Z" />
  <path class="g-shade" d="M8 0V-18h12V0ZM20 0V-30h4V0Z" />
  <path
    class="g-ink"
    d="M-20 0V-18h40V0M-24 0V-30h10V0M14 0V-30h10V0M-26 -30v-5h4v3h3v-3h4v5M12 -30v-5h4v3h3v-3h4v5M-7 -18V-36h14V-18M-9 -36l9 -12l9 12M-4 0v-7q4 -6 8 0v7M-30 0H30"
  />
  {#if flag}<path class="g-flag" d="M0 -48v-10l11 3l-11 4" />{/if}
{:else if kind === 'meeting'}
  <path class="g-fill" d="M-17 0L0 -24L17 0Z" />
  <path class="g-shade" d="M0 -24L17 0H4Z" />
  <path class="g-ink" d="M-17 0L0 -24L17 0M0 -24L4 0M-4 0q2 -8 4 -10M-22 0H22" />
  <path class="g-ink g-ink--blade" d="M-18 -46L10 -18M18 -46L-10 -18M-15 -26l7 7M15 -26l-7 7M-12 -16l-3 3M12 -16l3 3" />
{:else}
  <path class="g-fill" d="M-8 0V-30h16V0Z" />
  <path class="g-shade" d="M2 0V-30h6V0Z" />
  <path class="g-ink" d="M-8 0V-30h16V0M-11 -30h22l-11 -16zM0 -46v-6M-2 -20h4v-5h-4zM-3 0v-6q3 -4 6 0v6M-14 0H14" />
  {#if flag}<path class="g-flag" d="M0 -52l9 2l-9 3" />{/if}
{/if}

<style>
  .g-fill {
    fill: var(--paper, #f3e7cd);
  }

  .g-shade {
    fill: rgba(111, 59, 36, 0.28);
  }

  .g-ink {
    fill: none;
    stroke: var(--ink, #3d2a1e);
    stroke-width: 1.6;
    stroke-linejoin: round;
    stroke-linecap: round;
  }

  .g-ink--blade {
    stroke-width: 2.1;
  }

  .g-flag {
    fill: #a8442a;
    stroke: var(--ink, #3d2a1e);
    stroke-width: 1;
    stroke-linejoin: round;
    transform-box: fill-box;
    transform-origin: 0 50%;
    animation: wave-flag 2.6s ease-in-out infinite;
  }

  @keyframes wave-flag {
    0%,
    100% {
      transform: skewY(0deg) scaleX(1);
    }
    50% {
      transform: skewY(-9deg) scaleX(0.86);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .g-flag {
      animation: none;
    }
  }
</style>
