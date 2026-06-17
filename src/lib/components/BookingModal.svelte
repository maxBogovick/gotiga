<script lang="ts">
  import { fade, scale, fly } from 'svelte/transition';
  import { cubicOut, elasticOut } from 'svelte/easing';
  import { onMount } from 'svelte';
  import { api, resolveMediaUrl } from '$lib/api';
  import { t, brandName } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';
  import { isValidEmail } from '$lib/validation';
  import { focusTrap } from '$lib/actions/focusTrap';
  import type { FigurineSchedule, BookingRules } from '$lib/types/api';

  let avatarUrl = $derived(resolveMediaUrl(authStore.user?.avatarUrl));
  import DateRangePicker from '$lib/components/DateRangePicker.svelte';

  type ClaimData = { token: string; figurineName: string; startsAt: string; endsAt: string; submittedAt: string };

  let {
    isOpen          = false,
    figurineName    = '',
    figurineId      = '',
    schedule        = { entries: [] } as FigurineSchedule,
    onClose         = () => {},
    onBookingCreated = (_c: ClaimData) => {},
  } = $props();

  let bookingRules = $state<BookingRules | null>(null);

  onMount(async () => {
    bookingRules = await api.getBookingRules().catch(() => null);
  });

  let today = $derived.by(() => {
    if (!bookingRules || bookingRules.advanceDays <= 0) {
      return new Date().toISOString().split('T')[0];
    }
    const d = new Date();
    d.setDate(d.getDate() + bookingRules.advanceDays);
    return d.toISOString().split('T')[0];
  });

  function addDays(ds: string, n: number) {
    const d = new Date(ds + 'T00:00:00');
    d.setDate(d.getDate() + n);
    return d.toISOString().split('T')[0];
  }

  // ── Derived context ───────────────────────────────────────────────────────
  let upcomingShowings = $derived(
    schedule.entries.filter(
      e => e.entryType === 'showing' && e.endsAt >= new Date().toISOString().split('T')[0]
    )
  );

  let rulesHintParts = $derived.by(() => {
    if (!bookingRules) return [] as string[];
    const parts: string[] = [];
    if (bookingRules.minDays > 1)
      parts.push(`${$t('bookingRulesMin')} ${bookingRules.minDays} ${$t('bookingRulesDays')}`);
    if (bookingRules.advanceDays > 0)
      parts.push(`${$t('bookingRulesAdvance')} ${bookingRules.advanceDays} ${$t('bookingRulesAhead')}`);
    return parts;
  });

  // ── Form state ────────────────────────────────────────────────────────────
  let name         = $state('');
  let email        = $state('');
  let phone        = $state('');
  let displayType  = $state('');   // 'private' | 'exhibition' | 'photo'
  let venue        = $state('');
  let requirements = $state('');   // maps to `purpose` on the API (client notes)
  // Initialised from the raw current date; the $effect below snaps these forward once
  // bookingRules load and advanceDays shifts the minimum allowed start date.
  const initialToday = new Date().toISOString().split('T')[0];
  let startsAt    = $state(initialToday);
  let endsAt      = $state(addDays(initialToday, 1));
  let dateError   = $state('');
  let submitError = $state('');
  let isSubmitting  = $state(false);
  let isSealed      = $state(false);
  let cancelToken   = $state('');
  let copied        = $state(false);
  let savedDates    = $state({ startsAt: '', endsAt: '' });

  let venueRequired = $derived(displayType === 'exhibition' || displayType === 'photo');

  // bookingRules load asynchronously after the initial dates are set. Once advanceDays
  // pushes the minimum start date forward, the prefilled defaults can fall before it.
  // Snap them forward — but never clobber a valid future selection the user already made.
  $effect(() => {
    if (startsAt && startsAt < today) {
      startsAt = today;
      if (!endsAt || endsAt < today) endsAt = addDays(today, 1);
    }
  });

  // ── Handlers ──────────────────────────────────────────────────────────────
  function close() {
    if (isSubmitting) return;
    onClose();
    setTimeout(() => {
      isSealed     = false;
      name         = '';
      email        = '';
      phone        = '';
      displayType  = '';
      venue        = '';
      requirements = '';
      startsAt     = today;
      endsAt       = addDays(today, 1);
      dateError    = '';
      submitError  = '';
      cancelToken  = '';
      copied       = false;
      savedDates   = { startsAt: '', endsAt: '' };
    }, 500);
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    const effectiveName = authStore.isLoggedIn ? (authStore.user?.displayName ?? '') : name.trim();
    const effectiveEmail = authStore.isLoggedIn ? (authStore.user?.email ?? '') : email.trim();

    if (!effectiveName || !effectiveEmail) {
      submitError = $t('bookingFillFields');
      return;
    }
    if (!authStore.isLoggedIn && !isValidEmail(effectiveEmail)) {
      submitError = $t('formInvalidEmail');
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
    if (!displayType) {
      submitError = $t('bookingSelectDisplayType');
      return;
    }
    if (venueRequired && !venue.trim()) {
      submitError = $t('bookingFillVenue');
      return;
    }
    isSubmitting = true;
    submitError  = '';
    try {
      const res = await api.submitBooking({
        figurineId,
        figurineName,
        requesterName:  effectiveName,
        requesterEmail: effectiveEmail,
        requesterPhone: phone.trim() || null,
        purpose:        requirements.trim() || null,
        displayType:    displayType || null,
        venue:          venue.trim() || null,
        startsAt,
        endsAt,
      });
      cancelToken = res.cancelToken;
      savedDates  = { startsAt, endsAt };
      const claim: ClaimData = {
        token: res.cancelToken, figurineName, startsAt, endsAt,
        submittedAt: new Date().toISOString(),
      };
      // Persist as array so multiple bookings accumulate
      try {
        const key  = `gotiga_claims_${figurineId}`;
        const prev: ClaimData[] = JSON.parse(localStorage.getItem(key) ?? '[]');
        localStorage.setItem(key, JSON.stringify([...prev, claim]));
      } catch { /* storage unavailable */ }
      // If logged in, link the new token to the user account immediately
      // so it appears in /profile without requiring a re-login.
      if (authStore.isLoggedIn && authStore.token) {
        api.userLinkBookings(authStore.token, [res.cancelToken]).catch(() => {});
      }
      // Notify parent immediately — no page reload needed
      onBookingCreated(claim);
      isSealed = true;
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : '';
      submitError = msg.includes('409') || msg.toLowerCase().includes('conflict')
        ? $t('bookingErrorConflict')
        : $t('bookingErrorGeneric');
    } finally {
      isSubmitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  let copyTimer: ReturnType<typeof setTimeout>;
  function copyToken() {
    navigator.clipboard.writeText(cancelToken).then(() => {
      copied = true;
      clearTimeout(copyTimer);
      copyTimer = setTimeout(() => { copied = false; }, 2000);
    });
  }

  function saveTokenFile() {
    const fmt = (ds: string) => new Date(ds + 'T00:00:00').toLocaleDateString(undefined, {
      day: 'numeric', month: 'long', year: 'numeric'
    });
    const content = [
      `${$brandName.toUpperCase()} — Booking Claim`,
      '======================',
      `Code:     ${cancelToken}`,
      `Artifact: ${figurineName}`,
      `Dates:    ${fmt(savedDates.startsAt)} — ${fmt(savedDates.endsAt)}`,
      `Created:  ${new Date().toLocaleDateString(undefined, { day: 'numeric', month: 'long', year: 'numeric' })}`,
      '',
      'Keep this code to manage your booking request.',
      'Enter it on the artifact page to revoke if plans change.',
    ].join('\n');
    const blob = new Blob([content], { type: 'text/plain' });
    const url  = URL.createObjectURL(blob);
    const a    = document.createElement('a');
    a.href     = url;
    a.download = `gotiga-claim-${cancelToken}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <div
    class="fixed inset-0 z-[1000] flex items-center justify-center p-4 bg-[#6f3b24]/35 backdrop-blur-sm"
    transition:fade={{ duration: 400 }}
    role="presentation"
  >
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="booking-modal-title"
      tabindex="-1"
      class="relative w-full max-w-xl perspective-1000"
      in:fly={{ y: 50, duration: 800, easing: cubicOut }}
      use:focusTrap
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
                  <h3 id="booking-modal-title" class="font-['Fraunces'] text-3xl mb-2 text-[#6f3b24] tracking-wide">{$t('bookingTitle')}</h3>
                  <div class="flex items-center justify-center gap-3 text-[#5f4636]">
                    <span class="h-px w-8 bg-[#5f4636]/30"></span>
                    <p class="italic text-base font-semibold tracking-wide">{figurineName}</p>
                    <span class="h-px w-8 bg-[#5f4636]/30"></span>
                  </div>
                  <p class="booking-context-sub">{$t('bookingContextSubtitle')}</p>
                </div>

                <!-- Form -->
                <form class="space-y-6" onsubmit={handleSubmit}>

                  <!-- Calendar -->
                  <div>
                    <p class="text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase mb-3">{$t('bookingDatesLabel')}</p>
                    {#if upcomingShowings.length > 0}
                      <p class="booking-showings-hint">◈ {$t('bookingShowingsHint')}</p>
                    {/if}
                    <div class="border border-[#d8c6b1] p-3 bg-[#fdf8f2]">
                      <DateRangePicker
                        {schedule}
                        bind:startsAt
                        bind:endsAt
                        minDate={today}
                        {bookingRules}
                        onError={(msg) => { dateError = msg; }}
                      />
                    </div>
                    {#if rulesHintParts.length > 0}
                      <p class="booking-rules-hint">{rulesHintParts.join(' · ')}</p>
                    {/if}
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

                  {#if authStore.isLoggedIn}
                  <!-- Logged-in user: show avatar + name, hide inputs -->
                  <div class="flex items-center gap-2.5 border-b border-[#d8c6b1] pb-2">
                    {#if avatarUrl}
                      <img src={avatarUrl} alt="" class="w-7 h-7 rounded-full object-cover border border-[#d8c6b1] flex-shrink-0" />
                    {:else}
                      <span class="w-7 h-7 rounded-full bg-[#efe6d6] border border-[#d8c6b1] flex-shrink-0 flex items-center justify-center font-['Fraunces'] text-sm text-[#9a7c5c]">
                        {(authStore.user?.displayName ?? '?')[0].toUpperCase()}
                      </span>
                    {/if}
                    <p class="text-sm text-[#5f4636] italic">
                      {$t('formLoggedInAs')} <strong class="text-[#34251c] not-italic">{authStore.user?.displayName}</strong>
                    </p>
                  </div>
                  {:else}
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
                  {/if}

                  <!-- Phone / Telegram -->
                  <div class="space-y-1.5">
                    <label for="b-phone" class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase">
                      {$t('orderPhoneLabel')}
                    </label>
                    <input
                      id="b-phone"
                      type="tel"
                      bind:value={phone}
                      class="w-full bg-transparent border-0 border-b-2 border-[#d8c6b1] rounded-none py-2 text-xl italic font-serif text-[#34251c] focus:outline-none focus:ring-0 focus:border-[#c65f3c] transition-colors"
                      placeholder={$t('orderPhonePlaceholder')}
                    />
                  </div>

                  <!-- Display type (progressive disclosure trigger) -->
                  <div class="space-y-2">
                    <p class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase">{$t('bookingDisplayTypeLabel')}</p>
                    <div class="booking-type-options">
                      {#each [
                        { value: 'private',    label: $t('bookingDisplayPrivate') },
                        { value: 'exhibition', label: $t('bookingDisplayExhibition') },
                        { value: 'photo',      label: $t('bookingDisplayPhoto') },
                      ] as opt (opt.value)}
                        <label class="booking-type-option" class:booking-type-option--active={displayType === opt.value}>
                          <input type="radio" name="displayType" value={opt.value} bind:group={displayType} class="sr-only" />
                          {opt.label}
                        </label>
                      {/each}
                    </div>
                  </div>

                  <!-- Venue — shown only when required -->
                  {#if venueRequired}
                    <div class="space-y-1.5" transition:fly={{ y: -6, duration: 200 }}>
                      <label for="b-venue" class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase">
                        {$t('bookingVenueLabel')}
                      </label>
                      <p class="booking-purpose-note">{$t('bookingVenueHint')}</p>
                      <input
                        id="b-venue"
                        type="text"
                        bind:value={venue}
                        required
                        class="w-full bg-transparent border-0 border-b-2 border-[#d8c6b1] rounded-none py-2 text-base italic font-serif text-[#34251c] focus:outline-none focus:ring-0 focus:border-[#c65f3c] transition-colors"
                        placeholder={$t('bookingVenuePlaceholder')}
                      />
                    </div>
                  {/if}

                  <!-- Requirements / notes (optional, always shown after type chosen) -->
                  {#if displayType}
                    <div class="space-y-1.5" transition:fly={{ y: -6, duration: 200 }}>
                      <label for="b-requirements" class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase">{$t('bookingRequirementsLabel')}</label>
                      <p class="booking-purpose-note">{$t('bookingPurposeNote')}</p>
                      <textarea
                        id="b-requirements"
                        bind:value={requirements}
                        rows="2"
                        class="w-full bg-[#f8f1e7] border-0 border-b border-[#d8c6b1] rounded-none p-2 text-base italic text-[#34251c] focus:outline-none focus:ring-0 focus:border-[#c65f3c]/70 transition-colors placeholder-[#5f4636]/40 resize-none"
                        placeholder={$t('bookingRequirementsPlaceholder')}
                      ></textarea>
                    </div>
                  {/if}

                  <!-- Submit -->
                  <p class="booking-process-note">{$t('bookingProcessNote')}</p>
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
              <div class="flex flex-col items-center justify-center py-12 relative" in:scale={{ duration: 700, start: 0.95, easing: elasticOut }}>
                <button
                  type="button"
                  onclick={close}
                  aria-label="Close"
                  class="absolute top-0 right-0 w-7 h-7 flex items-center justify-center text-[#5f4636]/40 hover:text-[#34251c] transition-colors"
                >
                  <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5">
                    <path d="M1 1l12 12M13 1L1 13"/>
                  </svg>
                </button>
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

                {#if cancelToken}
                  <div class="claim-token-block">
                    <p class="claim-token-label">{$t('bookingClaimLabel')}</p>
                    <p class="claim-token-code">{cancelToken}</p>
                    <div class="claim-token-actions">
                      <button type="button" onclick={copyToken} class="claim-action-btn">
                        {#if copied}
                          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
                            <path d="M1.5 6l3 3 6-6"/>
                          </svg>
                          {$t('bookingClaimCopied')}
                        {:else}
                          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.3">
                            <rect x="4" y="4" width="7" height="7" rx="1"/>
                            <path d="M2 8V2a1 1 0 0 1 1-1h6"/>
                          </svg>
                          {$t('bookingClaimCopy')}
                        {/if}
                      </button>
                      <button type="button" onclick={saveTokenFile} class="claim-action-btn">
                        <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.3">
                          <path d="M6 1v7M3.5 5.5L6 8l2.5-2.5"/>
                          <path d="M1 9v1.5A0.5 0.5 0 0 0 1.5 11h9a0.5 0.5 0 0 0 0.5-0.5V9"/>
                        </svg>
                        {$t('bookingClaimSave')}
                      </button>
                    </div>
                    <p class="claim-token-hint">{$t('bookingClaimHint')}</p>
                    <a href="/cancel/{cancelToken}" target="_blank" rel="noopener" class="claim-page-link">
                      {$t('bookingClaimPageLink')} →
                    </a>
                  </div>
                {/if}
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

  /* Claim token shown after successful booking */
  .claim-token-block {
    margin-top: 1.5rem;
    padding: 0.875rem 1.25rem;
    background: rgba(52,37,28,0.04);
    border: 1px solid rgba(52,37,28,0.12);
    border-radius: 4px;
    text-align: center;
    max-width: 18rem;
  }
  .claim-token-label {
    font-family: 'Inter', sans-serif;
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: rgba(95,70,54,0.6);
    margin: 0 0 0.5rem;
    font-weight: 700;
  }
  .claim-token-code {
    font-family: 'Fraunces', serif;
    font-size: 1.75rem;
    letter-spacing: 0.12em;
    color: #34251c;
    margin: 0 0 0.5rem;
    font-weight: 600;
  }
  .claim-token-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: center;
    margin: 0.625rem 0 0.5rem;
  }
  .claim-action-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-family: 'Inter', sans-serif;
    font-size: 0.6rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: #5f4636;
    background: rgba(52,37,28,0.06);
    border: 1px solid rgba(52,37,28,0.18);
    border-radius: 3px;
    padding: 0.3rem 0.7rem;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }
  .claim-action-btn:hover { background: rgba(52,37,28,0.11); border-color: rgba(52,37,28,0.3); }
  .claim-token-hint {
    font-family: 'Inter', sans-serif;
    font-size: 0.65rem;
    color: rgba(95,70,54,0.55);
    margin: 0;
    line-height: 1.5;
  }
  .claim-page-link {
    display: inline-block;
    margin-top: 0.5rem;
    font-family: 'Inter', sans-serif;
    font-size: 0.65rem;
    color: #c65f3c;
    text-decoration: none;
    border-bottom: 1px solid rgba(198,95,60,0.35);
    transition: border-color 0.15s;
  }
  .claim-page-link:hover { border-color: #c65f3c; }

  /* ── Context additions ── */

  .booking-context-sub {
    font-family: 'Inter', sans-serif;
    font-size: 0.72rem;
    color: rgba(95,70,54,0.55);
    font-style: italic;
    margin: 0.55rem 0 0;
    letter-spacing: 0.01em;
    line-height: 1.5;
  }

  .booking-showings-hint {
    font-family: 'Inter', sans-serif;
    font-size: 0.68rem;
    color: #7a5520;
    background: rgba(198,150,60,0.08);
    border-left: 2px solid rgba(198,150,60,0.4);
    padding: 0.3rem 0.6rem;
    margin-bottom: 0.6rem;
    letter-spacing: 0.01em;
    line-height: 1.4;
  }

  .booking-rules-hint {
    font-family: 'Inter', sans-serif;
    font-size: 0.66rem;
    color: rgba(95,70,54,0.5);
    letter-spacing: 0.04em;
    margin-top: 0.45rem;
    text-align: right;
  }

  .booking-purpose-note {
    font-family: 'Inter', sans-serif;
    font-size: 0.65rem;
    color: rgba(95,70,54,0.45);
    font-style: italic;
    margin: 0 0 0.25rem;
    line-height: 1.4;
  }

  .booking-process-note {
    font-family: 'Inter', sans-serif;
    font-size: 0.72rem;
    color: rgba(95,70,54,0.5);
    font-style: italic;
    text-align: center;
    margin: 0.25rem 0 0;
    line-height: 1.5;
    letter-spacing: 0.01em;
  }

  .booking-type-options {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .booking-type-option {
    display: inline-flex;
    align-items: center;
    font-family: 'Inter', sans-serif;
    font-size: 0.72rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: #5f4636;
    border: 1px solid #d8c6b1;
    background: transparent;
    padding: 0.35rem 0.85rem;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s, color 0.15s;
    user-select: none;
  }
  .booking-type-option:hover {
    background: rgba(198,95,60,0.06);
    border-color: rgba(198,95,60,0.4);
  }
  .booking-type-option--active {
    background: rgba(198,95,60,0.1);
    border-color: #c65f3c;
    color: #9e452d;
  }
</style>
