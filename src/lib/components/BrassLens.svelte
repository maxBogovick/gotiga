<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';
  import { resolveWebpUrl, resolveSrcset } from '$lib/api';

  let {
    src,
    thumbSrc = null,
    sizes = '(min-width: 1280px) 860px, 96vw',
    alt,
    class: className = '',
    imageFit = 'cover',
    objectPosition = 'center 20%',
    lensEnabled = false,
    onOpenLightbox = () => {},
    onSwipeLeft = () => {},
    onSwipeRight = () => {},
  }: {
    src?: string | null;
    /** Small (~420px) variant already generated server-side — doubles as the
     * low-res width-descriptor candidate and the blur-up placeholder while
     * the full image loads. */
    thumbSrc?: string | null;
    sizes?: string;
    alt?: string;
    class?: string;
    imageFit?: 'cover' | 'contain';
    objectPosition?: string;
    lensEnabled?: boolean;
    onOpenLightbox?: () => void;
    onSwipeLeft?: () => void;
    onSwipeRight?: () => void;
  } = $props();

  let container: HTMLDivElement;
  let mainImg = $state<HTMLImageElement | null>(null);
  let isPointerFine = $state(true);
  let imageFailed = $state(false);
  let mainLoaded = $state(false);

  $effect(() => {
    void src;
    imageFailed = false;
    mainLoaded = Boolean(mainImg?.complete && (mainImg?.naturalWidth ?? 0) > 0);
  });

  onMount(() => {
    isPointerFine = window.matchMedia('(pointer: fine)').matches;
    if (mainImg?.complete && mainImg.naturalWidth > 0) mainLoaded = true;
  });

  // ── Desktop: brass magnifying lens ───────────────────────────────────────
  let showLens = $state(false);
  let lx = $state(0), ly = $state(0);
  let cw = $state(0), ch = $state(0);
  const lensSize = 220, zoom = 2.4;

  function handleMouseMove(e: MouseEvent) {
    if (!container || !isPointerFine || !lensEnabled) return;
    const r = container.getBoundingClientRect();
    lx = e.clientX - r.left; ly = e.clientY - r.top;
    cw = r.width; ch = r.height;
  }

  $effect(() => {
    if (!lensEnabled) showLens = false;
  });

  // ── Mobile: pinch-to-zoom + pan + double-tap ──────────────────────────────
  let scale       = $state(1);
  let panX        = $state(0);
  let panY        = $state(0);
  let isPanning    = false;
  let pinchStartD  = 0;
  let pinchStartS  = 1;
  let panStartX    = 0, panStartY = 0;
  let panOriginX   = 0, panOriginY = 0;
  let swipeStartX  = 0, swipeStartY = 0;
  let lastTap      = 0;
  let tapTimer:    ReturnType<typeof setTimeout>;
  let transitioning = $state(false);

  onDestroy(() => { clearTimeout(tapTimer); });

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

  // Touch: the brass overlay is desktop-only, so the lens button zooms the plate.
  $effect(() => {
    const enabled = lensEnabled;
    const fine = isPointerFine;
    if (fine) return;
    if (enabled) {
      transitioning = true;
      scale = 2.5;
      panX = 0;
      panY = 0;
      const timer = setTimeout(() => { transitioning = false; }, 300);
      return () => clearTimeout(timer);
    }
    resetZoom(true);
  });

  function handleTouchStart(e: TouchEvent) {
    if (isPointerFine) return;

    if (e.touches.length === 2) {
      e.preventDefault();
      pinchStartD = dist(e.touches);
      pinchStartS = scale;
      isPanning = false;
    } else if (e.touches.length === 1) {
      swipeStartX = e.touches[0].clientX;
      swipeStartY = e.touches[0].clientY;

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
      if (scale < 1.1) resetZoom(true);

      // Detect horizontal swipe for gallery navigation (when not zoomed in).
      if (scale <= 1.05 && e.changedTouches.length > 0) {
        const dx = e.changedTouches[0].clientX - swipeStartX;
        const dy = e.changedTouches[0].clientY - swipeStartY;
        const absX = Math.abs(dx);
        const absY = Math.abs(dy);
        // Require at least 48px horizontal movement and mostly horizontal direction.
        if (absX >= 48 && absX > absY * 1.4) {
          clearTimeout(tapTimer);
          if (dx < 0) onSwipeLeft();
          else onSwipeRight();
          return;
        }
      }
    }
  }
</script>

<div
  bind:this={container}
  class="relative w-full h-full overflow-hidden {isPointerFine && lensEnabled ? 'cursor-none' : ''} {className}"
  onmousemove={handleMouseMove}
  onmouseenter={() => { if (isPointerFine && lensEnabled) showLens = true; }}
  onmouseleave={() => showLens = false}
  ontouchstart={handleTouchStart}
  ontouchmove={handleTouchMove}
  ontouchend={handleTouchEnd}
  role="img"
  aria-label={alt}
>
  <!-- Blur-up placeholder: the already-generated ~420px thumb, blurred and
       slightly oversized to hide its own soft edge, visible only until the
       full image has decoded. Reuses an asset that exists for every image
       already, so this costs no extra request. -->
  {#if thumbSrc}
    <img
      src={thumbSrc}
      alt=""
      aria-hidden="true"
      class="absolute inset-0 w-full h-full pointer-events-none select-none {imageFit === 'contain' ? 'object-contain' : 'object-cover'}"
      style="
        object-position: {objectPosition};
        filter: {mainLoaded && !imageFailed ? 'none' : 'blur(18px) saturate(1.05)'};
        transform: {mainLoaded && !imageFailed ? 'none' : 'scale(1.08)'};
        opacity: {mainLoaded && !imageFailed ? 0 : 1};
        transition: opacity 0.4s ease;
      "
    />
  {/if}

  <!-- Main image. The responsive candidate set is thumb(420)/medium(900)/preview(1800)
       as JPEG + WebP (resolveSrcset). Including the 900px medium is load-bearing: with
       only 420 and 1800 to choose from, a phone (needs ~750-1100 physical px) rejected
       the thumb and pulled the full 1800px preview — ~470 KB to paint a ~390 px plate.
       Falls back to the plain `src` for non-figurine images that have no siblings. -->
  {#if src && !imageFailed}
    {@const responsive = resolveSrcset(src)}
    <picture style="display: contents;">
      {#if responsive?.webp}
        <source type="image/webp" srcset={responsive.webp} {sizes} />
      {:else if resolveWebpUrl(src)}
        <source type="image/webp" srcset={resolveWebpUrl(src)} />
      {/if}
      <img
        {src}
        srcset={responsive?.jpeg ?? undefined}
        sizes={responsive ? sizes : undefined}
        {alt}
        bind:this={mainImg}
        class="absolute inset-0 w-full h-full pointer-events-none select-none {imageFit === 'contain' ? 'object-contain' : 'object-cover'}"
        style="
          object-position: {objectPosition};
          transform: scale({scale}) translate({panX / scale}px, {panY / scale}px);
        opacity: {mainLoaded || imageFailed || !thumbSrc ? 1 : 0};
        transition: {transitioning ? 'transform 0.28s cubic-bezier(0.22,0.1,0.2,1)' : 'none'}, opacity 0.35s ease;
          touch-action: none;
          will-change: transform;
        "
        decoding="async"
        onload={() => (mainLoaded = true)}
        onerror={() => (imageFailed = true)}
      />
    </picture>
  {:else}
    <div class="lens-fallback" aria-hidden="true"></div>
  {/if}

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
  {#if showLens && isPointerFine && src && !imageFailed}
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
        <img {src} {alt} class="w-full h-full pointer-events-none {imageFit === 'contain' ? 'object-contain' : 'object-cover'}"
          style="object-position: {objectPosition}" />
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

  .lens-fallback {
    width: 100%;
    height: 100%;
    background:
      radial-gradient(circle at 50% 28%, rgba(255, 255, 255, 0.5), transparent 48%),
      rgba(244, 236, 222, 0.75);
  }
</style>
