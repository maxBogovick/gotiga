<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly, scale } from 'svelte/transition';
  import { cubicOut, elasticOut } from 'svelte/easing';
  import { api, resolveMediaUrl } from '$lib/api';
  import { t } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';
  import { isValidEmail } from '$lib/validation';
  import { focusTrap } from '$lib/actions/focusTrap';
  import DateRangePicker from '$lib/components/DateRangePicker.svelte';
  import type { BookingRules, FigurineSchedule, FigurineStatus } from '$lib/types/api';

  type RequestIntent = 'request' | 'reserve' | 'waitlist' | 'viewing' | 'similar' | 'question' | 'notify';
  type ClaimData = { token: string; figurineName: string; startsAt: string; endsAt: string; submittedAt: string };

  let {
    isOpen = false,
    figurineId = '',
    figurineName = '',
    status = 'available' as FigurineStatus,
    schedule = { entries: [] } as FigurineSchedule,
    initialIntent = 'request' as RequestIntent,
    onClose = () => {},
    onJoined = (_token: string, _position: number) => {},
    onNotified = (_token: string) => {},
    onBookingCreated = (_claim: ClaimData) => {},
  } = $props();

  let avatarUrl = $derived(resolveMediaUrl(authStore.user?.avatarUrl));
  let bookingRules = $state<BookingRules | null>(null);
  let intent = $state<RequestIntent>('request');
  let name = $state('');
  let email = $state('');
  let phone = $state('');
  let message = $state('');
  let displayType = $state('private');
  let venue = $state('');
  let submitting = $state(false);
  let submitError = $state('');
  let done = $state(false);
  let successTitle = $state('');
  let successText = $state('');

  function addDays(ds: string, n: number) {
    const d = new Date(ds + 'T00:00:00');
    d.setDate(d.getDate() + n);
    return d.toISOString().split('T')[0];
  }

  const initialToday = new Date().toISOString().split('T')[0];
  let startsAt = $state(initialToday);
  let endsAt = $state(addDays(initialToday, 1));
  let dateError = $state('');

  onMount(async () => {
    bookingRules = await api.getBookingRules().catch(() => null);
  });

  let today = $derived.by(() => {
    if (!bookingRules || bookingRules.advanceDays <= 0) return new Date().toISOString().split('T')[0];
    const d = new Date();
    d.setDate(d.getDate() + bookingRules.advanceDays);
    return d.toISOString().split('T')[0];
  });

  let upcomingShowings = $derived(
    schedule.entries.filter(e => e.entryType === 'showing' && e.endsAt >= new Date().toISOString().split('T')[0])
  );

  let canRequestWork = $derived(status === 'available');
  let canReserve = $derived(status === 'available');
  let canWaitlist = $derived(status === 'reserved');
  let canViewing = $derived(status === 'available' || upcomingShowings.length > 0);
  let canNotify = $derived(status === 'in_progress' || status === 'sold');

  let intentOptions = $derived.by(() => {
    const options: { value: RequestIntent; label: string; hint: string }[] = [];
    if (canRequestWork) options.push({ value: 'request', label: $t('unifiedIntentRequest'), hint: $t('unifiedIntentRequestHint') });
    if (canReserve) options.push({ value: 'reserve', label: $t('unifiedIntentReserve'), hint: $t('unifiedIntentReserveHint') });
    if (canWaitlist) options.push({ value: 'waitlist', label: $t('unifiedIntentWaitlist'), hint: $t('unifiedIntentWaitlistHint') });
    if (canViewing) options.push({ value: 'viewing', label: $t('unifiedIntentViewing'), hint: $t('unifiedIntentViewingHint') });
    options.push({ value: 'similar', label: $t('unifiedIntentSimilar'), hint: $t('unifiedIntentSimilarHint') });
    options.push({ value: 'question', label: $t('unifiedIntentQuestion'), hint: $t('unifiedIntentQuestionHint') });
    if (canNotify) options.unshift({ value: 'notify', label: $t('unifiedIntentNotify'), hint: $t('unifiedIntentNotifyHint') });
    return options;
  });

  $effect(() => {
    if (!isOpen) return;
    const allowed = intentOptions.some(opt => opt.value === initialIntent) ? initialIntent : intentOptions[0]?.value;
    if (allowed) intent = allowed;
  });

  $effect(() => {
    if (startsAt && startsAt < today) {
      startsAt = today;
      if (!endsAt || endsAt < today) endsAt = addDays(today, 1);
    }
  });

  let modalTitle = $derived(
    intent === 'reserve'
      ? $t('unifiedReserveTitle')
      : status === 'reserved'
      ? $t('unifiedReservedTitle')
      : status === 'in_progress'
        ? $t('unifiedProgressTitle')
        : status === 'sold'
          ? $t('unifiedSoldTitle')
          : $t('unifiedAvailableTitle')
  );

  let submitLabel = $derived(
    intent === 'waitlist'
      ? $t('unifiedSubmitWaitlist')
      : intent === 'viewing'
        ? $t('unifiedSubmitViewing')
        : intent === 'similar'
          ? $t('unifiedSubmitSimilar')
          : intent === 'question'
            ? $t('unifiedSubmitQuestion')
            : intent === 'notify'
              ? $t('unifiedSubmitNotify')
              : intent === 'reserve'
                ? $t('unifiedSubmitReserve')
                : $t('unifiedSubmitRequest')
  );

  function reset() {
    name = '';
    email = '';
    phone = '';
    message = '';
    displayType = 'private';
    venue = '';
    startsAt = today;
    endsAt = addDays(today, 1);
    dateError = '';
    submitError = '';
    done = false;
    successTitle = '';
    successText = '';
  }

  function close() {
    if (submitting) return;
    onClose();
    setTimeout(reset, 450);
  }

  function effectiveContact() {
    return {
      requesterName: authStore.isLoggedIn ? (authStore.user?.displayName ?? '') : name.trim(),
      requesterEmail: authStore.isLoggedIn ? (authStore.user?.email ?? '') : email.trim(),
    };
  }

  function validateContact() {
    const contact = effectiveContact();
    if (!contact.requesterName || !contact.requesterEmail) {
      submitError = $t('formFillFields');
      return null;
    }
    if (!authStore.isLoggedIn && !isValidEmail(contact.requesterEmail)) {
      submitError = $t('formInvalidEmail');
      return null;
    }
    return contact;
  }

  function orderMessage(prefix: string) {
    const text = message.trim();
    return text ? `${prefix}\n\n${text}` : prefix;
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    submitError = '';
    const contact = validateContact();
    if (!contact) return;

    if (intent === 'viewing') {
      if (!startsAt || !endsAt) { submitError = $t('bookingSelectPeriod'); return; }
      if (dateError) { submitError = dateError; return; }
      if ((displayType === 'exhibition' || displayType === 'photo') && !venue.trim()) {
        submitError = $t('bookingFillVenue');
        return;
      }
    }

    submitting = true;
    try {
      if (intent === 'waitlist') {
        const res = await api.joinWaitlist(figurineId, {
          figurineName,
          requesterName: contact.requesterName,
          requesterEmail: contact.requesterEmail,
          requesterPhone: phone.trim() || null,
          note: message.trim() || null,
        });
        onJoined(res.cancelToken, res.position);
        successTitle = $t('waitlistSuccessTitle');
        successText = $t('waitlistSuccessText');
      } else if (intent === 'viewing') {
        const res = await api.submitBooking({
          figurineId,
          figurineName,
          requesterName: contact.requesterName,
          requesterEmail: contact.requesterEmail,
          requesterPhone: phone.trim() || null,
          purpose: message.trim() || null,
          displayType,
          venue: venue.trim() || null,
          startsAt,
          endsAt,
        }, authStore.token);
        const claim = { token: res.cancelToken, figurineName, startsAt, endsAt, submittedAt: new Date().toISOString() };
        // Backend already ties the booking to the account when the token is sent;
        // this stays as a harmless fallback (no-op once user_id is set).
        if (authStore.isLoggedIn && authStore.token) {
          api.userLinkBookings(authStore.token, [res.cancelToken]).catch(() => {});
        }
        onBookingCreated(claim);
        successTitle = $t('bookingSuccessTitle');
        successText = $t('bookingSuccessText');
      } else {
        const mode = intent === 'notify'
          ? 'notify'
          : intent === 'question'
            ? 'question'
            : intent === 'reserve'
              ? 'reserve'
              : 'request';
        const prefix =
          intent === 'similar'
            ? $t('unifiedSimilarPrefix')
            : intent === 'request'
              ? $t('unifiedRequestPrefix')
              : intent === 'reserve'
                ? $t('unifiedReservePrefix')
              : intent === 'notify'
                ? $t('unifiedNotifyPrefix')
                : $t('unifiedQuestionPrefix');
        const res = await api.submitOrder({
          figurineId,
          figurineName,
          requesterName: contact.requesterName,
          requesterEmail: contact.requesterEmail,
          requesterPhone: phone.trim() || null,
          message: orderMessage(prefix),
          mode,
        }, authStore.token);
        if (intent === 'notify' && res?.cancelToken) onNotified(res.cancelToken);
        successTitle = $t('orderSuccessTitle');
        successText = $t('orderSuccessText');
      }
      done = true;
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : '';
      submitError = msg.includes('409') || msg.toLowerCase().includes('conflict')
        ? $t('bookingErrorConflict')
        : $t('orderSubmitError');
    } finally {
      submitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (isOpen && e.key === 'Escape') close();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <div
    class="unified-backdrop"
    transition:fade={{ duration: 250 }}
    onclick={(e) => { if (e.target === e.currentTarget) close(); }}
    role="presentation"
  >
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="unified-request-title"
      tabindex="-1"
      class="unified-dialog"
      in:fly={{ y: 36, duration: 420, easing: cubicOut }}
      use:focusTrap
    >
      <button type="button" class="unified-close" onclick={close} aria-label={$t('lightboxClose')}>
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
          <path d="M1 1l12 12M13 1L1 13" />
        </svg>
      </button>

      {#if !done}
        <form class="unified-form" onsubmit={handleSubmit}>
          <div class="unified-head">
            <span class="unified-kicker">{$t('unifiedKicker')}</span>
            <h3 id="unified-request-title">{modalTitle}</h3>
            <p>{figurineName}</p>
          </div>

          <fieldset class="unified-intents">
            <legend>{$t('unifiedIntentLegend')}</legend>
            {#each intentOptions as opt (opt.value)}
              <label class="unified-intent" class:unified-intent--active={intent === opt.value}>
                <input type="radio" name="request-intent" value={opt.value} bind:group={intent} />
                <span>{opt.label}</span>
                <small>{opt.hint}</small>
              </label>
            {/each}
          </fieldset>

          {#if intent === 'viewing'}
            <div class="unified-dates">
              <p class="unified-label">{$t('bookingDatesLabel')}</p>
              <DateRangePicker
                {schedule}
                bind:startsAt
                bind:endsAt
                minDate={today}
                {bookingRules}
                onError={(msg) => { dateError = msg; }}
              />
              {#if dateError}
                <p class="unified-error">{dateError}</p>
              {/if}
            </div>
          {/if}

          {#if intent === 'reserve'}
            <p class="unified-notice">{$t('unifiedReserveNotice')}</p>
          {/if}

          {#if authStore.isLoggedIn}
            <div class="unified-user">
              {#if avatarUrl}
                <img src={avatarUrl} alt="" />
              {:else}
                <span>{(authStore.user?.displayName ?? '?')[0].toUpperCase()}</span>
              {/if}
              <p>{$t('formLoggedInAs')} <strong>{authStore.user?.displayName}</strong></p>
            </div>
          {:else}
            <div class="unified-fields">
              <label>
                <span>{$t('orderNameLabel')}</span>
                <input type="text" bind:value={name} required />
              </label>
              <label>
                <span>{$t('orderEmailLabel')}</span>
                <input type="email" bind:value={email} required />
              </label>
            </div>
          {/if}

          <label class="unified-field">
            <span>{$t('orderPhoneLabel')}</span>
            <input type="tel" bind:value={phone} placeholder={$t('orderPhonePlaceholder')} />
          </label>

          {#if intent === 'viewing'}
            <label class="unified-field">
              <span>{$t('bookingDisplayTypeLabel')}</span>
              <select bind:value={displayType}>
                <option value="private">{$t('bookingDisplayPrivate')}</option>
                <option value="exhibition">{$t('bookingDisplayExhibition')}</option>
                <option value="photo">{$t('bookingDisplayPhoto')}</option>
              </select>
            </label>
            {#if displayType === 'exhibition' || displayType === 'photo'}
              <label class="unified-field">
                <span>{$t('bookingVenueLabel')}</span>
                <input type="text" bind:value={venue} placeholder={$t('bookingVenuePlaceholder')} />
              </label>
            {/if}
          {/if}

          <label class="unified-field">
            <span>{intent === 'waitlist' ? $t('waitlistNoteLabel') : $t('orderMessageLabel')}</span>
            <textarea bind:value={message} rows="3" placeholder={$t('unifiedMessagePlaceholder')}></textarea>
          </label>

          {#if submitError}
            <p class="unified-error">{submitError}</p>
          {/if}

          <div class="unified-actions">
            <button type="submit" class="unified-submit" disabled={submitting}>
              {submitting ? $t('orderSubmitting') : submitLabel}
            </button>
            <button type="button" class="unified-cancel" onclick={close}>{$t('bookingCancel')}</button>
          </div>
        </form>
      {:else}
        <div class="unified-success" in:scale={{ duration: 500, start: 0.96, easing: elasticOut }}>
          <div class="unified-seal">G</div>
          <h3>{successTitle}</h3>
          <p>{successText}</p>
          <button type="button" class="unified-cancel" onclick={close}>{$t('cancelGoToFigurine')}</button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .unified-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: rgba(111, 59, 36, 0.35);
    backdrop-filter: blur(8px);
  }

  .unified-dialog {
    position: relative;
    width: min(100%, 42rem);
    max-height: min(90vh, 48rem);
    overflow: auto;
    padding: 1.25rem;
    border: 1px solid #d8c6b1;
    border-radius: 8px;
    color: #34251c;
    background: #fff9f0;
    box-shadow: 0 24px 70px rgba(52, 37, 28, 0.2);
  }

  .unified-close {
    position: absolute;
    top: 0.75rem;
    right: 0.75rem;
    display: grid;
    place-items: center;
    width: 2rem;
    height: 2rem;
    border: 0;
    color: rgba(52, 37, 28, 0.52);
    background: transparent;
    cursor: pointer;
  }

  .unified-form {
    display: grid;
    gap: 1rem;
  }

  .unified-head {
    padding-right: 2rem;
  }

  .unified-kicker,
  .unified-label,
  .unified-intents legend,
  .unified-field span,
  .unified-fields span {
    color: #6f3b24;
    font-family: "Inter", sans-serif;
    font-size: 0.64rem;
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .unified-head h3 {
    margin: 0.25rem 0 0;
    color: #34251c;
    font-family: "Fraunces", Georgia, serif;
    font-size: 2rem;
    font-weight: 520;
    letter-spacing: 0;
    line-height: 1.05;
  }

  .unified-head p {
    margin: 0.35rem 0 0;
    color: rgba(95, 70, 54, 0.82);
    font-family: Georgia, serif;
    font-style: italic;
  }

  .unified-intents {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.65rem;
    padding: 0;
    border: 0;
  }

  .unified-intents legend {
    grid-column: 1 / -1;
    margin-bottom: 0.1rem;
  }

  .unified-intent {
    display: grid;
    gap: 0.2rem;
    min-height: 5rem;
    padding: 0.75rem;
    border: 1px solid rgba(111, 59, 36, 0.18);
    border-radius: 6px;
    background: rgba(248, 241, 231, 0.64);
    cursor: pointer;
  }

  .unified-intent--active {
    border-color: rgba(198, 95, 60, 0.65);
    background: rgba(198, 95, 60, 0.08);
  }

  .unified-intent input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .unified-intent span {
    font-family: "Inter", sans-serif;
    font-size: 0.78rem;
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .unified-intent small {
    color: rgba(95, 70, 54, 0.76);
    font-family: "Inter", sans-serif;
    font-size: 0.76rem;
    line-height: 1.35;
  }

  .unified-dates {
    display: grid;
    gap: 0.5rem;
    padding: 0.75rem;
    border: 1px solid rgba(111, 59, 36, 0.14);
    background: #fdf8f2;
  }

  .unified-user,
  .unified-fields {
    display: grid;
    gap: 0.75rem;
  }

  .unified-user {
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid rgba(216, 198, 177, 0.9);
  }

  .unified-user img,
  .unified-user span {
    width: 1.8rem;
    height: 1.8rem;
    border: 1px solid #d8c6b1;
    border-radius: 999px;
  }

  .unified-user span {
    display: grid;
    place-items: center;
    color: #9a7c5c;
    background: #efe6d6;
    font-family: "Fraunces", Georgia, serif;
  }

  .unified-field,
  .unified-fields label {
    display: grid;
    gap: 0.35rem;
  }

  .unified-field input,
  .unified-field select,
  .unified-field textarea,
  .unified-fields input {
    width: 100%;
    border: 0;
    border-bottom: 2px solid #d8c6b1;
    border-radius: 0;
    color: #34251c;
    background: transparent;
    font-family: Georgia, serif;
    font-size: 1rem;
    outline: none;
  }

  .unified-field textarea {
    padding: 0.6rem;
    border: 1px solid #d8c6b1;
    background: #f8f1e7;
    resize: vertical;
  }

  .unified-field input:focus,
  .unified-field select:focus,
  .unified-field textarea:focus,
  .unified-fields input:focus {
    border-color: #c65f3c;
  }

  .unified-error {
    margin: 0;
    color: #991b1b;
    font-family: "Inter", sans-serif;
    font-size: 0.82rem;
  }

  .unified-notice {
    margin: 0;
    padding: 0.75rem;
    border: 1px solid rgba(111, 59, 36, 0.16);
    background: rgba(248, 241, 231, 0.7);
    color: rgba(95, 70, 54, 0.9);
    font-family: "Inter", sans-serif;
    font-size: 0.82rem;
    line-height: 1.45;
  }

  .unified-actions,
  .unified-success {
    display: grid;
    justify-items: center;
    gap: 0.85rem;
  }

  .unified-submit {
    min-height: 3rem;
    padding: 0 1.5rem;
    border: 1px solid rgba(52, 37, 28, 0.2);
    border-radius: 8px;
    color: #fff9f0;
    background: #34251c;
    cursor: pointer;
    font-family: "Inter", sans-serif;
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .unified-submit:disabled {
    cursor: wait;
    opacity: 0.7;
  }

  .unified-cancel {
    border: 0;
    color: rgba(95, 70, 54, 0.82);
    background: transparent;
    cursor: pointer;
    font-family: "Inter", sans-serif;
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .unified-success {
    padding: 3rem 1rem;
    text-align: center;
  }

  .unified-success h3 {
    margin: 0;
    color: #6f3b24;
    font-family: "Fraunces", Georgia, serif;
    font-size: 2rem;
  }

  .unified-success p {
    max-width: 24rem;
    margin: 0;
    color: #5f4636;
    font-family: "Inter", sans-serif;
    line-height: 1.55;
  }

  .unified-seal {
    display: grid;
    place-items: center;
    width: 5rem;
    height: 5rem;
    border-radius: 48% 52% 45% 55%;
    color: rgba(111, 59, 36, 0.75);
    background: #c65f3c;
    font-family: "Fraunces", Georgia, serif;
    font-size: 2.5rem;
  }

  @media (max-width: 640px) {
    .unified-dialog {
      padding: 1rem;
    }

    .unified-head h3 {
      font-size: 1.55rem;
    }

    .unified-intents {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
