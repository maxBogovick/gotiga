<script lang="ts">
  import { goto } from '$app/navigation';
  import { fade } from 'svelte/transition';

  interface Props {
    title?: string;
    message?: string;
    backHref?: string;
    backLabel?: string;
  }

  let {
    title = 'Увы',
    message = 'Сей экспонат утрачен или никогда не существовал в реестре.',
    backHref = '/',
    backLabel = 'Вернуться в зал'
  }: Props = $props();
</script>

<div class="fixed inset-0 bg-[#0a0806] -z-50"></div>
<div class="fixed inset-0 bg-noise opacity-[0.06] mix-blend-overlay pointer-events-none" aria-hidden="true"></div>
<div class="fixed inset-0 bg-vignette pointer-events-none" aria-hidden="true"></div>

<div
  class="min-h-screen flex flex-col items-center justify-center p-8 relative z-10"
  in:fade={{ duration: 800 }}
>
  <div class="text-center max-w-md">
    <!-- Gothic Title -->
    <h1 class="font-gothic text-5xl text-[#8a7f70] mb-6 opacity-80">
      {title}
    </h1>

    <!-- Decorative line -->
    <div class="w-24 h-px bg-gradient-to-r from-transparent via-[#d4c5b0]/30 to-transparent mx-auto mb-8" aria-hidden="true"></div>

    <!-- Message -->
    <p class="font-cinzel text-[#d4c5b0] mb-10 leading-relaxed text-sm tracking-wide">
      {message}
    </p>

    <!-- Button -->
    <button
      class="btn-gothic"
      onclick={() => goto(backHref)}
    >
      {backLabel}
    </button>
  </div>
</div>
