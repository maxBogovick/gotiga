<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import type { WorkshopItem } from '$lib/types/api';
  import { BackButton, LoadingScreen } from '$lib/components';

  // State (Svelte 5 Runes)
  let items = $state<WorkshopItem[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
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

  // Lifecycle
  onMount(async () => {
    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    prefersReducedMotion = mediaQuery.matches;

    try {
      items = await api.getWorkshopContent();
      // Небольшая задержка для проявления "пыли"
      await new Promise(r => setTimeout(r, 400));
    } catch (e) {
      console.error('Failed to load workshop:', e);
      error = 'Чертежи утеряны в тенях...';
    } finally {
      isLoading = false;
    }
  });
</script>

<svelte:head>
  <title>Мастерская — Anno 2024</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
  <link href="https://fonts.googleapis.com/css2?family=Cinzel:wght@400;600&family=UnifrakturMaguntia&display=swap" rel="stylesheet">
</svelte:head>

<div class="fixed inset-0 bg-[#0a0806] -z-50 overflow-hidden">
  <div class="absolute inset-0 opacity-[0.15] pointer-events-none wood-texture-heavy"></div>
  <div class="absolute -top-20 -left-20 w-[60vw] h-[60vw] bg-[#d4c5b0]/5 rounded-full blur-[120px] pointer-events-none"></div>
  <div class="absolute inset-0 pointer-events-none bg-[radial-gradient(circle_at_center,transparent_0%,#0a0806_95%)]"></div>
  <div class="absolute inset-0 pointer-events-none bg-noise opacity-[0.06] mix-blend-overlay"></div>
</div>

{#if isLoading}
  <LoadingScreen />
{:else if error}
  <div class="min-h-screen flex flex-col items-center justify-center p-8 z-10 relative font-['Cinzel']">
    <p class="text-[#8a7f70] mb-6 tracking-widest uppercase text-sm">{error}</p>
    <button class="px-6 py-2 border border-[#d4c5b0]/20 text-[#d4c5b0] hover:bg-[#d4c5b0]/5" onclick={() => window.location.reload()}>
      Попробовать снова
    </button>
  </div>
{:else}
  <div class="min-h-screen relative z-10 p-6 lg:p-16 font-['Cinzel'] text-[#d4c5b0]">

    <div class="mb-10" in:fade={{ duration: 800 }}>
      <a href="/" class="text-[10px] tracking-[0.4em] text-[#8a7f70] hover:text-[#d4c5b0] transition-colors group">
        ← ВЕРНУТЬСЯ В ЗАЛ
      </a>
    </div>

    <header class="mb-20 max-w-xl">
      <h1 class="font-['UnifrakturMaguntia'] text-5xl lg:text-7xl text-[#e6decb] mb-4 opacity-90 drop-shadow-2xl"
          in:fly={{ x: -20, duration: 1000 }}>
        Мастерская
      </h1>
      <p class="text-[#8a7f70] text-xs leading-relaxed tracking-widest uppercase border-l border-[#d4c5b0]/20 pl-6"
         in:fade={{ delay: 500 }}>
        Место, где глина обретает имя, а тени — форму. Внимательно изучайте чертежи.
      </p>
    </header>

    {#if items.length > 0}
      <div class="relative min-h-[120vh] mt-10">
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
              <div class="bg-[#1c1917] p-4 shadow-[10px_10px_30px_rgba(0,0,0,0.5)] border border-[#2a2622] group-hover:border-[#d4c5b0]/30 transition-colors duration-500">

                {#if item.imageUrl}
                  <div class="relative aspect-square overflow-hidden mb-4 bg-black">
                    <img
                            src={item.imageUrl}
                            alt=""
                            class="w-full h-full object-cover opacity-60 grayscale group-hover:grayscale-0 group-hover:opacity-90 transition-all duration-1000"
                            loading="lazy"
                    />
                    <div class="absolute inset-0 bg-[#d4c5b0]/5 mix-blend-multiply pointer-events-none"></div>
                    <div class="absolute inset-0 shadow-[inset_0_0_40px_rgba(0,0,0,0.7)] pointer-events-none"></div>
                  </div>
                {/if}

                {#if item.caption}
                  <p class="font-serif text-[13px] text-[#8a7f70] leading-snug italic opacity-80 group-hover:opacity-100 group-hover:text-[#d4c5b0] transition-all">
                    {item.caption}
                  </p>
                {/if}

                {#if expandedItem === item.id}
                  <div class="absolute -top-3 -right-3 w-8 h-8 rounded-full bg-red-900/20 border border-red-900/40 flex items-center justify-center text-[10px] text-red-900 font-bold rotate-12" in:fade>
                    EX
                  </div>
                {/if}
              </div>

              <div class="absolute -bottom-4 -right-2 -z-10 w-[95%] h-full bg-black/40 blur-xl opacity-0 group-hover:opacity-100 transition-opacity"></div>
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <div class="text-center py-40 border border-dashed border-[#d4c5b0]/10" in:fade>
        <p class="font-['UnifrakturMaguntia'] text-2xl text-[#8a7f70] opacity-40 uppercase tracking-[0.2em]">
          Мастерская заброшена...
        </p>
      </div>
    {/if}

    <div class="h-40"></div>
  </div>
{/if}

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
    background-color: #0a0806;
    cursor: default; /* На этой странице можно оставить стандартный курсор или адаптировать наш */
  }
</style>