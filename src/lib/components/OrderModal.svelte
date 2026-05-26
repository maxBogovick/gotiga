<script lang="ts">
  import { fade, scale, fly } from 'svelte/transition';
  import { cubicOut, elasticOut } from 'svelte/easing';
  import { api, isTauri } from '$lib/api';
  import { t } from '$lib/i18n';

  let { isOpen = false, figurineName = '', figurineId = '', onClose = () => {} } = $props();

  let name = $state('');
  let email = $state('');
  let message = $state('');
  let isSubmitting = $state(false);
  let isSealed = $state(false);

  function close() {
    if (!isSubmitting) {
      onClose();
      // Reset state after transition
      setTimeout(() => {
        isSealed = false;
        name = '';
        email = '';
        message = '';
      }, 500);
    }
  }

  async function handleSubmit() {
    if (!name.trim() || !email.trim()) return;

    isSubmitting = true;

    const subject = encodeURIComponent(`${$t('orderEmailSubject')}${figurineName}`);
    const body = encodeURIComponent(
      `${$t('orderEmailPetitioner')}${name.trim()}\n${$t('orderEmailAddress')}${email.trim()}\n\n${$t('orderEmailMessage')}${message.trim() || '—'}\n\n${$t('orderEmailSentVia')}`
    );
    const contactEmail = localStorage.getItem('gotiga_contact_email') || 'info@gotiga.art';
    const href = `mailto:${contactEmail}?subject=${subject}&body=${body}`;

    // Trigger mailto immediately (before any await to keep user-gesture context)
    const a = document.createElement('a');
    a.href = href;
    a.style.display = 'none';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);

    // Notify server (Telegram) in parallel — silently ignore failures
    if (!isTauri) {
      api.submitOrder({
        figurineId: figurineId || 'unknown',
        figurineName,
        requesterName: name.trim(),
        requesterEmail: email.trim(),
        message: message.trim() || null,
      }).catch(() => {});
    }

    await new Promise(r => setTimeout(r, 800));

    isSubmitting = false;
    isSealed = true;

    setTimeout(() => {
      close();
    }, 3000);
  }

  function handleBackdropKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      close();
    }
  }
</script>

{#if isOpen}
  <!-- Backdrop -->
  <div
          class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-[#050403]/90 backdrop-blur-sm"
          transition:fade={{ duration: 400 }}
          onclick={close}
          onkeydown={handleBackdropKeydown}
          role="button"
          tabindex="0"
          aria-label={$t('lightboxClose')}
  >
    <!-- The Letter/Scroll Container -->
    <div
            class="relative w-full max-w-lg perspective-1000"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
            tabindex="-1"
            in:fly={{ y: 50, duration: 800, easing: cubicOut }}
    >

      <!-- Paper Texture Background -->
      <!-- Используем более темный и насыщенный цвет пергамента для контраста и атмосферы -->
      <div class="relative bg-[#E6DCC8] shadow-[0_20px_60px_rgba(0,0,0,0.7)] p-1 overflow-hidden transform rotate-1 transition-transform duration-500 hover:rotate-0 border border-[#2A2622]/20 rounded-sm">
        
        <!-- Inner Border for "Document" feel -->
        <div class="border-[3px] border-double border-[#2A2622]/10 p-8 md:p-10 h-full relative bg-[#E6DCC8]">
            
            <!-- Paper grains/texture overlay -->
            <div class="absolute inset-0 pointer-events-none opacity-20 mix-blend-multiply bg-noise"></div>
            <!-- Dirty edges/vignette -->
            <div class="absolute inset-0 pointer-events-none shadow-[inset_0_0_120px_rgba(46,43,40,0.15)]"></div>
            <!-- Watermark / Background Symbol -->
            <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-64 h-64 border-4 border-[#2A2622]/5 rounded-full flex items-center justify-center pointer-events-none">
                 <div class="w-48 h-48 border border-[#2A2622]/5 rotate-45"></div>
            </div>

            <!-- Content -->
            <div class="relative z-10 font-['Cormorant_Garamond'] text-[#2A2622]">

              {#if !isSealed}
                <div out:fade={{ duration: 300 }}>
                  <div class="text-center mb-10 relative">
                    <span class="absolute -top-6 left-1/2 -translate-x-1/2 text-5xl opacity-10 font-['UnifrakturMaguntia']">~</span>
                    <h3 class="font-['UnifrakturMaguntia'] text-4xl mb-2 text-[#1A1816] drop-shadow-sm tracking-wide">{$t('orderTitle')}</h3>
                    <div class="flex items-center justify-center gap-3 text-[#5A524C]">
                        <span class="h-px w-8 bg-[#5A524C]/30"></span>
                        <p class="italic text-lg font-semibold tracking-wide">Ref: {figurineName}</p>
                        <span class="h-px w-8 bg-[#5A524C]/30"></span>
                    </div>
                  </div>

                  <form class="space-y-8" onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>

                    <div class="relative group">
                      <input
                              id="name"
                              type="text"
                              bind:value={name}
                              required
                              class="peer w-full bg-transparent border-b-2 border-[#2A2622]/20 py-2 text-2xl text-[#1A1816] font-['UnifrakturMaguntia'] focus:outline-none focus:border-[#7A2E2E] transition-colors placeholder-transparent"
                              placeholder={$t('orderNameLabel')}
                      />
                      <label for="name" class="absolute left-0 -top-4 text-xs font-['Cinzel'] font-bold tracking-[0.2em] text-[#5A524C] uppercase transition-all peer-placeholder-shown:text-base peer-placeholder-shown:top-2 peer-placeholder-shown:text-[#5A524C]/60 peer-focus:-top-4 peer-focus:text-xs peer-focus:text-[#7A2E2E]">
                          {$t('orderNameLabel')}
                      </label>
                    </div>

                    <div class="relative group">
                      <input
                              id="email"
                              type="email"
                              bind:value={email}
                              required
                              class="peer w-full bg-transparent border-b-2 border-[#2A2622]/20 py-2 text-xl italic font-serif text-[#1A1816] focus:outline-none focus:border-[#7A2E2E] transition-colors placeholder-transparent"
                              placeholder="Email"
                      />
                       <label for="email" class="absolute left-0 -top-4 text-xs font-['Cinzel'] font-bold tracking-[0.2em] text-[#5A524C] uppercase transition-all peer-placeholder-shown:text-base peer-placeholder-shown:top-2 peer-placeholder-shown:text-[#5A524C]/60 peer-focus:-top-4 peer-focus:text-xs peer-focus:text-[#7A2E2E]">
                          {$t('orderEmailLabel')}
                      </label>
                    </div>

                    <div class="relative group pt-2">
                       <label for="message" class="block text-xs font-['Cinzel'] font-bold tracking-[0.2em] text-[#5A524C] uppercase mb-2">
                          {$t('orderMessageLabel')}
                       </label>
                      <textarea
                              id="message"
                              bind:value={message}
                              rows="3"
                              class="w-full bg-[#2A2622]/5 border border-[#2A2622]/10 p-3 text-lg italic text-[#1A1816] focus:outline-none focus:border-[#7A2E2E]/50 focus:bg-[#2A2622]/10 transition-colors placeholder-[#5A524C]/40 resize-none rounded-sm"
                              placeholder={$t('orderMessagePlaceholder')}
                      ></textarea>
                    </div>

                    <div class="pt-8 flex justify-center">
                      <button
                              type="submit"
                              disabled={isSubmitting}
                              class="group relative inline-flex items-center justify-center px-10 py-4 overflow-hidden font-['Cinzel'] font-bold tracking-widest text-[#E6DCC8] bg-[#4A1C1C] hover:bg-[#5C2222] transition-all duration-500 shadow-lg disabled:opacity-80 disabled:cursor-not-allowed border border-[#2A2622]/20 clip-corners"
                      >
                         <span class="absolute inset-0 w-full h-full bg-noise opacity-10"></span>
                         <span class="absolute w-0 h-0 transition-all duration-500 ease-out bg-[#7A2E2E] rounded-full group-hover:w-72 group-hover:h-72 opacity-50"></span>
                         
                         <span class="relative flex items-center gap-3 z-10">
                           {#if isSubmitting}
                             <span class="w-4 h-4 border-2 border-[#E6DCC8]/50 border-t-[#E6DCC8] rounded-full animate-spin"></span>
                             <span class="animate-pulse">{$t('orderSubmitting')}</span>
                           {:else}
                             <span>{$t('orderSubmit')}</span>
                             <span class="text-lg opacity-70">✒</span>
                           {/if}
                         </span>
                      </button>
                    </div>
                     
                    <div class="text-center mt-6">
                        <button onclick={close} type="button" class="text-xs font-['Cinzel'] text-[#5A524C]/60 hover:text-[#7A2E2E] tracking-widest uppercase border-b border-transparent hover:border-[#7A2E2E]/30 transition-all">
                            {$t('orderCancel')}
                        </button>
                    </div>

                  </form>
                </div>
              {:else}
                <!-- Success / Sealed State -->
                <div class="flex flex-col items-center justify-center py-12" in:scale={{ duration: 700, start: 0.95, easing: elasticOut }}>
                   
                   <!-- The Wax Seal Animation -->
                   <div class="relative w-40 h-40 mb-8 filter drop-shadow-2xl">
                      <!-- Wax Body -->
                      <div class="absolute inset-0 bg-gradient-to-br from-[#9E3B3B] via-[#7A2E2E] to-[#4A1C1C] rounded-full transform rotate-12 flex items-center justify-center border-4 border-[#3A1515]/20 box-border wax-seal-shape animate-seal-press">
                          <!-- Inner Ring -->
                          <div class="w-28 h-28 border-2 border-[#3A1515]/10 rounded-full flex items-center justify-center shadow-inner">
                              <!-- Symbol -->
                              <span class="font-['UnifrakturMaguntia'] text-6xl text-[#3A1515] drop-shadow-md opacity-60 mt-2 ml-1">G</span>
                          </div>
                      </div>
                      <!-- Realistic Shine -->
                      <div class="absolute top-8 left-10 w-8 h-4 bg-white opacity-20 blur-sm rounded-full rotate-45"></div>
                   </div>

                   <h3 class="font-fraktur text-5xl text-[#2A2622] mb-4 tracking-wide">{$t('orderSuccessTitle')}</h3>

                   <div class="relative max-w-xs text-center">
                       <span class="absolute -left-4 top-0 text-4xl text-[#2A2622]/10 font-serif">"</span>
                       <p class="font-['Cinzel'] text-[#5A524C] text-sm leading-relaxed font-semibold">
                         {$t('orderSuccessText')}
                       </p>
                       <span class="absolute -right-2 bottom-0 text-4xl text-[#2A2622]/10 font-serif rotate-180">"</span>
                   </div>
                </div>
              {/if}

            </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .perspective-1000 {
    perspective: 1000px;
  }
  
  /* Custom clip-path for button corners */
  .clip-corners {
      clip-path: polygon(
          10px 0, 100% 0, 
          100% calc(100% - 10px), calc(100% - 10px) 100%, 
          0 100%, 0 10px
      );
  }

  /* Irregular shape for wax seal using border-radius trick */
  .wax-seal-shape {
      border-radius: 45% 55% 48% 52% / 51% 46% 54% 49%;
      box-shadow: 
        inset 2px 2px 15px rgba(0,0,0,0.3),
        inset -2px -2px 10px rgba(255,255,255,0.1);
  }

  .font-fraktur {
    font-family: 'UnifrakturMaguntia', serif;
  }

  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }

  @keyframes sealPress {
      0% { transform: scale(1.1) rotate(12deg); opacity: 0; }
      100% { transform: scale(1) rotate(12deg); opacity: 1; }
  }
  
  .animate-seal-press {
      animation: sealPress 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275) forwards;
  }
</style>
