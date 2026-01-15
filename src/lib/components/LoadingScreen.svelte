<script lang="ts">
  import { fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

  interface Props {
    message?: string;
  }

  let { message = '' }: Props = $props();
</script>

<div
  class="min-h-screen bg-cabinet-bg flex items-center justify-center"
  in:fade={{ duration: 300, easing: cubicOut }}
  out:fade={{ duration: 200, easing: cubicOut }}
>
  <div class="text-center">
    <!-- Три точки с анимацией -->
    <div class="flex justify-center gap-1 mb-4" aria-hidden="true">
      <span class="w-1.5 h-1.5 bg-cabinet-dust rounded-full animate-pulse" style="animation-delay: 0ms;"></span>
      <span class="w-1.5 h-1.5 bg-cabinet-dust rounded-full animate-pulse" style="animation-delay: 150ms;"></span>
      <span class="w-1.5 h-1.5 bg-cabinet-dust rounded-full animate-pulse" style="animation-delay: 300ms;"></span>
    </div>

    {#if message}
      <p class="text-cabinet-dust text-sm">{message}</p>
    {/if}
  </div>
</div>
