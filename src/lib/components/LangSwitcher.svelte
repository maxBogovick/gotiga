<script lang="ts">
  import { lang, setLang, type Lang } from '$lib/i18n';

  // optional: a variant for the dark top-bar (default) vs a light context
  let { variant = 'dark' }: { variant?: 'dark' | 'light' } = $props();

  const baseClass = 'text-[10px] tracking-[0.15em] font-[\'Cinzel\'] uppercase transition-colors duration-200 px-1.5 py-0.5';
  const activeClass  = variant === 'dark' ? 'text-[#d4c5b0]'       : 'text-[#1a1816]';
  const inactiveClass = variant === 'dark' ? 'text-[#8a7f70]/50 hover:text-[#8a7f70]' : 'text-[#5a524c]/50 hover:text-[#5a524c]';
  const sepClass = variant === 'dark' ? 'text-[#8a7f70]/30' : 'text-[#5a524c]/30';
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
