<script lang="ts">
  /**
   * A single silent loop of the maker's hands at work — plays only while in view,
   * freezes on its poster frame for reduced-motion visitors, never autoplays sound.
   */
  import { onMount } from 'svelte';

  let {
    webm = '/images/workshop/atelier-reel.webm',
    mp4 = '/images/workshop/atelier-reel.mp4',
    poster = '/images/workshop/atelier-reel-poster.jpg',
  }: { webm?: string; mp4?: string; poster?: string } = $props();

  let videoEl = $state<HTMLVideoElement>();
  const reducedMotion =
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  onMount(() => {
    if (reducedMotion || !videoEl) return;
    const el = videoEl;
    const io = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) el.play().catch(() => {});
        else el.pause();
      },
      { threshold: 0.25 },
    );
    io.observe(el);
    return () => io.disconnect();
  });
</script>

<div class="atelier-reel">
  <video
    bind:this={videoEl}
    class="atelier-reel-video"
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
  <div class="atelier-reel-vignette" aria-hidden="true"></div>
</div>

<style>
  .atelier-reel {
    position: absolute;
    inset: 0;
  }

  .atelier-reel-video {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    animation: reel-breathe 14s ease-in-out infinite;
  }

  /* Old-house atmosphere: grain + edge darkening, matching the figurine-detail
     media treatment rather than inventing a new look for this one plate. */
  .atelier-reel-vignette {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(ellipse at 50% 50%, transparent 55%, rgba(40, 20, 8, 0.28) 100%);
    pointer-events: none;
  }

  .atelier-reel-vignette::after {
    content: '';
    position: absolute;
    inset: 0;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='200' height='200'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.68' numOctaves='4' stitchTiles='stitch'/%3E%3CfeColorMatrix type='saturate' values='0'/%3E%3C/filter%3E%3Crect width='200' height='200' filter='url(%23n)' opacity='0.05'/%3E%3C/svg%3E");
    pointer-events: none;
  }

  @keyframes reel-breathe {
    0%, 100% { transform: scale(1); }
    50% { transform: scale(1.025); }
  }

  @media (prefers-reduced-motion: reduce) {
    .atelier-reel-video {
      animation: none;
    }
  }
</style>
