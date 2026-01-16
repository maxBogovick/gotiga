<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';

  let { 
    finalImage, 
    steps = [], 
    isOpen = false, 
    onClose = () => {} 
  } = $props();

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null;
  let container: HTMLDivElement;
  
  // Dimensions
  let width = $state(0);
  let height = $state(0);
  
  let loadedFinalImage: HTMLImageElement | null = null;
  let currentStepIndex = $state(0);
  let restoreInterval: number;
  let isDrawing = false;

  // 1. Load Image
  $effect(() => {
      if (isOpen && finalImage) {
          const img = new Image();
          img.crossOrigin = "Anonymous";
          img.src = finalImage;
          img.onload = () => {
              loadedFinalImage = img;
              // Force draw immediately after load
              drawFullState(true);
          };
      }
  });

  // 2. Handle Resize / Init
  $effect(() => {
      if (isOpen && width > 0 && height > 0 && canvas) {
          canvas.width = width;
          canvas.height = height;
          ctx = canvas.getContext('2d');
          
          if (loadedFinalImage) {
              drawFullState(true);
          } else {
              // Placeholder while loading
              if (ctx) {
                  ctx.fillStyle = '#141210';
                  ctx.fillRect(0, 0, width, height);
              }
          }
      }
  });

  function drawFullState(reset = false) {
      if (!ctx || !loadedFinalImage) return;

      const cw = canvas.width;
      const ch = canvas.height;
      const iw = loadedFinalImage.width;
      const ih = loadedFinalImage.height;
      const scale = Math.max(cw / iw, ch / ih);
      const x = (cw - iw * scale) / 2;
      const y = (ch - ih * scale) / 2;
      
      ctx.globalCompositeOperation = 'source-over';
      
      if (reset) {
          // Draw Opaque Final Image (The Mirror Reflection)
          // We add a slight blur to simulate "The Mirror Surface"
          ctx.filter = 'blur(2px) contrast(1.1)'; 
          ctx.globalAlpha = 1.0; 
          ctx.drawImage(loadedFinalImage, x, y, iw * scale, ih * scale);
          
          // Add "Steam" Tint
          ctx.fillStyle = "rgba(200, 220, 230, 0.1)";
          ctx.fillRect(0,0, cw, ch);
          ctx.filter = 'none';
      } else {
          // Restoration (Fogging up again)
          ctx.globalAlpha = 0.03; // Speed of restoration
          ctx.filter = 'blur(4px)';
          ctx.drawImage(loadedFinalImage, x, y, iw * scale, ih * scale);
          ctx.filter = 'none';
          ctx.globalAlpha = 1.0;
      }
  }

  function restoreFog() {
      if (isOpen && !isDrawing) {
          drawFullState(false);
      }
  }

  // Input Handling
  function getPos(e: MouseEvent | TouchEvent) {
      const rect = canvas.getBoundingClientRect();
      let cx, cy;
      if (e instanceof MouseEvent) {
          cx = e.clientX;
          cy = e.clientY;
      } else {
          cx = e.touches[0].clientX;
          cy = e.touches[0].clientY;
      }
      return { x: cx - rect.left, y: cy - rect.top };
  }

  function startDraw(e: MouseEvent | TouchEvent) {
      isDrawing = true;
      draw(e);
  }

  function stopDraw() {
      isDrawing = false;
      ctx?.beginPath();
  }

  function draw(e: MouseEvent | TouchEvent) {
      if (!isDrawing || !ctx) return;
      // Prevent scroll on touch
      if (window.TouchEvent && e instanceof TouchEvent) e.preventDefault();

      const { x, y } = getPos(e);
      
      // Erase Top Layer to reveal Bottom Layer
      ctx.globalCompositeOperation = 'destination-out';
      ctx.beginPath();
      ctx.arc(x, y, 50, 0, Math.PI * 2);
      ctx.fill();
      ctx.globalCompositeOperation = 'source-over';
  }

  onMount(() => {
      restoreInterval = setInterval(restoreFog, 80);
  });

  onDestroy(() => {
      if (typeof window !== 'undefined') clearInterval(restoreInterval);
  });
</script>

{#if isOpen}
  <div 
    class="fixed inset-0 z-[80] bg-[#0a0806]/95 backdrop-blur-md flex flex-col items-center justify-center p-4"
    transition:fade={{ duration: 800 }}
  >
    <div class="absolute top-8 text-center pointer-events-none z-10">
        <h2 class="font-['UnifrakturMaguntia'] text-4xl text-[#d4c5b0] opacity-80 mb-2">Зеркало Памяти</h2>
        <p class="font-['Cinzel'] text-xs text-[#8a7f70] tracking-[0.3em] uppercase">
            Протрите стекло, чтобы увидеть прошлое
        </p>
    </div>

    <!-- CONTAINER -->
    <div 
        bind:clientWidth={width}
        bind:clientHeight={height}
        bind:this={container}
        class="relative w-full max-w-4xl aspect-[4/3] bg-[#141210] shadow-[0_0_100px_rgba(0,0,0,0.8)] border border-[#2a2622] rounded-sm overflow-hidden select-none"
    >
        <!-- LAYER 1 (BOTTOM): THE PAST (Sketch) -->
        <!-- This is what appears when you wipe -->
        {#key currentStepIndex}
            <div 
                class="absolute inset-0 flex items-center justify-center bg-[#F5F1E6]"
                transition:fade={{ duration: 500 }}
            >
                {#if steps[currentStepIndex]}
                    <img 
                        src={steps[currentStepIndex].imageUrl} 
                        alt="Past Step" 
                        class="w-full h-full object-cover sepia-[0.6] contrast-[0.9] opacity-90"
                    />
                    <!-- Overlay Note -->
                    <div class="absolute bottom-8 left-0 right-0 text-center pointer-events-none">
                        <span class="bg-black/60 backdrop-blur-sm px-6 py-2 font-['Cormorant_Garamond'] text-2xl text-[#d4c5b0] italic rounded-full border border-[#d4c5b0]/20">
                             {steps[currentStepIndex].description}
                        </span>
                    </div>
                {/if}
                <!-- Texture -->
                <div class="absolute inset-0 bg-[url('https://www.transparenttextures.com/patterns/aged-paper.png')] opacity-30 mix-blend-multiply"></div>
            </div>
        {/key}

        <!-- LAYER 2 (TOP): THE PRESENT (Canvas) -->
        <!-- This covers the past. We erase it. -->
        <canvas
            bind:this={canvas}
            class="absolute inset-0 w-full h-full cursor-crosshair touch-none"
            onmousedown={startDraw}
            onmouseup={stopDraw}
            onmouseleave={stopDraw}
            onmousemove={draw}
            ontouchstart={startDraw}
            ontouchend={stopDraw}
            ontouchmove={draw}
        ></canvas>

        <!-- DECORATIONS -->
        <div class="absolute inset-0 pointer-events-none border-[1px] border-[#d4c5b0]/10 m-4"></div>
        <div class="absolute inset-0 pointer-events-none shadow-[inset_0_0_150px_rgba(0,0,0,0.9)]"></div>

    </div>

    <!-- CONTROLS -->
    <div class="mt-8 flex gap-4 z-50">
        {#each steps as step, i}
            <button
                class="group flex flex-col items-center gap-2 transition-all duration-300 {currentStepIndex === i ? 'opacity-100 scale-110' : 'opacity-40 hover:opacity-80'}"
                onclick={() => currentStepIndex = i}
            >
                <div class="w-12 h-12 rounded-full border border-[#8a7f70] overflow-hidden relative shadow-lg bg-black">
                    <img src={step.imageUrl} alt="" class="w-full h-full object-cover grayscale" />
                    {#if currentStepIndex === i}
                        <div class="absolute inset-0 border-2 border-[#d4c5b0] rounded-full"></div>
                    {/if}
                </div>
                <span class="font-['Cinzel'] text-[10px] uppercase tracking-widest text-[#8a7f70]">{step.stepType}</span>
            </button>
        {/each}
    </div>

    <!-- Close -->
    <button 
        onclick={onClose}
        class="absolute top-8 right-8 text-[#5c4d41] hover:text-[#d4c5b0] transition-colors font-['UnifrakturMaguntia'] text-3xl z-50"
    >
        ✕
    </button>
  </div>
{/if}
