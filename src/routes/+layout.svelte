<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { onNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import SiteHeader from '$lib/components/SiteHeader.svelte';
  import DustParticles from '$lib/components/DustParticles.svelte';
  import { themeConfig, themeCSS, startListeningForPreview } from '$lib/stores/theme.svelte';
  import { setCopyOverrides } from '$lib/i18n';
  import { api } from '$lib/api';
  import type { Lang } from '$lib/i18n';

  let canonicalUrl = $derived(`${page.url.origin}${page.url.pathname}`);
  let { children } = $props();
  let showSiteHeader = $derived(!page.url.pathname.startsWith('/admin'));
  let hasHeaderOffset = $derived(showSiteHeader && page.url.pathname !== '/');
  // Detail page has its own DustParticles at higher intensity — skip in layout to avoid double canvas
  let showDust = $derived(showSiteHeader && !page.url.pathname.startsWith('/figurines/'));

  let stopPreviewListener: (() => void) | null = null;

  onMount(() => {
    // Load theme and copy overrides
    Promise.all([
      api.getThemeConfig().catch(() => null),
      api.getCopyOverrides().catch(() => null),
    ]).then(([themeData, copyData]) => {
      if (themeData) themeConfig.set(themeData);
      if (copyData) setCopyOverrides(copyData as Record<Lang, Record<string, string>>);
    });
    // Listen for live preview broadcasts on all non-admin pages
    if (!page.url.pathname.startsWith('/admin')) {
      stopPreviewListener = startListeningForPreview();
    }
  });

  onDestroy(() => stopPreviewListener?.());

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
  {#if $themeCSS}
    {@html `<style id="theme-override">${$themeCSS}</style>`}
  {/if}
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
