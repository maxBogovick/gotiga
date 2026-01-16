<script lang="ts">
  import { fade } from 'svelte/transition';

  let { src, alt, class: className = '' } = $props();

  let container: HTMLDivElement;
  let showLens = $state(false);
  let x = $state(0);
  let y = $state(0);
  let width = $state(0);
  let height = $state(0);

  const lensSize = 240;
  const zoom = 2.5;

  function handleMouseMove(e: MouseEvent) {
    if (!container) return;
    const rect = container.getBoundingClientRect();
    x = e.clientX - rect.left;
    y = e.clientY - rect.top;
    width = rect.width;
    height = rect.height;
  }
</script>

<div
  bind:this={container}
  class="relative w-full h-full overflow-hidden cursor-none group {className}"
  onmousemove={handleMouseMove}
  onmouseenter={() => showLens = true}
  onmouseleave={() => showLens = false}
  role="img"
  aria-label={alt}
>
  <!-- Main Image -->
  <!-- Note: We apply pointer-events-none to prevent image dragging affecting the custom cursor -->
  <img {src} {alt} class="w-full h-full object-contain pointer-events-none select-none transition-opacity duration-500" />

  <!-- Lens Element -->
  {#if showLens}
    <div
      class="absolute z-50 rounded-full overflow-hidden pointer-events-none"
      transition:fade={{ duration: 200 }}
      style="
        width: {lensSize}px;
        height: {lensSize}px;
        top: {y - lensSize / 2}px;
        left: {x - lensSize / 2}px;
        /* Brass Rim & Shadows */
        box-shadow: 
            0 20px 50px rgba(0,0,0,0.8),
            inset 0 0 40px rgba(20,10,0,0.9);
        border: 2px solid #b5a642;
        outline: 4px solid #5c5322;
        background: #0a0806;
      "
    >
      <!-- Zoomed Content Wrapper -->
      <!-- We transform this container to position the zoomed image correctly relative to the lens center -->
      <div
         style="
            width: {width}px;
            height: {height}px;
            transform-origin: 0 0;
            transform: translate({lensSize/2 - x * zoom}px, {lensSize/2 - y * zoom}px) scale({zoom});
         "
      >
         <!-- The Zoomed Image -->
         <img {src} {alt} class="w-full h-full object-contain" />
      </div>

      <!-- ATMOSPHERE LAYERS -->

      <!-- 1. Chromatic Aberration Hint (Color Fringe) -->
      <div class="absolute inset-0 rounded-full shadow-[inset_3px_0_0_rgba(255,0,0,0.15),inset_-3px_0_0_rgba(0,255,255,0.15)] mix-blend-color-dodge pointer-events-none"></div>

      <!-- 2. Glass Convex Reflection -->
      <div class="absolute inset-0 rounded-full bg-gradient-to-br from-white/20 via-transparent to-black/40 pointer-events-none mix-blend-overlay"></div>
      <div class="absolute top-4 left-6 w-12 h-6 bg-white/10 blur-md rounded-full rotate-[-45deg] pointer-events-none"></div>

      <!-- 3. Dust/Scratches on the lens itself -->
      <div class="absolute inset-0 bg-noise opacity-30 mix-blend-soft-light pointer-events-none"></div>
      
      <!-- 4. Vignette inside lens -->
      <div class="absolute inset-0 rounded-full bg-[radial-gradient(circle,transparent_50%,rgba(0,0,0,0.6)_100%)] pointer-events-none"></div>
    </div>
  {/if}
</div>

<style>
  /* Ensure bg-noise works if not globally available, though it should be from app.css */
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }
</style>