<script lang="ts">
  import { fade } from 'svelte/transition';

  interface Props {
    message?: string;
  }

  let { message = 'Извлечение из хранилища...' }: Props = $props();
</script>

<div
  class="fixed inset-0 z-50 flex flex-col items-center justify-center bg-[#0a0806]"
  in:fade={{ duration: 300 }}
  out:fade={{ duration: 500 }}
>
  <!-- Noise overlay -->
  <div class="absolute inset-0 bg-noise opacity-[0.06] mix-blend-overlay pointer-events-none" aria-hidden="true"></div>

  <!-- Vignette -->
  <div class="absolute inset-0 bg-vignette pointer-events-none" aria-hidden="true"></div>

  <!-- Content -->
  <div class="relative z-10 text-center">
    <!-- Spinning border -->
    <div
      class="w-16 h-16 border border-[#d4c5b0]/20 border-t-[#d4c5b0]/60 rounded-full animate-spin mb-6 mx-auto"
      aria-hidden="true"
    ></div>

    <!-- Message -->
    <p
      class="font-cinzel text-[#8a7f70] text-xs tracking-[0.4em] uppercase animate-pulse"
      aria-live="polite"
    >
      {message}
    </p>
  </div>
</div>

<style>
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
