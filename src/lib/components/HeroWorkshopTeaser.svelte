<script lang="ts">
  /**
   * A small looping locket in the hero, hinting at the workshop reel further
   * down the page. The poster sits in the prerendered HTML (~5 KB). The video
   * itself is mounted after `load`, so it never shares the Slow 4G pipe with
   * the hero photograph.
   *
   * When nested inside a door row the outer control owns the click, so this
   * renders as a decorative span rather than a nested button.
   */
  import { onMount } from 'svelte';
  import { afterLoadIdle } from '$lib/after-load-idle';

  let {
    webm = '/images/workshop/atelier-reel-tiny.webm',
    mp4 = '/images/workshop/atelier-reel-tiny.mp4',
    poster = '/images/workshop/atelier-reel-tiny-poster.jpg',
    label,
    delayMs = 0,
    size = 'locket',
    interactive = true,
    onSelect,
  }: {
    webm?: string;
    mp4?: string;
    poster?: string;
    label: string;
    delayMs?: number;
    size?: 'locket' | 'door';
    interactive?: boolean;
    onSelect?: (e: MouseEvent) => void;
  } = $props();

  let videoEl = $state<HTMLVideoElement>();
  let armed = $state(false);
  const reducedMotion =
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  onMount(() => {
    const stopArm = afterLoadIdle(() => { armed = true; });
    return () => stopArm();
  });

  $effect(() => {
    if (!armed || reducedMotion || !videoEl) return;
    const el = videoEl;
    const startTimer = setTimeout(() => el.play().catch(() => {}), delayMs);

    function onVisibility() {
      if (document.hidden) el.pause();
      else el.play().catch(() => {});
    }
    document.addEventListener('visibilitychange', onVisibility);

    return () => {
      clearTimeout(startTimer);
      document.removeEventListener('visibilitychange', onVisibility);
    };
  });
</script>

{#snippet media()}
  <span class="hw-ring" aria-hidden="true"></span>
  {#if armed}
    <video
      bind:this={videoEl}
      class="hw-video"
      {poster}
      muted
      loop
      playsinline
      preload="none"
      aria-hidden="true"
    >
      <source src={webm} type="video/webm" />
      <source src={mp4} type="video/mp4" />
    </video>
  {:else}
    <img src={poster} alt="" class="hw-video" decoding="async" fetchpriority="low" />
  {/if}
{/snippet}

{#if interactive}
  <button
    type="button"
    class="hw-teaser"
    class:hw-teaser-door={size === 'door'}
    aria-label={label}
    onclick={(e) => onSelect?.(e)}
  >
    {@render media()}
  </button>
{:else}
  <span class="hw-teaser" class:hw-teaser-door={size === 'door'} aria-hidden="true">
    {@render media()}
  </span>
{/if}

<style>
  .hw-teaser {
    position: relative;
    display: block;
    width: 46px;
    height: 46px;
    flex-shrink: 0;
    padding: 0;
    border: none;
    background: none;
    cursor: pointer;
    border-radius: 50%;
    overflow: hidden;
    box-shadow: 0 6px 16px rgba(52,37,28,0.22);
  }

  span.hw-teaser {
    cursor: inherit;
  }

  .hw-teaser.hw-teaser-door {
    width: 64px;
    height: 64px;
    overflow: visible;
  }

  .hw-teaser-door .hw-video {
    border-radius: 50%;
  }

  .hw-video {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: saturate(0.85) contrast(1.03);
  }

  .hw-ring {
    position: absolute;
    inset: -1px;
    z-index: 1;
    border-radius: 50%;
    border: 1px solid rgba(198,95,60,0.5);
    pointer-events: none;
  }

  .hw-ring::after {
    content: '';
    position: absolute;
    inset: -4px;
    border-radius: 50%;
    border: 1px solid rgba(198,95,60,0.32);
    animation: hw-pulse 3.6s ease-out infinite;
  }

  @keyframes hw-pulse {
    0%   { transform: scale(1);   opacity: 0.55; }
    70%  { transform: scale(1.35); opacity: 0; }
    100% { transform: scale(1.35); opacity: 0; }
  }

  @media (prefers-reduced-motion: reduce) {
    .hw-ring::after {
      animation: none;
      opacity: 0;
    }
  }
</style>
