<script lang="ts">
  import { fade, fly, scale } from 'svelte/transition';
  import { cubicOut, elasticOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';

  let {
    isOpen        = false,
    figurineId    = '',
    figurineName  = '',
    onClose       = () => {},
  }: {
    isOpen?: boolean;
    figurineId?: string;
    figurineName?: string;
    onClose?: () => void;
  } = $props();

  let name    = $state('');
  let email   = $state('');
  let phone   = $state('');
  let note    = $state('');
  let submitting = $state(false);
  let submitError = $state('');
  let done    = $state(false);

  function close() {
    if (submitting) return;
    onClose();
    setTimeout(() => {
      name = ''; email = ''; phone = ''; note = '';
      submitError = ''; done = false;
    }, 400);
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    const effectiveName  = authStore.isLoggedIn ? (authStore.user?.displayName ?? '') : name.trim();
    const effectiveEmail = authStore.isLoggedIn ? (authStore.user?.email ?? '') : email.trim();
    if (!effectiveName || !effectiveEmail) { submitError = $t('waitlistFillFields'); return; }
    submitting = true; submitError = '';
    try {
      await api.joinWaitlist(figurineId, {
        figurineName,
        requesterName: effectiveName,
        requesterEmail: effectiveEmail,
        requesterPhone: phone.trim() || null,
        note: note.trim() || null,
      });
      done = true;
    } catch {
      submitError = $t('waitlistError');
    } finally {
      submitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <div
    class="fixed inset-0 z-[220] flex items-center justify-center p-4 bg-[#6f3b24]/35 backdrop-blur-sm"
    transition:fade={{ duration: 400 }}
    role="presentation"
  >
    <div
      role="dialog"
      aria-modal="true"
      class="relative w-full max-w-md perspective-1000"
      in:fly={{ y: 40, duration: 600, easing: cubicOut }}
    >
      <div class="relative bg-[#fff9f0] shadow-[0_20px_60px_rgba(111,59,36,0.18)] p-1 overflow-hidden transform rotate-1 transition-transform duration-500 hover:rotate-0 border border-[#d8c6b1]">
        <div class="border-[3px] border-double border-[#c9a875]/35 relative bg-[#fff9f0] max-h-[90vh] overflow-hidden">
          <div class="absolute inset-0 pointer-events-none opacity-20 mix-blend-multiply bg-noise z-0"></div>
          <div class="relative z-10 overflow-y-auto max-h-[90vh] p-7 font-['Georgia'] text-[#34251c]">

            {#if !done}
              <div out:fade={{ duration: 250 }}>
                <div class="text-center mb-5">
                  <span class="text-3xl opacity-20 font-['Fraunces'] block mb-1">~</span>
                  <h3 class="font-['Fraunces'] text-2xl text-[#6f3b24] tracking-wide mb-1">{$t('waitlistTitle')}</h3>
                  <p class="text-sm italic text-[#5f4636] font-semibold">{figurineName}</p>
                  <p class="text-xs text-[#5f4636]/70 mt-2 leading-relaxed">{$t('waitlistHint')}</p>
                </div>

                <form class="space-y-5" onsubmit={handleSubmit}>

                  {#if authStore.isLoggedIn}
                    <div class="flex items-center gap-2 border-b border-[#d8c6b1] pb-2">
                      <span class="w-7 h-7 rounded-full bg-[#efe6d6] border border-[#d8c6b1] flex items-center justify-center font-['Fraunces'] text-sm text-[#9a7c5c]">
                        {(authStore.user?.displayName ?? '?')[0].toUpperCase()}
                      </span>
                      <p class="text-sm text-[#5f4636] italic">
                        {$t('formLoggedInAs')} <strong class="text-[#34251c] not-italic">{authStore.user?.displayName}</strong>
                      </p>
                    </div>
                  {:else}
                    <div class="space-y-1.5">
                      <label for="wl-name" class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase">{$t('orderNameLabel')}</label>
                      <input id="wl-name" type="text" bind:value={name} required
                        class="w-full bg-transparent border-0 border-b-2 border-[#d8c6b1] py-2 text-xl text-[#34251c] font-['Fraunces'] focus:outline-none focus:border-[#c65f3c] transition-colors" />
                    </div>
                    <div class="space-y-1.5">
                      <label for="wl-email" class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase">{$t('orderEmailLabel')}</label>
                      <input id="wl-email" type="email" bind:value={email} required
                        class="w-full bg-transparent border-0 border-b-2 border-[#d8c6b1] py-2 text-xl italic font-serif text-[#34251c] focus:outline-none focus:border-[#c65f3c] transition-colors" />
                    </div>
                  {/if}

                  <div class="space-y-1.5">
                    <label for="wl-phone" class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase">{$t('orderPhoneLabel')}</label>
                    <input id="wl-phone" type="tel" bind:value={phone}
                      placeholder={$t('orderPhonePlaceholder')}
                      class="w-full bg-transparent border-0 border-b-2 border-[#d8c6b1] py-2 text-xl italic font-serif text-[#34251c] focus:outline-none focus:border-[#c65f3c] transition-colors" />
                  </div>

                  <div class="space-y-1.5">
                    <label for="wl-note" class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase">{$t('waitlistNoteLabel')}</label>
                    <textarea id="wl-note" bind:value={note} rows="2"
                      placeholder={$t('waitlistNotePlaceholder')}
                      class="w-full bg-[#f8f1e7] border-0 border-b border-[#d8c6b1] p-2 text-base italic text-[#34251c] focus:outline-none focus:border-[#c65f3c]/70 transition-colors placeholder-[#5f4636]/40 resize-none" />
                  </div>

                  <div class="pt-3 flex justify-center">
                    <button type="submit" disabled={submitting}
                      class="inline-flex items-center justify-center px-10 py-3 font-['Inter'] font-bold tracking-wide text-[#fff9f0] bg-[#6f3b24] hover:bg-[#34251c] transition-colors disabled:opacity-70 disabled:cursor-not-allowed border border-[#34251c]/20">
                      {#if submitting}
                        <span class="w-4 h-4 border-2 border-[#fff9f0]/50 border-t-[#fff9f0] rounded-full animate-spin mr-2"></span>
                        <span class="animate-pulse">{$t('waitlistSubmitting')}</span>
                      {:else}
                        {$t('waitlistSubmit')}
                      {/if}
                    </button>
                  </div>

                  {#if submitError}
                    <p class="text-sm text-red-800 text-center font-['Inter'] italic">{submitError}</p>
                  {/if}

                  <div class="text-center">
                    <button type="button" onclick={close}
                      class="text-xs font-['Inter'] text-[#5f4636]/90 hover:text-[#c65f3c] tracking-wide uppercase border-b border-transparent hover:border-[#c65f3c]/30 transition-all">
                      {$t('bookingCancel')}
                    </button>
                  </div>
                </form>
              </div>

            {:else}
              <div class="flex flex-col items-center justify-center py-10" in:scale={{ duration: 600, start: 0.95, easing: elasticOut }}>
                <div class="text-5xl text-[#6f3b24]/30 mb-4">✦</div>
                <h3 class="font-['Fraunces'] text-3xl text-[#6f3b24] mb-3">{$t('waitlistSuccessTitle')}</h3>
                <p class="text-sm text-[#5f4636] italic text-center leading-relaxed max-w-xs">{$t('waitlistSuccessText')}</p>
                <button onclick={close} class="mt-6 text-xs font-['Inter'] text-[#5f4636]/70 hover:text-[#c65f3c] tracking-wide uppercase border-b border-transparent hover:border-[#c65f3c]/30 transition-all">
                  {$t('cancelGoToFigurine')}
                </button>
              </div>
            {/if}

          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .perspective-1000 { perspective: 1000px; }
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }
</style>
