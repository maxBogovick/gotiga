<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import type { ProcessStep } from '$lib/types/api';

  let { steps = [], title = 'Chronicle', isOpen = false, onClose = () => {} } = $props();

  let currentPage = $state(0);
  // Total pages: Cover (0) + Intro (1) + Steps (2..n+1) + Back Cover
  let totalPages = $derived(steps.length + 3); 

  // Audio for page flips
  // Ideally, use a few variations for realism
  const playFlipSound = () => {
      // Simple toggle sound logic (placeholder URL)
      // const audio = new Audio('/sounds/page-flip.mp3'); 
      // audio.volume = 0.4;
      // audio.play().catch(() => {});
  };

  function nextPage() {
    if (currentPage < totalPages - 1) {
      currentPage++;
      playFlipSound();
    }
  }

  function prevPage() {
    if (currentPage > 0) {
      currentPage--;
      playFlipSound();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!isOpen) return;
    if (e.key === 'ArrowRight') nextPage();
    if (e.key === 'ArrowLeft') prevPage();
    if (e.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <!-- Backdrop -->
  <div 
    class="fixed inset-0 z-[60] bg-black/90 backdrop-blur-sm flex items-center justify-center p-4"
    transition:fade={{ duration: 500 }}
    onclick={(e) => { if(e.target === e.currentTarget) onClose(); }}
  >
    <!-- Book Container -->
    <div 
        class="relative perspective-2000 w-full max-w-4xl h-[600px] flex items-center justify-center"
        in:scale={{ start: 0.95, duration: 800, easing: cubicOut }}
    >
      <div class="book relative w-[400px] md:w-[480px] h-[640px] transform-style-3d">
        
        <!-- PAGES -->
        
        <!-- BACK COVER (Last) -->
        <div 
            class="page absolute inset-0 origin-left transition-transform duration-1000 ease-[cubic-bezier(0.645,0.045,0.355,1)] transform-style-3d"
            style="z-index: 0; transform: rotateY(0deg);"
        >
             <div class="face front absolute inset-0 bg-[#1A130F] rounded-r-lg shadow-xl border-l border-[#0F0A08]"></div>
             <div class="face back absolute inset-0 bg-[#1A130F] rounded-l-lg transform rotateY(180deg)"></div>
        </div>

        <!-- Dynamic Step Pages -->
        {#each steps as step, i}
             {@const pageIndex = i + 2} <!-- 0=Cover, 1=Intro -->
             {@const isFlipped = currentPage > pageIndex}
             
             <!-- 
                Z-Index Logic:
                - Unflipped (Right stack): totalPages - pageIndex
                - Flipped (Left stack): pageIndex
                
                Transition Logic:
                - Flipping (Right -> Left): Z-index goes High -> Low. We need it High during anim. Delay the change.
                - Returning (Left -> Right): Z-index goes Low -> High. We need it High during anim. Change immediately.
             -->
             <div 
                class="page absolute inset-0 origin-left transform-style-3d cursor-pointer"
                style="
                    z-index: {isFlipped ? pageIndex : totalPages - pageIndex}; 
                    transform: rotateY({isFlipped ? -180 : 0}deg);
                    transition: transform 1.2s cubic-bezier(0.645, 0.045, 0.355, 1), z-index 0s {isFlipped ? '0.6s' : '0s'};
                "
                onclick={() => isFlipped ? prevPage() : nextPage()}
             >
                <!-- FRONT of Step Page -->
                <!-- Important: backface-visibility: hidden is set via 'face' class. -->
                <div class="face front absolute inset-0 bg-[#F5F1E6] rounded-r-md overflow-hidden border-l border-[#c0b09d]">
                    <!-- Spine Shadow Gradient -->
                    <div class="absolute top-0 bottom-0 left-0 w-12 bg-gradient-to-r from-black/20 to-transparent pointer-events-none z-20"></div>
                    <!-- Paper Texture -->
                    <div class="absolute inset-0 bg-noise opacity-20 mix-blend-multiply pointer-events-none"></div>
                    <!-- Page Shadow/Lighting -->
                    <div class="absolute inset-0 shadow-[inset_0_0_60px_rgba(0,0,0,0.1)] pointer-events-none"></div>
                    
                    <div class="p-8 pl-12 h-full flex flex-col relative z-10">
                        <span class="absolute top-4 right-4 text-cabinet-wood/30 font-['UnifrakturMaguntia'] text-4xl">{i + 1}</span>
                        
                        <div class="flex-1 flex flex-col items-center justify-center gap-8">
                            <!-- Image Frame -->
                            <div class="relative p-3 bg-white shadow-lg transform rotate-1 w-full max-h-[320px] flex items-center justify-center overflow-hidden border border-gray-200">
                                <div class="absolute -top-3 left-1/2 -translate-x-1/2 w-24 h-8 bg-[#e8e1d0] opacity-90 rotate-1 shadow-sm z-20"></div> <!-- Tape -->
                                <img src={step.imageUrl} alt="Sketch" class="max-w-full max-h-[300px] object-contain sepia-[0.2] contrast-[0.95]" />
                            </div>
                            
                            <!-- Handwritten Text -->
                            <div class="w-full relative">
                                <div class="absolute -left-4 top-0 bottom-0 w-[2px] bg-red-900/10"></div>
                                <h4 class="font-['Cinzel'] text-[10px] uppercase tracking-[0.3em] text-red-900/50 mb-3">{step.stepType}</h4>
                                <p class="font-['Reenie_Beanie'] text-3xl text-[#2c2825] leading-8">
                                    {step.description}
                                </p>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- BACK of Step Page -->
                <div class="face back absolute inset-0 bg-[#EBE5CE] rounded-l-md overflow-hidden transform rotateY(180deg) border-r border-[#c0b09d]">
                    <!-- Spine Shadow Gradient (Right side for back face) -->
                    <div class="absolute top-0 bottom-0 right-0 w-12 bg-gradient-to-l from-black/20 to-transparent pointer-events-none z-20"></div>
                    <!-- Paper Texture -->
                    <div class="absolute inset-0 bg-noise opacity-25 mix-blend-multiply pointer-events-none"></div>
                    
                    <!-- Content -->
                    <div class="h-full flex items-center justify-center opacity-10 select-none">
                         <span class="font-['UnifrakturMaguntia'] text-9xl text-[#2c2825]">†</span>
                    </div>
                </div>
             </div>
        {/each}

        <!-- INTRO PAGE (Index 1) -->
        <!-- Logic: If currentPage > 1, intro is flipped. -->
        
        <div 
            class="page absolute inset-0 origin-left transform-style-3d cursor-pointer"
            style="
                z-index: {currentPage > 1 ? 1 : totalPages - 1}; 
                transform: rotateY({currentPage > 1 ? -180 : 0}deg);
                transition: transform 1.2s cubic-bezier(0.645, 0.045, 0.355, 1), z-index 0s {currentPage > 1 ? '0.6s' : '0s'};
            "
            onclick={() => currentPage > 1 ? prevPage() : nextPage()}
        >
            <!-- FRONT: Intro -->
            <div class="face front absolute inset-0 bg-[#F5F1E6] rounded-r-md overflow-hidden border-l border-[#c0b09d]">
                <div class="absolute top-0 bottom-0 left-0 w-10 bg-gradient-to-r from-black/15 to-transparent pointer-events-none z-20"></div>
                <div class="absolute inset-0 bg-noise opacity-30 mix-blend-multiply pointer-events-none"></div>
                
                <div class="p-12 pl-16 h-full flex flex-col justify-center items-center text-center">
                    <h2 class="font-['UnifrakturMaguntia'] text-6xl text-[#2A1F18] mb-6 drop-shadow-sm">{title}</h2>
                    <div class="w-24 h-px bg-[#2A1F18]/40 mb-2"></div>
                    <div class="w-16 h-px bg-[#2A1F18]/40 mb-10"></div>
                    
                    <p class="font-['Cinzel'] text-xs text-[#5A524C] uppercase tracking-[0.4em] mb-16">Гримуар Создания</p>
                    
                    <div class="relative px-8 py-6 border-y border-double border-[#2A1F18]/10">
                        <p class="font-['Cormorant_Garamond'] text-2xl italic text-[#2A1F18]/80 leading-relaxed">
                            "Здесь хранятся тени мыслей и первые штрихи, из которых родилась форма."
                        </p>
                    </div>
                    
                    <div class="mt-auto opacity-40">
                        <span class="font-['UnifrakturMaguntia'] text-4xl">Ex Libris</span>
                    </div>
                </div>
            </div>
            
            <!-- BACK: Blank/Texture -->
            <div class="face back absolute inset-0 bg-[#EBE5CE] rounded-l-md transform rotateY(180deg) border-r border-[#c0b09d]">
                <div class="absolute top-0 bottom-0 right-0 w-10 bg-gradient-to-l from-black/15 to-transparent pointer-events-none z-20"></div>
                <div class="absolute inset-0 bg-noise opacity-20 mix-blend-multiply pointer-events-none"></div>
            </div>
        </div>

        <!-- FRONT COVER (Index 0) -->
        <!-- Logic: If currentPage > 0, cover is flipped. -->
        
        <div 
            class="page absolute inset-0 origin-left transform-style-3d cursor-pointer"
            style="
                z-index: {currentPage > 0 ? 0 : totalPages}; 
                transform: rotateY({currentPage > 0 ? -180 : 0}deg);
                transition: transform 1.2s cubic-bezier(0.645, 0.045, 0.355, 1), z-index 0s {currentPage > 0 ? '0.6s' : '0s'};
            "
            onclick={() => currentPage > 0 ? prevPage() : nextPage()}
        >
            <!-- FRONT COVER FACE -->
            <div class="face front absolute inset-0 bg-[#140F0C] rounded-r-lg shadow-2xl overflow-hidden border-l-4 border-[#0F0A08]">
                <!-- Leather Texture -->
                <div class="absolute inset-0 opacity-60 bg-[url('https://www.transparenttextures.com/patterns/black-scales.png')] mix-blend-overlay"></div>
                <!-- Spine Highlight -->
                <div class="absolute top-0 bottom-0 left-0 w-8 bg-gradient-to-r from-white/10 to-transparent pointer-events-none"></div>
                
                <!-- Gold Embossing Frame -->
                <div class="absolute inset-0 border-[2px] border-[#Cda45e]/30 m-3 rounded-sm"></div>
                <div class="absolute inset-0 border-[1px] border-[#Cda45e]/20 m-5 rounded-sm"></div>

                <div class="h-full flex flex-col items-center justify-center p-8 text-center relative z-10">
                    <div class="w-40 h-40 border-2 border-[#Cda45e] rounded-full flex items-center justify-center mb-10 shadow-[0_0_30px_rgba(205,164,94,0.15)] bg-black/20 backdrop-blur-sm">
                        <span class="font-['UnifrakturMaguntia'] text-8xl text-[#Cda45e] drop-shadow-lg mt-2">G</span>
                    </div>
                    <h1 class="font-['UnifrakturMaguntia'] text-5xl text-[#Cda45e] tracking-widest drop-shadow-xl mb-4">{title}</h1>
                    <div class="h-px w-20 bg-[#Cda45e]/50 mb-2"></div>
                    <span class="text-[#8a7f70] font-['Cinzel'] text-[10px] tracking-[0.5em] uppercase">Secretum Archivum</span>
                </div>
            </div>
            
            <!-- INSIDE FRONT COVER -->
            <div class="face back absolute inset-0 bg-[#2A1F18] rounded-l-lg transform rotateY(180deg) shadow-inner">
                 <div class="absolute inset-0 opacity-30 bg-[url('https://www.transparenttextures.com/patterns/black-scales.png')] mix-blend-overlay"></div>
                 <div class="absolute top-0 bottom-0 right-0 w-12 bg-gradient-to-l from-black/50 to-transparent pointer-events-none"></div>
            </div>
        </div>

      </div>
      
      <!-- Close Button (Outside Book) -->
      <button 
        onclick={onClose}
        class="absolute -top-12 right-0 md:-right-16 text-cabinet-bone/50 hover:text-cabinet-bone hover:rotate-90 transition-all duration-500 font-['UnifrakturMaguntia'] text-4xl"
        aria-label="Закрыть"
      >
        ✕
      </button>

      <!-- Navigation Hints -->
      <div class="absolute -bottom-20 left-0 right-0 flex justify-center gap-12 text-cabinet-bone/40 font-['Cinzel'] text-xs tracking-widest uppercase select-none">
          <button onclick={prevPage} class="hover:text-cabinet-bone transition-colors flex items-center gap-3 group disabled:opacity-20" disabled={currentPage === 0}>
              <span class="group-hover:-translate-x-1 transition-transform">←</span> Назад
          </button>
          <span class="opacity-30">|</span>
          <button onclick={nextPage} class="hover:text-cabinet-bone transition-colors flex items-center gap-3 group disabled:opacity-20" disabled={currentPage === totalPages - 1}>
              Далее <span class="group-hover:translate-x-1 transition-transform">→</span>
          </button>
      </div>

    </div>
  </div>
{/if}

<style>
  .perspective-2000 {
    perspective: 2000px;
  }
  
  .transform-style-3d {
    transform-style: preserve-3d;
  }
  
  .face {
    backface-visibility: hidden;
  }
  
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }
</style>