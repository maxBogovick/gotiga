<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/state';
  import { fade, fly, slide } from 'svelte/transition';
  import { cubicOut, quintOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import type { Figurine } from '$lib/types/api';
  import OrderModal from '$lib/components/OrderModal.svelte';
  import BrassLens from '$lib/components/BrassLens.svelte';
  import DustParticles from '$lib/components/DustParticles.svelte';
  import CandleReveal from '$lib/components/CandleReveal.svelte';
  import MemoryMirror from '$lib/components/MemoryMirror.svelte';
  import SecretText from '$lib/components/SecretText.svelte';
  import Lightbox from '$lib/components/Lightbox.svelte';
  import { t } from '$lib/i18n';

  // State
  let figurine = $state<Figurine | null>(null);
  let selectedImageIndex = $state(0);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let isGrimoireOpen = $state(false);
  let showOrderModal = $state(false);
  let isAudioPlaying = $state(false);
  let isCandleLit = $state(false);
  let showLightbox = $state(false);
  let lightboxStartIndex = $state(0);
  let audioRef = $state<HTMLAudioElement | null>(null);
  let audioVolume = $state(0);
  let videoRef = $state<HTMLVideoElement | null>(null);

  function toggleFullscreen() {
      if (!videoRef) return;
      if (document.fullscreenElement) {
          document.exitFullscreen();
      } else {
          videoRef.requestFullscreen().catch(() => {});
      }
  }

  // Derived
  let id = $derived(page.params.id ?? '');

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

  // Helper
  function resolveUrl(path: string | undefined | null) {
      return path ?? '';
  }

  function openLightbox(index: number) {
    lightboxStartIndex = index;
    showLightbox = true;
  }

  let lightboxImages = $derived(
    sortedImages.map(img => ({ url: resolveUrl(img.originalUrl ?? img.url), alt: img.altText ?? '' }))
  );

  // Functions
  function selectImage(index: number) {
    if (index !== selectedImageIndex) {
      selectedImageIndex = index;
    }
  }

  function toggleGrimoire() {
    isGrimoireOpen = !isGrimoireOpen;
  }
  
  function toggleCandle() {
      isCandleLit = !isCandleLit;
  }
  
  function toggleAudio() {
      if (!audioRef || !figurine?.ambiencePath) return;
      
      if (isAudioPlaying) {
          // Fade out
          fadeOutAudio();
      } else {
          // Fade in
          audioRef.volume = 0;
          audioRef.play().catch(e => console.error("Audio play failed", e));
          isAudioPlaying = true;
          fadeInAudio();
      }
  }
  
  function fadeInAudio() {
      if (!audioRef) return;
      let vol = 0;
      const interval = setInterval(() => {
          if (vol < 0.5) {
              vol += 0.05;
              audioRef!.volume = vol;
          } else {
              clearInterval(interval);
          }
      }, 100);
  }

  function fadeOutAudio() {
      if (!audioRef) return;
      let vol = audioRef.volume;
      const interval = setInterval(() => {
          if (vol > 0.05) {
              vol -= 0.05;
              audioRef!.volume = vol;
          } else {
              clearInterval(interval);
              audioRef!.pause();
              isAudioPlaying = false;
          }
      }, 100);
  }

  // Lifecycle
  onMount(async () => {
    try {
      const result = await api.getFigurine(id);
      if (!result) {
        error = $t('figurineError');
      } else {
        figurine = result;
        // Небольшая задержка для атмосферы
        if (figurine) await new Promise(r => setTimeout(r, 300));
      }
    } catch (e) {
      console.error('Failed to load figurine:', e);
      error = $t('figurineError');
    } finally {
      isLoading = false;
    }
  });

  onDestroy(() => {
      if (audioRef) {
          audioRef.pause();
          audioRef = null;
      }
  });
</script>

<svelte:head>
  <title>{figurine?.name ?? $t('zoneShowcase')} — Details</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=Fraunces:opsz,wght@9..144,500;9..144,650;9..144,750&display=swap" rel="stylesheet">
</svelte:head>

<div class="fixed inset-0 bg-cabinet-bg -z-50"></div>
<div class="fixed inset-0 pointer-events-none z-0 bg-noise opacity-[0.08] mix-blend-overlay"></div>
<div class="fixed inset-0 pointer-events-none z-0 bg-[radial-gradient(circle_at_top,rgba(198,95,60,0.3)_0%,#f1e3d1_90%)]"></div>

<DustParticles />

{#if isLoading}
  <div class="min-h-screen flex flex-col items-center justify-center text-cabinet-bone" out:fade>
    <div class="relative w-16 h-16 mb-8">
       <div class="absolute inset-0 border border-cabinet-bone/20 rounded-full animate-ping"></div>
       <div class="absolute inset-0 border-t border-cabinet-bone rounded-full animate-spin"></div>
    </div>
    <span class="font-['Inter'] tracking-[0.08em] text-xs uppercase animate-pulse text-cabinet-dust">{$t('figurineExtracting')}</span>
  </div>

{:else if error}
  <div class="min-h-screen flex flex-col items-center justify-center p-8 text-center" in:fade>
    <h2 class="font-['Fraunces'] text-5xl text-cabinet-fabric mb-6">{$t('figurineError')}</h2>
    <p class="font-['Inter'] text-cabinet-bone mb-12 text-lg">{error}</p>
    <a href="/figurines" class="px-8 py-3 border border-cabinet-bone/30 text-cabinet-bone font-['Inter'] hover:bg-cabinet-wood-light transition-colors uppercase text-sm tracking-wide relative group">
      <span class="absolute inset-0 w-0 bg-cabinet-bone/5 transition-all duration-300 group-hover:w-full"></span>
      <span class="relative">{$t('figurineErrorBack')}</span>
    </a>
  </div>

{:else if figurine}
  <!-- Audio Element -->
  {#if figurine.ambiencePath}
      <audio bind:this={audioRef} src={resolveUrl(figurine.ambiencePath)} loop></audio>
  {/if}
  
  <CandleReveal isActive={isCandleLit} />

  <div class="min-h-screen relative z-10 text-cabinet-bone font-['Inter'] pb-24">

    <OrderModal
            isOpen={showOrderModal}
            figurineName={figurine.name}
            figurineId={figurine.id}
            onClose={() => showOrderModal = false}
    />

    {#if showLightbox}
      <Lightbox
        images={lightboxImages}
        startIndex={lightboxStartIndex}
        onClose={() => showLightbox = false}
      />
    {/if}

    <div class="max-w-7xl mx-auto px-6 lg:px-12 py-12">

      <!-- Navigation -->
      <nav class="mb-16 flex justify-between items-center" in:fade={{ duration: 800 }}>
        <a href="/figurines" class="inline-flex items-center text-xs tracking-[0.06em] text-cabinet-dust hover:text-cabinet-bone transition-colors group opacity-70 hover:opacity-100">
          <span class="mr-3 transform group-hover:-translate-x-1 transition-transform font-serif text-lg">←</span>
          {$t('figurineBackToArchive')}
        </a>
        <div class="flex items-center gap-6">
            <button
                onclick={toggleCandle}
                class="flex items-center gap-3 text-xs tracking-[0.06em] uppercase transition-colors {isCandleLit ? 'text-[#ffaa00] opacity-100 drop-shadow-[0_0_5px_rgba(255,170,0,0.5)]' : 'text-cabinet-wood-muted opacity-60 hover:opacity-100'}"
                aria-label={$t('figurineCandle')}
            >
                <span class="text-base">{isCandleLit ? '🔥' : '🕯️'}</span>
                {isCandleLit ? $t('figurineExtinguish') : $t('figurineCandle')}
            </button>

            {#if figurine.ambiencePath}
                <button 
                    onclick={toggleAudio}
                    class="flex items-center gap-3 text-xs tracking-[0.06em] uppercase transition-colors {isAudioPlaying ? 'text-cabinet-bone opacity-100' : 'text-cabinet-wood-muted opacity-60 hover:opacity-100'}"
                    aria-label={$t('figurineWhisper')}
                >
                    <span class="relative flex h-3 w-3">
                      {#if isAudioPlaying}
                        <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-cabinet-bone opacity-75"></span>
                      {/if}
                      <span class="relative inline-flex rounded-full h-3 w-3 {isAudioPlaying ? 'bg-cabinet-bone' : 'bg-cabinet-wood'}"></span>
                    </span>
                    {isAudioPlaying ? $t('figurineSilence') : $t('figurineWhisper')}
                </button>
            {/if}
            <span class="text-[10px] tracking-[0.08em] text-cabinet-wood-muted uppercase border border-cabinet-wood-muted/30 px-3 py-1 rounded-full">
              Ref. {id.slice(-3)}
            </span>
        </div>
      </nav>

      <div class="grid lg:grid-cols-12 gap-12 lg:gap-24 items-start mb-20">

        <!-- Left Column: Visuals -->
        <div class="lg:col-span-7 space-y-10 sticky top-10" in:fly={{ y: 20, duration: 1000, delay: 200, easing: cubicOut }}>

          <!-- Main Image Frame -->
          <div class="relative p-2 bg-[#fff9f0] shadow-cabinet-lg group">
             <!-- Frame decorations -->
            <div class="absolute top-0 left-0 w-16 h-16 border-t border-l border-cabinet-bone/30 z-20"></div>
            <div class="absolute top-0 right-0 w-16 h-16 border-t border-r border-cabinet-bone/30 z-20"></div>
            <div class="absolute bottom-0 left-0 w-16 h-16 border-b border-l border-cabinet-bone/30 z-20"></div>
            <div class="absolute bottom-0 right-0 w-16 h-16 border-b border-r border-cabinet-bone/30 z-20"></div>

            <div class="relative aspect-[4/5] overflow-hidden bg-[#6f3b24]/25 group/main">
              {#key currentImage?.id}
                <div class="absolute inset-0 w-full h-full" in:fade={{ duration: 600 }}>
                    <BrassLens
                            src={currentImage?.url}
                            alt={figurine.name}
                            class="w-full h-full"
                    />
                </div>
              {/key}
              <!-- Lightbox trigger hint -->
              {#if sortedImages.length > 0}
                <button
                  onclick={() => openLightbox(selectedImageIndex)}
                  class="absolute bottom-3 right-3 z-30 flex items-center gap-1.5 px-2.5 py-1.5 bg-[#6f3b24]/20 border border-[#fff9f0]/25 hover:border-[#fff9f0]/45 hover:bg-[#6f3b24]/30 transition-all duration-200 opacity-0 group-hover/main:opacity-100 font-['Inter'] text-[9px] tracking-wide text-[#fff9f0]/70 hover:text-[#fff9f0]/90"
                  aria-label={$t('figurineFullscreen')}
                >
                  <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.2">
                    <path d="M1 4V1h3M6 1h3v3M9 6v3H6M4 9H1V6"/>
                  </svg>
                  {$t('figurineFullscreen')}
                </button>
              {/if}
              <!-- Texture overlays -->
              <div class="absolute inset-0 pointer-events-none bg-noise opacity-[0.12] mix-blend-overlay"></div>
              <div class="absolute inset-0 pointer-events-none shadow-[inset_0_0_80px_rgba(111,59,36,0.20)]"></div>
              
              <!-- Subtle vignette animation -->
              <div class="absolute inset-0 pointer-events-none bg-gradient-to-t from-cabinet-bg-deep/80 via-transparent to-transparent opacity-60"></div>
            </div>
          </div>

          <!-- Thumbnails -->
          {#if sortedImages.length > 1}
            <div class="flex flex-wrap gap-3 pt-2 justify-center lg:justify-start">
              {#each sortedImages as img, i}
                <div class="relative group/thumb">
                  <button
                    class="relative w-20 h-20 border transition-all duration-500 overflow-hidden
                     {selectedImageIndex === i ? 'border-cabinet-bone opacity-100 scale-105' : 'border-cabinet-wood opacity-70 hover:opacity-80 hover:border-cabinet-bone/60'}"
                    onclick={() => selectImage(i)}
                    aria-label="{$t('figurineShowView')} {i + 1}"
                  >
                    <img src={resolveUrl(img.thumbUrl ?? img.url)} alt="" class="w-full h-full object-cover grayscale group-hover/thumb:grayscale-0 transition-all duration-500" />
                    {#if selectedImageIndex === i}
                      <div class="absolute inset-0 bg-cabinet-bone/10 pointer-events-none mix-blend-overlay"></div>
                    {/if}
                  </button>
                  <!-- Lightbox open on double-click / expand icon -->
                  <button
                    onclick={() => openLightbox(i)}
                    class="absolute inset-0 flex items-end justify-end p-1 opacity-0 group-hover/thumb:opacity-100 transition-opacity"
                    aria-label={$t('figurineOpenEnlarged')}
                  >
                    <svg width="9" height="9" viewBox="0 0 10 10" fill="none" stroke="rgba(198, 95, 60,0.7)" stroke-width="1.5">
                      <path d="M1 4V1h3M6 1h3v3M9 6v3H6M4 9H1V6"/>
                    </svg>
                  </button>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <!-- Right Column: Narrative & Data -->
        <div class="lg:col-span-5 relative space-y-12" in:fly={{ y: 20, duration: 1000, delay: 400, easing: cubicOut }}>

          <!-- Header Section -->
          <div class="relative">
            <div class="absolute -left-6 top-2 bottom-2 w-[1px] bg-gradient-to-b from-transparent via-cabinet-bone/30 to-transparent"></div>
            
            <h1 class="font-['Fraunces'] text-5xl sm:text-7xl text-[#6f3b24] leading-[0.9] mb-6 drop-shadow-2xl">
              {figurine.name}
            </h1>
            
            {#if figurine.secretText}
               <div class="absolute -top-10 right-0 max-w-[300px] text-right transform rotate-2 z-20">
                  <SecretText text={figurine.secretText} isCandleLit={isCandleLit} />
               </div>
            {/if}
            
            <div class="flex items-center gap-6 text-cabinet-dust text-sm tracking-[0.06em] uppercase">
              {#if figurine.year}
                <span class="opacity-80">Anno {figurine.year}</span>
              {/if}
              <span class="w-1 h-1 bg-cabinet-bone/40 rounded-full"></span>
              <span class="{figurine.status === 'sold' ? 'text-accent-red' : figurine.status === 'reserved' ? 'text-accent-olive' : 'text-cabinet-bone'}">
                {figurine.status === 'sold' ? $t('figurineStatusSold') : figurine.status === 'reserved' ? $t('figurineStatusReserved') : $t('figurineStatusAvailable')}
              </span>
            </div>
          </div>

          <!-- Short Narrative Quote -->
          {#if figurine.shortText}
            <blockquote class="relative my-8">
              <span class="absolute -top-4 -left-2 text-6xl text-cabinet-wood/20 font-serif leading-none">“</span>
              <p class="text-xl text-cabinet-bone italic leading-relaxed opacity-90 font-['Georgia'] pl-6 border-l-2 border-cabinet-bone/10">
                {figurine.shortText}
              </p>
            </blockquote>
          {/if}

          <!-- Full Description (The Story) -->
          {#if figurine.fullDescription}
             <div class="prose prose-invert prose-p:text-cabinet-dust prose-p:font-['Georgia'] prose-p:text-lg prose-p:leading-8 prose-p:mb-4">
                <h3 class="text-cabinet-bone font-['Inter'] text-xs tracking-[0.08em] uppercase mb-4 opacity-70 border-b border-cabinet-bone/10 pb-2 inline-block">{$t('figurineHistory')}</h3>
                <p class="first-letter:text-5xl first-letter:font-['Fraunces'] first-letter:text-cabinet-bone first-letter:float-left first-letter:mr-3 first-letter:mt-[-10px]">
                    {figurine.fullDescription}
                </p>
             </div>
          {/if}

          <!-- Artifact Details Grid -->
          <div class="bg-cabinet-wood-light/10 border border-cabinet-bone/10 p-6 relative">
             <div class="absolute top-0 left-0 w-2 h-2 border-t border-l border-cabinet-bone/40"></div>
             <div class="absolute bottom-0 right-0 w-2 h-2 border-b border-r border-cabinet-bone/40"></div>
             
             <h3 class="text-center font-['Inter'] text-xs tracking-[0.10em] text-cabinet-bone/60 uppercase mb-8 flex items-center justify-center gap-4">
                <span class="h-[1px] w-8 bg-cabinet-bone/20"></span>
                {$t('figurineAttributes')}
                <span class="h-[1px] w-8 bg-cabinet-bone/20"></span>
             </h3>

             <div class="grid grid-cols-1 gap-y-6 text-sm font-['Inter']">
                
                {#if figurine.dimensions}
                <div class="flex justify-between items-baseline border-b border-dashed border-cabinet-bone/10 pb-2 hover:bg-cabinet-wood/10 transition-colors px-2">
                   <span class="text-cabinet-dust uppercase tracking-wide text-xs">{$t('figurineDimensions')}</span>
                   <span class="text-cabinet-bone text-right">{figurine.dimensions}</span>
                </div>
                {/if}

                {#if figurine.material}
                <div class="flex justify-between items-baseline border-b border-dashed border-cabinet-bone/10 pb-2 hover:bg-cabinet-wood/10 transition-colors px-2">
                   <span class="text-cabinet-dust uppercase tracking-wide text-xs">{$t('figurineMaterial')}</span>
                   <span class="text-cabinet-bone text-right max-w-[60%]">{figurine.material}</span>
                </div>
                {/if}

                 {#if figurine.technique}
                <div class="flex justify-between items-baseline border-b border-dashed border-cabinet-bone/10 pb-2 hover:bg-cabinet-wood/10 transition-colors px-2">
                   <span class="text-cabinet-dust uppercase tracking-wide text-xs">{$t('figurineTechnique')}</span>
                   <span class="text-cabinet-bone text-right max-w-[60%]">{figurine.technique}</span>
                </div>
                {/if}
                
                <div class="flex justify-between items-baseline border-b border-dashed border-cabinet-bone/10 pb-2 hover:bg-cabinet-wood/10 transition-colors px-2">
                   <span class="text-cabinet-dust uppercase tracking-wide text-xs">{$t('figurineCode')}</span>
                   <span class="text-cabinet-bone text-right font-mono text-xs opacity-60">ARC-{id.toUpperCase()}</span>
                </div>
             </div>
          </div>
          
           <!-- Action Button -->
          {#if figurine.status === 'available'}
            <div class="pt-6">
                <button
                        onclick={() => showOrderModal = true}
                        class="w-full group relative px-8 py-5 bg-transparent overflow-hidden transition-all duration-500 hover:bg-cabinet-bone/5 border border-cabinet-bone/30 hover:border-cabinet-bone/80 cursor-pointer"
                >
                  <div class="absolute inset-0 w-0 bg-cabinet-bone/10 transition-all duration-[400ms] ease-out group-hover:w-full"></div>
                  
                  <span class="relative text-cabinet-bone tracking-[0.08em] uppercase text-sm font-semibold flex items-center justify-center gap-4">
                    <span class="w-1.5 h-1.5 border border-cabinet-bone rotate-45 group-hover:rotate-90 transition-transform duration-500"></span>
                    {$t('figurineRequest')}
                    <span class="w-1.5 h-1.5 border border-cabinet-bone rotate-45 group-hover:rotate-90 transition-transform duration-500"></span>
                  </span>
                </button>
                <p class="text-center text-[10px] text-cabinet-wood-muted mt-4 tracking-wider uppercase opacity-60 font-serif italic">
                   {$t('figurineRequestNote')}
                </p>
            </div>
          {/if}

        </div>
      </div>
      
       <!-- Grimoire (Chronicle of Creation) -->
      {#if figurine.processSteps && figurine.processSteps.length > 0}
         <div class="border-t border-cabinet-bone/20 pt-16">
            <button
                onclick={toggleGrimoire}
                class="mx-auto flex flex-col items-center gap-4 group cursor-pointer"
            >
                <span class="relative font-['Fraunces'] text-2xl text-cabinet-bone opacity-80 group-hover:opacity-100 transition-opacity">
                   {$t('figurineGrimoire')}
                   <span class="absolute -top-1 -right-3 w-2 h-2 rounded-full bg-[#34251c]/60 animate-ping"></span>
                </span>
                 <div class="w-px h-16 bg-gradient-to-b from-cabinet-bone/0 via-cabinet-bone/40 to-cabinet-bone/0 group-hover:h-24 transition-all duration-500"></div>
            </button>
            
            <MemoryMirror 
                isOpen={isGrimoireOpen} 
                steps={figurine.processSteps}
                finalImage={resolveUrl(currentImage?.url)}
                onClose={() => isGrimoireOpen = false} 
            />
         </div>
      {/if}

      <!-- Visual Chronicle (Video Projection) -->
      {#if figurine.videoUrl}
         <div class="mt-24 relative">
             <div class="flex items-center justify-center gap-6 mb-12">
                 <div class="h-px w-16 bg-gradient-to-r from-transparent to-cabinet-bone/30"></div>
                 <h3 class="font-['Fraunces'] text-3xl text-cabinet-bone text-center tracking-wide">
                     {$t('figurineVideo')}
                 </h3>
                 <div class="h-px w-16 bg-gradient-to-l from-transparent to-cabinet-bone/30"></div>
             </div>

             <div class="relative w-full max-w-4xl mx-auto group perspective-container">
                 <!-- Projection Frame -->
                 <div class="relative bg-[#fff9f0] p-3 shadow-[0_20px_50px_rgba(111,59,36,0.18)] border border-cabinet-bone/10 transition-transform duration-700 hover:scale-[1.01]">
                     
                     <!-- Ornate Corners -->
                     <div class="absolute top-0 left-0 w-8 h-8 border-t border-l border-cabinet-bone/40 transition-all duration-500 group-hover:w-12 group-hover:h-12"></div>
                     <div class="absolute top-0 right-0 w-8 h-8 border-t border-r border-cabinet-bone/40 transition-all duration-500 group-hover:w-12 group-hover:h-12"></div>
                     <div class="absolute bottom-0 left-0 w-8 h-8 border-b border-l border-cabinet-bone/40 transition-all duration-500 group-hover:w-12 group-hover:h-12"></div>
                     <div class="absolute bottom-0 right-0 w-8 h-8 border-b border-r border-cabinet-bone/40 transition-all duration-500 group-hover:w-12 group-hover:h-12"></div>

                     <!-- The Screen -->
                     <div class="relative aspect-video overflow-hidden bg-[#2f2117]">
                         <div class="absolute inset-0 bg-[radial-gradient(circle,rgba(198,95,60,0.2)_0%,rgba(111,59,36,0.18)_100%)] pointer-events-none z-10"></div>
                         
                         <video
                            bind:this={videoRef}
                            controls
                            class="w-full h-full object-cover opacity-80 group-hover:opacity-100 transition-opacity duration-1000 sepia-[0.3]"
                            poster={resolveUrl(currentImage?.url)}
                            preload="metadata"
                         >
                             <source src={resolveUrl(figurine.videoUrl)} type="video/mp4" />
                             {$t('figurineBrowserNoVideo')}
                         </video>

                         <!-- Old Film Grain Overlay -->
                         <div class="absolute inset-0 pointer-events-none bg-noise opacity-[0.07] mix-blend-overlay z-20"></div>

                         <!-- Custom fullscreen button -->
                         <button
                             onclick={toggleFullscreen}
                             class="absolute top-3 right-3 z-30 bg-[#6f3b24]/25 hover:bg-[#6f3b24]/35 border border-cabinet-bone/20 hover:border-cabinet-bone/50 p-2 transition-all opacity-0 group-hover:opacity-100 font-['Inter'] text-[10px] uppercase tracking-wide text-cabinet-bone"
                             title={$t('figurineFullscreen')}
                         >⛶</button>
                         
                         <!-- Scratches/Artifacts (CSS animation could go here) -->
                     </div>
                 </div>

                 <!-- Ambient Glow -->
                 <div class="absolute -inset-4 bg-cabinet-bone/5 blur-3xl -z-10 opacity-0 group-hover:opacity-100 transition-opacity duration-1000"></div>
             </div>
             
             <p class="text-center font-['Inter'] text-[10px] tracking-[0.10em] text-cabinet-wood-muted mt-8 uppercase opacity-60">
                 {$t('figurineVideoFilm')}{id.slice(-3)}
             </p>
         </div>
      {/if}

      <!-- Neighboring Shadows (Related Items) -->
      {#if figurine.relatedItems && figurine.relatedItems.length > 0}
        <div class="mt-24 pt-12 border-t border-cabinet-bone/10 relative">
            <h3 class="font-['Fraunces'] text-3xl text-cabinet-bone/80 text-center mb-12 flex items-center justify-center gap-4">
               <span class="opacity-60">~</span> {$t('figurineRelated')} <span class="opacity-60">~</span>
            </h3>
            
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-8">
               {#each figurine.relatedItems as item}
                  <a href="/figurines/{item.id}" class="group relative block bg-[#fff9f0] border border-[#d8c6b1] p-4 transition-all duration-500 hover:-translate-y-2 hover:shadow-[0_10px_30px_rgba(111,59,36,0.14)]">
                      <div class="aspect-square overflow-hidden mb-4 relative">
                          <img 
                            src={resolveUrl(item.faceImageUrl)} 
                            alt={item.name} 
                            class="w-full h-full object-cover transition-transform duration-700 group-hover:scale-110 grayscale group-hover:grayscale-0"
                          />
                          <div class="absolute inset-0 bg-noise opacity-[0.1] mix-blend-overlay pointer-events-none"></div>
                          <div class="absolute inset-0 shadow-[inset_0_0_30px_rgba(111,59,36,0.18)] pointer-events-none"></div>
                      </div>
                      
                      <div class="text-center">
                          <h4 class="font-['Fraunces'] text-xl text-cabinet-bone/90 mb-1 group-hover:text-cabinet-bone transition-colors">{item.name}</h4>
                          <span class="text-[10px] uppercase tracking-[0.06em] text-cabinet-wood-muted">
                              {item.status === 'sold' ? $t('figurineStatusSold') : item.status === 'reserved' ? $t('figurineStatusReserved') : $t('figurineStatusAvailable')}
                          </span>
                      </div>
                  </a>
               {/each}
            </div>
        </div>
      {/if}
      
    </div>
  </div>
{/if}

<style>
  /* Noise Texture */
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }
</style>
