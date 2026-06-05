<script lang="ts">
  import { fade, scale, fly } from 'svelte/transition';
  import { cubicOut, elasticOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { FigurineSchedule } from '$lib/types/api';
  import DateRangePicker from '$lib/components/DateRangePicker.svelte';

  let {
    isOpen       = false,
    figurineName = '',
    figurineId   = '',
    schedule     = { entries: [] } as FigurineSchedule,
    onClose      = () => {}
  } = $props();

  const today = new Date().toISOString().split('T')[0];

  function addDays(ds: string, n: number) {
    const d = new Date(ds + 'T00:00:00');
    d.setDate(d.getDate() + n);
    return d.toISOString().split('T')[0];
  }

  // ── Form state ────────────────────────────────────────────────────────────
  let name        = $state('');
  let email       = $state('');
  let purpose     = $state('');
  let startsAt    = $state(today);
  let endsAt      = $state(addDays(today, 1));
  let dateError   = $state('');
  let submitError = $state('');
  let isSubmitting = $state(false);
  let isSealed    = $state(false);

  // ── Handlers ──────────────────────────────────────────────────────────────
  function close() {
    if (isSubmitting) return;
    onClose();
    setTimeout(() => {
      isSealed     = false;
      name         = '';
      email        = '';
      purpose      = '';
      startsAt     = today;
      endsAt       = addDays(today, 1);
      dateError    = '';
      submitError  = '';
    }, 500);
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (!name.trim() || !email.trim()) {
      submitError = $t('bookingFillFields');
      return;
    }
    if (!startsAt || !endsAt) {
      submitError = $t('bookingSelectPeriod');
      return;
    }
    if (dateError) {
      submitError = dateError;
      return;
    }
    isSubmitting = true;
    submitError  = '';
    try {
      await api.submitBooking({
        figurineId,
        figurineName,
        requesterName:  name.trim(),
        requesterEmail: email.trim(),
        purpose:        purpose.trim() || null,
        startsAt,
        endsAt,
      });
      isSealed = true;
      setTimeout(() => close(), 3500);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : '';
      submitError = msg.includes('409') || msg.toLowerCase().includes('conflict')
        ? $t('bookingErrorConflict')
        : $t('bookingErrorGeneric');
    } finally {
      isSubmitting = false;
    }
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 z-[220] flex items-center justify-center p-4 bg-[#6f3b24]/35 backdrop-blur-sm"
    transition:fade={{ duration: 400 }}
    role="presentation"
  >
    <div
      role="dialog"
      aria-modal="true"
      class="relative w-full max-w-xl perspective-1000"
      in:fly={{ y: 50, duration: 800, easing: cubicOut }}
    >
      <div class="relative bg-[#fff9f0] shadow-[0_20px_60px_rgba(111,59,36,0.18)] p-1 overflow-hidden transform rotate-1 transition-transform duration-500 hover:rotate-0 border border-[#d8c6b1] rounded-sm">
        <!-- Декор: вне scroll-контейнера, всегда покрывает весь фрейм -->
        <div class="border-[3px] border-double border-[#c9a875]/35 relative bg-[#fff9f0] max-h-[90vh] overflow-hidden">

          <div class="absolute inset-0 pointer-events-none opacity-20 mix-blend-multiply bg-noise z-0"></div>
          <div class="absolute inset-0 pointer-events-none shadow-[inset_0_0_120px_rgba(46,43,40,0.15)] z-0"></div>
          <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-64 h-64 border-4 border-[#c9a875]/10 rounded-full flex items-center justify-center pointer-events-none z-0">
            <div class="w-48 h-48 border border-[#c9a875]/10 rotate-45"></div>
          </div>

          <!-- Скроллится только контент, декор остаётся на месте -->
          <div class="relative z-10 overflow-y-auto max-h-[90vh] p-7 md:p-9 font-['Georgia'] text-[#34251c]">

            {#if !isSealed}
              <div out:fade={{ duration: 300 }}>

                <!-- Header -->
                <div class="text-center mb-6 relative">
                  <span class="absolute -top-6 left-1/2 -translate-x-1/2 text-5xl opacity-10 font-['Fraunces']">~</span>
                  <h3 class="font-['Fraunces'] text-3xl mb-2 text-[#6f3b24] tracking-wide">{$t('bookingTitle')}</h3>
                  <div class="flex items-center justify-center gap-3 text-[#5f4636]">
                    <span class="h-px w-8 bg-[#5f4636]/30"></span>
                    <p class="italic text-base font-semibold tracking-wide">{figurineName}</p>
                    <span class="h-px w-8 bg-[#5f4636]/30"></span>
                  </div>
                </div>

                <!-- Form -->
                <form class="space-y-6" onsubmit={handleSubmit}>

                  <!-- Calendar -->
                  <div>
                    <p class="text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase mb-3">{$t('bookingDatesLabel')}</p>
                    <div class="border border-[#d8c6b1] p-3 bg-[#fdf8f2]">
                      <DateRangePicker
                        {schedule}
                        bind:startsAt
                        bind:endsAt
                        minDate={today}
                        onError={(msg) => { dateError = msg; }}
                      />
                    </div>
                    {#if dateError}
                      <div class="mt-2 flex items-start gap-2 px-3 py-2 bg-red-50 border border-red-200 rounded-sm">
                        <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="#991b1b" stroke-width="1.5" class="flex-shrink-0 mt-0.5">
                          <circle cx="6.5" cy="6.5" r="5.5"/>
                          <path d="M6.5 3.5v3.5M6.5 9v.5"/>
                        </svg>
                        <p class="text-xs text-red-800 font-['Inter']">{dateError}</p>
                      </div>
                    {/if}
                  </div>

                  <!-- Name -->
                  <div class="space-y-1.5">
                    <label for="b-name" class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase">
                      {$t('orderNameLabel')}
                    </label>
                    <input
                      id="b-name"
                      type="text"
                      bind:value={name}
                      required
                      class="w-full bg-transparent border-0 border-b-2 border-[#d8c6b1] rounded-none py-2 text-xl text-[#34251c] font-['Fraunces'] focus:outline-none focus:ring-0 focus:border-[#c65f3c] transition-colors"
                      placeholder=""
                    />
                  </div>

                  <!-- Email -->
                  <div class="space-y-1.5">
                    <label for="b-email" class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase">
                      {$t('orderEmailLabel')}
                    </label>
                    <input
                      id="b-email"
                      type="email"
                      bind:value={email}
                      required
                      class="w-full bg-transparent border-0 border-b-2 border-[#d8c6b1] rounded-none py-2 text-xl italic font-serif text-[#34251c] focus:outline-none focus:ring-0 focus:border-[#c65f3c] transition-colors"
                      placeholder=""
                    />
                  </div>

                  <!-- Purpose -->
                  <div class="space-y-1.5">
                    <label for="b-purpose" class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase">{$t('bookingPurposeLabel')}</label>
                    <textarea
                      id="b-purpose"
                      bind:value={purpose}
                      rows="2"
                      class="w-full bg-[#f8f1e7] border-0 border-b border-[#d8c6b1] rounded-none p-2 text-base italic text-[#34251c] focus:outline-none focus:ring-0 focus:border-[#c65f3c]/70 transition-colors placeholder-[#5f4636]/40 resize-none"
                      placeholder={$t('bookingPurposePlaceholder')}
                    ></textarea>
                  </div>

                  <!-- Submit -->
                  <div class="pt-4 flex justify-center">
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
                          <span class="animate-pulse">{$t('bookingSubmitting')}</span>
                        {:else}
                          <span>{$t('bookingSubmit')}</span>
                          <span class="text-lg opacity-70">✒</span>
                        {/if}
                      </span>
                    </button>
                  </div>

                  {#if submitError}
                    <div class="flex items-start gap-2 px-3 py-2 bg-red-50 border border-red-200 rounded-sm">
                      <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="#991b1b" stroke-width="1.5" class="flex-shrink-0 mt-0.5">
                        <circle cx="6.5" cy="6.5" r="5.5"/>
                        <path d="M6.5 3.5v3.5M6.5 9v.5"/>
                      </svg>
                      <p class="text-sm text-red-800 font-['Inter']">{submitError}</p>
                    </div>
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
              <!-- Success -->
              <div class="flex flex-col items-center justify-center py-12" in:scale={{ duration: 700, start: 0.95, easing: elasticOut }}>
                <div class="relative w-40 h-40 mb-8 filter drop-shadow-2xl">
                  <div class="absolute inset-0 bg-gradient-to-br from-[#c65f3c] via-[#a86124] to-[#9e452d] rounded-full transform rotate-12 flex items-center justify-center border-4 border-[#6f3b24]/20 wax-seal-shape animate-seal-press">
                    <div class="w-28 h-28 border-2 border-[#6f3b24]/20 rounded-full flex items-center justify-center shadow-inner">
                      <span class="font-['Fraunces'] text-6xl text-[#6f3b24] opacity-70 mt-2 ml-1">G</span>
                    </div>
                  </div>
                  <div class="absolute top-8 left-10 w-8 h-4 bg-[#fff9f0] opacity-25 blur-sm rounded-full rotate-45"></div>
                </div>
                <h3 class="font-['Fraunces'] text-5xl text-[#6f3b24] mb-4 tracking-wide">{$t('bookingSuccessTitle')}</h3>
                <div class="relative max-w-xs text-center">
                  <span class="absolute -left-4 top-0 text-4xl text-[#6f3b24]/15 font-serif">"</span>
                  <p class="font-['Inter'] text-[#5f4636] text-sm leading-relaxed font-semibold">{$t('bookingSuccessText')}</p>
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
  .perspective-1000 { perspective: 1000px; }
  .clip-corners {
    clip-path: polygon(10px 0, 100% 0, 100% calc(100% - 10px), calc(100% - 10px) 100%, 0 100%, 0 10px);
  }
  .wax-seal-shape {
    border-radius: 45% 55% 48% 52% / 51% 46% 54% 49%;
    box-shadow: inset 2px 2px 15px rgba(111,59,36,0.16), inset -2px -2px 10px rgba(255,255,255,0.1);
  }
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }
  @keyframes sealPress {
    0%   { transform: scale(1.1) rotate(12deg); opacity: 0; }
    100% { transform: scale(1) rotate(12deg); opacity: 1; }
  }
  .animate-seal-press { animation: sealPress 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275) forwards; }
</style>
