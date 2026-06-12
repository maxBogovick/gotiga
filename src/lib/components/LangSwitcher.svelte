<script lang="ts">
  import { lang, setLang, type Lang } from '$lib/i18n';

  // optional: a variant for the dark top-bar vs a light page context
  let { variant = 'light' }: { variant?: 'dark' | 'light' } = $props();

  const baseClass = 'text-[10px] tracking-[0.15em] uppercase transition-colors duration-200 px-1.5 py-0.5';
  const activeClass = $derived(variant === 'dark' ? 'text-[#fff9f0]' : 'text-[#34251c]');
  const inactiveClass = $derived(variant === 'dark' ? 'text-[#fff9f0]/65 hover:text-[#fff9f0]/90' : 'text-[#7c6554]/72 hover:text-[#5f4636]');
  const sepClass = $derived(variant === 'dark' ? 'text-[#fff9f0]/40' : 'text-[#7c6554]/48');
</script>

<div class="flex items-center gap-0" role="group" aria-label="Language">
  {#each (['en', 'ru'] as Lang[]) as l, i}
    {#if i > 0}
      <span class="{sepClass} select-none text-[10px]">/</span>
    {/if}
    <button
      onclick={() => setLang(l)}
      class="{baseClass} {$lang === l ? activeClass : inactiveClass}"
      aria-current={$lang === l ? 'true' : undefined}
    >{l.toUpperCase()}</button>
  {/each}
</div>
