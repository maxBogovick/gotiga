<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';

  let { isActive = false } = $props();

  let x = $state(-100);
  let y = $state(-100);
  let reduced = $state(false);

  onMount(() => {
    reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  });

  function handleMouseMove(e: MouseEvent) {
    if (!isActive) return;
    x = e.clientX;
    y = e.clientY;
  }

  function handleTouchMove(e: TouchEvent) {
    if (!isActive || e.touches.length === 0) return;
    x = e.touches[0].clientX;
    y = e.touches[0].clientY;
  }

  onMount(() => {
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('touchmove', handleTouchMove, { passive: true });
  });

  onDestroy(() => {
      if (typeof window !== 'undefined') {
          window.removeEventListener('mousemove', handleMouseMove);
          window.removeEventListener('touchmove', handleTouchMove);
      }
  });
</script>

{#if isActive && !reduced}
  <div class="fixed inset-0 pointer-events-none z-[100] overflow-hidden" transition:fade={{ duration: 1000 }}>
    <!-- Darkening layer to make the room feel dimmer -->
    <div class="absolute inset-0 bg-[#6f3b24]/10"></div>

    <!-- The Candle Light -->
    <div
      class="absolute rounded-full pointer-events-none mix-blend-color-dodge transition-opacity duration-100"
      style="
        left: {x}px;
        top: {y}px;
        width: 400px;
        height: 400px;
        transform: translate(-50%, -50%);
        background: radial-gradient(circle, rgba(255, 200, 100, 1) 0%, rgba(200, 100, 50, 0.5) 30%, transparent 70%);
        opacity: 0.8;
      "
    ></div>
    
    <!-- Secondary Glow (Warmth) -->
    <div
      class="absolute rounded-full pointer-events-none mix-blend-screen"
      style="
        left: {x}px;
        top: {y}px;
        width: 600px;
        height: 600px;
        transform: translate(-50%, -50%);
        background: radial-gradient(circle, rgba(255, 100, 50, 0.2) 0%, transparent 60%);
      "
    ></div>
    
    <!-- Cursor Flame Icon -->
    <div 
        class="absolute pointer-events-none text-2xl filter drop-shadow-[0_0_10px_rgba(255,160,0,0.8)] animate-pulse"
        style="
            left: {x}px;
            top: {y}px;
            transform: translate(-50%, -120%);
        "
    >
        🔥
    </div>

  </div>
{/if}

<style>
  /* Global style for secret text to react to this light */
  /* This needs to be globally available or applied to specific elements */
  :global(.secret-ink) {
    color: #d8c6b1;
    transition: color 1s ease;
    user-select: none;
  }
  
  /* When candle is active, we rely on mix-blend-mode to reveal it. 
     Alternatively, we could use a different technique if blend modes are tricky with text colors.
     
     Actually, let's use a simpler CSS variable approach for the text itself?
     No, the mix-blend-mode `color-dodge` over warm parchment text on `#f8f1e7` background 
     should make the text pop out as golden/bright when the orange light hits it.
  */
</style>
