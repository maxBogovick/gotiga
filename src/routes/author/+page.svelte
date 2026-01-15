<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import type { AuthorText } from '$lib/types/api';

  // State (Svelte 5 Runes)
  let texts = $state<AuthorText[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let prefersReducedMotion = $state(false);

  // Конфигурация стиля для эффекта разбросанных записок
  function getNoteStyle(index: number) {
    const seed = (index + 1) * 31;

    // Чередуем выравнивание для создания ритма "дневника"
    const alignments = [
      'ml-0 mr-auto',      // Слева
      'mx-auto',           // Центр
      'ml-auto mr-0',      // Справа
      'ml-[10%] mr-auto',  // С небольшим отступом
    ];

    const rotation = ((seed % 7) - 3) * 0.7; // Легкий наклон (-2 до +2 градусов)

    return {
      alignment: alignments[seed % alignments.length],
      rotation: prefersReducedMotion ? '0deg' : `${rotation}deg`,
      maxWidth: ['max-w-md', 'max-w-lg', 'max-w-xl'][seed % 3],
    };
  }

  onMount(async () => {
    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    prefersReducedMotion = mediaQuery.matches;

    try {
      texts = await api.getAuthorTexts();
      if (texts.length > 0) await new Promise(r => setTimeout(r, 600));
    } catch (e) {
      console.error('Failed to load author texts:', e);
      error = 'Слова растворились в тишине...';
    } finally {
      isLoading = false;
    }
  });
</script>

<svelte:head>
  <title>Голосъ Создателя — Архивъ</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
  <link href="https://fonts.googleapis.com/css2?family=Cinzel:ital,wght@0,400;1,400&family=UnifrakturMaguntia&display=swap" rel="stylesheet">
</svelte:head>

<div class="fixed inset-0 bg-[#0a0806] -z-50 overflow-hidden">
  <div class="absolute inset-0 pointer-events-none bg-noise opacity-[0.07] mix-blend-overlay"></div>
  <div class="absolute inset-0 pointer-events-none bg-[radial-gradient(circle_at_center,transparent_0%,#0a0806_90%)]"></div>

  <div class="absolute bottom-[-10%] right-[-5%] w-[50vw] h-[50vw] bg-[#2a1a10]/20 rounded-full blur-[100px] pointer-events-none"></div>
</div>

{#if isLoading}
  <div class="min-h-screen flex items-center justify-center" out:fade>
    <span class="font-['Cinzel'] text-[#8a7f70] tracking-[0.5em] text-xs animate-pulse uppercase">Тишина...</span>
  </div>
{:else if error}
  <div class="min-h-screen flex flex-col items-center justify-center p-8 text-center" in:fade>
    <p class="font-['Cinzel'] text-[#8a7f70] mb-6 tracking-widest">{error}</p>
    <a href="/" class="text-[#d4c5b0] border-b border-[#d4c5b0]/20 pb-1 text-xs tracking-widest">ВЕРНУТЬСЯ</a>
  </div>
{:else}
  <div class="min-h-screen relative z-10 font-['Cinzel'] text-[#d4c5b0] pb-32">
    <div class="max-w-5xl mx-auto px-6 py-12 lg:py-20">

      <nav class="mb-24" in:fade={{ duration: 1000 }}>
        <a href="/" class="group flex items-center text-[10px] tracking-[0.4em] text-[#8a7f70] hover:text-[#d4c5b0] transition-colors uppercase">
          <span class="mr-3 transition-transform group-hover:-translate-x-1">←</span> К порогу кабинета
        </a>
      </nav>

      <header class="mb-32 text-center">
        <h1 class="font-['UnifrakturMaguntia'] text-5xl lg:text-7xl text-[#e6decb] mb-6 opacity-80 drop-shadow-2xl">
          Голосъ Автора
        </h1>
        <div class="w-24 h-px bg-gradient-to-r from-transparent via-[#d4c5b0]/30 to-transparent mx-auto"></div>
      </header>

      {#if texts.length > 0}
        <div class="space-y-32 lg:space-y-48 relative" role="list">
          {#each texts as text, i}
            {@const style = getNoteStyle(i)}
            <div
                    class="relative {style.alignment} {style.maxWidth} group"
                    style="transform: rotate({style.rotation});"
                    role="listitem"
                    in:fly={{ y: 40, opacity: 0, duration: 1200, delay: i * 200, easing: cubicOut }}
            >
              <div class="relative p-8 lg:p-12 bg-[#d4c5b0]/[0.02] border border-[#d4c5b0]/5 backdrop-blur-[2px] shadow-2xl transition-all duration-700 group-hover:bg-[#d4c5b0]/[0.04] group-hover:border-[#d4c5b0]/10">

                <span class="absolute -top-4 -left-4 font-['UnifrakturMaguntia'] text-2xl text-[#8a7f70]/30 group-hover:text-[#8a7f70]/60 transition-colors">
                  {i + 1}
                </span>

                <blockquote class="relative">
                  <p class="text-lg lg:text-xl text-[#d4c5b0] leading-relaxed italic opacity-90 font-serif">
                    «{text.content}»
                  </p>
                </blockquote>

                {#if i % 2 === 0}
                  <div class="mt-8 flex justify-end opacity-20 group-hover:opacity-40 transition-opacity">
                    <svg width="60" height="20" viewBox="0 0 60 20" fill="none" xmlns="http://www.w3.org/2000/svg">
                      <path d="M1 18C15 -2 45 22 59 2" stroke="#d4c5b0" stroke-width="0.5"/>
                    </svg>
                  </div>
                {/if}
              </div>

              <div class="absolute inset-0 bg-black/40 blur-2xl -z-10 translate-y-4 translate-x-2 opacity-0 group-hover:opacity-100 transition-opacity duration-700"></div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="text-center py-20 opacity-40">
          <p class="tracking-[0.3em] uppercase text-xs">Листы чисты...</p>
        </div>
      {/if}

      <div class="mt-40 text-center opacity-10 select-none pointer-events-none" in:fade={{ delay: 1500 }}>
        <span class="font-['UnifrakturMaguntia'] text-8xl text-[#d4c5b0]">Finis.</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }

  /* Мягкое свечение при наведении на текст */
  blockquote p {
    text-shadow: 0 0 20px rgba(212, 197, 176, 0);
    transition: text-shadow 0.5s ease;
  }

  .group:hover blockquote p {
    text-shadow: 0 0 15px rgba(212, 197, 176, 0.2);
  }

  :global(body) {
    background-color: #0a0806;
    scrollbar-color: #2a2622 #0a0806;
  }
</style>