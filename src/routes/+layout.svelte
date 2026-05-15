<script lang="ts">
  import '../app.css';
  import { fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { page } from '$app/state';
  import LangSwitcher from '$lib/components/LangSwitcher.svelte';

  let { children } = $props();

  let key = $derived(page.url.pathname);

  // hide the global switcher on admin (it has its own space) and home (it has a custom bar)
  let hideSwitcher = $derived(
    page.url.pathname === '/' || page.url.pathname.startsWith('/admin')
  );
</script>

<div class="min-h-screen bg-[#0a0806]">
  <!-- Persistent language switcher — top-right, shown on all content pages -->
  {#if !hideSwitcher}
    <div class="fixed top-4 right-5 z-50">
      <LangSwitcher />
    </div>
  {/if}

  {#key key}
    <main
      class="min-h-screen"
      in:fade={{ duration: 600, delay: 300, easing: cubicOut }}
      out:fade={{ duration: 400, easing: cubicOut }}
    >
      {@render children()}
    </main>
  {/key}
</div>
