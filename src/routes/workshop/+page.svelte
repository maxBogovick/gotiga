<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { t, brandName } from '$lib/i18n';
  import { SITE_URL } from '$lib/site';
  import AppImage from '$lib/components/AppImage.svelte';

  // Data from the universal load (+page.ts): real items at prerender time so bots see
  // the workshop, fresh fetch on client-side navigation.
  let { data } = $props();
  let items = $derived(data.items);
  let expandedItem = $state<string | null>(null);
  let prefersReducedMotion = $state(false);

  // Генерируем "хаотичный" стиль для предметов на верстаке
  function getItemStyle(index: number) {
    const seed = (index + 1) * 23;
    const col = index % 3;
    const row = Math.floor(index / 3);

    const baseX = 10 + col * 30;
    const baseY = 5 + row * 40;

    const offsetX = (seed % 12) - 6;
    const offsetY = ((seed * 7) % 10) - 5;
    const rotation = prefersReducedMotion ? 0 : ((seed % 10) - 5) * 1.5;

    return {
      left: `${baseX + offsetX}%`,
      top: `${baseY + offsetY}%`,
      rotation: `${rotation}deg`,
      zIndex: index,
    };
  }

  function toggleExpand(id: string) {
    expandedItem = expandedItem === id ? null : id;
  }

  onMount(() => {
    prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  });
</script>

<svelte:head>
  <title>Workshop — {$brandName}</title>
  <meta name="description" content={$t('workshopSubtitle')} />
  <meta property="og:site_name" content={$brandName} />
  <meta property="og:locale" content="en_US" />
  <meta property="og:type" content="website" />
  <meta property="og:title" content="Workshop — {$brandName}" />
  <meta property="og:description" content={$t('workshopSubtitle')} />
  <meta property="og:url" content="{SITE_URL}/workshop" />
  <meta property="og:image" content="{SITE_URL}/images/cabinet-room.jpg" />
  <!-- Fonts loaded once globally in app.html -->
</svelte:head>

<div class="fixed inset-0 bg-[#f8f1e7] -z-50 overflow-hidden">
  <div class="absolute inset-0 opacity-[0.15] pointer-events-none wood-texture-heavy"></div>
  <div class="absolute -top-20 -left-20 w-[60vw] h-[60vw] bg-[#34251c]/5 rounded-full blur-[120px] pointer-events-none"></div>
  <div class="absolute inset-0 pointer-events-none bg-[radial-gradient(circle_at_center,transparent_0%,#f8f1e7_95%)]"></div>
  <div class="absolute inset-0 pointer-events-none bg-noise opacity-[0.06] mix-blend-overlay"></div>
</div>

<div class="min-h-screen relative z-10 p-6 lg:p-16 font-['Inter'] text-[#34251c]">

    <div class="mb-10" in:fade={{ duration: 800 }}>
      <a href="/" class="text-[10px] tracking-[0.10em] text-[#5f4636] hover:text-[#34251c] transition-colors group">
        {$t('workshopBack')}
      </a>
    </div>

    <header class="mb-20 max-w-xl">
      <h1 class="font-['Fraunces'] text-5xl lg:text-7xl text-[#6f3b24] mb-4 opacity-90 drop-shadow-2xl"
          in:fly={{ x: -20, duration: 1000 }}>
        {$t('zoneDesk')}
      </h1>
      <p class="text-[#5f4636] text-xs leading-relaxed tracking-wide uppercase border-l border-[#34251c]/20 pl-6"
         in:fade={{ delay: 500 }}>
        {$t('workshopSubtitle')}
      </p>
    </header>

    {#if items.length > 0}
      <!-- Desktop: scattered absolute layout. Mobile: responsive grid -->
      <div class="hidden md:block relative min-h-[120vh] mt-10">
        {#each items as item, i}
          {@const style = getItemStyle(i)}
          <div
                  class="absolute transition-all duration-700 ease-out group"
                  style="
              left: {style.left};
              top: {style.top};
              transform: rotate({style.rotation}) scale({expandedItem === item.id ? 1.15 : 1});
              z-index: {expandedItem === item.id ? 100 : style.zIndex};
            "
                  in:fly={{ y: 50, opacity: 0, duration: 800, delay: 200 + (i * 100) }}
          >
            <button
                    class="relative block w-56 lg:w-72 text-left focus:outline-none"
                    onclick={() => toggleExpand(item.id)}
            >
              <div class="bg-[#fff9f0] p-4 shadow-[10px_10px_30px_rgba(111,59,36,0.14)] border border-[#d8c6b1] group-hover:border-[#34251c]/30 transition-colors duration-500">

                {#if item.imageUrl}
                  <div class="relative aspect-square overflow-hidden mb-4 bg-[#2f2117]">
                    <AppImage
                            src={item.imageUrl}
                            class="w-full h-full object-cover opacity-60 grayscale group-hover:grayscale-0 group-hover:opacity-90 transition-all duration-1000"
                            loading="lazy"
                    />
                    <div class="absolute inset-0 bg-[#34251c]/5 mix-blend-multiply pointer-events-none"></div>
                    <div class="absolute inset-0 shadow-[inset_0_0_40px_rgba(111,59,36,0.18)] pointer-events-none"></div>
                  </div>
                {/if}

                {#if item.caption}
                  <p class="font-serif text-[13px] text-[#5f4636] leading-snug italic opacity-80 group-hover:opacity-100 group-hover:text-[#34251c] transition-all">
                    {item.caption}
                  </p>
                {/if}

                {#if expandedItem === item.id}
                  <div class="absolute -top-3 -right-3 w-8 h-8 rounded-full bg-red-900/20 border border-red-900/40 flex items-center justify-center text-[10px] text-red-900 font-bold rotate-12" in:fade>
                    EX
                  </div>
                {/if}
              </div>

              <div class="absolute -bottom-4 -right-2 -z-10 w-[95%] h-full bg-[#6f3b24]/10 blur-xl opacity-0 group-hover:opacity-100 transition-opacity"></div>
            </button>
          </div>
        {/each}
      </div>
      <!-- Mobile grid layout -->
      <div class="grid md:hidden grid-cols-1 sm:grid-cols-2 gap-8 mt-10">
        {#each items as item, i}
          <div in:fly={{ y: 30, opacity: 0, duration: 600, delay: 100 + (i * 80) }}>
            <button
                    class="relative block w-full text-left focus:outline-none group"
                    onclick={() => toggleExpand(item.id)}
            >
              <div class="bg-[#fff9f0] p-4 shadow-[10px_10px_30px_rgba(111,59,36,0.14)] border border-[#d8c6b1] group-hover:border-[#34251c]/30 transition-colors duration-500">
                {#if item.imageUrl}
                  <div class="relative aspect-square overflow-hidden mb-4 bg-[#2f2117]">
                    <AppImage src={item.imageUrl} class="w-full h-full object-cover opacity-60 grayscale group-hover:grayscale-0 group-hover:opacity-90 transition-all duration-1000" loading="lazy" />
                    <div class="absolute inset-0 shadow-[inset_0_0_40px_rgba(111,59,36,0.18)] pointer-events-none"></div>
                  </div>
                {/if}
                {#if item.caption}
                  <p class="font-serif text-[13px] text-[#5f4636] leading-snug italic opacity-80 group-hover:opacity-100 group-hover:text-[#34251c] transition-all">{item.caption}</p>
                {/if}
                {#if expandedItem === item.id}
                  <div class="mt-4 text-sm text-[#34251c]/70 italic leading-relaxed border-t border-[#34251c]/10 pt-4">{item.content}</div>
                {/if}
              </div>
            </button>
          </div>
        {/each}
      </div>

    {:else}
      <div class="text-center py-40 border border-dashed border-[#34251c]/10" in:fade>
        <p class="font-['Fraunces'] text-2xl text-[#5f4636] opacity-70 uppercase tracking-[0.06em]">
          {$t('workshopEmpty')}
        </p>
      </div>
    {/if}

    <div class="h-40"></div>
  </div>

<style>
  /* Тяжелая древесная текстура */
  .wood-texture-heavy {
    background-image: url('data:image/svg+xml,%3Csvg viewBox=%220 0 200 200%22 xmlns=%22http://www.w3.org/2000/svg%22%3E%3Cfilter id=%22wood%22%3E%3CfeTurbulence type=%22fractalNoise%22 baseFrequency=%220.01 0.1%22 numOctaves=%224%22 seed=%2250%22/%3E%3CfeColorMatrix values=%220 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.5 0%22/%3E%3C/filter%3E%3Crect width=%22100%25%22 height=%22100%25%22 filter=%22url(%23wood)%22/%3E%3C/svg%3E');
  }

  /* Шум */
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }

  :global(body) {
    background-color: #f8f1e7;
    cursor: default; /* На этой странице можно оставить стандартный курсор или адаптировать наш */
  }
</style>