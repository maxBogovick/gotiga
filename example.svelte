<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { fade, scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import type { CabinetZone } from '$lib/types/api';

  // State management
  let zones = $state<CabinetZone[]>([]);
  let isLoaded = $state(false);
  let imageLoaded = $state(false);
  let hoveredZoneId = $state<string | null>(null);
  let isNavigating = $state(false);
  let showContent = $state(false);
  let mouseX = $state(0);
  let mouseY = $state(0);

  // Constants
  const INITIAL_FADE_DELAY = 1000;
  const ZONE_ANIMATION_BASE_DELAY = 1800;
  const ZONE_ANIMATION_STAGGER = 150;
  const IMAGE_URL = 'https://i.etsystatic.com/16575799/r/il/80711a/7549758914/il_300x300.7549758914_45hp.jpg';

  const DEFAULT_ZONES: Readonly<CabinetZone[]> = [
    {
      id: 'showcase',
      zoneType: 'showcase',
      x: 15,
      y: 25,
      width: 30,
      height: 45,
      targetRoute: '/figurines'
    },
    {
      id: 'desk',
      zoneType: 'desk',
      x: 35,
      y: 65,
      width: 35,
      height: 30,
      targetRoute: '/workshop'
    },
    {
      id: 'shelf',
      zoneType: 'shelf',
      x: 65,
      y: 20,
      width: 25,
      height: 35,
      targetRoute: '/figurines'
    },
    {
      id: 'note',
      zoneType: 'note',
      x: 70,
      y: 70,
      width: 15,
      height: 15,
      targetRoute: '/author'
    },
  ];

  const ZONE_LABELS: Readonly<Record<string, string>> = {
    showcase: 'Витрина с фигурами',
    desk: 'Рабочий стол',
    shelf: 'Полка с обитателями',
    note: 'Записка автора',
  };

  // Functions
  async function loadZones(): Promise<void> {
    try {
      const dbZones = await api.getCabinetZones();
      zones = dbZones.length > 0 ? dbZones : [...DEFAULT_ZONES];
    } catch (error) {
      console.error('Failed to load cabinet zones:', error);
      zones = [...DEFAULT_ZONES];
    }
  }

  function preloadImage(url: string): Promise<void> {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => {
        imageLoaded = true;
        resolve();
      };
      img.onerror = reject;
      img.src = url;
    });
  }

  async function handleZoneClick(zone: CabinetZone): Promise<void> {
    if (isNavigating) return;
    isNavigating = true;

    try {
      await goto(zone.targetRoute);
    } catch (error) {
      console.error('Navigation error:', error);
      isNavigating = false;
    }
  }

  function getZoneLabel(zoneType: string): string {
    return ZONE_LABELS[zoneType] ?? zoneType;
  }

  function handleZoneKeydown(event: KeyboardEvent, zone: CabinetZone): void {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleZoneClick(zone);
    }
  }

  function getZoneAnimationDelay(index: number): number {
    return ZONE_ANIMATION_BASE_DELAY + index * ZONE_ANIMATION_STAGGER;
  }

  function handleMouseMove(e: MouseEvent): void {
    mouseX = (e.clientX / window.innerWidth - 0.5) * 2;
    mouseY = (e.clientY / window.innerHeight - 0.5) * 2;
  }

  // Lifecycle
  onMount(async () => {
    await Promise.allSettled([
      loadZones(),
      preloadImage(IMAGE_URL)
    ]);

    setTimeout(() => {
      isLoaded = true;
    }, INITIAL_FADE_DELAY);

    setTimeout(() => {
      showContent = true;
    }, INITIAL_FADE_DELAY + 300);
  });
</script>

<svelte:head>
  <title>Cabinet of Curiosities</title>
  <meta name="description" content="A preserved presence" />
</svelte:head>

<svelte:window onmousemove={handleMouseMove} />

<div class="relative w-full h-screen overflow-hidden bg-[#1A1816]">
  <!-- Initial darkness -->
  {#if !isLoaded}
    <div class="absolute inset-0 bg-[#1A1816]" aria-label="Входим в кабинет"></div>
  {/if}

  <!-- Main scene -->
{#if imageLoaded}
<div
class="absolute inset-0"
in:fade={{ duration: 2000, easing: cubicOut }}
>
<!-- Background with subtle parallax -->
<div
class="w-full h-full will-change-transform"
style="transform: translate({mouseX * 8}px, {mouseY * 8}px) scale(1.05);"
in:scale={{
duration: 2800,
start: 1.04,
opacity: 1,
easing: cubicOut
}}
>
<img
src={IMAGE_URL}
alt="Кабинет редкостей"
class="w-full h-full object-cover object-center"
style="filter: contrast(1.05) brightness(0.92) saturate(0.95);"
/>
</div>

      <!-- Enhanced atmospheric vignette -->
      <div
        class="absolute inset-0 pointer-events-none"
        style="
          background:
            radial-gradient(
              ellipse 75% 65% at 50% 45%,
              transparent 0%,
              rgba(26, 24, 22, 0.15) 35%,
              rgba(26, 24, 22, 0.5) 65%,
              rgba(26, 24, 22, 0.8) 90%,
              rgba(26, 24, 22, 0.95) 100%
            ),
            linear-gradient(
              180deg,
              rgba(26, 24, 22, 0.3) 0%,
              transparent 30%,
              transparent 70%,
              rgba(26, 24, 22, 0.5) 100%
            );
        "
        aria-hidden="true"
      ></div>

      <!-- Film grain texture -->
      {#if showContent}
        <div
          class="absolute inset-0 pointer-events-none opacity-[0.045] mix-blend-overlay"
          style="
            background-image: url('data:image/svg+xml,%3Csvg viewBox=%220 0 400 400%22 xmlns=%22http://www.w3.org/2000/svg%22%3E%3Cfilter id=%22noise%22%3E%3CfeTurbulence type=%22fractalNoise%22 baseFrequency=%223%22 numOctaves=%225%22 stitchTiles=%22stitch%22/%3E%3C/filter%3E%3Crect width=%22100%25%22 height=%22100%25%22 filter=%22url(%23noise)%22/%3E%3C/svg%3E');
            animation: grain 8s steps(10) infinite;
          "
          in:fade={{ duration: 2500 }}
          aria-hidden="true"
        ></div>
      {/if}

      <!-- Subtle light rays -->
      {#if showContent}
        <div
          class="absolute inset-0 pointer-events-none opacity-[0.06]"
          style="
            background:
              linear-gradient(
                135deg,
                transparent 0%,
                rgba(207, 198, 184, 0.3) 40%,
                transparent 60%
              );
          "
          in:fade={{ duration: 3000, delay: 500 }}
          aria-hidden="true"
        ></div>
      {/if}

      <!-- Interactive zones -->
      {#if showContent}
        {#each zones as zone, index (zone.id)}
          <button
            class="
              absolute group
              transition-all duration-500 ease-out
              focus:outline-none
              disabled:cursor-default
              will-change-transform
            "
            style:left="{zone.x}%"
            style:top="{zone.y}%"
            style:width="{zone.width}%"
            style:height="{zone.height}%"
            style:transform={hoveredZoneId === zone.id ? 'scale(1.02)' : 'scale(1)'}
            onclick={() => handleZoneClick(zone)}
            onkeydown={(e) => handleZoneKeydown(e, zone)}
            onmouseenter={() => hoveredZoneId = zone.id}
            onmouseleave={() => hoveredZoneId = null}
            onfocus={() => hoveredZoneId = zone.id}
            onblur={() => hoveredZoneId = null}
            aria-label={getZoneLabel(zone.zoneType)}
            disabled={isNavigating}
            in:fade={{
              delay: getZoneAnimationDelay(index),
              duration: 1400
            }}
          >
            <!-- Ambient glow -->
            <div
              class="
                absolute inset-0 -inset-2
                transition-all duration-700 ease-out
                {hoveredZoneId === zone.id
                  ? 'bg-[#D4CDB8] opacity-[0.12] shadow-[0_0_50px_rgba(212,205,184,0.2)] blur-sm'
                  : 'opacity-0'}
              "
              aria-hidden="true"
            ></div>

            <!-- Inner highlight -->
            <div
              class="
                absolute inset-0
                transition-all duration-600 ease-out
                {hoveredZoneId === zone.id
                  ? 'bg-gradient-to-br from-[#CFC6B8]/10 to-transparent opacity-100'
                  : 'opacity-0'}
              "
              aria-hidden="true"
            ></div>

            <!-- Border with gradient -->
            <div
              class="
                absolute inset-0
                transition-all duration-600 ease-out
                {hoveredZoneId === zone.id
                  ? 'border border-[#9C8E7D]/40 shadow-[inset_0_0_20px_rgba(207,198,184,0.1)]'
                  : 'border border-transparent'}
              "
              style="border-radius: 2px;"
              aria-hidden="true"
            ></div>

            <!-- Focus ring -->
            <div
              class="
                absolute inset-0 -inset-1
                ring-2 ring-[#9C8E7D] ring-opacity-0 ring-offset-2 ring-offset-[#1A1816]
                transition-all duration-300
                group-focus-visible:ring-opacity-50
              "
              style="border-radius: 3px;"
              aria-hidden="true"
            ></div>

            <span class="sr-only">{getZoneLabel(zone.zoneType)}</span>
          </button>
        {/each}
      {/if}

      <!-- Subtle hint text for first-time visitors -->
      {#if showContent}
        <div
          class="absolute bottom-8 left-1/2 -translate-x-1/2"
          in:fade={{ delay: 3500, duration: 2000 }}
        >
          <p
            class="
              text-[#9C8E7D]/60 text-sm font-light tracking-widest
              transition-opacity duration-1000
              {hoveredZoneId ? 'opacity-0' : 'opacity-100'}
            "
            style="font-variant: small-caps; letter-spacing: 0.2em;"
          >
            Исследуйте кабинет
          </p>
        </div>
      {/if}
    </div>
{/if}

  <!-- Loading state -->
{#if !imageLoaded}
<div
class="absolute inset-0 flex items-center justify-center bg-[#1A1816]"
aria-live="polite"
aria-busy="true"
>
<div class="flex flex-col items-center gap-6">
<div
class="w-2 h-2 rounded-full bg-[#9C8E7D]"
style="animation: breathe 2.2s ease-in-out infinite;"
aria-label="Загрузка"
></div>
</div>
</div>
{/if}
</div>

<style>
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border-width: 0;
  }

  @keyframes breathe {
    0%, 100% {
      opacity: 0.25;
      transform: scale(1);
    }
    50% {
      opacity: 0.65;
      transform: scale(1.3);
    }
  }

  @keyframes grain {
    0%, 100% { transform: translate(0, 0); }
    10% { transform: translate(-5%, -5%); }
    20% { transform: translate(-10%, 5%); }
    30% { transform: translate(5%, -10%); }
    40% { transform: translate(-5%, 15%); }
    50% { transform: translate(-10%, 5%); }
    60% { transform: translate(15%, 0); }
    70% { transform: translate(0, 10%); }
    80% { transform: translate(-15%, 0); }
    90% { transform: translate(10%, 5%); }
  }
</style>