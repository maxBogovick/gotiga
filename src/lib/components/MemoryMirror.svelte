<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t } from '$lib/i18n';

  let {
    finalImage,
    steps = [],
    isOpen = false,
    onClose = () => {}
  } = $props<{
    finalImage: string;
    steps: { id: string; stepType: string; description: string | null; imageUrl: string }[];
    isOpen: boolean;
    onClose: () => void;
  }>();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let ctx: CanvasRenderingContext2D | null;
  let container = $state<HTMLDivElement | null>(null);

  let width  = $state(0);
  let height = $state(0);

  let loadedFinalImage: HTMLImageElement | null = null;
  let currentStepIndex = $state(0);
  let restoreInterval: ReturnType<typeof setInterval>;
  let isDrawing = false;

  // ── Image loading ──────────────────────────────────────────────────────────
  $effect(() => {
    if (isOpen && finalImage) {
      const img = new Image();
      img.crossOrigin = 'Anonymous';
      img.src = finalImage;
      img.onload = () => { loadedFinalImage = img; drawFullState(true); };
    }
  });

  $effect(() => {
    if (isOpen && width > 0 && height > 0 && canvas) {
      canvas.width  = width;
      canvas.height = height;
      ctx = canvas.getContext('2d');
      if (loadedFinalImage) {
        drawFullState(true);
      } else if (ctx) {
        ctx.fillStyle = '#fff9f0';
        ctx.fillRect(0, 0, width, height);
      }
    }
  });

  function drawFullState(reset = false) {
    if (!ctx || !loadedFinalImage || !canvas) return;
    const cw = canvas.width, ch = canvas.height;
    const iw = loadedFinalImage.width, ih = loadedFinalImage.height;
    const scale = Math.max(cw / iw, ch / ih);
    const x = (cw - iw * scale) / 2;
    const y = (ch - ih * scale) / 2;
    ctx.globalCompositeOperation = 'source-over';
    if (reset) {
      ctx.filter = 'blur(2px) contrast(1.1)';
      ctx.globalAlpha = 1.0;
      ctx.drawImage(loadedFinalImage, x, y, iw * scale, ih * scale);
      ctx.fillStyle = 'rgba(200,220,230,0.08)';
      ctx.fillRect(0, 0, cw, ch);
      ctx.filter = 'none';
    } else {
      ctx.globalAlpha = 0.03;
      ctx.filter = 'blur(4px)';
      ctx.drawImage(loadedFinalImage, x, y, iw * scale, ih * scale);
      ctx.filter = 'none';
      ctx.globalAlpha = 1.0;
    }
  }

  function restoreFog() {
    if (isOpen && !isDrawing) drawFullState(false);
  }

  // ── Pointer handling ───────────────────────────────────────────────────────
  function getPos(e: MouseEvent | TouchEvent) {
    if (!canvas) return { x: 0, y: 0 };
    const rect = canvas.getBoundingClientRect();
    if (e instanceof MouseEvent) {
      return { x: e.clientX - rect.left, y: e.clientY - rect.top };
    }
    return { x: e.touches[0].clientX - rect.left, y: e.touches[0].clientY - rect.top };
  }

  function startDraw(e: MouseEvent | TouchEvent) { isDrawing = true; draw(e); }
  function stopDraw() { isDrawing = false; ctx?.beginPath(); }

  function draw(e: MouseEvent | TouchEvent) {
    if (!isDrawing || !ctx) return;
    if (window.TouchEvent && e instanceof TouchEvent) e.preventDefault();
    const { x, y } = getPos(e);
    ctx.globalCompositeOperation = 'destination-out';
    ctx.beginPath();
    ctx.arc(x, y, 52, 0, Math.PI * 2);
    ctx.fill();
    ctx.globalCompositeOperation = 'source-over';
  }

  // ── Keyboard ───────────────────────────────────────────────────────────────
  function handleKey(e: KeyboardEvent) {
    if (!isOpen) return;
    if (e.key === 'Escape') onClose();
    if (e.key === 'ArrowRight') currentStepIndex = Math.min(steps.length - 1, currentStepIndex + 1);
    if (e.key === 'ArrowLeft')  currentStepIndex = Math.max(0, currentStepIndex - 1);
  }

  let reduced = $state(false);

  onMount(() => {
    reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (!reduced) restoreInterval = setInterval(restoreFog, 80);
    window.addEventListener('keydown', handleKey);
  });

  onDestroy(() => {
    clearInterval(restoreInterval);
    window.removeEventListener('keydown', handleKey);
  });

  let currentStep = $derived(steps[currentStepIndex]);
  let stepLabel = $derived(
    currentStep?.stepType === 'sketch'    ? $t('figurineStepSketch')    :
    currentStep?.stepType === 'prototype' ? $t('figurineStepPrototype') :
    currentStep?.stepType === 'modeling'  ? $t('figurineStepModeling')  :
    currentStep?.stepType === 'painting'  ? $t('figurineStepPainting')  :
    $t('figurineStepFinish')
  );
</script>

{#if isOpen}
  <div
    class="mirror-overlay"
    transition:fade={{ duration: 600 }}
    role="dialog"
    aria-modal="true"
    aria-label={$t('mirrorTitle')}
  >
    <!-- ── HEADER ──────────────────────────────────────────────────────────── -->
    <header class="mirror-header">
      <div class="mirror-header-left">
        <span class="mirror-eyebrow">{$t('mirrorTitle')}</span>
        {#if currentStep}
          <span class="mirror-step-name">{stepLabel}</span>
        {/if}
      </div>

      <p class="mirror-hint">{$t('mirrorHint')}</p>

      <button class="mirror-close" onclick={onClose} aria-label={$t('figurineGrimoireClose')}>
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M1 1l12 12M13 1L1 13"/>
        </svg>
        <span>{$t('figurineGrimoireClose')}</span>
      </button>
    </header>

    <!-- ── CANVAS STAGE ────────────────────────────────────────────────────── -->
    <div class="mirror-stage">
      <div
        bind:clientWidth={width}
        bind:clientHeight={height}
        bind:this={container}
        class="mirror-canvas-wrap"
      >
        <!-- Layer 1: past image (revealed by wiping) -->
        {#key currentStepIndex}
          <div class="mirror-past" transition:fade={{ duration: reduced ? 0 : 400 }}>
            {#if currentStep}
              <img
                src={currentStep.imageUrl}
                alt=""
                class="mirror-past-img"
              />
              {#if currentStep.description}
                <div class="mirror-caption">
                  <span class="mirror-caption-text">{currentStep.description}</span>
                </div>
              {/if}
            {/if}
            <div class="paper-texture"></div>
          </div>
        {/key}

        <!-- Layer 2: fog canvas (hidden when reduced-motion) -->
        {#if !reduced}
        <canvas
          bind:this={canvas}
          class="mirror-canvas"
          onmousedown={startDraw}
          onmouseup={stopDraw}
          onmouseleave={stopDraw}
          onmousemove={draw}
          ontouchstart={startDraw}
          ontouchend={stopDraw}
          ontouchmove={draw}
        ></canvas>
        {/if}

        <!-- Decorative vignette -->
        <div class="mirror-vignette" aria-hidden="true"></div>
        <div class="mirror-border" aria-hidden="true"></div>
      </div>
    </div>

    <!-- ── STEP FILMSTRIP ──────────────────────────────────────────────────── -->
    {#if steps.length > 1}
      <nav class="mirror-filmstrip" aria-label="Steps">
        {#each steps as step, i}
          <button
            class="film-frame {i === currentStepIndex ? 'film-frame--active' : ''}"
            onclick={() => currentStepIndex = i}
            aria-label="Step {i + 1}"
            aria-current={i === currentStepIndex ? 'step' : undefined}
          >
            <div class="film-thumb">
              <img src={step.imageUrl} alt="" class="film-img" />
              <div class="film-overlay"></div>
            </div>
            <span class="film-label">
              {step.stepType === 'sketch'    ? $t('figurineStepSketch')    :
               step.stepType === 'prototype' ? $t('figurineStepPrototype') :
               step.stepType === 'modeling'  ? $t('figurineStepModeling')  :
               step.stepType === 'painting'  ? $t('figurineStepPainting')  :
               $t('figurineStepFinish')}
            </span>
            {#if i < steps.length - 1}
              <span class="film-connector" aria-hidden="true"></span>
            {/if}
          </button>
        {/each}
      </nav>
    {/if}
  </div>
{/if}

<style>
  /* ── Overlay ── */
  .mirror-overlay {
    position: fixed;
    inset: 0;
    /* Above SiteHeader and its dropdown layers. */
    z-index: 1000;
    display: flex;
    flex-direction: column;
    background: rgba(248, 241, 231, 0.97);
    backdrop-filter: blur(12px) saturate(1.2);
    -webkit-backdrop-filter: blur(12px) saturate(1.2);
  }

  /* ── Header ── */
  .mirror-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0 2rem;
    height: 60px;
    flex-shrink: 0;
    border-bottom: 1px solid rgba(180, 140, 100, 0.18);
    background: rgba(253, 250, 245, 0.80);
  }

  .mirror-header-left {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    min-width: 0;
  }

  .mirror-eyebrow {
    font-family: var(--font-display);
    font-size: 1rem;
    font-weight: 400;
    letter-spacing: -0.01em;
    color: var(--color-ink-primary);
    white-space: nowrap;
  }

  .mirror-step-name {
    font-family: var(--font-body);
    font-size: 0.6875rem;
    font-weight: 500;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--color-ember);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mirror-hint {
    font-family: var(--font-body);
    font-size: 0.6875rem;
    font-weight: 400;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-ink-muted);
    flex: 1;
    text-align: center;
    display: none;
  }
  @media (min-width: 640px) { .mirror-hint { display: block; } }

  /* ── Кнопка закрытия — намеренно крупная и очевидная ── */
  .mirror-close {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 1rem;
    font-family: var(--font-body);
    font-size: 0.6875rem;
    font-weight: 500;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--color-ink-secondary);
    background: transparent;
    border: 1px solid var(--color-border-default);
    border-radius: 100px;
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
    transition: all var(--duration-default) var(--ease-atelier);
  }
  .mirror-close:hover {
    color: var(--color-ink-primary);
    background: var(--color-canvas-raised);
    border-color: var(--color-border-strong);
    box-shadow: 0 2px 8px rgba(60,25,10,0.08);
  }
  .mirror-close:focus-visible {
    outline: 2px solid var(--color-ember);
    outline-offset: 3px;
  }

  /* ── Canvas stage ── */
  .mirror-stage {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1.5rem 2rem;
  }

  .mirror-canvas-wrap {
    position: relative;
    width: 100%;
    max-width: 860px;
    height: 100%;
    max-height: 520px;
    aspect-ratio: 4/3;
    background: var(--color-canvas-raised);
    border: 1px solid rgba(180,140,100,0.22);
    box-shadow:
      0 0 0 1px rgba(255,249,240,0.6) inset,
      0 8px 60px rgba(111,59,36,0.14),
      0 0 120px rgba(111,59,36,0.08);
    overflow: hidden;
    user-select: none;
  }

  .mirror-past {
    position: absolute;
    inset: 0;
    background: #f0ebe0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .mirror-past-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: sepia(0.55) contrast(0.92);
  }

  .mirror-caption {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    padding: 2.5rem 2rem 1.5rem;
    background: linear-gradient(to top, rgba(44,23,16,0.55) 0%, transparent 100%);
    display: flex;
    justify-content: center;
  }

  .mirror-caption-text {
    font-family: var(--font-serif);
    font-size: 1rem;
    font-style: italic;
    color: rgba(255, 249, 240, 0.92);
    letter-spacing: 0.02em;
    text-shadow: 0 1px 8px rgba(0,0,0,0.3);
    text-align: center;
    max-width: 36rem;
  }

  .mirror-canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    cursor: crosshair;
    touch-action: none;
  }

  .mirror-vignette {
    position: absolute;
    inset: 0;
    pointer-events: none;
    box-shadow: inset 0 0 140px rgba(111,59,36,0.22);
  }

  .mirror-border {
    position: absolute;
    inset: 10px;
    pointer-events: none;
    border: 1px solid rgba(52,37,28,0.08);
  }

  .paper-texture {
    position: absolute;
    inset: 0;
    pointer-events: none;
    opacity: 0.18;
    mix-blend-mode: multiply;
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
    background-size: 150px 150px;
  }

  /* ── Filmstrip navigation ── */
  .mirror-filmstrip {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0;
    padding: 1rem 2rem 1.25rem;
    flex-shrink: 0;
    border-top: 1px solid rgba(180,140,100,0.14);
    background: rgba(253,250,245,0.7);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .mirror-filmstrip::-webkit-scrollbar { display: none; }

  .film-frame {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4rem;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0 0.75rem;
    transition: transform var(--duration-default) var(--ease-atelier);
  }
  .film-frame:hover { transform: translateY(-2px); }

  .film-thumb {
    position: relative;
    width: 48px;
    height: 48px;
    border-radius: 4px;
    overflow: hidden;
    border: 1.5px solid rgba(180,140,100,0.28);
    transition: border-color var(--duration-default) var(--ease-atelier), box-shadow var(--duration-default) var(--ease-atelier);
  }

  .film-frame--active .film-thumb {
    border-color: var(--color-ember);
    box-shadow: 0 0 0 2px rgba(192,88,44,0.18);
  }

  .film-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: grayscale(0.4) saturate(0.85);
    transition: filter var(--duration-default) var(--ease-atelier);
  }
  .film-frame--active .film-img,
  .film-frame:hover .film-img { filter: grayscale(0) saturate(1); }

  .film-overlay {
    position: absolute;
    inset: 0;
    background: rgba(44,23,16,0.18);
    transition: opacity var(--duration-default) var(--ease-atelier);
  }
  .film-frame--active .film-overlay,
  .film-frame:hover .film-overlay { opacity: 0; }

  .film-label {
    font-family: var(--font-body);
    font-size: 0.5625rem;
    font-weight: 500;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-ink-muted);
    white-space: nowrap;
    transition: color var(--duration-default) var(--ease-atelier);
  }
  .film-frame--active .film-label { color: var(--color-ember); }

  /* Соединитель между кадрами */
  .film-connector {
    position: absolute;
    right: -4px;
    top: 20px;
    width: 8px;
    height: 1px;
    background: rgba(180,140,100,0.28);
    pointer-events: none;
  }

  @media (max-width: 480px) {
    .mirror-header { padding: 0 1rem; }
    .mirror-stage  { padding: 1rem; }
    .film-thumb    { width: 38px; height: 38px; }
    .mirror-close span { display: none; }
    .mirror-close { padding: 0.45rem 0.6rem; }
  }
</style>
