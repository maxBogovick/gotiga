<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '$lib/i18n';
  import { focusTrap } from '$lib/actions/focusTrap';

  type LightboxImage = { url: string; alt?: string };

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

  $effect(() => { current = startIndex; });
  $effect(() => { current; isImageLoaded = false; });

  function prev() { current = (current - 1 + images.length) % images.length; }
  function next() { current = (current + 1) % images.length; }

  function handleKey(e: KeyboardEvent) {
    if (e.key === 'Escape')      onClose();
    if (e.key === 'ArrowLeft')   prev();
    if (e.key === 'ArrowRight')  next();
  }

  function handleStageClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }

  function handleTouchStart(e: TouchEvent) {
    touchStartX = e.touches[0].clientX;
    touchStartY = e.touches[0].clientY;
  }

  function handleTouchEnd(e: TouchEvent) {
    const dx = e.changedTouches[0].clientX - touchStartX;
    const dy = e.changedTouches[0].clientY - touchStartY;
    if (Math.abs(dx) > Math.abs(dy) && Math.abs(dx) > 48) {
      dx < 0 ? next() : prev();
    }
  }

  let hasMultiple = $derived(images.length > 1);
</script>

<svelte:window onkeydown={handleKey} />

<div
  class="lb-overlay"
  transition:fade={{ duration: 200 }}
  role="dialog"
  aria-modal="true"
  aria-label="Image viewer"
  tabindex="-1"
  use:focusTrap
  ontouchstart={handleTouchStart}
  ontouchend={handleTouchEnd}
>
  <!-- ── HEADER ── -->
  <header class="lb-header">
    <!-- Left: navigation pills -->
    {#if hasMultiple}
      <div class="lb-nav-pills">
        <button class="lb-pill" onclick={prev} aria-label={$t('lightboxPrevious')}>
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M8 2L4 6l4 4"/>
          </svg>
        </button>
        <button class="lb-pill" onclick={next} aria-label={$t('lightboxNext')}>
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M4 2l4 4-4 4"/>
          </svg>
        </button>
      </div>
    {:else}
      <div></div>
    {/if}

    <!-- Center: counter -->
    <div class="lb-counter" aria-live="polite">
      {#if hasMultiple}
        <span class="lb-counter-current">{current + 1}</span>
        <span class="lb-counter-sep">/</span>
        <span class="lb-counter-total">{images.length}</span>
      {:else}
        <span class="lb-counter-label">Full size</span>
      {/if}
    </div>

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
      onclick={handleStageClick}
      role="button"
      tabindex="-1"
      aria-label="Close"
      onkeydown={(e) => { if (e.key === 'Enter') onClose(); }}
    >
      <!-- Prev / Next arrows inside stage -->
      {#if hasMultiple}
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
        <div class="lb-image-wrap" transition:fade={{ duration: 130 }}>
          {#if !isImageLoaded}
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

      {#if hasMultiple}
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

      <!-- Caption -->
      {#if images[current]?.alt}
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
            <img src={img.url} alt="" class="lb-thumb-img" loading="lazy" draggable="false" />
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
    /* выше SiteHeader (200) и MemoryMirror (220) */
    z-index: 230;
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
    cursor: default;
  }

  .lb-image-wrap {
    position: relative;
    max-width: 100%;
    max-height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
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
