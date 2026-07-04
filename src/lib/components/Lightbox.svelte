<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '$lib/i18n';
  import { focusTrap } from '$lib/actions/focusTrap';
  import { resolveWebpUrl } from '$lib/api';

  type LightboxImage = { url: string; alt?: string; thumbUrl?: string; focalX?: number | null; focalY?: number | null };

  let {
    images,
    startIndex = 0,
    onClose,
  }: {
    images: LightboxImage[];
    startIndex?: number;
    onClose: () => void;
  } = $props();

  let current = $state(0);
  let isImageLoaded = $state(false);
  let touchStartX = 0;
  let touchStartY = 0;

  // ── Zoom + pan (deep-look mode) ──────────────────────────────────────────
  // Mirrors BrassLens' interaction model so the lightbox — the place a
  // visitor opens specifically to examine paint detail up close — isn't the
  // one surface in the gallery that can't actually zoom.
  let scale = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let transitioning = $state(false);
  let stageEl: HTMLElement;
  let zoomed = $derived(scale > 1.05);

  function resetZoom(animated = true) {
    transitioning = animated;
    scale = 1; panX = 0; panY = 0;
    if (animated) setTimeout(() => { transitioning = false; }, 260);
  }

  function clampPan(s: number, px: number, py: number) {
    if (!stageEl) return { px, py };
    const r = stageEl.getBoundingClientRect();
    const maxX = (r.width * (s - 1)) / 2;
    const maxY = (r.height * (s - 1)) / 2;
    return {
      px: Math.max(-maxX, Math.min(maxX, px)),
      py: Math.max(-maxY, Math.min(maxY, py)),
    };
  }

  $effect(() => { current = startIndex; });
  $effect(() => { current; isImageLoaded = false; resetZoom(false); });

  function prev() { if (zoomed) return; current = (current - 1 + images.length) % images.length; }
  function next() { if (zoomed) return; current = (current + 1) % images.length; }

  function handleKey(e: KeyboardEvent) {
    if (e.key === 'Escape')      { zoomed ? resetZoom() : onClose(); }
    if (e.key === 'ArrowLeft')   prev();
    if (e.key === 'ArrowRight')  next();
  }

  function handleStageClick(e: MouseEvent) {
    if (dragged) { dragged = false; return; }
    if (e.target === e.currentTarget) onClose();
  }

  // ── Desktop: wheel-zoom centred on the cursor + click-drag pan ──────────
  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    if (!stageEl) return;
    const r = stageEl.getBoundingClientRect();
    const cx = e.clientX - r.left - r.width / 2;
    const cy = e.clientY - r.top - r.height / 2;
    const delta = -e.deltaY * 0.0022;
    const nextScale = Math.max(1, Math.min(5, scale * (1 + delta)));
    if (nextScale <= 1.02) { resetZoom(); return; }
    // Keep the point under the cursor stationary while scaling.
    const k = nextScale / scale;
    const c = clampPan(nextScale, (panX - cx) * k + cx, (panY - cy) * k + cy);
    transitioning = false;
    scale = nextScale; panX = c.px; panY = c.py;
  }

  function handleDoubleClick(e: MouseEvent) {
    if (zoomed) { resetZoom(); return; }
    if (!stageEl) return;
    const r = stageEl.getBoundingClientRect();
    const tx = e.clientX - r.left - r.width / 2;
    const ty = e.clientY - r.top - r.height / 2;
    transitioning = true;
    scale = 2.4;
    const c = clampPan(2.4, -tx * 1.4, -ty * 1.4);
    panX = c.px; panY = c.py;
    setTimeout(() => { transitioning = false; }, 260);
  }

  let dragging = false;
  let dragged = false;
  let dragStartX = 0, dragStartY = 0;
  let panOriginX = 0, panOriginY = 0;

  function handleMouseDown(e: MouseEvent) {
    if (!zoomed || e.button !== 0) return;
    dragging = true;
    dragged = false;
    dragStartX = e.clientX; dragStartY = e.clientY;
    panOriginX = panX; panOriginY = panY;
  }

  function handleMouseMove(e: MouseEvent) {
    if (!dragging) return;
    const dx = e.clientX - dragStartX;
    const dy = e.clientY - dragStartY;
    if (Math.abs(dx) > 3 || Math.abs(dy) > 3) dragged = true;
    const c = clampPan(scale, panOriginX + dx, panOriginY + dy);
    panX = c.px; panY = c.py;
  }

  function handleMouseUp() { dragging = false; }

  // ── Mobile: pinch-to-zoom + pan + double-tap ────────────────────────────
  let pinchStartD = 0;
  let pinchStartS = 1;
  let isPanning = false;
  let panStartX = 0, panStartY = 0;
  let lastTap = 0;

  // ── Mobile: swipe-down-to-dismiss ───────────────────────────────────────
  // Once a single-finger drag reveals itself as mostly-vertical (and not a
  // pinch/pan-while-zoomed), the photo follows the finger down and the
  // overlay dims out; release past the threshold closes, otherwise it snaps
  // back. Only armed while at rest (not zoomed) so it never fights the pan
  // gesture used to look around a magnified image.
  let dismissY = $state(0);
  let dismissSnapBack = $state(false);
  let dismissGesture: 'pending' | 'vertical' | 'horizontal' | null = null;
  const DISMISS_THRESHOLD = 110;

  let dismissProgress = $derived(Math.min(1, dismissY / DISMISS_THRESHOLD));

  function touchDist(t: TouchList) {
    return Math.hypot(t[1].clientX - t[0].clientX, t[1].clientY - t[0].clientY);
  }

  function handleTouchStart(e: TouchEvent) {
    if (e.touches.length === 2) {
      e.preventDefault();
      pinchStartD = touchDist(e.touches);
      pinchStartS = scale;
      isPanning = false;
      return;
    }
    touchStartX = e.touches[0].clientX;
    touchStartY = e.touches[0].clientY;
    dismissY = 0;
    dismissGesture = null;

    if (zoomed) {
      isPanning = true;
      panStartX = e.touches[0].clientX;
      panStartY = e.touches[0].clientY;
      panOriginX = panX; panOriginY = panY;
      return;
    }

    dismissGesture = 'pending';

    const now = Date.now();
    if (now - lastTap < 280 && stageEl) {
      const r = stageEl.getBoundingClientRect();
      const tx = e.touches[0].clientX - r.left - r.width / 2;
      const ty = e.touches[0].clientY - r.top - r.height / 2;
      transitioning = true;
      scale = 2.4;
      const c = clampPan(2.4, -tx * 1.4, -ty * 1.4);
      panX = c.px; panY = c.py;
      setTimeout(() => { transitioning = false; }, 260);
    }
    lastTap = now;
  }

  function handleTouchMove(e: TouchEvent) {
    if (e.touches.length === 2) {
      e.preventDefault();
      const d = touchDist(e.touches);
      const s = Math.max(1, Math.min(5, pinchStartS * (d / pinchStartD)));
      const c = clampPan(s, panX, panY);
      scale = s; panX = c.px; panY = c.py;
      return;
    }
    if (e.touches.length === 1 && isPanning && zoomed) {
      e.preventDefault();
      const dx = e.touches[0].clientX - panStartX;
      const dy = e.touches[0].clientY - panStartY;
      const c = clampPan(scale, panOriginX + dx, panOriginY + dy);
      panX = c.px; panY = c.py;
      return;
    }

    if (e.touches.length === 1 && dismissGesture && !zoomed) {
      const dx = e.touches[0].clientX - touchStartX;
      const dy = e.touches[0].clientY - touchStartY;
      if (dismissGesture === 'pending') {
        if (Math.abs(dx) < 8 && Math.abs(dy) < 8) return;
        dismissGesture = Math.abs(dy) > Math.abs(dx) && dy > 0 ? 'vertical' : 'horizontal';
      }
      if (dismissGesture === 'vertical') {
        e.preventDefault();
        dismissY = Math.max(0, dy);
      }
    }
  }

  function handleTouchEnd(e: TouchEvent) {
    if (e.touches.length > 0) return;
    isPanning = false;
    if (scale < 1.1) resetZoom(true);
    if (zoomed) { dismissGesture = null; return; }

    if (dismissGesture === 'vertical') {
      if (dismissY > DISMISS_THRESHOLD) {
        onClose();
      } else {
        dismissSnapBack = true;
        dismissY = 0;
        setTimeout(() => { dismissSnapBack = false; }, 220);
      }
      dismissGesture = null;
      return;
    }
    dismissGesture = null;

    const dx = e.changedTouches[0].clientX - touchStartX;
    const dy = e.changedTouches[0].clientY - touchStartY;
    if (Math.abs(dx) > Math.abs(dy) && Math.abs(dx) > 48) {
      dx < 0 ? next() : prev();
    }
  }

  let hasMultiple = $derived(images.length > 1);
</script>

<svelte:window onkeydown={handleKey} onmousemove={handleMouseMove} onmouseup={handleMouseUp} />

<div
  class="lb-overlay"
  style={dismissY > 0 ? `background-color: rgba(16, 8, 4, ${(0.97 - dismissProgress * 0.65).toFixed(3)});` : ''}
  transition:fade={{ duration: 200 }}
  role="dialog"
  aria-modal="true"
  aria-label="Image viewer"
  tabindex="-1"
  use:focusTrap
  ontouchstart={handleTouchStart}
  ontouchmove={handleTouchMove}
  ontouchend={handleTouchEnd}
>
  <!-- ── HEADER ── -->
  <header class="lb-header">
    <!-- Left: navigation pills -->
    {#if hasMultiple}
      <div class="lb-nav-pills">
        <button class="lb-pill" onclick={prev} disabled={zoomed} aria-label={$t('lightboxPrevious')}>
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M8 2L4 6l4 4"/>
          </svg>
        </button>
        <button class="lb-pill" onclick={next} disabled={zoomed} aria-label={$t('lightboxNext')}>
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M4 2l4 4-4 4"/>
          </svg>
        </button>
      </div>
    {:else}
      <div></div>
    {/if}

    <!-- Center: counter (visual + a screen-reader-only live announcement) -->
    <div class="lb-counter" aria-hidden="true">
      {#if hasMultiple}
        <span class="lb-counter-current">{current + 1}</span>
        <span class="lb-counter-sep">/</span>
        <span class="lb-counter-total">{images.length}</span>
      {:else}
        <span class="lb-counter-label">{$t('lightboxFullSize')}</span>
      {/if}
    </div>
    <span class="sr-only" aria-live="polite">
      {#if hasMultiple}
        {$t('lightboxPhoto')} {current + 1} / {images.length}
      {:else}
        {$t('lightboxFullSize')}
      {/if}
    </span>

    <!-- Right: close -->
    <button class="lb-close" onclick={onClose} aria-label={$t('figurineGrimoireClose')}>
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M1 1l10 10M11 1L1 11"/>
      </svg>
      <span class="lb-close-label">{$t('figurineGrimoireClose')}</span>
    </button>
  </header>

  <!-- ── BODY ── -->
  <div class="lb-body">

    <!-- Image stage -->
    <div
      class="lb-stage"
      class:lb-stage--zoomed={zoomed}
      bind:this={stageEl}
      style={dismissY > 0 || dismissSnapBack
        ? `transform: translateY(${dismissY}px); opacity: ${1 - dismissProgress * 0.4}; transition: ${dismissSnapBack ? 'transform 0.22s cubic-bezier(0.22,0.1,0.2,1), opacity 0.22s ease' : 'none'};`
        : ''}
      onclick={handleStageClick}
      onwheel={handleWheel}
      ondblclick={handleDoubleClick}
      onmousedown={handleMouseDown}
      role="button"
      tabindex="-1"
      aria-label="Close"
      onkeydown={(e) => { if (e.key === 'Enter') onClose(); }}
    >
      <!-- Prev / Next arrows inside stage -->
      {#if hasMultiple && !zoomed}
        <button
          class="lb-arrow lb-arrow--prev"
          onclick={(e) => { e.stopPropagation(); prev(); }}
          aria-label={$t('lightboxPrevious')}
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M9 2L5 7l4 5"/>
          </svg>
        </button>
      {/if}

      {#key current}
        <div
          class="lb-image-wrap"
          transition:fade={{ duration: 130 }}
          style="
            transform: scale({scale}) translate({panX / scale}px, {panY / scale}px);
            transition: {transitioning ? 'transform 0.26s cubic-bezier(0.22,0.1,0.2,1)' : 'none'};
          "
        >
          {#if !isImageLoaded && images[current].thumbUrl}
            <img
              src={images[current].thumbUrl}
              alt=""
              aria-hidden="true"
              class="lb-image-blur"
            />
          {:else if !isImageLoaded}
            <div class="lb-loading" aria-hidden="true">
              <div class="lb-spinner"></div>
            </div>
          {/if}
          <img
            src={images[current].url}
            alt={images[current].alt ?? ''}
            class="lb-image"
            class:lb-image--loaded={isImageLoaded}
            style="filter: sepia(0.06) contrast(1.02);"
            onload={() => { isImageLoaded = true; }}
            draggable="false"
          />
          <!-- Grain overlay on image -->
          <div class="lb-grain" aria-hidden="true"></div>
        </div>
      {/key}

      {#if hasMultiple && !zoomed}
        <button
          class="lb-arrow lb-arrow--next"
          onclick={(e) => { e.stopPropagation(); next(); }}
          aria-label={$t('lightboxNext')}
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M5 2l4 5-4 5"/>
          </svg>
        </button>
      {/if}

      {#if zoomed}
        <button
          type="button"
          class="lb-zoom-reset"
          onclick={(e) => { e.stopPropagation(); resetZoom(); }}
          aria-label={$t('lightboxResetZoom')}
        >
          ×{scale.toFixed(1)} · {$t('lightboxResetZoom')}
        </button>
      {/if}

      <!-- Caption -->
      {#if images[current]?.alt && !zoomed}
        <p class="lb-caption">{images[current].alt}</p>
      {/if}
    </div>

    <!-- ── THUMBNAIL SIDEBAR (desktop only) ── -->
    {#if hasMultiple}
      <aside class="lb-sidebar" aria-label="Thumbnails">
        {#each images as img, i}
          <button
            class="lb-thumb"
            class:lb-thumb--active={i === current}
            onclick={() => current = i}
            aria-label="{$t('lightboxPhoto')} {i + 1}"
            aria-current={i === current ? 'true' : undefined}
          >
            <picture>
              <source type="image/webp" srcset={resolveWebpUrl(img.thumbUrl ?? img.url) ?? undefined} />
              <img
                src={img.thumbUrl ?? img.url}
                alt=""
                class="lb-thumb-img"
                loading="lazy"
                decoding="async"
                draggable="false"
                style={img.focalX != null && img.focalY != null
                  ? `object-position: ${img.focalX * 100}% ${img.focalY * 100}%;`
                  : undefined}
              />
            </picture>
          </button>
        {/each}
      </aside>
    {/if}

  </div>
</div>

<style>
  /* ── Overlay ── */
  .lb-overlay {
    position: fixed;
    inset: 0;
    /* Above SiteHeader and its dropdown layers. */
    z-index: 1000;
    background: rgba(16, 8, 4, 0.97);
    display: flex;
    flex-direction: column;
  }

  /* ── Header bar ── */
  .lb-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 54px;
    padding: 0 1.25rem;
    border-bottom: 1px solid rgba(255, 249, 240, 0.07);
    flex-shrink: 0;
    gap: 1rem;
  }

  /* Nav pills (prev / next in header) */
  .lb-nav-pills {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .lb-pill {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border: 1px solid rgba(255, 249, 240, 0.16);
    border-radius: 50%;
    color: rgba(255, 249, 240, 0.5);
    background: transparent;
    cursor: pointer;
    transition: color 0.18s ease, border-color 0.18s ease, background 0.18s ease;
  }
  .lb-pill:hover {
    color: #fff9f0;
    border-color: rgba(255, 249, 240, 0.38);
    background: rgba(255, 249, 240, 0.07);
  }
  .lb-pill:disabled {
    opacity: 0.3;
    cursor: default;
    pointer-events: none;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  /* Counter */
  .lb-counter {
    display: flex;
    align-items: center;
    gap: 0.3em;
    font-family: var(--font-body);
    font-size: 0.6875rem;
    font-weight: 500;
    letter-spacing: 0.1em;
    color: rgba(255, 249, 240, 0.4);
    user-select: none;
  }
  .lb-counter-current { color: rgba(255, 249, 240, 0.75); }
  .lb-counter-sep     { color: rgba(255, 249, 240, 0.22); }
  .lb-counter-label   { color: rgba(255, 249, 240, 0.35); text-transform: uppercase; letter-spacing: 0.1em; font-size: 0.5625rem; }

  /* Close button */
  .lb-close {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.38rem 0.875rem;
    border: 1px solid rgba(255, 249, 240, 0.16);
    border-radius: 100px;
    color: rgba(255, 249, 240, 0.55);
    background: transparent;
    cursor: pointer;
    font-family: var(--font-body);
    font-size: 0.5625rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    white-space: nowrap;
    transition: color 0.18s ease, border-color 0.18s ease, background 0.18s ease;
  }
  .lb-close:hover {
    color: #fff9f0;
    border-color: rgba(255, 249, 240, 0.38);
    background: rgba(255, 249, 240, 0.07);
  }

  /* ── Body ── */
  .lb-body {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  /* ── Image stage ── */
  .lb-stage {
    flex: 1;
    min-width: 0;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1.5rem 4rem;
    cursor: zoom-in;
    touch-action: none;
    overflow: hidden;
  }
  .lb-stage--zoomed {
    cursor: grab;
  }
  .lb-stage--zoomed:active {
    cursor: grabbing;
  }

  .lb-image-wrap {
    position: relative;
    max-width: 100%;
    max-height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    transform-origin: center center;
    will-change: transform;
  }

  .lb-image {
    display: block;
    max-width: 100%;
    max-height: calc(100vh - 54px - 3rem);
    width: auto;
    height: auto;
    object-fit: contain;
    user-select: none;
    opacity: 0;
    transition: opacity 0.28s ease;
  }
  .lb-image--loaded { opacity: 1; }

  .lb-zoom-reset {
    position: absolute;
    bottom: 1rem;
    left: 50%;
    transform: translateX(-50%);
    z-index: 10;
    display: inline-flex;
    align-items: center;
    gap: 0.4em;
    padding: 0.32rem 0.8rem;
    font-family: var(--font-body);
    font-size: 0.625rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: rgba(255, 249, 240, 0.75);
    background: rgba(44, 23, 16, 0.55);
    backdrop-filter: blur(4px);
    border: 1px solid rgba(255, 249, 240, 0.2);
    border-radius: 100px;
    cursor: pointer;
    white-space: nowrap;
  }

  .lb-grain {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
    opacity: 0.04;
    mix-blend-mode: overlay;
  }

  .lb-loading {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* Blur-up placeholder — the same small thumb already used in the sidebar,
     stretched and blurred to fill the stage while the full-resolution image
     decodes. No extra request: it's the asset the sidebar thumbnail loads. */
  .lb-image-blur {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: blur(22px) saturate(1.05);
    transform: scale(1.1);
    opacity: 0.85;
  }

  .lb-spinner {
    width: 28px;
    height: 28px;
    border: 1.5px solid rgba(255, 249, 240, 0.2);
    border-top-color: rgba(255, 249, 240, 0.65);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  /* Arrows inside stage */
  .lb-arrow {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    z-index: 10;
    width: 38px;
    height: 38px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(255, 249, 240, 0.16);
    border-radius: 50%;
    color: rgba(255, 249, 240, 0.45);
    background: rgba(16, 8, 4, 0.4);
    backdrop-filter: blur(4px);
    cursor: pointer;
    transition: color 0.18s ease, border-color 0.18s ease, background 0.18s ease, transform 0.18s ease;
  }
  .lb-arrow:hover {
    color: #fff9f0;
    border-color: rgba(255, 249, 240, 0.38);
    background: rgba(255, 249, 240, 0.1);
  }
  .lb-arrow--prev {
    left: 0.75rem;
  }
  .lb-arrow--prev:hover { transform: translateY(-50%) translateX(-2px); }

  .lb-arrow--next {
    right: 0.75rem;
  }
  .lb-arrow--next:hover { transform: translateY(-50%) translateX(2px); }

  /* Caption */
  .lb-caption {
    position: absolute;
    bottom: 1rem;
    left: 50%;
    transform: translateX(-50%);
    font-family: var(--font-body);
    font-size: 0.625rem;
    font-weight: 400;
    letter-spacing: 0.08em;
    color: rgba(255, 249, 240, 0.38);
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 28rem;
    pointer-events: none;
  }

  /* ── Thumbnail sidebar ── */
  .lb-sidebar {
    width: 76px;
    flex-shrink: 0;
    border-left: 1px solid rgba(255, 249, 240, 0.07);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.75rem 0.5rem;
    scrollbar-width: none;
    align-items: center;
  }
  .lb-sidebar::-webkit-scrollbar { display: none; }

  /* Hide sidebar on mobile — swipe to navigate */
  @media (max-width: 767px) {
    .lb-sidebar { display: none; }
    .lb-stage   { padding: 1rem 2.5rem; }
  }

  .lb-thumb {
    flex-shrink: 0;
    width: 56px;
    height: 56px;
    overflow: hidden;
    border: 1.5px solid rgba(255, 249, 240, 0.1);
    border-radius: 3px;
    background: rgba(255, 249, 240, 0.04);
    cursor: pointer;
    padding: 0;
    transition:
      border-color 0.18s ease,
      transform 0.18s ease;
  }
  .lb-thumb:hover {
    border-color: rgba(255, 249, 240, 0.3);
    transform: translateX(-2px);
  }
  .lb-thumb--active {
    border-color: rgba(192, 88, 44, 0.7);
    box-shadow: 0 0 0 1px rgba(192, 88, 44, 0.2);
  }

  .lb-thumb picture {
    display: block;
    width: 100%;
    height: 100%;
  }

  .lb-thumb-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 20%;
    filter: grayscale(0.45);
    transition: filter 0.18s ease;
    display: block;
  }
  .lb-thumb:hover .lb-thumb-img,
  .lb-thumb--active .lb-thumb-img { filter: grayscale(0); }
</style>
