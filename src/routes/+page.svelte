<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { spring } from 'svelte/motion';
  import { api } from '$lib/api';
  import type { CabinetZone } from '$lib/types/api';

  // --- Types ---
  type MousePos = { x: number; y: number };

  // --- Props & State ---
  let zones = $state<CabinetZone[]>([]);
  let isLoaded = $state(false);
  let imageLoaded = $state(false);
  let hoveredZone = $state<CabinetZone | null>(null);
  let isNavigating = $state(false);

  // Physics-based stores
  const cursorSpring = spring({ x: 50, y: 50 }, { stiffness: 0.05, damping: 0.35 });
  const parallaxSpring = spring({ x: 0, y: 0 }, { stiffness: 0.03, damping: 0.4 });

  // Путь к картинке (Убедитесь, что сохранили её в папку static)
  const IMAGE_URL = '/images/bg-main.png';

  // Новые координаты зон под сгенерированное изображение
  const DEFAULT_ZONES: CabinetZone[] = [
    // Центральная фигура (Кукла)
    { id: 'curator', zoneType: 'curator', x: 38, y: 15, width: 24, height: 75, targetRoute: '/author' },
    // Правый шкаф (Коллекция)
    { id: 'cabinet', zoneType: 'cabinet', x: 76, y: 42, width: 20, height: 45, targetRoute: '/collection' },
    // Левый портрет (О музее)
    { id: 'portrait', zoneType: 'portrait', x: 8, y: 28, width: 15, height: 30, targetRoute: '/about' },
    // Витражи сверху (Галерея)
    { id: 'windows', zoneType: 'windows', x: 25, y: 2, width: 50, height: 20, targetRoute: '/gallery' },
  ];

  const ZONE_LABELS: Record<string, string> = {
    curator: 'Куратор',
    cabinet: 'Архивъ',
    portrait: 'История',
    windows: 'Витражи',
  };

  const ZONE_DESCRIPTIONS: Record<string, string> = {
    curator: 'Создатель и его тень',
    cabinet: 'Реестр миниатюр',
    portrait: 'Хроники музея',
    windows: 'Свет и цвет',
  };

  // --- Logic ---
  async function init() {
    try {
      const [dbZones] = await Promise.all([
        api.getCabinetZones().catch(() => DEFAULT_ZONES),
        preloadImage(IMAGE_URL)
      ]);
      // Если база пуста, используем дефолтные зоны, иначе берем из базы
      zones = dbZones && dbZones.length > 0 ? dbZones : DEFAULT_ZONES;

      setTimeout(() => isLoaded = true, 800);
    } catch (e) {
      zones = DEFAULT_ZONES;
      isLoaded = true;
    }
  }

  function preloadImage(url: string): Promise<void> {
    return new Promise((resolve) => {
      const img = new Image();
      img.onload = () => { imageLoaded = true; resolve(); };
      img.onerror = () => {
        console.error("Не удалось загрузить изображение. Проверьте путь.");
        // Fallback чтобы интерфейс всё равно показался
        imageLoaded = true;
        resolve();
      };
      img.src = url;
    });
  }

  function handleMouseMove(e: MouseEvent) {
    const { innerWidth, innerHeight } = window;
    cursorSpring.set({
      x: (e.clientX / innerWidth) * 100,
      y: (e.clientY / innerHeight) * 100
    });

    // Parallax values -1 to 1
    parallaxSpring.set({
      x: (e.clientX / innerWidth - 0.5) * 2,
      y: (e.clientY / innerHeight - 0.5) * 2
    });
  }

  async function handleZoneInteraction(zone: CabinetZone) {
    if (isNavigating) return;
    isNavigating = true;
    await goto(zone.targetRoute);
  }

  // --- Lifecycle ---
  onMount(() => {
    init();
  });
</script>

<svelte:head>
  <title>Gothic Museum</title>
  <meta name="theme-color" content="#0a0806" />
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
  <link href="https://fonts.googleapis.com/css2?family=Cinzel:wght@400;600&family=UnifrakturMaguntia&display=swap" rel="stylesheet">
</svelte:head>

<svelte:window onmousemove={handleMouseMove} />

<div class="relative w-full h-[100dvh] overflow-hidden bg-[#0a0806] text-[#d4c5b0] cursor-none select-none font-cinzel">

  {#if !isLoaded}
    <div class="absolute inset-0 flex flex-col items-center justify-center z-50 bg-[#0a0806]" out:fade={{ duration: 1000 }}>
      <span class="text-4xl font-gothic text-[#d4c5b0] animate-pulse mb-4">Gothic Museum</span>
      <span class="text-xs tracking-[0.5em] opacity-40 font-serif uppercase">Подготовка экспозиции...</span>
    </div>
  {/if}

  {#if imageLoaded}
    <main
            class="relative w-full h-full flex items-center justify-center perspective-container"
            in:fade={{ duration: 2000, delay: 200 }}
    >
      <div
              class="relative w-full h-full max-w-[140vh] max-h-[100vh] aspect-square transition-transform duration-100 ease-linear shadow-2xl"
              style="
          transform: perspective(1500px)
                     rotateY({$parallaxSpring.x * -1.5}deg)
                     rotateX({$parallaxSpring.y * 1.5}deg)
                     scale(1.02);
        "
      >

        <img
                src={IMAGE_URL}
                alt="Museum Interior"
                class="absolute inset-0 w-full h-full object-contain drop-shadow-[0_20px_50px_rgba(0,0,0,0.8)]"
                draggable="false"
        />

        <div class="absolute inset-0 bg-vignette pointer-events-none z-10"></div>
        <div class="absolute inset-0 bg-noise opacity-[0.12] pointer-events-none mix-blend-overlay z-10"></div>
        <div class="absolute inset-0 pointer-events-none overflow-hidden z-10 opacity-30">
          <div class="dust-particle p1"></div>
          <div class="dust-particle p2"></div>
          <div class="dust-particle p3"></div>
        </div>

        {#if isLoaded}
          <div class="absolute inset-0 z-20">
            {#each zones as zone (zone.id)}
              {@render zoneButton(zone)}
            {/each}
          </div>
        {/if}

      </div>
    </main>
  {/if}

  <div class="pointer-events-none fixed inset-0 z-50 overflow-hidden">

    <div
            class="absolute w-12 h-12 -ml-6 -mt-6 border border-[#d4c5b0]/40 rounded-full transition-all duration-500 ease-out flex items-center justify-center mix-blend-difference"
            style="
        left: {$cursorSpring.x}%;
        top: {$cursorSpring.y}%;
        transform: scale({hoveredZone ? 1.8 : 1}) rotate({hoveredZone ? 45 : 0}deg);
        background: {hoveredZone ? 'rgba(212, 197, 176, 0.05)' : 'transparent'};
        border-radius: {hoveredZone ? '0%' : '50%'}; /* Circle to Diamond */
      "
    >
      <div class="w-1.5 h-1.5 bg-[#d4c5b0] rotate-45 opacity-80 shadow-[0_0_10px_rgba(212,197,176,0.8)]"></div>
    </div>

    {#if hoveredZone}
      <div
              class="absolute ml-8 mt-8 transition-opacity duration-300"
              style="left: {$cursorSpring.x}%; top: {$cursorSpring.y}%;"
              in:fly={{ y: 20, duration: 600, easing: cubicOut }}
      >
        <div class="flex flex-col items-start bg-black/60 backdrop-blur-sm p-4 border-l-2 border-[#d4c5b0]/50 shadow-2xl">
          <span class="text-3xl font-gothic text-[#d4c5b0] drop-shadow-lg leading-none mb-1">
            {ZONE_LABELS[hoveredZone.zoneType] ?? hoveredZone.zoneType}
          </span>
          <span class="text-[10px] tracking-widest uppercase text-white/60 font-cinzel">
             {ZONE_DESCRIPTIONS[hoveredZone.zoneType] ?? 'Исследовать'}
          </span>
        </div>
      </div>
    {/if}

    <div class="absolute bottom-0 left-0 w-full h-[20vh] bg-gradient-to-t from-[#0a0806] via-[#0a0806]/90 to-transparent flex items-end justify-center pb-10">
      {#if !hoveredZone && isLoaded}
        <div in:fade={{ delay: 1000 }} class="text-center opacity-40">
          <p class="text-[10px] tracking-[0.6em] uppercase animate-pulse font-cinzel text-[#8a7f70]">
            Коснитесь теней
          </p>
        </div>
      {/if}
    </div>
  </div>

</div>

{#snippet zoneButton(zone: CabinetZone)}
  <button
          class="absolute group focus:outline-none"
          style="
      left: {zone.x}%;
      top: {zone.y}%;
      width: {zone.width}%;
      height: {zone.height}%;
    "
          onclick={() => handleZoneInteraction(zone)}
          onmouseenter={() => hoveredZone = zone}
          onmouseleave={() => hoveredZone = null}
          onfocus={() => hoveredZone = zone}
          onblur={() => hoveredZone = null}
          disabled={isNavigating}
          aria-label={ZONE_LABELS[zone.zoneType]}
  >
    <span class="
      absolute inset-0
      opacity-0 group-hover:opacity-100
      transition-all duration-1000
      bg-radial-gradient-hover
      blur-md
    "></span>

    <span class="absolute top-0 left-0 w-4 h-4 border-t border-l border-[#d4c5b0]/60 opacity-0 group-hover:opacity-100 transition-all duration-500 scale-90 group-hover:scale-100"></span>
    <span class="absolute bottom-0 right-0 w-4 h-4 border-b border-r border-[#d4c5b0]/60 opacity-0 group-hover:opacity-100 transition-all duration-500 scale-90 group-hover:scale-100"></span>
  </button>
{/snippet}

<style>
  /* Fonts */
  .font-gothic {
    font-family: 'UnifrakturMaguntia', cursive;
  }
  .font-cinzel {
    font-family: 'Cinzel', serif;
  }

  /* Background Effects */
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }

  .bg-vignette {
    background: radial-gradient(circle at 50% 50%, rgba(0,0,0,0) 20%, rgba(10, 8, 6, 0.5) 60%, rgba(10, 8, 6, 1) 95%);
  }

  .bg-radial-gradient-hover {
    background: radial-gradient(circle at center, rgba(255, 255, 255, 0.07) 0%, transparent 70%);
  }

  /* Dust Particles Animation */
  .dust-particle {
    position: absolute;
    background: white;
    border-radius: 50%;
    opacity: 0.3;
    animation: float-dust 20s infinite linear;
  }
  .p1 { top: 10%; left: 20%; width: 2px; height: 2px; animation-duration: 25s; }
  .p2 { top: 60%; left: 80%; width: 3px; height: 3px; animation-duration: 35s; animation-delay: -5s; }
  .p3 { top: 40%; left: 40%; width: 1px; height: 1px; animation-duration: 45s; animation-delay: -10s; }

  @keyframes float-dust {
    0% { transform: translateY(0) translateX(0); opacity: 0; }
    50% { opacity: 0.5; }
    100% { transform: translateY(-100px) translateX(50px); opacity: 0; }
  }

  :global(body) {
    background-color: #0a0806;
  }
</style>