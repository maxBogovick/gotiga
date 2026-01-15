<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import type { Figurine } from '$lib/types/api';

  // State
  let figurine = $state<Figurine | null>(null);
  let selectedImageIndex = $state(0);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  // Derived
  let id = $derived(page.params.id);

  let sortedImages = $derived(
          figurine?.images
                  .slice()
                  .sort((a, b) => {
                    if (a.imageType === 'face') return -1;
                    if (b.imageType === 'face') return 1;
                    return 0;
                  }) ?? []
  );

  let currentImage = $derived(sortedImages[selectedImageIndex]);

  // Functions
  function selectImage(index: number) {
    if (index !== selectedImageIndex) {
      selectedImageIndex = index;
    }
  }

  // Lifecycle
  onMount(async () => {
    try {
      const result = await api.getFigurine(id);
      if (!result) {
        error = 'Запись в архиве отсутствует';
      } else {
        figurine = result;
        // Небольшая задержка для атмосферы
        if (figurine) await new Promise(r => setTimeout(r, 300));
      }
    } catch (e) {
      console.error('Failed to load figurine:', e);
      error = 'Страница архива повреждена';
    } finally {
      isLoading = false;
    }
  });
</script>

<svelte:head>
  <title>{figurine?.name ?? 'Архив'} — Детали</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
  <link href="https://fonts.googleapis.com/css2?family=Cinzel:wght@400;500;600&family=UnifrakturMaguntia&display=swap" rel="stylesheet">
</svelte:head>

<div class="fixed inset-0 bg-[#0a0806] -z-50"></div>
<div class="fixed inset-0 pointer-events-none z-0 bg-noise opacity-[0.08] mix-blend-overlay"></div>
<div class="fixed inset-0 pointer-events-none z-0 bg-[radial-gradient(circle_at_top,rgba(40,35,30,0.2)_0%,#0a0806_80%)]"></div>

{#if isLoading}
  <div class="min-h-screen flex flex-col items-center justify-center text-[#d4c5b0]" out:fade>
    <div class="w-12 h-12 border border-[#d4c5b0]/20 border-t-[#d4c5b0] rounded-full animate-spin mb-6"></div>
    <span class="font-['Cinzel'] tracking-[0.3em] text-xs uppercase animate-pulse">Извлечение из хранилища...</span>
  </div>

{:else if error}
  <div class="min-h-screen flex flex-col items-center justify-center p-8 text-center" in:fade>
    <h2 class="font-['UnifrakturMaguntia'] text-4xl text-[#8a7f70] mb-4">Увы</h2>
    <p class="font-['Cinzel'] text-[#d4c5b0] mb-8">{error}</p>
    <a href="/figurines" class="px-6 py-2 border border-[#d4c5b0]/30 text-[#d4c5b0] font-['Cinzel'] hover:bg-[#d4c5b0]/10 transition-colors uppercase text-xs tracking-widest">
      Вернуться к списку
    </a>
  </div>

{:else if figurine}
  <div class="min-h-screen relative z-10 text-[#d4c5b0] font-['Cinzel'] pb-20">

    <div class="max-w-7xl mx-auto px-6 lg:px-12 py-10">

      <nav class="mb-12" in:fade={{ duration: 800 }}>
        <a href="/figurines" class="inline-flex items-center text-xs tracking-[0.2em] text-[#8a7f70] hover:text-[#d4c5b0] transition-colors group">
          <span class="mr-2 transform group-hover:-translate-x-1 transition-transform">←</span>
          ВЕРНУТЬСЯ В АРХИВ
        </a>
      </nav>

      <div class="grid lg:grid-cols-12 gap-12 lg:gap-20 items-start">

        <div class="lg:col-span-7 space-y-8" in:fly={{ y: 20, duration: 1000, delay: 200, easing: cubicOut }}>

          <div class="relative p-1 bg-[#141210] border border-[#2a2622] shadow-2xl group">
            <div class="absolute top-0 left-0 w-8 h-8 border-t-2 border-l-2 border-[#d4c5b0]/20 z-20"></div>
            <div class="absolute top-0 right-0 w-8 h-8 border-t-2 border-r-2 border-[#d4c5b0]/20 z-20"></div>
            <div class="absolute bottom-0 left-0 w-8 h-8 border-b-2 border-l-2 border-[#d4c5b0]/20 z-20"></div>
            <div class="absolute bottom-0 right-0 w-8 h-8 border-b-2 border-r-2 border-[#d4c5b0]/20 z-20"></div>

            <div class="relative aspect-[4/5] overflow-hidden bg-black/50">
              {#key currentImage?.id}
                <img
                        src={currentImage?.url}
                        alt={figurine.name}
                        class="absolute inset-0 w-full h-full object-contain transition-all duration-700 ease-in-out"
                        in:fade={{ duration: 400 }}
                />
              {/key}
              <div class="absolute inset-0 pointer-events-none bg-noise opacity-[0.15] mix-blend-overlay"></div>
              <div class="absolute inset-0 pointer-events-none shadow-[inset_0_0_50px_rgba(0,0,0,0.8)]"></div>
            </div>
          </div>

          {#if sortedImages.length > 1}
            <div class="flex flex-wrap gap-4 pt-4 justify-center lg:justify-start">
              {#each sortedImages as img, i}
                <button
                        class="relative w-16 h-16 sm:w-20 sm:h-20 border transition-all duration-300 overflow-hidden group
                         {selectedImageIndex === i ? 'border-[#d4c5b0] opacity-100' : 'border-[#d4c5b0]/20 opacity-50 hover:opacity-80 hover:border-[#d4c5b0]/50'}"
                        onclick={() => selectImage(i)}
                        aria-label="Показать вид {i + 1}"
                >
                  <img src={img.url} alt="" class="w-full h-full object-cover grayscale group-hover:grayscale-0 transition-all duration-500" />
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <div class="lg:col-span-5 relative" in:fly={{ y: 20, duration: 1000, delay: 400, easing: cubicOut }}>

          <div class="mb-8 border-b border-[#d4c5b0]/10 pb-8 relative">
            <h1 class="font-['UnifrakturMaguntia'] text-5xl sm:text-6xl text-[#e6decb] leading-none mb-4 drop-shadow-lg">
              {figurine.name}
            </h1>
            {#if figurine.year}
              <p class="text-[#8a7f70] text-sm tracking-[0.3em]">ANNO {figurine.year}</p>
            {/if}

            <div class="absolute top-0 right-0 lg:-right-4 transform rotate-12 opacity-80 pointer-events-none border-2 border-double rounded px-4 py-2 uppercase tracking-widest text-xs font-bold mix-blend-screen
                {figurine.status === 'sold' ? 'text-red-900 border-red-900/40 bg-red-900/5' :
                 figurine.status === 'reserved' ? 'text-amber-700 border-amber-700/40 bg-amber-900/5' :
                 'text-green-900 border-green-900/40 bg-green-900/5'}">
              {#if figurine.status === 'sold'}Утрачено
              {:else if figurine.status === 'reserved'}Бронь
              {:else}В наличии{/if}
            </div>
          </div>

          {#if figurine.shortText}
            <blockquote class="relative mb-10 pl-6 border-l-2 border-[#d4c5b0]/20">
              <p class="text-lg text-[#d4c5b0] italic leading-relaxed opacity-90 font-serif">
                "{figurine.shortText}"
              </p>
            </blockquote>
          {/if}

          <div class="space-y-4 mb-12 text-sm text-[#8a7f70]">
            <div class="flex justify-between border-b border-[#d4c5b0]/5 pb-2 border-dashed">
              <span class="uppercase tracking-widest">Категория</span>
              <span class="text-[#d4c5b0]">Миниатюра</span>
            </div>
            <div class="flex justify-between border-b border-[#d4c5b0]/5 pb-2 border-dashed">
              <span class="uppercase tracking-widest">Материал</span>
              <span class="text-[#d4c5b0]">Полимер, Акрил</span>
            </div>
            <div class="flex justify-between border-b border-[#d4c5b0]/5 pb-2 border-dashed">
              <span class="uppercase tracking-widest">Реестровый №</span>
              <span class="text-[#d4c5b0]">#{id.slice(0,6).toUpperCase()}</span>
            </div>
          </div>

          {#if figurine.status === 'available'}
            <button class="w-full group relative px-8 py-4 bg-transparent overflow-hidden transition-all hover:bg-[#d4c5b0]/5 border border-[#d4c5b0]/30 hover:border-[#d4c5b0]/60">
              <div class="absolute inset-0 w-0 bg-[#d4c5b0]/10 transition-all duration-[250ms] ease-out group-hover:w-full"></div>
              <span class="relative text-[#d4c5b0] tracking-[0.3em] uppercase text-sm font-semibold flex items-center justify-center gap-3">
                <span class="w-1 h-1 bg-[#d4c5b0] rounded-full"></span>
                Отправить запрос
                <span class="w-1 h-1 bg-[#d4c5b0] rounded-full"></span>
              </span>
            </button>
            <p class="text-center text-[10px] text-[#8a7f70] mt-3 tracking-wider uppercase opacity-60">
              * Доставка вороном не осуществляется
            </p>
          {/if}

        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Noise Texture */
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }
</style>