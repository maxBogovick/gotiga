<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { birdWalking, ravenCircleEl } from '$lib/stores/bird-walk';
  import { afterLoadIdle } from '$lib/after-load-idle';

  const FRAMES = 8;
  const frames = Array.from({ length: FRAMES }, (_, i) => `/images/bird-see/frame-${i + 1}.jpg`);

  let circleEl = $state<HTMLElement | null>(null);
  let framesReady = $state(false);
  let stopFrames: (() => void) | null = null;
  let scrollY = $state(0);
  let maxScroll = $state(1);

  let frameIndex = $derived(
    Math.min(FRAMES - 1, Math.floor((scrollY / maxScroll) * FRAMES))
  );

  $effect(() => { ravenCircleEl.set(circleEl); });

  function onScroll() {
    scrollY = window.scrollY;
    maxScroll = Math.max(1, document.documentElement.scrollHeight - window.innerHeight);
  }

  onMount(() => {
    maxScroll = Math.max(1, document.documentElement.scrollHeight - window.innerHeight);
    window.addEventListener('scroll', onScroll, { passive: true });
    stopFrames = afterLoadIdle(() => { framesReady = true; });
  });

  onDestroy(() => {
    stopFrames?.();
    if (typeof window !== 'undefined') window.removeEventListener('scroll', onScroll);
    ravenCircleEl.set(null);
  });
</script>

<div class="raven-circle" class:is-away={$birdWalking} bind:this={circleEl} aria-hidden="true">
  <!-- Окно рамки: подложка и кадры ворона обрезаны по её проёму. -->
  <div class="raven-window">
    {#if framesReady}
      {#each frames as src, i}
        <img
          {src}
          alt=""
          class="bird-frame"
          class:visible={i === frameIndex}
          draggable="false"
          decoding="async"
        />
      {/each}
    {/if}
  </div>
  <img class="raven-frame" src="/images/raven-frame.webp" alt="" draggable="false" decoding="async" />
</div>

<style>
  /* Рамка из ветвей — картинка 520 × 480, проём в ней 219 × 262 с отступом
     (156, 113). Размер задаёт высота проёма (`--raven-h`), остальное — доли
     от него: рамка выступает за коробку, а коробка остаётся проёмом, по
     которому ходит ворон (`HeaderBirdWalk` берёт её центр). */
  .raven-circle {
    --raven-h: 26px;
    position: relative;
    height: var(--raven-h);
    aspect-ratio: 219 / 262;
    flex-shrink: 0;
    margin-inline: calc(var(--raven-h) * 0.4);
    view-transition-name: none;
    transition: transform 0.4s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .raven-window {
    position: absolute;
    inset: 0;
    background: var(--bird-circle-color, #D4860A);
    -webkit-mask: url('/images/raven-frame-fill.webp') calc(var(--raven-h) * -0.5954) calc(var(--raven-h) * -0.4313) / calc(var(--raven-h) * 1.9847) calc(var(--raven-h) * 1.8321) no-repeat;
    mask: url('/images/raven-frame-fill.webp') calc(var(--raven-h) * -0.5954) calc(var(--raven-h) * -0.4313) / calc(var(--raven-h) * 1.9847) calc(var(--raven-h) * 1.8321) no-repeat;
    overflow: hidden;
  }

  .raven-frame {
    position: absolute;
    left: -71.2%;
    top: -43.1%;
    width: 237.4%;
    height: 183.2%;
    max-width: none;
    pointer-events: none;
    user-select: none;
    -webkit-user-drag: none;
    filter: drop-shadow(0 1px 2px rgba(52, 37, 28, 0.25));
  }

  .raven-circle.is-away .raven-window :global(.bird-frame) {
    opacity: 0;
  }

  :global(.brand:hover) .raven-circle:not(.is-away) {
    transform: rotate(-4deg) scale(1.05);
  }

  .bird-frame {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 60%;
    mix-blend-mode: multiply;
    opacity: 0;
    user-select: none;
    -webkit-user-drag: none;
  }

  .bird-frame.visible { opacity: 1; }
</style>
