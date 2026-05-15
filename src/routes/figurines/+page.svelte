<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import type { FigurineListItem } from '$lib/types/api';
  import { fade } from 'svelte/transition';
  import { BackButton, LoadingScreen } from '$lib/components';
  import { t } from '$lib/i18n';

  type StatusFilter = 'all' | 'available' | 'reserved' | 'sold';

  // State
  let figurines = $state<FigurineListItem[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let prefersReducedMotion = $state(false);
  let searchQuery = $state('');
  let statusFilter = $state<StatusFilter>('all');

  const PAGE_SIZE = 12;
  let displayLimit = $state(PAGE_SIZE);

  let filtered = $derived(
    figurines
      .filter(f => statusFilter === 'all' || f.status === statusFilter)
      .filter(f => !searchQuery.trim() || f.name.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  let visible = $derived(filtered.slice(0, displayLimit));
  let hasMore = $derived(filtered.length > displayLimit);

  $effect(() => {
    // Reset pagination whenever the filter/search changes
    void statusFilter;
    void searchQuery;
    displayLimit = PAGE_SIZE;
  });

  let statusCounts = $derived({
    all: figurines.length,
    available: figurines.filter(f => f.status === 'available').length,
    reserved: figurines.filter(f => f.status === 'reserved').length,
    sold: figurines.filter(f => f.status === 'sold').length,
  });

  let countText = $derived(() => {
    const total = figurines.length;
    const shown = filtered.length;
    if (total === 0) return $t('archiveEmpty');
    if (statusFilter !== 'all' || searchQuery.trim()) {
      return `${shown} / ${toRoman(total)}`;
    }
    return `${toRoman(total)}`;
  });

  // Helper для римских цифр (для атмосферы)
  function toRoman(num: number): string {
    const lookup: Record<string, number> = {M:1000,CM:900,D:500,CD:400,C:100,XC:90,L:50,XL:40,X:10,IX:9,V:5,IV:4,I:1};
    let roman = '', i;
    for ( i in lookup ) {
      while ( num >= lookup[i] ) {
        roman += i;
        num -= lookup[i];
      }
    }
    return roman;
  }

  // Lifecycle
  onMount(async () => {
    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    prefersReducedMotion = mediaQuery.matches;

    try {
      figurines = await api.getAllFigurines();
      if (figurines.length < 5) await new Promise(r => setTimeout(r, 500));
    } catch (e) {
      console.error('Failed to load figurines:', e);
      error = 'archive_damaged';
    } finally {
      isLoading = false;
    }
  });
</script>

<svelte:head>
  <title>Archive — Gothic Miniatures Collection</title>
  <meta name="description" content="Gothic miniatures registry" />
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
  <link href="https://fonts.googleapis.com/css2?family=Cinzel:wght@400;600&family=UnifrakturMaguntia&display=swap" rel="stylesheet">
</svelte:head>

<div class="fixed inset-0 bg-[#0a0806] -z-50"></div>
<div class="fixed inset-0 pointer-events-none z-0 bg-noise opacity-[0.07] mix-blend-overlay"></div>
<div class="fixed inset-0 pointer-events-none z-0 bg-[radial-gradient(circle_at_50%_50%,transparent_0%,#0a0806_90%)]"></div>

{#if isLoading}
  <div class="min-h-screen flex flex-col items-center justify-center z-50 text-[#d4c5b0]" out:fade>
    <div class="w-16 h-16 border-t-2 border-b-2 border-[#d4c5b0]/30 rounded-full animate-spin mb-4"></div>
    <span class="font-['Cinzel'] tracking-[0.3em] text-xs animate-pulse">{$t('archiveStudying')}</span>
  </div>
{:else if error}
  <div class="min-h-screen flex items-center justify-center p-8 z-10 relative">
    <div class="text-center max-w-md border border-red-900/30 bg-black/40 p-10 backdrop-blur-sm">
      <h3 class="font-['UnifrakturMaguntia'] text-3xl text-red-900/60 mb-4">✕</h3>
      <p class="font-['Cinzel'] text-[#8a7f70] mb-8 text-sm tracking-wide">{$t('archiveDamaged')}</p>
      <button
              class="px-8 py-3 border border-[#d4c5b0]/20 text-[#d4c5b0] font-['Cinzel'] hover:bg-[#d4c5b0]/5 transition-colors uppercase text-xs tracking-widest"
              onclick={() => window.location.reload()}
      >
        {$t('archiveRepeat')}
      </button>
    </div>
  </div>
{:else}
  <div class="min-h-screen relative z-10 overflow-hidden font-['Cinzel'] text-[#d4c5b0]">

    <div class="container mx-auto px-6 sm:px-12 py-12">
      <div class="mb-16 sm:mb-24 border-b border-[#d4c5b0]/10 pb-6" in:fade={{ duration: 1000 }}>
        <div class="flex justify-between items-end mb-8">
          <div>
            <a href="/" class="group flex items-center text-xs tracking-[0.2em] text-[#8a7f70] hover:text-[#d4c5b0] transition-colors mb-4 opacity-60 hover:opacity-100">
              {$t('archiveBackLink')}
            </a>
            <h1 class="font-['UnifrakturMaguntia'] text-5xl sm:text-7xl text-[#e6decb] opacity-90 drop-shadow-2xl tracking-wide">
              {$t('archivePageTitle')}
            </h1>
          </div>
          <div class="hidden sm:block text-right">
            <p class="text-xs tracking-[0.4em] text-[#8a7f70] uppercase mb-1">{$t('archiveStatusCount')}</p>
            <p class="text-xl text-[#d4c5b0] border-l-2 border-[#d4c5b0]/20 pl-4">{countText()}</p>
          </div>
        </div>
        <!-- Search + Filters -->
        <div class="flex flex-col sm:flex-row gap-4 items-start sm:items-center">
          <div class="relative w-full sm:max-w-xs">
            <input
              bind:value={searchQuery}
              type="text"
              placeholder={$t('archiveSearchPlaceholder')}
              class="w-full bg-transparent border border-[#d4c5b0]/15 focus:border-[#d4c5b0]/40 px-4 py-2 text-xs tracking-widest text-[#d4c5b0] placeholder-[#8a7f70]/50 outline-none transition-colors font-['Cinzel'] uppercase"
            />
            {#if searchQuery}
              <button onclick={() => searchQuery = ''} class="absolute right-3 top-1/2 -translate-y-1/2 text-[#8a7f70] hover:text-[#d4c5b0] text-xs">✕</button>
            {/if}
          </div>

          <div class="flex flex-wrap gap-2" role="group" aria-label="Status filter">
            {#each ([
              { key: 'all',       labelKey: 'archiveStatusAll',       count: statusCounts.all },
              { key: 'available', labelKey: 'archiveStatusAvailable', count: statusCounts.available },
              { key: 'reserved',  labelKey: 'archiveStatusReserved',  count: statusCounts.reserved },
              { key: 'sold',      labelKey: 'archiveStatusSold',      count: statusCounts.sold },
            ] as const) as pill}
              {#if pill.key === 'all' || pill.count > 0}
                <button
                  onclick={() => statusFilter = pill.key}
                  class="flex items-center gap-1.5 px-3 py-1 text-[10px] tracking-widest uppercase border transition-all duration-200 font-['Cinzel']
                    {statusFilter === pill.key
                      ? 'border-[#d4c5b0]/50 text-[#d4c5b0] bg-[#d4c5b0]/8'
                      : 'border-[#d4c5b0]/10 text-[#8a7f70] hover:border-[#d4c5b0]/25 hover:text-[#d4c5b0]/70'}"
                >
                  {#if pill.key === 'available'}
                    <span class="w-1.5 h-1.5 rounded-full bg-emerald-500/70 flex-shrink-0"></span>
                  {:else if pill.key === 'reserved'}
                    <span class="w-1.5 h-1.5 rounded-full bg-amber-500/70 flex-shrink-0"></span>
                  {:else if pill.key === 'sold'}
                    <span class="w-1.5 h-1.5 rounded-full bg-red-800/70 flex-shrink-0"></span>
                  {/if}
                  {$t(pill.labelKey)}
                  <span class="opacity-40">{pill.count}</span>
                </button>
              {/if}
            {/each}
          </div>
        </div>
      </div>

      {#if filtered.length > 0}
        <ul class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-x-8 gap-y-16">
          {#each visible as figurine, i (figurine.id)}
            <li class="group perspective-container" in:fade={{ delay: i * 100, duration: 800 }}>
              <button
                      class="w-full text-left relative focus:outline-none"
                      onclick={() => goto(`/figurines/${figurine.id}`)}
                      aria-label="{figurine.name}"
              >
                <div class="relative aspect-[3/4] mb-6 overflow-hidden bg-[#141210] border border-[#d4c5b0]/10 shadow-2xl transition-all duration-700 group-hover:border-[#d4c5b0]/30 group-hover:shadow-[0_0_30px_-10px_rgba(212,197,176,0.15)] group-hover:-translate-y-2">

                  {#if figurine.faceImageUrl}
                    <img
                            src={figurine.faceImageUrl}
                            alt=""
                            class="w-full h-full object-cover opacity-70 grayscale transition-all duration-700 ease-out group-hover:opacity-100 group-hover:grayscale-0 group-hover:scale-105"
                            loading="lazy"
                    />
                  {:else}
                    <div class="w-full h-full flex items-center justify-center opacity-20">
                      <span class="font-['UnifrakturMaguntia'] text-2xl text-[#d4c5b0]">?</span>
                    </div>
                  {/if}

                  <div class="absolute inset-0 bg-[radial-gradient(circle_at_center,transparent_0%,rgba(10,8,6,0.8)_100%)] pointer-events-none transition-opacity duration-500 group-hover:opacity-60"></div>

                  <div class="absolute top-2 left-2 w-4 h-4 border-t border-l border-[#d4c5b0]/20 group-hover:border-[#d4c5b0]/60 transition-colors"></div>
                  <div class="absolute bottom-2 right-2 w-4 h-4 border-b border-r border-[#d4c5b0]/20 group-hover:border-[#d4c5b0]/60 transition-colors"></div>
                </div>

                <div class="pl-2 border-l border-transparent group-hover:border-[#d4c5b0]/40 transition-all duration-500">
                  <h2 class="font-['UnifrakturMaguntia'] text-xl sm:text-2xl text-[#d4c5b0] mb-1 group-hover:text-white transition-colors tracking-wide">
                    {figurine.name}
                  </h2>
                  <div class="flex items-center gap-2">
                    <p class="text-[10px] tracking-[0.2em] uppercase text-[#8a7f70] group-hover:text-[#d4c5b0]/70 transition-colors">
                      {$t('archiveExhibit')}{i + 1}
                    </p>
                    <span class="text-[#8a7f70]/30">·</span>
                    <span class="flex items-center gap-1 text-[10px] tracking-[0.15em] uppercase
                      {figurine.status === 'available' ? 'text-emerald-600/70' : figurine.status === 'reserved' ? 'text-amber-600/70' : 'text-[#5c544a]'}">
                      <span class="w-1 h-1 rounded-full flex-shrink-0
                        {figurine.status === 'available' ? 'bg-emerald-500/60' : figurine.status === 'reserved' ? 'bg-amber-500/60' : 'bg-[#5c544a]'}
                      "></span>
                      {figurine.status === 'available' ? $t('archiveStatusAvailableLabel') : figurine.status === 'reserved' ? $t('archiveStatusReservedLabel') : $t('archiveStatusSoldLabel')}
                    </span>
                  </div>
                </div>
              </button>
            </li>
          {/each}
        </ul>

        {#if hasMore}
          <div class="mt-20 flex justify-center" in:fade>
            <button
              onclick={() => displayLimit += PAGE_SIZE}
              class="group flex items-center gap-4 px-10 py-4 border border-[#d4c5b0]/20 hover:border-[#d4c5b0]/50 text-[#8a7f70] hover:text-[#d4c5b0] font-['Cinzel'] text-xs tracking-[0.3em] uppercase transition-all duration-500"
            >
              <span>{$t('archiveLoadMore')}</span>
              <span class="text-[#d4c5b0]/30 group-hover:text-[#d4c5b0]/60 transition-colors">{Math.min(PAGE_SIZE, filtered.length - displayLimit)}</span>
              <span class="transition-transform group-hover:translate-y-0.5">↓</span>
            </button>
          </div>
        {/if}
      {:else if searchQuery}
        <div class="flex flex-col items-center justify-center py-32 border border-dashed border-[#d4c5b0]/10 rounded-lg" in:fade>
          <p class="font-['UnifrakturMaguntia'] text-3xl text-[#8a7f70] mb-2 opacity-50">{$t('archiveNotFound')}</p>
          <p class="text-xs tracking-widest text-[#5c544a] uppercase">{$t('archiveNoEntry')} «{searchQuery}»</p>
        </div>
      {:else}
        <div class="flex flex-col items-center justify-center py-32 border border-dashed border-[#d4c5b0]/10 rounded-lg">
          <p class="font-['UnifrakturMaguntia'] text-3xl text-[#8a7f70] mb-2 opacity-50">{$t('archiveEmpty')}</p>
          <p class="text-xs tracking-widest text-[#5c544a] uppercase">{$t('archiveEmptyHint')}</p>
        </div>
      {/if}

      <div class="h-32"></div>
    </div>
  </div>
{/if}

<style>
  /* Noise Texture Utility Class */
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }
</style>