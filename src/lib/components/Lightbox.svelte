<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '$lib/i18n';

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

  let current = $state(startIndex);
  let isImageLoaded = $state(false);
  let touchStartX = 0;
  let touchStartY = 0;

  $effect(() => {
    // Reset loaded state when image changes
    current;
    isImageLoaded = false;
  });

  function prev() {
    current = (current - 1 + images.length) % images.length;
  }

  function next() {
    current = (current + 1) % images.length;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
    if (e.key === 'ArrowLeft') prev();
    if (e.key === 'ArrowRight') next();
  }

  function handleBackdropClick(e: MouseEvent) {
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
      if (dx < 0) next();
      else prev();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="fixed inset-0 z-[200] bg-black/96 flex items-center justify-center"
  transition:fade={{ duration: 220 }}
  onclick={handleBackdropClick}
  ontouchstart={handleTouchStart}
  ontouchend={handleTouchEnd}
  role="dialog"
  aria-modal="true"
  aria-label="Image viewer"
>
  <!-- Top bar -->
  <div class="absolute top-0 inset-x-0 flex items-center justify-between px-6 py-5 z-10">
    {#if images.length > 1}
      <span class="font-['Cinzel'] text-xs tracking-[0.3em] text-white/30 select-none">
        {current + 1}&thinsp;/&thinsp;{images.length}
      </span>
    {:else}
      <span></span>
    {/if}
    <button
      onclick={onClose}
      class="w-10 h-10 flex items-center justify-center text-white/40 hover:text-white border border-white/10 hover:border-white/30 transition-all duration-200 text-lg leading-none"
      aria-label={$t('lightboxClose')}
    >✕</button>
  </div>

  <!-- Main image area -->
  <div class="relative flex items-center justify-center w-full h-full px-16 py-20">

    {#key current}
      <div class="relative max-w-[90vw] max-h-[80vh]" transition:fade={{ duration: 150 }}>
        <!-- Loading skeleton -->
        {#if !isImageLoaded}
          <div class="absolute inset-0 flex items-center justify-center">
            <div class="w-8 h-8 border border-white/20 border-t-white/60 rounded-full animate-spin"></div>
          </div>
        {/if}

        <img
          src={images[current].url}
          alt={images[current].alt ?? ''}
          class="block max-w-[90vw] max-h-[80vh] object-contain select-none transition-opacity duration-300 {isImageLoaded ? 'opacity-100' : 'opacity-0'}"
          style="filter: sepia(0.08) contrast(1.02);"
          onload={() => { isImageLoaded = true; }}
          draggable="false"
        />

        <!-- Film grain overlay on image -->
        <div class="absolute inset-0 pointer-events-none bg-noise opacity-[0.04] mix-blend-overlay"></div>
      </div>
    {/key}

    <!-- Caption -->
    {#if images[current]?.alt}
      <div class="absolute bottom-8 left-1/2 -translate-x-1/2 font-['Cinzel'] text-xs tracking-widest text-white/30 text-center max-w-sm px-4">
        {images[current].alt}
      </div>
    {/if}
  </div>

  <!-- Prev arrow -->
  {#if images.length > 1}
    <button
      onclick={(e) => { e.stopPropagation(); prev(); }}
      class="absolute left-3 top-1/2 -translate-y-1/2 z-10 w-12 h-12 flex items-center justify-center text-white/30 hover:text-white border border-white/10 hover:border-white/30 hover:bg-white/5 transition-all duration-200 group"
      aria-label={$t('lightboxPrevious')}
    >
      <span class="text-lg transition-transform duration-200 group-hover:-translate-x-0.5">←</span>
    </button>

    <!-- Next arrow -->
    <button
      onclick={(e) => { e.stopPropagation(); next(); }}
      class="absolute right-3 top-1/2 -translate-y-1/2 z-10 w-12 h-12 flex items-center justify-center text-white/30 hover:text-white border border-white/10 hover:border-white/30 hover:bg-white/5 transition-all duration-200 group"
      aria-label={$t('lightboxNext')}
    >
      <span class="text-lg transition-transform duration-200 group-hover:translate-x-0.5">→</span>
    </button>
  {/if}

  <!-- Thumbnail strip -->
  {#if images.length > 1}
    <div class="absolute bottom-0 inset-x-0 flex justify-center gap-2 px-6 py-5 overflow-x-auto">
      {#each images as img, i}
        <button
          onclick={(e) => { e.stopPropagation(); current = i; }}
          class="flex-shrink-0 w-12 h-12 overflow-hidden border transition-all duration-200
            {i === current ? 'border-white/60 opacity-100' : 'border-white/10 opacity-30 hover:opacity-60 hover:border-white/30'}"
          aria-label="{$t('lightboxPhoto')} {i + 1}"
        >
          <img src={img.url} alt="" class="w-full h-full object-cover" draggable="false" />
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }
</style>
