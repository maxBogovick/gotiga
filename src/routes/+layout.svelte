<script lang="ts">
  import '../app.css';
  import { fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { page } from '$app/state';
  import SiteHeader from '$lib/components/SiteHeader.svelte';

  let { children } = $props();

  let key = $derived(page.url.pathname);
  let showSiteHeader = $derived(!page.url.pathname.startsWith('/admin'));
  let hasHeaderOffset = $derived(showSiteHeader && page.url.pathname !== '/');
</script>

<div class="min-h-screen bg-[#f8f1e7]">
  {#if showSiteHeader}
    <SiteHeader />
  {/if}

  {#key key}
    <main
      class="min-h-screen"
      class:with-site-header={hasHeaderOffset}
      in:fade={{ duration: 600, delay: 300, easing: cubicOut }}
      out:fade={{ duration: 400, easing: cubicOut }}
    >
      {@render children()}
    </main>
  {/key}
</div>

<style>
  .with-site-header {
    padding-top: 68px;
  }

  @media (max-width: 680px) {
    .with-site-header {
      padding-top: 58px;
    }
  }
</style>
