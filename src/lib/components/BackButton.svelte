<script lang="ts">
  import { goto } from '$app/navigation';
  import { fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

  interface Props {
    href?: string;
    label?: string;
    delay?: number;
  }

  let { href = '/', label = 'Назад в кабинет', delay = 200 }: Props = $props();

  function handleClick() {
    goto(href);
  }
</script>

<button
  class="group flex items-center gap-2 text-cabinet-dust
         hover:text-cabinet-bone transition-colors duration-400"
  onclick={handleClick}
  in:fade={{ delay, duration: 400, easing: cubicOut }}
>
  <span
    class="transform group-hover:-translate-x-1 transition-transform duration-300"
    aria-hidden="true"
  >←</span>
  <span class="text-sm">{label}</span>
</button>
