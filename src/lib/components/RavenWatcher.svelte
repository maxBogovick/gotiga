<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { birdWalking, ravenCircleEl } from '$lib/stores/bird-walk';

  const FRAMES = 8;
  const frames = Array.from({ length: FRAMES }, (_, i) => `/images/bird-see/frame-${i + 1}.jpg`);

  let circleEl = $state<HTMLElement | null>(null);
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
  });

  onDestroy(() => {
    if (typeof window !== 'undefined') window.removeEventListener('scroll', onScroll);
    ravenCircleEl.set(null);
  });
</script>

<div class="raven-circle" class:is-away={$birdWalking} bind:this={circleEl} aria-hidden="true">
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
</div>

<style>
  .raven-circle {
    position: relative;
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    border-radius: 50%;
    background: var(--bird-circle-color, #D4860A);
    view-transition-name: none;
    box-shadow:
      0 0 0 1px color-mix(in srgb, var(--bird-circle-color, #D4860A) 60%, #000),
      0 1px 4px rgba(52, 37, 28, 0.22);
    overflow: hidden;
    transition:
      opacity 0.3s ease,
      transform 0.4s cubic-bezier(0.16, 1, 0.3, 1),
      box-shadow 0.4s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .raven-circle.is-away {
    opacity: 0;
    pointer-events: none;
  }

  :global(.brand:hover) .raven-circle:not(.is-away) {
    transform: rotate(-4deg) scale(1.05);
    box-shadow:
      0 0 0 1px color-mix(in srgb, var(--bird-circle-color, #D4860A) 80%, #000),
      0 2px 8px rgba(52, 37, 28, 0.28);
  }

  .bird-frame {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
    object-position: center 60%;
    mix-blend-mode: multiply;
    opacity: 0;
    user-select: none;
    -webkit-user-drag: none;
  }

  .bird-frame.visible { opacity: 1; }
</style>
