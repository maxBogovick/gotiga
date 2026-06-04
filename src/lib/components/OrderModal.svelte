<script lang="ts">
  import { fade, scale, fly } from 'svelte/transition';
  import { cubicOut, elasticOut } from 'svelte/easing';
  import { api, isTauri } from '$lib/api';
  import { t } from '$lib/i18n';

  let {
    isOpen = false,
    figurineName = '',
    figurineId = '',
    mode = 'request' as 'request' | 'question' | 'notify',
    onClose = () => {}
  } = $props();

  let name = $state('');
  let email = $state('');
  let message = $state('');
  let isSubmitting = $state(false);
  let isSealed = $state(false);
  let submitError = $state('');

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

  // Вычисляемые строки для разных режимов
  let modalTitle  = $derived(mode === 'question' ? $t('figurineAskQuestion') : mode === 'notify' ? $t('figurineNotify') : $t('orderTitle'));
  let submitLabel = $derived(mode === 'question' ? $t('figurineAskQuestion') : mode === 'notify' ? $t('figurineNotify') : $t('orderSubmit'));
  let subjectPrefix = $derived(
    mode === 'question' ? 'Question: '
    : mode === 'notify'   ? 'Notify me: '
    : ''
  );

  async function handleSubmit() {
    if (!email.trim()) return;
    if (mode === 'request' && !name.trim()) return;

    isSubmitting = true;
    submitError = '';

    try {
      await api.submitOrder({
        figurineId: figurineId || 'unknown',
        figurineName,
        requesterName: name.trim() || '—',
        requesterEmail: email.trim(),
        message: message.trim() || null,
        mode,
      });

      isSealed = true;
      setTimeout(() => { close(); }, 3000);
    } catch {
      submitError = $t('orderSubmitError');
    } finally {
      isSubmitting = false;
    }
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
          class="fixed inset-0 z-[220] flex items-center justify-center p-4 bg-[#6f3b24]/35 backdrop-blur-sm"
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
      <div class="relative bg-[#fff9f0] shadow-[0_20px_60px_rgba(111,59,36,0.18)] p-1 overflow-hidden transform rotate-1 transition-transform duration-500 hover:rotate-0 border border-[#d8c6b1] rounded-sm">
        
        <!-- Inner Border for "Document" feel -->
        <div class="border-[3px] border-double border-[#c9a875]/35 p-8 md:p-10 h-full relative bg-[#fff9f0]">
            
            <!-- Paper grains/texture overlay -->
            <div class="absolute inset-0 pointer-events-none opacity-20 mix-blend-multiply bg-noise"></div>
            <!-- Dirty edges/vignette -->
            <div class="absolute inset-0 pointer-events-none shadow-[inset_0_0_120px_rgba(46,43,40,0.15)]"></div>
            <!-- Watermark / Background Symbol -->
            <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-64 h-64 border-4 border-[#c9a875]/15 rounded-full flex items-center justify-center pointer-events-none">
                 <div class="w-48 h-48 border border-[#c9a875]/15 rotate-45"></div>
            </div>

            <!-- Content -->
            <div class="relative z-10 font-['Georgia'] text-[#34251c]">

              {#if !isSealed}
                <div out:fade={{ duration: 300 }}>
                  <div class="text-center mb-10 relative">
                    <span class="absolute -top-6 left-1/2 -translate-x-1/2 text-5xl opacity-10 font-['Fraunces']">~</span>
                    <h3 class="font-['Fraunces'] text-4xl mb-2 text-[#6f3b24] drop-shadow-sm tracking-wide">{modalTitle}</h3>
                    <div class="flex items-center justify-center gap-3 text-[#5f4636]">
                        <span class="h-px w-8 bg-[#5f4636]/30"></span>
                        <p class="italic text-lg font-semibold tracking-wide">Ref: {figurineName}</p>
                        <span class="h-px w-8 bg-[#5f4636]/30"></span>
                    </div>
                  </div>

                  <form class="space-y-8" onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>

                    {#if mode === 'request'}
                    <div class="relative group">
                      <input
                              id="name"
                              type="text"
                              bind:value={name}
                              required
                              class="peer w-full bg-transparent border-b-2 border-[#d8c6b1] py-2 text-2xl text-[#34251c] font-['Fraunces'] focus:outline-none focus:border-[#c65f3c] transition-colors placeholder-transparent"
                              placeholder={$t('orderNameLabel')}
                      />
                      <label for="name" class="absolute left-0 -top-4 text-xs font-['Inter'] font-bold tracking-[0.06em] text-[#5f4636] uppercase transition-all peer-placeholder-shown:text-base peer-placeholder-shown:top-2 peer-placeholder-shown:text-[#5f4636]/90 peer-focus:-top-4 peer-focus:text-xs peer-focus:text-[#c65f3c]">
                          {$t('orderNameLabel')}
                      </label>
                    </div>
                    {/if}

                    <div class="relative group">
                      <input
                              id="email"
                              type="email"
                              bind:value={email}
                              required
                              class="peer w-full bg-transparent border-b-2 border-[#d8c6b1] py-2 text-xl italic font-serif text-[#34251c] focus:outline-none focus:border-[#c65f3c] transition-colors placeholder-transparent"
                              placeholder="Email"
                      />
                       <label for="email" class="absolute left-0 -top-4 text-xs font-['Inter'] font-bold tracking-[0.06em] text-[#5f4636] uppercase transition-all peer-placeholder-shown:text-base peer-placeholder-shown:top-2 peer-placeholder-shown:text-[#5f4636]/90 peer-focus:-top-4 peer-focus:text-xs peer-focus:text-[#c65f3c]">
                          {$t('orderEmailLabel')}
                      </label>
                    </div>

                    <div class="relative group pt-2">
                       <label for="message" class="block text-xs font-['Inter'] font-bold tracking-[0.06em] text-[#5f4636] uppercase mb-2">
                          {$t('orderMessageLabel')}
                       </label>
                      <textarea
                              id="message"
                              bind:value={message}
                              rows="3"
                              class="w-full bg-[#f8f1e7] border border-[#d8c6b1] p-3 text-lg italic text-[#34251c] focus:outline-none focus:border-[#c65f3c]/50 focus:bg-[#fff9f0] transition-colors placeholder-[#5f4636]/55 resize-none rounded-sm"
                              placeholder={$t('orderMessagePlaceholder')}
                      ></textarea>
                    </div>

                    <div class="pt-8 flex justify-center">
                      <button
                              type="submit"
                              disabled={isSubmitting}
                              class="group relative inline-flex items-center justify-center px-10 py-4 overflow-hidden font-['Inter'] font-bold tracking-wide text-[#fff9f0] bg-[#9e452d] hover:bg-[#6f3b24] transition-all duration-500 shadow-lg disabled:opacity-80 disabled:cursor-not-allowed border border-[#6f3b24]/20 clip-corners"
                      >
                         <span class="absolute inset-0 w-full h-full bg-noise opacity-10"></span>
                         <span class="absolute w-0 h-0 transition-all duration-500 ease-out bg-[#c65f3c] rounded-full group-hover:w-72 group-hover:h-72 opacity-75"></span>
                         
                         <span class="relative flex items-center gap-3 z-10">
                           {#if isSubmitting}
                             <span class="w-4 h-4 border-2 border-[#fff9f0]/50 border-t-[#fff9f0] rounded-full animate-spin"></span>
                             <span class="animate-pulse">{$t('orderSubmitting')}</span>
                           {:else}
                             <span>{submitLabel}</span>
                             <span class="text-lg opacity-70">✒</span>
                           {/if}
                         </span>
                      </button>
                    </div>
                     
                    {#if submitError}
                      <p class="text-center text-sm text-red-700 font-['Inter']">{submitError}</p>
                    {/if}

                    <div class="text-center mt-6">
                        <button onclick={close} type="button" class="text-xs font-['Inter'] text-[#5f4636]/90 hover:text-[#c65f3c] tracking-wide uppercase border-b border-transparent hover:border-[#c65f3c]/30 transition-all">
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
                      <div class="absolute inset-0 bg-gradient-to-br from-[#c65f3c] via-[#a86124] to-[#9e452d] rounded-full transform rotate-12 flex items-center justify-center border-4 border-[#6f3b24]/20 box-border wax-seal-shape animate-seal-press">
                          <!-- Inner Ring -->
                          <div class="w-28 h-28 border-2 border-[#6f3b24]/20 rounded-full flex items-center justify-center shadow-inner">
                              <!-- Symbol -->
                              <span class="font-['Fraunces'] text-6xl text-[#6f3b24] drop-shadow-md opacity-70 mt-2 ml-1">G</span>
                          </div>
                      </div>
                      <!-- Realistic Shine -->
                      <div class="absolute top-8 left-10 w-8 h-4 bg-[#fff9f0] opacity-25 blur-sm rounded-full rotate-45"></div>
                   </div>

                   <h3 class="font-fraktur text-5xl text-[#6f3b24] mb-4 tracking-wide">{$t('orderSuccessTitle')}</h3>

                   <div class="relative max-w-xs text-center">
                       <span class="absolute -left-4 top-0 text-4xl text-[#6f3b24]/15 font-serif">"</span>
                       <p class="font-['Inter'] text-[#5f4636] text-sm leading-relaxed font-semibold">
                         {$t('orderSuccessText')}
                       </p>
                       <span class="absolute -right-2 bottom-0 text-4xl text-[#6f3b24]/15 font-serif rotate-180">"</span>
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
        inset 2px 2px 15px rgba(111,59,36,0.16),
        inset -2px -2px 10px rgba(255,255,255,0.1);
  }

  .font-fraktur {
    font-family: 'Fraunces', serif;
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
