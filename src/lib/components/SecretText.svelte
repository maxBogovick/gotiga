<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';

  let { text = '', isCandleLit = false } = $props();

  let mouseX = $state(-1000);
  let mouseY = $state(-1000);
  let element: HTMLElement;
  let isRevealed = $state(false);
  
  // Generate cipher string once
  const symbols = "†‡§ℵℶℷℸ℺℻∂∆∏∑−∫≈≠≡≤≥";
  let cipherText = $derived(
      text.split('').map(char => {
          if (char === ' ') return ' ';
          return symbols[Math.floor(Math.random() * symbols.length)];
      }).join('')
  );

  function handleMouseMove(e: MouseEvent) {
      if (!isCandleLit) return;
      mouseX = e.clientX;
      mouseY = e.clientY;
      
      if (element) {
          const rect = element.getBoundingClientRect();
          const elX = rect.left + rect.width / 2;
          const elY = rect.top + rect.height / 2;
          const dist = Math.sqrt(Math.pow(mouseX - elX, 2) + Math.pow(mouseY - elY, 2));
          
          // Reveal radius: 150px
          isRevealed = dist < 150;
      }
  }

  $effect(() => {
      if (isCandleLit) {
          window.addEventListener('mousemove', handleMouseMove);
      } else {
          window.removeEventListener('mousemove', handleMouseMove);
          isRevealed = false; // Reset when candle off
      }
      return () => {
          if (typeof window !== 'undefined') window.removeEventListener('mousemove', handleMouseMove);
      };
  });
</script>

<div bind:this={element} class="relative inline-block select-none cursor-help transition-all duration-1000">
    
    <!-- THE CIPHER (Always visible when not revealed) -->
    <span 
        class="font-['Cinzel'] text-xl tracking-widest transition-all duration-700
        {isRevealed ? 'opacity-0 blur-sm scale-95' : 'opacity-40 blur-[1px]'}
        {isCandleLit ? 'text-[#8a7f70]' : 'text-[#2a2622]'}"
    >
        {cipherText}
    </span>

    <!-- THE REVEALED TEXT (Visible only when revealed) -->
    <span 
        class="absolute inset-0 font-['Cormorant_Garamond'] text-2xl italic tracking-wide text-[#ffcc00] drop-shadow-[0_0_8px_rgba(255,200,0,0.8)]
        transition-all duration-500 transform
        {isRevealed ? 'opacity-100 scale-100' : 'opacity-0 scale-105'}"
    >
        {text}
    </span>
    
    <!-- Hint particle if candle is NOT lit? -->
    {#if !isCandleLit}
        <div class="absolute -right-4 -top-2 text-[10px] text-[#2a2622] opacity-50 animate-pulse">?</div>
    {/if}

</div>
