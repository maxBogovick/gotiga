<script lang="ts">
  import { onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';
  import type { Figurine } from '$lib/types/api';
  import OrderModal from '$lib/components/OrderModal.svelte';
  import BrassLens from '$lib/components/BrassLens.svelte';
  import CandleReveal from '$lib/components/CandleReveal.svelte';
  import MemoryMirror from '$lib/components/MemoryMirror.svelte';
  import SecretText from '$lib/components/SecretText.svelte';
  import Lightbox from '$lib/components/Lightbox.svelte';
  import { t } from '$lib/i18n';

  let { figurine, id }: { figurine: Figurine; id: string } = $props();

  let selectedImageIndex = $state(0);
  let isGrimoireOpen = $state(false);
  let showOrderModal = $state(false);
  let isAudioPlaying = $state(false);
  let isCandleLit = $state(false);
  let showLightbox = $state(false);
  let lightboxStartIndex = $state(0);
  let audioRef = $state<HTMLAudioElement | null>(null);
  let videoRef = $state<HTMLVideoElement | null>(null);

  let sortedImages = $derived(
    figurine.images.slice().sort((a, b) => {
      if (a.imageType === 'face') return -1;
      if (b.imageType === 'face') return 1;
      return 0;
    })
  );

  let currentImage = $derived(sortedImages[selectedImageIndex]);
  let lightboxImages = $derived(
    sortedImages.map((img) => ({ url: resolveUrl(img.originalUrl ?? img.url), alt: img.altText ?? '' }))
  );

  function resolveUrl(path: string | undefined | null) { return path ?? ''; }
  function selectImage(index: number) { if (index !== selectedImageIndex) selectedImageIndex = index; }
  function openLightbox(index: number) { lightboxStartIndex = index; showLightbox = true; }
  function toggleGrimoire() { isGrimoireOpen = !isGrimoireOpen; }
  function toggleCandle() { isCandleLit = !isCandleLit; }

  function toggleFullscreen() {
    if (!videoRef) return;
    document.fullscreenElement ? document.exitFullscreen() : videoRef.requestFullscreen().catch(() => {});
  }

  function toggleAudio() {
    if (!audioRef || !figurine.ambiencePath) return;
    isAudioPlaying ? fadeOutAudio() : (audioRef.volume = 0, audioRef.play().catch(console.error), isAudioPlaying = true, fadeInAudio());
  }

  function fadeInAudio() {
    if (!audioRef) return;
    let vol = 0;
    const iv = setInterval(() => { vol < 0.5 ? (vol += 0.05, audioRef!.volume = vol) : clearInterval(iv); }, 100);
  }

  function fadeOutAudio() {
    if (!audioRef) return;
    let vol = audioRef.volume;
    const iv = setInterval(() => {
      vol > 0.05 ? (vol -= 0.05, audioRef!.volume = vol) : (clearInterval(iv), audioRef!.pause(), isAudioPlaying = false);
    }, 100);
  }

  onDestroy(() => { if (audioRef) { audioRef.pause(); audioRef = null; } });
</script>

{#if figurine.ambiencePath}
  <audio bind:this={audioRef} src={resolveUrl(figurine.ambiencePath)} loop></audio>
{/if}

<CandleReveal isActive={isCandleLit} />

<div
  class="page-root"
  style="background: radial-gradient(ellipse 70% 55% at 72% 38%, rgba(198, 95, 60, 0.07) 0%, transparent 65%), radial-gradient(ellipse 50% 70% at 18% 72%, rgba(201, 168, 117, 0.06) 0%, transparent 60%), #f8f1e7;"
>
  <OrderModal
    isOpen={showOrderModal}
    figurineName={figurine.name}
    figurineId={figurine.id}
    onClose={() => (showOrderModal = false)}
  />

  {#if showLightbox}
    <Lightbox images={lightboxImages} startIndex={lightboxStartIndex} onClose={() => (showLightbox = false)} />
  {/if}

  <div class="page-container">

    <!-- ── NAV ── -->
    <nav class="topnav" in:fade={{ duration: 600 }}>
      <a href="/figurines" class="nav-link back-link">
        <svg class="back-arrow" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M10 3L5 8l5 5"/>
        </svg>
        {$t('figurineBackToArchive')}
      </a>

      <div class="topnav-controls">
        <button
          onclick={toggleCandle}
          class="control-btn {isCandleLit ? 'control-btn--lit' : ''}"
          aria-label={$t('figurineCandle')}
        >
          <span class="control-icon">{isCandleLit ? '🔥' : '🕯️'}</span>
          {isCandleLit ? $t('figurineExtinguish') : $t('figurineCandle')}
        </button>

        {#if figurine.ambiencePath}
          <button
            onclick={toggleAudio}
            class="control-btn {isAudioPlaying ? 'control-btn--active' : ''}"
            aria-label={$t('figurineWhisper')}
          >
            <span class="audio-indicator {isAudioPlaying ? 'audio-indicator--on' : ''}"></span>
            {isAudioPlaying ? $t('figurineSilence') : $t('figurineWhisper')}
          </button>
        {/if}

        <span class="ref-tag">ARC-{id.toUpperCase()}</span>
      </div>
    </nav>

    <!-- ── MAIN GRID ── -->
    <div class="main-grid">

      <!-- LEFT: Gallery -->
      <div class="gallery-col">
        <div class="image-frame group">
          <span class="corner-tl"></span>
          <span class="corner-tr"></span>
          <span class="corner-bl"></span>
          <span class="corner-br"></span>

          <div class="image-stage">
            {#key currentImage?.id}
              <div class="image-layer" in:fade={{ duration: 450 }}>
                <BrassLens src={currentImage?.url} alt={figurine.name} class="w-full h-full" />
              </div>
            {/key}

            {#if sortedImages.length > 0}
              <button
                onclick={() => openLightbox(selectedImageIndex)}
                class="expand-btn"
                aria-label={$t('figurineFullscreen')}
              >
                <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5">
                  <path d="M1 4V1h3M6 1h3v3M9 6v3H6M4 9H1V6"/>
                </svg>
                {$t('figurineFullscreen')}
              </button>
            {/if}

            <div class="image-vignette"></div>
          </div>
        </div>

        {#if sortedImages.length > 1}
          <div class="thumbs">
            {#each sortedImages as img, i}
              <div class="thumb-item group">
                <button
                  class="thumb {selectedImageIndex === i ? 'thumb--active' : ''}"
                  onclick={() => selectImage(i)}
                  aria-label="{$t('figurineShowView')} {i + 1}"
                >
                  <img src={resolveUrl(img.thumbUrl ?? img.url)} alt="" class="thumb-img" />
                </button>
                <button
                  onclick={() => openLightbox(i)}
                  class="thumb-zoom"
                  aria-label={$t('figurineOpenEnlarged')}
                >
                  <svg width="8" height="8" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5">
                    <path d="M1 4V1h3M6 1h3v3M9 6v3H6M4 9H1V6"/>
                  </svg>
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- RIGHT: Details -->
      <div class="details-col" style="background: #fff9f0;">

        <!-- ── HEADER BLOCK ── -->
        <div class="detail-block detail-block--header">
          {#if figurine.secretText}
            <div class="secret-anchor">
              <SecretText text={figurine.secretText} isCandleLit={isCandleLit} />
            </div>
          {/if}

          <div class="eyebrow-row">
            <div class="eyebrow-left">
              {#if figurine.year}
                <span class="eyebrow-year">Anno {figurine.year}</span>
                <span class="eyebrow-div"></span>
              {/if}
              <span class="eyebrow-code">ARC-{id.toUpperCase()}</span>
            </div>
            <span class="status-pill status-pill--{figurine.status}">
              {figurine.status === 'sold'
                ? $t('figurineStatusSold')
                : figurine.status === 'reserved'
                  ? $t('figurineStatusReserved')
                  : $t('figurineStatusAvailable')}
            </span>
          </div>

          <h1 class="figurine-title">{figurine.name}</h1>

          {#if figurine.shortText}
            <p class="lore-short">{figurine.shortText}</p>
          {/if}
        </div>

        <!-- ── HISTORY BLOCK ── -->
        {#if figurine.fullDescription}
          <div class="detail-block">
            <div class="block-label-row">
              <span class="block-label">{$t('figurineHistory')}</span>
            </div>
            <p class="history-body drop-cap">{figurine.fullDescription}</p>
          </div>
        {/if}

        <!-- ── ATTRIBUTES BLOCK ── -->
        <div class="detail-block">
          <div class="block-label-row">
            <span class="block-label">{$t('figurineAttributes')}</span>
          </div>
          <dl class="attrs-dl">
            {#if figurine.dimensions}
              <div class="attr-row">
                <dt class="attr-label">{$t('figurineDimensions')}</dt>
                <dd class="attr-value">{figurine.dimensions}</dd>
              </div>
            {/if}
            {#if figurine.material}
              <div class="attr-row">
                <dt class="attr-label">{$t('figurineMaterial')}</dt>
                <dd class="attr-value">{figurine.material}</dd>
              </div>
            {/if}
            {#if figurine.technique}
              <div class="attr-row">
                <dt class="attr-label">{$t('figurineTechnique')}</dt>
                <dd class="attr-value">{figurine.technique}</dd>
              </div>
            {/if}
            <div class="attr-row">
              <dt class="attr-label">{$t('figurineCode')}</dt>
              <dd class="attr-value attr-value--code">ARC-{id.toUpperCase()}</dd>
            </div>
          </dl>
        </div>

        <!-- ── CTA BLOCK ── -->
        {#if figurine.status === 'available'}
          <div class="detail-block detail-block--cta">
            <button onclick={() => (showOrderModal = true)} class="cta-btn">
              <span class="cta-btn-label">{$t('figurineRequest')}</span>
              <svg class="cta-arrow" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
                <path d="M3 8h10M9 4l4 4-4 4"/>
              </svg>
            </button>
            <p class="cta-note">{$t('figurineRequestNote')}</p>
          </div>
        {/if}

      </div>
    </div>

    <!-- ── GRIMOIRE ── -->
    {#if figurine.processSteps && figurine.processSteps.length > 0}
      <div class="grimoire-section">
        <button onclick={toggleGrimoire} class="grimoire-trigger">
          <span class="grimoire-name">
            {$t('figurineGrimoire')}
            <span class="grimoire-dot"></span>
          </span>
          <span class="grimoire-stem"></span>
        </button>

        <MemoryMirror
          isOpen={isGrimoireOpen}
          steps={figurine.processSteps}
          finalImage={resolveUrl(currentImage?.url)}
          onClose={() => (isGrimoireOpen = false)}
        />
      </div>
    {/if}

    <!-- ── VIDEO ── -->
    {#if figurine.videoUrl}
      <div class="video-section">
        <div class="divider-text video-heading">
          <span class="section-label">{$t('figurineVideo')}</span>
        </div>

        <div class="video-wrap group">
          <div class="video-frame card group">
            <span class="corner-tl"></span>
            <span class="corner-tr"></span>
            <span class="corner-bl"></span>
            <span class="corner-br"></span>

            <div class="video-stage">
              <video
                bind:this={videoRef}
                controls
                class="video-el"
                poster={resolveUrl(currentImage?.url)}
                preload="metadata"
              >
                <source src={resolveUrl(figurine.videoUrl)} type="video/mp4" />
                {$t('figurineBrowserNoVideo')}
              </video>

              <button onclick={toggleFullscreen} class="video-fs-btn" title={$t('figurineFullscreen')}>
                <svg width="14" height="14" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5">
                  <path d="M1 4V1h3M6 1h3v3M9 6v3H6M4 9H1V6"/>
                </svg>
              </button>
            </div>
          </div>
        </div>

        <p class="video-caption text-label">{$t('figurineVideoFilm')}{id.slice(-3)}</p>
      </div>
    {/if}

    <!-- ── RELATED ── -->
    {#if figurine.relatedItems && figurine.relatedItems.length > 0}
      <div class="related-section">
        <div class="divider-text">
          <span class="section-label">{$t('figurineRelated')}</span>
        </div>

        <div class="related-grid">
          {#each figurine.relatedItems as item}
            <a href="/figurines/{item.id}" class="card-product related-card group">
              <div class="product-image">
                <img
                  src={resolveUrl(item.faceImageUrl)}
                  alt={item.name}
                  class="grayscale-hover zoom-hover"
                />
                <div class="related-img-vignette"></div>
              </div>
              <div class="related-card-body">
                <h4 class="related-name">{item.name}</h4>
                <span class="badge related-status
                  {item.status === 'sold' ? 'badge-ember' : item.status === 'reserved' ? 'badge-ochre' : 'badge-sage'}">
                  {item.status === 'sold'
                    ? $t('figurineStatusSold')
                    : item.status === 'reserved'
                      ? $t('figurineStatusReserved')
                      : $t('figurineStatusAvailable')}
                </span>
              </div>
            </a>
          {/each}
        </div>
      </div>
    {/if}

  </div>
</div>

<style>
  /* ── Page shell ── */
  .page-root {
    min-height: 100svh;
    background:
      radial-gradient(ellipse 70% 55% at 72% 38%, rgba(198, 95, 60, 0.07) 0%, transparent 65%),
      radial-gradient(ellipse 50% 70% at 18% 72%, rgba(201, 168, 117, 0.06) 0%, transparent 60%),
      #f8f1e7;
    color: #2c1710;
    padding-bottom: 8rem;
  }

  .page-container {
    max-width: 1280px;
    margin: 0 auto;
    padding: 2.5rem 1.5rem 0;
  }
  @media (min-width: 1024px) {
    .page-container { padding: 3rem 3.5rem 0; }
  }

  /* ── Top nav ── */
  .topnav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 0.75rem;
    margin-bottom: 3rem;
  }

  .back-link {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-family: var(--font-body);
    font-size: 0.6875rem;
    font-weight: 500;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
    transition: color var(--duration-default) var(--ease-atelier), gap var(--duration-default) var(--ease-atelier);
    padding: 0.25rem 0;
  }
  .back-link:hover { color: var(--color-ink-primary); gap: 0.6rem; }

  .back-arrow {
    transition: transform var(--duration-default) var(--ease-atelier);
  }
  .back-link:hover .back-arrow { transform: translateX(-3px); }

  .topnav-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .control-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.3rem 0.85rem;
    font-family: var(--font-body);
    font-size: 0.6875rem;
    font-weight: 500;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
    background: transparent;
    border: 1px solid var(--color-border-subtle);
    border-radius: 100px;
    cursor: pointer;
    transition: all var(--duration-default) var(--ease-atelier);
  }
  .control-btn:hover {
    color: var(--color-ink-primary);
    border-color: var(--color-border-default);
    background: var(--color-canvas-raised);
    box-shadow: var(--shadow-xs);
  }
  .control-btn--active {
    color: var(--color-ink-primary);
    border-color: var(--color-border-default);
    background: var(--color-canvas-raised);
  }
  .control-btn--lit {
    color: var(--color-ember);
    border-color: var(--color-border-ember);
    background: var(--color-ember-subtle);
  }

  .control-icon { font-size: 0.85rem; line-height: 1; }

  .audio-indicator {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: var(--color-border-default);
    flex-shrink: 0;
    position: relative;
    transition: background var(--duration-default) var(--ease-atelier);
  }
  .audio-indicator--on { background: var(--color-ember); }
  .audio-indicator--on::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: var(--color-ember);
    opacity: 0.5;
    animation: audioPing 1s cubic-bezier(0,0,.2,1) infinite;
  }
  @keyframes audioPing { 75%,100% { transform: scale(2.2); opacity: 0; } }

  .ref-tag {
    font-family: var(--font-body);
    font-size: 0.625rem;
    font-weight: 500;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-ink-muted);
    border: 1px solid var(--color-border-subtle);
    border-radius: 100px;
    padding: 0.28rem 0.75rem;
  }

  /* ── Main 2-col grid ── */
  .main-grid {
    display: grid;
    gap: 3rem;
    align-items: start;
    margin-bottom: 5rem;
  }
  @media (min-width: 1024px) {
    .main-grid { grid-template-columns: 7fr 5fr; gap: 4.5rem; }
    .gallery-col { position: sticky; top: 2rem; }
  }

  /* ── Image frame ── */
  .image-frame {
    position: relative;
    border: 1px solid var(--color-border-subtle);
    border-radius: 4px;
    overflow: hidden;
    background: var(--color-canvas-raised);
    box-shadow: var(--shadow-lg);
    transition: box-shadow var(--duration-slow) var(--ease-atelier);
  }
  .image-frame:hover { box-shadow: var(--shadow-xl); }

  .image-stage {
    position: relative;
    aspect-ratio: 4/5;
    overflow: hidden;
    background: var(--color-canvas-sunken);
  }

  .image-layer {
    position: absolute;
    inset: 0;
  }

  .image-vignette {
    position: absolute;
    inset: 0;
    pointer-events: none;
    box-shadow: inset 0 0 60px rgba(60,25,10,0.12);
  }

  .expand-btn {
    position: absolute;
    bottom: 0.75rem;
    right: 0.75rem;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.35rem 0.65rem;
    font-family: var(--font-body);
    font-size: 0.625rem;
    font-weight: 500;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
    background: var(--color-canvas-raised);
    border: 1px solid var(--color-border-default);
    border-radius: 4px;
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--duration-default) var(--ease-atelier), box-shadow var(--duration-default) var(--ease-atelier);
  }
  .image-stage:hover .expand-btn { opacity: 1; }
  .expand-btn:hover {
    box-shadow: var(--shadow-sm);
    color: var(--color-ink-primary);
  }

  /* ── Thumbnails ── */
  .thumbs {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    padding-top: 0.75rem;
  }

  .thumb-item { position: relative; }

  .thumb {
    width: 4.25rem;
    height: 4.25rem;
    overflow: hidden;
    border: 1px solid var(--color-border-subtle);
    border-radius: 4px;
    cursor: pointer;
    padding: 0;
    background: none;
    transition: border-color var(--duration-default) var(--ease-atelier), transform var(--duration-default) var(--ease-atelier), box-shadow var(--duration-default) var(--ease-atelier);
  }
  .thumb:hover {
    border-color: var(--color-border-default);
    transform: translateY(-2px);
    box-shadow: var(--shadow-sm);
  }
  .thumb--active {
    border-color: var(--color-ember);
    box-shadow: 0 0 0 2px var(--color-ember-subtle);
  }

  .thumb-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: filter var(--duration-slow) var(--ease-atelier);
    filter: grayscale(1);
  }
  .thumb:hover .thumb-img,
  .thumb--active .thumb-img { filter: grayscale(0); }

  .thumb-zoom {
    position: absolute;
    bottom: 2px;
    right: 2px;
    width: 1rem;
    height: 1rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(253,250,245,0.85);
    border: none;
    border-radius: 2px;
    cursor: pointer;
    color: var(--color-ember);
    opacity: 0;
    transition: opacity var(--duration-default) var(--ease-atelier);
    padding: 0;
  }
  .thumb-item:hover .thumb-zoom { opacity: 1; }

  /* ═══════════════════════════════════════════
     DETAILS COLUMN — полностью переработано
  ═══════════════════════════════════════════ */

  .details-col {
    display: flex;
    flex-direction: column;
    align-self: start;
    border-radius: 20px;
    overflow: hidden;
    border: 1px solid rgba(52, 37, 28, 0.12);
    background: #fff9f0;
    box-shadow:
      0 0 0 1px rgba(255, 255, 255, 0.7) inset,
      0 32px 72px rgba(52, 37, 28, 0.10),
      0 4px 16px rgba(52, 37, 28, 0.05);
  }

  /* ── Общий блок ── */
  .detail-block {
    padding: 1.75rem 1.875rem;
    border-bottom: 1px solid rgba(52, 37, 28, 0.10);
    position: relative;
  }
  .detail-block:last-child {
    border-bottom: none;
  }

  /* ── Шапка ── */
  .detail-block--header {
    padding-bottom: 1.5rem;
    background:
      radial-gradient(ellipse 120% 80% at 50% 0%, rgba(198, 95, 60, 0.055) 0%, transparent 70%);
  }

  .secret-anchor {
    position: absolute;
    top: -2rem;
    right: 0;
    max-width: 16rem;
    text-align: right;
    transform: rotate(1.2deg);
    z-index: 10;
  }

  .eyebrow-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .eyebrow-left {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .eyebrow-year {
    font-family: var(--font-body);
    font-size: 0.6875rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: rgba(95, 70, 54, 0.78);
  }

  .eyebrow-div {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: rgba(52, 37, 28, 0.24);
    flex-shrink: 0;
  }

  .eyebrow-code {
    font-family: var(--font-body);
    font-size: 0.625rem;
    font-weight: 500;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: rgba(95, 70, 54, 0.42);
  }

  .figurine-title {
    font-family: var(--font-display);
    font-size: clamp(2.1rem, 3.8vw, 3.4rem);
    font-weight: 500;
    letter-spacing: -0.025em;
    line-height: 1.03;
    color: #2c1710;
    margin: 0 0 1rem;
    text-wrap: balance;
  }

  .lore-short {
    font-family: var(--font-serif);
    font-size: 1rem;
    font-style: italic;
    line-height: 1.68;
    color: #6f3b24;
    margin: 0;
    padding-top: 0.5rem;
    border-top: 1px solid rgba(52, 37, 28, 0.10);
    letter-spacing: 0.01em;
  }

  /* Статус-пилюля */
  .status-pill {
    display: inline-flex;
    align-items: center;
    padding: 0.22rem 0.7rem;
    font-family: var(--font-body);
    font-size: 0.6rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    border-radius: 100px;
    white-space: nowrap;
  }
  .status-pill--available {
    background: var(--color-sage-subtle);
    color: var(--color-sage-ink);
    border: 1px solid rgba(107, 138, 86, 0.25);
  }
  .status-pill--sold {
    background: var(--color-ember-subtle);
    color: var(--color-ember-ink);
    border: 1px solid rgba(192, 88, 44, 0.22);
  }
  .status-pill--reserved {
    background: var(--color-ochre-subtle);
    color: var(--color-ochre-ink);
    border: 1px solid rgba(176, 136, 32, 0.22);
  }

  /* ── Метка блока ── */
  .block-label-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }
  .block-label-row::after {
    content: '';
    flex: 1;
    height: 1px;
    background: rgba(52, 37, 28, 0.10);
  }

  .block-label {
    font-family: var(--font-body);
    font-size: 0.6rem;
    font-weight: 800;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #c65f3c;
    flex-shrink: 0;
  }

  /* ── История ── */
  .history-body {
    font-family: var(--font-serif);
    font-size: 1.04rem;
    line-height: 1.84;
    color: #34251c;
    margin: 0;
    font-weight: 500;
  }
  .history-body.drop-cap::first-letter {
    font-family: var(--font-display);
    font-size: 3.1rem;
    font-weight: 600;
    float: left;
    line-height: 0.82;
    margin-right: 0.3rem;
    margin-top: 0.07em;
    color: #c65f3c;
  }

  /* ── Атрибуты ── */
  .attrs-dl {
    margin: 0;
    display: flex;
    flex-direction: column;
  }

  .attr-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    padding: 0.72rem 0.6rem;
    margin: 0 -0.6rem;
    border-radius: 8px;
    transition: background 140ms ease;
  }
  .attr-row + .attr-row {
    border-top: 1px solid rgba(52, 37, 28, 0.08);
  }
  .attr-row:hover {
    background: rgba(198, 95, 60, 0.055);
  }

  .attr-label {
    font-family: var(--font-body);
    font-size: 0.6875rem;
    font-weight: 700;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: rgba(95, 70, 54, 0.76);
    flex-shrink: 0;
    line-height: 1.4;
  }

  .attr-value {
    font-family: var(--font-body);
    font-size: 0.875rem;
    font-weight: 600;
    line-height: 1.45;
    color: #34251c;
    text-align: right;
    max-width: 58%;
    margin: 0;
  }

  .attr-value--code {
    font-family: var(--font-body);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.07em;
    font-size: 0.7rem;
    font-weight: 600;
    color: #6f3b24;
    background: rgba(201, 168, 117, 0.14);
    border: 1px solid rgba(52, 37, 28, 0.12);
    padding: 0.18rem 0.55rem;
    border-radius: 5px;
  }

  /* ── CTA блок ── */
  .detail-block--cta {
    background: rgba(198, 95, 60, 0.04);
    border-top: 1px solid rgba(198, 95, 60, 0.12);
    border-bottom: none;
    display: flex;
    flex-direction: column;
    gap: 0.875rem;
  }

  .cta-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.6rem;
    padding: 0.95rem 1.5rem;
    font-family: var(--font-body);
    font-size: 0.8125rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    border-radius: 12px;
    background: #6f3b24;
    color: #fff9f0;
    border: none;
    cursor: pointer;
    position: relative;
    overflow: hidden;
    transition: background 200ms ease, transform 150ms ease, box-shadow 200ms ease;
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.2) inset,
      0 6px 20px rgba(111, 59, 36, 0.30);
  }
  .cta-btn::before {
    content: '';
    position: absolute;
    inset: 0;
    background: rgba(255, 255, 255, 0);
    transition: background 200ms ease;
  }
  .cta-btn:hover {
    background: #c65f3c;
    transform: translateY(-1px);
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.2) inset,
      0 10px 28px rgba(111, 59, 36, 0.34);
  }
  .cta-btn:active {
    transform: translateY(0);
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.15) inset,
      0 3px 10px rgba(111, 59, 36, 0.25);
  }

  .cta-btn-label {
    position: relative;
    z-index: 1;
  }

  .cta-arrow {
    position: relative;
    z-index: 1;
    flex-shrink: 0;
    transition: transform 200ms ease;
  }
  .cta-btn:hover .cta-arrow {
    transform: translateX(3px);
  }

  .cta-note {
    text-align: center;
    font-family: var(--font-serif);
    font-size: 0.75rem;
    font-style: italic;
    color: rgba(95, 70, 54, 0.76);
    margin: 0;
    line-height: 1.55;
  }

  /* ── Grimoire ── */
  .grimoire-section {
    border-top: 1px solid var(--color-border-subtle);
    padding-top: 3.5rem;
    margin-bottom: 1.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .grimoire-trigger {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }

  .grimoire-name {
    position: relative;
    font-family: var(--font-display);
    font-size: 1.4rem;
    font-weight: 300;
    letter-spacing: -0.01em;
    color: var(--color-ink-primary);
    opacity: 0.65;
    transition: opacity var(--duration-default) var(--ease-atelier);
  }
  .grimoire-trigger:hover .grimoire-name { opacity: 1; }

  .grimoire-dot {
    position: absolute;
    top: -0.15rem;
    right: -0.6rem;
    width: 0.4rem;
    height: 0.4rem;
    border-radius: 50%;
    background: var(--color-ember);
    animation: audioPing 1.2s cubic-bezier(0,0,.2,1) infinite;
  }

  .grimoire-stem {
    width: 1px;
    height: 3.5rem;
    background: linear-gradient(to bottom, var(--color-border-default), transparent);
    transition: height var(--duration-slow) var(--ease-atelier);
  }
  .grimoire-trigger:hover .grimoire-stem { height: 5rem; }

  /* ── Video section ── */
  .video-section { margin-top: 5rem; }

  .video-heading {
    margin-bottom: 2rem;
  }

  .video-wrap {
    max-width: 54rem;
    margin: 0 auto;
    position: relative;
  }

  .video-frame {
    padding: 0.5rem;
    border-radius: 10px;
    box-shadow: var(--shadow-xl);
    transition: transform var(--duration-slow) var(--ease-atelier), box-shadow var(--duration-slow) var(--ease-atelier);
  }
  .video-frame:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow-xl), 0 0 0 1px rgba(192,88,44,0.08);
  }

  .video-stage {
    position: relative;
    aspect-ratio: 16/9;
    overflow: hidden;
    border-radius: 6px;
    background: var(--color-canvas-deep);
  }

  .video-el {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .video-fs-btn {
    position: absolute;
    top: 0.6rem;
    right: 0.6rem;
    z-index: 20;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    background: rgba(253,250,245,0.85);
    border: 1px solid var(--color-border-default);
    border-radius: 4px;
    color: var(--color-ink-secondary);
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--duration-default) var(--ease-atelier);
  }
  .video-stage:hover .video-fs-btn { opacity: 1; }
  .video-fs-btn:hover { background: var(--color-canvas-raised); color: var(--color-ink-primary); }

  .video-caption {
    text-align: center;
    color: var(--color-ink-muted);
    margin-top: 1rem;
  }

  /* ── Related items ── */
  .related-section { margin-top: 5rem; }

  .related-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 17rem), 1fr));
    gap: 1.25rem;
    margin-top: 2rem;
  }

  .related-card {
    text-decoration: none;
    border-radius: 14px;
    overflow: hidden;
  }

  .related-img-vignette {
    position: absolute;
    inset: 0;
    box-shadow: inset 0 0 24px rgba(60,25,10,0.1);
    pointer-events: none;
    border-radius: inherit;
  }

  .related-card-body {
    padding: 0.85rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    background: var(--color-canvas-raised);
  }

  .related-name {
    font-family: var(--font-display);
    font-size: 1.1rem;
    font-weight: 400;
    letter-spacing: -0.015em;
    color: var(--color-ink-primary);
    margin: 0;
    transition: color var(--duration-default) var(--ease-atelier);
  }
  .related-card:hover .related-name { color: var(--color-ember); }

  .related-status { align-self: flex-start; }

  /* ── Utility ── */
  .section-label {
    display: inline-flex;
    align-items: center;
    font-family: var(--font-body);
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.13em;
    text-transform: uppercase;
    color: #a84f2f;
  }

  .divider-text {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .divider-text::before,
  .divider-text::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--color-border-subtle);
  }
</style>
