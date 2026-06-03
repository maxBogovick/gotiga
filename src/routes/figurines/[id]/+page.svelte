<script lang="ts">
  import { page } from '$app/state';
  import { fade } from 'svelte/transition';
  import DustParticles from '$lib/components/DustParticles.svelte';
  import FigurineDetailView from '$lib/components/FigurineDetailView.svelte';
  import { t } from '$lib/i18n';

  let { data } = $props();
  let figurine = $derived(data.figurine);
  let id = $derived(page.params.id ?? '');

  // Absolute URL for OG image (relative paths get current-origin prepended)
  let ogImage = $derived(() => {
    const img = figurine?.images?.find((i: { imageType: string }) => i.imageType === 'face')?.url
      ?? figurine?.images?.[0]?.url ?? '';
    if (!img) return '';
    if (img.startsWith('http')) return img;
    return typeof window !== 'undefined' ? `${window.location.origin}${img}` : img;
  });

  let statusAvailability = $derived(
    figurine?.status === 'available'
      ? 'https://schema.org/InStock'
      : figurine?.status === 'reserved'
        ? 'https://schema.org/LimitedAvailability'
        : 'https://schema.org/SoldOut'
  );

  let jsonLd = $derived(() => JSON.stringify({
    '@context': 'https://schema.org',
    '@type': 'Product',
    name: figurine?.name ?? '',
    description: figurine?.shortText ?? figurine?.fullDescription ?? '',
    image: ogImage(),
    offers: {
      '@type': 'Offer',
      availability: statusAvailability,
      priceCurrency: 'RUB',
    },
    ...(figurine?.material ? { material: figurine.material } : {}),
  }));
</script>

<svelte:head>
  <title>{figurine?.name ?? $t('zoneShowcase')} — Gothic Miniatures</title>
  <meta name="description" content={figurine?.shortText ?? figurine?.fullDescription ?? 'Gothic handcrafted miniature'} />

  <!-- Open Graph -->
  <meta property="og:type"        content="product" />
  <meta property="og:title"       content="{figurine?.name ?? 'Gothic Miniature'} — Gotiga" />
  <meta property="og:description" content={figurine?.shortText ?? figurine?.fullDescription ?? ''} />
  {#if ogImage()}
    <meta property="og:image" content={ogImage()} />
    <meta property="og:image:width"  content="1800" />
    <meta property="og:image:height" content="1800" />
  {/if}
  <meta property="og:url" content={typeof window !== 'undefined' ? window.location.href : ''} />

  <!-- Twitter / X -->
  <meta name="twitter:card"        content="summary_large_image" />
  <meta name="twitter:title"       content="{figurine?.name ?? 'Gothic Miniature'} — Gotiga" />
  <meta name="twitter:description" content={figurine?.shortText ?? ''} />
  {#if ogImage()}
    <meta name="twitter:image" content={ogImage()} />
  {/if}

  <!-- JSON-LD (п.7) -->
  {#if figurine}
    {@html `<script type="application/ld+json">${jsonLd()}<\/script>`}
  {/if}

  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=Fraunces:opsz,wght@9..144,500;9..144,650;9..144,750&display=swap" rel="stylesheet">
</svelte:head>

<div class="fixed inset-0 bg-cabinet-bg -z-50"></div>
<div class="fixed inset-0 pointer-events-none z-0 bg-noise opacity-[0.08] mix-blend-overlay"></div>
<div class="fixed inset-0 pointer-events-none z-0 detail-backdrop"></div>

<DustParticles />

{#if !figurine}
  <!-- Skeleton (п.5) — показывается только при прямом переходе без prefetch -->
  <div class="min-h-screen py-10 px-6 sm:px-12 max-w-7xl mx-auto" out:fade>
    <div class="h-3 w-24 bg-[#34251c]/8 rounded animate-pulse mb-14"></div>
    <div class="grid grid-cols-1 lg:grid-cols-[minmax(0,5fr)_minmax(0,6fr)] gap-12 lg:gap-20">
      <!-- Gallery skeleton -->
      <div>
        <div class="relative aspect-[4/5] bg-[#34251c]/6 overflow-hidden">
          <div class="absolute inset-0 -translate-x-full animate-[shimmer_1.6s_infinite] bg-gradient-to-r from-transparent via-[#fff9f0]/60 to-transparent"></div>
        </div>
        <div class="flex gap-2 mt-3">
          {#each Array(3) as _}
            <div class="w-14 h-14 bg-[#34251c]/6 animate-pulse"></div>
          {/each}
        </div>
      </div>
      <!-- Details skeleton -->
      <div class="space-y-5 pt-2">
        <div class="h-2.5 w-32 bg-[#34251c]/8 rounded animate-pulse"></div>
        <div class="h-11 w-3/4 bg-[#34251c]/10 rounded animate-pulse"></div>
        <div class="h-3 w-20 bg-[#34251c]/6 rounded animate-pulse"></div>
        <div class="space-y-2 pt-2">
          {#each Array(5) as _}
            <div class="h-3 w-full bg-[#34251c]/6 rounded animate-pulse"></div>
          {/each}
        </div>
        <div class="border-t border-[#34251c]/8 pt-5 space-y-3">
          {#each [28, 20, 32] as w}
            <div class="flex gap-6">
              <div class="h-2.5 w-16 bg-[#34251c]/8 rounded animate-pulse"></div>
              <div class="h-2.5 bg-[#34251c]/6 rounded animate-pulse" style="width:{w}%"></div>
            </div>
          {/each}
        </div>
        <div class="h-12 w-48 bg-[#34251c]/8 rounded animate-pulse mt-2"></div>
      </div>
    </div>
  </div>
{:else}
  <FigurineDetailView {figurine} {id} />
{/if}

<style>
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }

  .detail-backdrop {
    background:
      radial-gradient(ellipse 70% 55% at 72% 38%, rgba(198,95,60,0.07) 0%, transparent 65%),
      radial-gradient(ellipse 50% 70% at 18% 72%, rgba(201,168,117,0.06) 0%, transparent 60%),
      #f8f1e7;
  }
</style>
