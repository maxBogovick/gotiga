<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import { focusTrap } from '$lib/actions/focusTrap';
  import { lockBodyScroll } from '$lib/actions/lockBodyScroll';
  import { portal } from '$lib/actions/portal';
  import { resolveWebpUrl } from '$lib/api';
  import '$lib/styles/lightbox.css';

  type LightboxImage = { url: string; alt?: string; thumbUrl?: string; focalX?: number | null; focalY?: number | null };

  let {
    images,
    startIndex = 0,
    onClose,
  }: {
    images: LightboxImage[];
    startIndex?: number;
    onClose: (index?: number) => void;
  } = $props();

  let current = $state(startIndex);
  let loadedUrl = $state('');
  let touchStartX = 0;
  let touchStartY = 0;

  let currentImage = $derived(images[current]);
  let isImageLoaded = $derived(Boolean(currentImage && loadedUrl === currentImage.url));

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
  $effect(() => { current; resetZoom(false); });

  function close() { onClose(current); }

  function urlAlreadyDecoded(url: string): boolean {
    if (typeof Image === 'undefined' || !url) return false;
    const probe = new Image();
    probe.src = url;
    return probe.complete && probe.naturalWidth > 0;
  }

  // Bound to the visible <img>. When `url` changes (same node, new src) `update`
  // re-reads `.complete` so a cached file never waits on a second load event —
  // and never shares a loaded flag with a previous frame.
  function revealWhenReady(node: HTMLImageElement, url: string) {
    let bound = url;
    const nodeShowsBound = () => node.getAttribute('src') === bound;
    const done = () => {
      if (nodeShowsBound()) loadedUrl = bound;
    };
    const sync = () => {
      if (nodeShowsBound() && node.complete && node.naturalWidth > 0) done();
    };
    sync();
    node.addEventListener('load', done);
    node.addEventListener('error', done);
    return {
      update(next: string) {
        bound = next;
        sync();
      },
      destroy() {
        node.removeEventListener('load', done);
        node.removeEventListener('error', done);
      },
    };
  }

  function goTo(index: number) {
    if (images.length === 0) return;
    const nextIndex = (index + images.length) % images.length;
    if (nextIndex === current) return;
    const nextUrl = images[nextIndex]?.url ?? '';
    current = nextIndex;
    loadedUrl = urlAlreadyDecoded(nextUrl) ? nextUrl : '';
  }

  function prev() { if (zoomed) return; goTo(current - 1); }
  function next() { if (zoomed) return; goTo(current + 1); }

  function handleKey(e: KeyboardEvent) {
    if (e.key === 'Escape')      { zoomed ? resetZoom() : close(); }
    if (e.key === 'ArrowLeft')   prev();
    if (e.key === 'ArrowRight')  next();
  }

  function handleStageClick(e: MouseEvent) {
    if (dragged) { dragged = false; return; }
    if (e.target === e.currentTarget) close();
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
        close();
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
  let overlayEl: HTMLElement;
  let overlayStyle = $derived(
    `position:fixed;inset:0;z-index:10000;display:flex;flex-direction:column;width:100vw;height:100dvh;background:rgba(16,8,4,${
      dismissY > 0 ? (0.97 - dismissProgress * 0.65).toFixed(3) : '0.97'
    });`
  );

  onMount(() => {
    const el = overlayEl;
    if (!el) return;
    if (el.parentNode !== document.body) document.body.appendChild(el);
    // Svelte's ontouchmove may be passive; iOS needs preventDefault to keep
    // the document from scrolling under the overlay.
    function blockPageScroll(e: TouchEvent) {
      if ((e.target as HTMLElement | null)?.closest?.('.lb-sidebar')) return;
      e.preventDefault();
    }
    el.addEventListener('touchmove', blockPageScroll, { passive: false });
    return () => el.removeEventListener('touchmove', blockPageScroll);
  });
</script>

<svelte:window onkeydown={handleKey} onmousemove={handleMouseMove} onmouseup={handleMouseUp} />

<div
  bind:this={overlayEl}
  class="lb-overlay"
  style={overlayStyle}
  role="dialog"
  aria-modal="true"
  aria-label="Image viewer"
  tabindex="-1"
  use:portal
  use:focusTrap
  use:lockBodyScroll
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
    <button class="lb-close" onclick={close} aria-label={$t('figurineGrimoireClose')}>
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
      onkeydown={(e) => { if (e.key === 'Enter') close(); }}
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

      {#if currentImage}
        <div
          class="lb-image-wrap"
          style="
            transform: scale({scale}) translate({panX / scale}px, {panY / scale}px);
            transition: {transitioning ? 'transform 0.26s cubic-bezier(0.22,0.1,0.2,1)' : 'none'};
          "
        >
          {#if !isImageLoaded && currentImage.thumbUrl}
            <img
              src={currentImage.thumbUrl}
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
            src={currentImage.url}
            alt={currentImage.alt ?? ''}
            class="lb-image"
            class:lb-image--loaded={isImageLoaded}
            style="filter: sepia(0.06) contrast(1.02);"
            use:revealWhenReady={currentImage.url}
            draggable="false"
          />
          <div class="lb-grain" aria-hidden="true"></div>
        </div>
      {/if}

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
            onclick={() => goTo(i)}
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

