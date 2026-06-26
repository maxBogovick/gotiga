<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';

  let { isActive = false } = $props();

  let x = $state(-100);
  let y = $state(-100);
  let reduced = $state(false);

  let rawX = -100;
  let rawY = -100;
  let rafId: number | null = null;

  onMount(() => {
    reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  });

  function scheduleUpdate() {
    if (rafId !== null) return;
    rafId = requestAnimationFrame(() => {
      x = rawX;
      y = rawY;
      rafId = null;
    });
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isActive) return;
    rawX = e.clientX;
    rawY = e.clientY;
    scheduleUpdate();
  }

  function handleTouchMove(e: TouchEvent) {
    if (!isActive || e.touches.length === 0) return;
    rawX = e.touches[0].clientX;
    rawY = e.touches[0].clientY;
    scheduleUpdate();
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
    if (rafId !== null) cancelAnimationFrame(rafId);
  });
</script>

{#if isActive && !reduced}
  <div class="fixed inset-0 pointer-events-none z-[100] overflow-hidden" transition:fade={{ duration: 1000 }}>
    <!-- Darkening layer to make the room feel dimmer -->
    <div class="absolute inset-0 bg-[#2f170e]/[0.06]"></div>

    <!-- The Candle Light -->
    <div
      class="absolute rounded-full pointer-events-none mix-blend-soft-light transition-opacity duration-100"
      style="
        left: {x}px;
        top: {y}px;
        width: 360px;
        height: 360px;
        transform: translate(-50%, -50%);
        background: radial-gradient(circle, rgba(255, 214, 126, 0.82) 0%, rgba(211, 96, 41, 0.28) 36%, transparent 72%);
        opacity: 0.62;
      "
    ></div>
    
    <!-- Secondary Glow (Warmth) -->
    <div
      class="absolute rounded-full pointer-events-none mix-blend-multiply"
      style="
        left: {x}px;
        top: {y}px;
        width: 560px;
        height: 560px;
        transform: translate(-50%, -50%);
        background: radial-gradient(circle, transparent 0%, transparent 42%, rgba(78, 33, 18, 0.08) 72%, transparent 100%);
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
