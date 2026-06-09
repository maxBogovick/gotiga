<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';

  let {
    src,
    alt,
    class: className = '',
    onOpenLightbox = () => {},
  }: {
    src?: string | null;
    alt?: string;
    class?: string;
    onOpenLightbox?: () => void;
  } = $props();

  let container: HTMLDivElement;
  let isPointerFine = $state(true);

  onMount(() => {
    isPointerFine = window.matchMedia('(pointer: fine)').matches;
  });

  // ── Desktop: brass magnifying lens ───────────────────────────────────────
  let showLens = $state(false);
  let lx = $state(0), ly = $state(0);
  let cw = $state(0), ch = $state(0);
  const lensSize = 220, zoom = 2.4;

  function handleMouseMove(e: MouseEvent) {
    if (!container || !isPointerFine) return;
    const r = container.getBoundingClientRect();
    lx = e.clientX - r.left; ly = e.clientY - r.top;
    cw = r.width; ch = r.height;
  }

  // ── Mobile: pinch-to-zoom + pan + double-tap ──────────────────────────────
  let scale       = $state(1);
  let panX        = $state(0);
  let panY        = $state(0);
  let isPanning   = false;
  let pinchStartD = 0;
  let pinchStartS = 1;
  let panStartX   = 0, panStartY = 0;
  let panOriginX  = 0, panOriginY = 0;
  let lastTap     = 0;
  let tapTimer:   ReturnType<typeof setTimeout>;
  let transitioning = $state(false);

  function dist(t: TouchList) {
    return Math.hypot(t[1].clientX - t[0].clientX, t[1].clientY - t[0].clientY);
  }

  function clampPan(s: number, px: number, py: number) {
    if (!container) return { px, py };
    const r = container.getBoundingClientRect();
    const maxX = (r.width  * (s - 1)) / 2;
    const maxY = (r.height * (s - 1)) / 2;
    return {
      px: Math.max(-maxX, Math.min(maxX, px)),
      py: Math.max(-maxY, Math.min(maxY, py)),
    };
  }

  function resetZoom(animated = true) {
    transitioning = animated;
    scale = 1; panX = 0; panY = 0;
    if (animated) setTimeout(() => { transitioning = false; }, 300);
  }

  function handleTouchStart(e: TouchEvent) {
    if (isPointerFine) return;

    if (e.touches.length === 2) {
      e.preventDefault();
      pinchStartD = dist(e.touches);
      pinchStartS = scale;
      isPanning = false;
    } else if (e.touches.length === 1) {
      if (scale > 1.05) {
        // Pan mode
        isPanning = true;
        panStartX  = e.touches[0].clientX;
        panStartY  = e.touches[0].clientY;
        panOriginX = panX;
        panOriginY = panY;
      } else {
        // Double-tap detection
        const now = Date.now();
        if (now - lastTap < 280) {
          clearTimeout(tapTimer);
          // Zoom in to tap position
          const r = container.getBoundingClientRect();
          const tx = e.touches[0].clientX - r.left - r.width  / 2;
          const ty = e.touches[0].clientY - r.top  - r.height / 2;
          transitioning = true;
          scale = 2.5;
          const c = clampPan(2.5, -tx * 1.5, -ty * 1.5);
          panX = c.px; panY = c.py;
          setTimeout(() => { transitioning = false; }, 300);
        } else {
          // Single tap — might open lightbox after delay
          tapTimer = setTimeout(() => {
            if (scale <= 1.05) onOpenLightbox();
          }, 240);
        }
        lastTap = now;
      }
    }
  }

  function handleTouchMove(e: TouchEvent) {
    if (isPointerFine) return;

    if (e.touches.length === 2) {
      e.preventDefault();
      const d = dist(e.touches);
      const s = Math.max(1, Math.min(5, pinchStartS * (d / pinchStartD)));
      const c = clampPan(s, panX, panY);
      scale = s; panX = c.px; panY = c.py;
    } else if (e.touches.length === 1 && isPanning && scale > 1.05) {
      e.preventDefault();
      clearTimeout(tapTimer);
      const dx = e.touches[0].clientX - panStartX;
      const dy = e.touches[0].clientY - panStartY;
      const c = clampPan(scale, panOriginX + dx, panOriginY + dy);
      panX = c.px; panY = c.py;
    }
  }

  function handleTouchEnd(e: TouchEvent) {
    if (isPointerFine) return;
    if (e.touches.length === 0) {
      isPanning = false;
      // Snap back if scale < 1.1
      if (scale < 1.1) resetZoom(true);
    }
  }
</script>

<div
  bind:this={container}
  class="relative w-full h-full overflow-hidden {isPointerFine ? 'cursor-none' : ''} {className}"
  onmousemove={handleMouseMove}
  onmouseenter={() => { if (isPointerFine) showLens = true; }}
  onmouseleave={() => showLens = false}
  ontouchstart={handleTouchStart}
  ontouchmove={handleTouchMove}
  ontouchend={handleTouchEnd}
  role="img"
  aria-label={alt}
>
  <!-- Main image -->
  <img
    {src}
    {alt}
    class="w-full h-full object-cover pointer-events-none select-none"
    style="
      object-position: center 20%;
      transform: scale({scale}) translate({panX / scale}px, {panY / scale}px);
      transition: {transitioning ? 'transform 0.28s cubic-bezier(0.22,0.1,0.2,1)' : 'none'};
      touch-action: none;
      will-change: transform;
    "
  />

  <!-- Pinch-zoom reset hint (mobile, when zoomed) -->
  {#if !isPointerFine && scale > 1.2}
    <button
      class="absolute bottom-2 left-2 z-20 text-[10px] font-medium tracking-wide uppercase
             text-[rgba(255,249,240,0.75)] bg-[rgba(44,23,16,0.45)] backdrop-blur-sm
             px-2 py-1 rounded-full border border-[rgba(255,249,240,0.2)]"
      onclick={() => resetZoom()}
      aria-label="Reset zoom"
      transition:fade={{ duration: 150 }}
    >
      ×{scale.toFixed(1)}  Reset
    </button>
  {/if}

  <!-- Desktop brass magnifying lens -->
  {#if showLens && isPointerFine && src}
    <div
      class="absolute z-50 rounded-full overflow-hidden pointer-events-none"
      transition:fade={{ duration: 150 }}
      style="
        width: {lensSize}px; height: {lensSize}px;
        top: {ly - lensSize / 2}px; left: {lx - lensSize / 2}px;
        box-shadow: 0 16px 48px rgba(111,59,36,0.22), inset 0 0 32px rgba(10,5,0,0.85);
        border: 2px solid #b5a642; outline: 4px solid #5c5322; background: #0a0500;
      "
    >
      <div style="
        width: {cw}px; height: {ch}px; transform-origin: 0 0;
        transform: translate({lensSize/2 - lx * zoom}px, {lensSize/2 - ly * zoom}px) scale({zoom});
        overflow: hidden;
      ">
        <img {src} {alt} class="w-full h-full object-cover pointer-events-none"
          style="object-position: center 20%" />
      </div>
      <div class="absolute inset-0 rounded-full pointer-events-none"
        style="box-shadow: inset 3px 0 0 rgba(255,0,0,0.12), inset -3px 0 0 rgba(0,220,255,0.12); mix-blend-mode: color-dodge;"></div>
      <div class="absolute inset-0 rounded-full pointer-events-none"
        style="background: linear-gradient(135deg, rgba(255,255,255,0.18) 0%, transparent 45%, rgba(0,0,0,0.35) 100%); mix-blend-mode: overlay;"></div>
      <div class="absolute pointer-events-none"
        style="top:14px;left:20px;width:40px;height:18px;background:rgba(255,255,255,0.09);border-radius:50%;filter:blur(8px);transform:rotate(-40deg)"></div>
      <div class="absolute inset-0 rounded-full pointer-events-none bg-noise"
        style="opacity:0.5;mix-blend-mode:soft-light"></div>
      <div class="absolute inset-0 rounded-full pointer-events-none"
        style="background:radial-gradient(circle,transparent 48%,rgba(80,40,10,0.28) 100%)"></div>
    </div>
  {/if}
</div>

<style>
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }
</style>
