<script lang="ts">
  import { goto } from '$app/navigation';
  import { fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

  interface Props {
    title?: string;
    message?: string;
    backHref?: string;
    backLabel?: string;
  }

  let {
    title = 'Не найдено',
    message = 'То, что вы ищете, здесь больше не обитает.',
    backHref = '/',
    backLabel = 'Вернуться в кабинет'
  }: Props = $props();
</script>

<div
  class="min-h-screen bg-cabinet-bg flex items-center justify-center p-8"
  in:fade={{ duration: 400, easing: cubicOut }}
>
  <div class="text-center max-w-md">
    <h1 class="font-display text-3xl text-cabinet-bone mb-4">{title}</h1>
    <p class="text-cabinet-dust mb-8 leading-relaxed">{message}</p>

    <button
      class="btn-cabinet"
      onclick={() => goto(backHref)}
    >
      {backLabel}
    </button>
  </div>
</div>
