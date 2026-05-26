<script lang="ts">
  import { goto } from '$app/navigation';
  import { fade } from 'svelte/transition';
  import { t } from '$lib/i18n';

  interface Props {
    href?: string;
    label?: string;
  }

  let { href = '/', label }: Props = $props();
  let displayLabel = $derived(label ?? $t('notFoundBack'));

  function handleClick() {
    goto(href);
  }
</script>

<button
  class="group inline-flex items-center gap-3 font-cinzel text-[10px] tracking-[0.08em] uppercase
         text-[#5f4636] hover:text-[#34251c] transition-all duration-500
         focus:outline-none focus-visible:ring-1 focus-visible:ring-[#34251c]/50"
  onclick={handleClick}
  in:fade={{ duration: 800 }}
>
  <span
    class="transform transition-transform duration-500 group-hover:-translate-x-1"
    aria-hidden="true"
  >
    ←
  </span>
  <span>{displayLabel}</span>
</button>
