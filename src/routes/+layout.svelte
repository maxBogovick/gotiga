<script lang="ts">
  import '../app.css';
  import { onNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import SiteHeader from '$lib/components/SiteHeader.svelte';
  import DustParticles from '$lib/components/DustParticles.svelte';

  let canonicalUrl = $derived(`${page.url.origin}${page.url.pathname}`);

  let { children } = $props();

  let showSiteHeader = $derived(!page.url.pathname.startsWith('/admin'));
  let hasHeaderOffset = $derived(showSiteHeader && page.url.pathname !== '/');
  // Detail page has its own DustParticles at higher intensity — skip in layout to avoid double canvas
  let showDust = $derived(showSiteHeader && !page.url.pathname.startsWith('/figurines/'));

  onNavigate((navigation) => {
    if (!('startViewTransition' in document)) return;
    return new Promise<void>((resolve) => {
      (document as Document & { startViewTransition(cb: () => Promise<void>): void })
        .startViewTransition(async () => {
          resolve();
          await navigation.complete;
        });
    });
  });
</script>

<svelte:head>
  <link rel="canonical" href={canonicalUrl} />
</svelte:head>

<div class="min-h-screen bg-[#f8f1e7]">
  {#if showSiteHeader}
    <SiteHeader />
  {/if}

  {#if showDust}
    <DustParticles opacity={0.2} />
  {/if}

  <main class="min-h-screen" class:with-site-header={hasHeaderOffset}>
    {@render children()}
  </main>
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
