<script lang="ts">
  /**
   * VisitorBook — the house guest book at the museum's exit.
   *
   * Two faces of one ledger spread:
   *  • Not signed — invitation: value stated before the field, a single required
   *    input (email; name optional), an honest privacy line, a wax seal as reward.
   *    The proven shape of a high-converting opt-in, dressed as a ledger.
   *  • Signed — recognition: the page no longer sells; it *knows* the returning
   *    reader by name, marks how long they've been in the book, counts what is new
   *    since they signed, and points them to the book-holders' "first look". This is
   *    the research-backed move — recognition outperforms a repeated ask — and it
   *    removes the contradiction of pitching benefits to someone already in.
   *
   * Standing is kept on this device by the `visitorBook` store (single opt-in, no
   * account), shared with the first-look band so both react together. A hidden
   * honeypot traps the simplest bots. Store-smell rules hold — no urgency, no popup.
   */
  import { onMount } from 'svelte';
  import { fly, scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import { t, lang } from '$lib/i18n';
  import { visitorBook } from '$lib/stores/visitor-book.svelte';
  import type { FigurineListItem } from '$lib/types/api';

  let { figurines = [] }: { figurines?: FigurineListItem[] } = $props();

  let email = $state('');
  let name = $state('');
  let ageConfirmed = $state(false);
  let honeypot = $state(''); // bots fill this; humans never see it
  let sealing = $state(false); // transient "pressing the seal" beat
  let submitting = $state(false);
  let leaving = $state(false);
  let error = $state('');
  let justLeft = $state(false);

  onMount(() => visitorBook.load());

  // Show the recognition spread once signed, except during the brief seal press.
  let showSigned = $derived(visitorBook.signed && !sealing);

  let locale = $derived($lang === 'ru' ? 'ru-RU' : 'en-US');
  let signedDate = $derived(
    visitorBook.signedAt
      ? new Date(visitorBook.signedAt).toLocaleDateString(locale, {
          day: 'numeric',
          month: 'long',
          year: 'numeric',
        })
      : ''
  );

  // Works that entered the archive since this reader signed — the payoff of being
  // in the book, surfaced as a quiet count (not a notification badge).
  let newSince = $derived.by(() => {
    if (!visitorBook.signed || !visitorBook.signedAt) return 0;
    const since = new Date(visitorBook.signedAt).getTime();
    if (Number.isNaN(since)) return 0;
    return figurines.filter(
      (f) => f.createdAt && new Date(f.createdAt).getTime() > since
    ).length;
  });

  const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

  async function sign(e: SubmitEvent) {
    e.preventDefault();
    if (submitting) return;
    error = '';
    if (honeypot.trim()) return; // silently drop bots
    const addr = email.trim();
    if (!EMAIL_RE.test(addr)) {
      error = $t('visitorBookErrorEmail');
      return;
    }
    if (!ageConfirmed) {
      error = $t('formAgeConfirmRequired');
      return;
    }
    submitting = true;
    try {
      const res = await api.subscribe({
        email: addr,
        name: name.trim() || null,
        source: 'home_visitor_book',
        lang: $lang,
        ageConfirmed,
      });
      visitorBook.sign(res.unsubscribeToken, addr, name);
      justLeft = false;
      // brief "pressing the seal" beat, then the recognition spread
      sealing = true;
      setTimeout(() => { sealing = false; }, 900);
    } catch {
      error = $t('visitorBookError');
    } finally {
      submitting = false;
    }
  }

  async function leave() {
    const token = visitorBook.token;
    if (!token || leaving) return;
    leaving = true;
    try {
      await api.unsubscribeByToken(token);
    } catch {
      /* idempotent on the server; treat as left either way */
    } finally {
      visitorBook.leave();
      email = '';
      name = '';
      ageConfirmed = false;
      justLeft = true;
      leaving = false;
    }
  }
</script>

<section class="book" aria-labelledby="visitor-book-title">
  <div class="leaf">
    {#if showSigned}
      <!-- ── Recognition spread (returning, signed) ───────────────── -->
      <div class="page page-invite" in:fly={{ y: 10, duration: 450, easing: cubicOut }}>
        <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('visitorBookEyebrow')}</p>
        <h2 id="visitor-book-title" class="book-title">
          {$t('visitorBookWelcomeBack')}{#if visitorBook.name}, {visitorBook.name}{/if}
        </h2>
        <p class="book-lead">{$t('visitorBookSignedLead')}</p>

        <dl class="standing">
          {#if signedDate}
            <div class="standing-row">
              <dt>{$t('visitorBookInBookSince')}</dt>
              <dd>{signedDate}</dd>
            </div>
          {/if}
          {#if newSince > 0}
            <div class="standing-row">
              <dt>{$t('visitorBookNewSince')}</dt>
              <dd class="standing-count">{newSince}</dd>
            </div>
          {/if}
        </dl>
      </div>

      <div class="page page-sign">
        <div class="signed" in:fly={{ y: 12, duration: 500, easing: cubicOut }} aria-live="polite">
          <span class="wax" aria-hidden="true"><span class="wax-mark">❧</span></span>
          <a class="first-look-cta" href="#first-look">
            {$t('visitorBookFirstLookCta')}
            <svg width="18" height="9" viewBox="0 0 18 9" fill="none" aria-hidden="true">
              <path d="M0 4.5H17M17 4.5L12.5 1M17 4.5L12.5 8" stroke="currentColor" stroke-width="1"/>
            </svg>
          </a>
          {#if visitorBook.email}
            <p class="signed-as">{$t('visitorBookSignedAs')}<br /><strong>{visitorBook.email}</strong></p>
          {/if}
          <button class="leave-link" type="button" onclick={leave} disabled={leaving}>
            {leaving ? $t('visitorBookLeaving') : $t('visitorBookLeave')}
          </button>
        </div>
      </div>
    {:else}
      <!-- ── Invitation spread (not signed) ───────────────────────── -->
      <div class="page page-invite">
        <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('visitorBookEyebrow')}</p>
        <h2 id="visitor-book-title" class="book-title">{$t('visitorBookTitle')}</h2>
        <p class="book-lead">{$t('visitorBookLead')}</p>

        <ul class="ledger-lines" aria-label={$t('visitorBookTitle')}>
          <li><span class="ll-mark" aria-hidden="true">❧</span>{$t('visitorBookValue1')}</li>
          <li><span class="ll-mark" aria-hidden="true">❧</span>{$t('visitorBookValue2')}</li>
          <li><span class="ll-mark" aria-hidden="true">❧</span>{$t('visitorBookValue3')}</li>
        </ul>
      </div>

      <div class="page page-sign">
        <form class="sign-form" onsubmit={sign} novalidate>
          {#if sealing}
            <div class="seal-press" in:scale={{ start: 0.4, duration: 400 }} aria-hidden="true">
              <span class="wax wax-pressing"><span class="wax-mark">❧</span></span>
            </div>
          {/if}

          <label class="field field-name">
            <span class="field-label">{$t('visitorBookNameLabel')}</span>
            <input
              id="visitor-book-name"
              name="name"
              type="text"
              bind:value={name}
              placeholder={$t('visitorBookNamePlaceholder')}
              autocomplete="name"
              maxlength="100"
              disabled={submitting || sealing}
            />
          </label>

          <label class="field field-email">
            <span class="field-label">{$t('visitorBookEmailLabel')}</span>
            <input
              id="visitor-book-email"
              name="email"
              type="email"
              bind:value={email}
              placeholder={$t('visitorBookEmailPlaceholder')}
              autocomplete="email"
              required
              maxlength="200"
              aria-invalid={Boolean(error)}
              disabled={submitting || sealing}
            />
          </label>

          <!-- honeypot: visually hidden, off-screen; real users never fill it -->
          <div class="hp" aria-hidden="true">
            <label>Leave this empty<input id="visitor-book-hp" name="website" type="text" bind:value={honeypot} tabindex="-1" autocomplete="off" /></label>
          </div>

          <label class="consent">
            <input id="visitor-book-age" name="age-confirm" type="checkbox" bind:checked={ageConfirmed} required disabled={submitting || sealing} />
            <span>{$t('formAgeConfirm')}</span>
          </label>

          {#if error}<p class="form-error" role="alert">{error}</p>{/if}
          {#if justLeft}<p class="form-note" aria-live="polite">{$t('visitorBookLeftText')}</p>{/if}

          <button class="sign-btn" type="submit" disabled={submitting || sealing}>
            {submitting || sealing ? $t('visitorBookSubmitting') : $t('visitorBookSubmit')}
            <svg width="18" height="9" viewBox="0 0 18 9" fill="none" aria-hidden="true">
              <path d="M0 4.5H17M17 4.5L12.5 1M17 4.5L12.5 8" stroke="currentColor" stroke-width="1"/>
            </svg>
          </button>

          <p class="privacy">{$t('visitorBookPrivacy')}</p>
        </form>
      </div>
    {/if}
  </div>
</section>

<style>
  .book {
    max-width: 1520px;
    margin: 0 auto;
    padding: clamp(28px, 4vw, 60px) clamp(20px, 4.5vw, 64px) clamp(48px, 6vw, 88px);
  }

  /* an open ledger spread: invitation page | signing page */
  .leaf {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 0.92fr);
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 14%, transparent);
    border-radius: 4px;
    background:
      radial-gradient(ellipse 60% 50% at 22% 18%, rgba(198, 95, 60, 0.05) 0%, transparent 60%),
      linear-gradient(180deg, var(--color-canvas-raised, #fffaf2), var(--color-canvas-base, #f8f1e7));
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.7),
      0 14px 40px rgba(60, 36, 22, 0.10);
    overflow: hidden;
  }

  .page {
    padding: clamp(24px, 3vw, 46px);
  }
  /* the gutter seam between the two pages */
  .page-sign {
    border-left: 1px solid color-mix(in srgb, var(--color-ink-primary) 12%, transparent);
    background:
      linear-gradient(90deg, rgba(60, 36, 22, 0.05) 0%, transparent 4%),
      transparent;
    display: flex;
    align-items: center;
  }

  .eyebrow {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
  }
  .eyebrow-rule {
    display: inline-block;
    width: 26px;
    height: 1px;
    background: var(--color-ember, #c65f3c);
    opacity: 0.65;
  }

  .book-title {
    margin: 0 0 14px;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(30px, 3vw, 46px);
    font-weight: 300;
    line-height: 0.98;
    color: var(--color-ink-primary, #34251c);
  }

  .book-lead {
    margin: 0 0 22px;
    max-width: 40ch;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(16px, 1.4vw, 20px);
    font-style: italic;
    font-weight: 300;
    line-height: 1.45;
    color: var(--color-ink-secondary, #5f4636);
  }

  .ledger-lines {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 11px;
  }
  .ledger-lines li {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding-bottom: 11px;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 9%, transparent);
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(15px, 1.25vw, 18px);
    line-height: 1.4;
    color: var(--color-ink-primary, #34251c);
  }
  .ledger-lines li:last-child { border-bottom: none; }
  .ll-mark {
    flex-shrink: 0;
    font-size: 13px;
    line-height: 1;
    color: var(--color-ember, #c65f3c);
    opacity: 0.8;
  }

  /* ── standing (signed recognition) ──────────────────────────── */
  .standing {
    margin: 0;
    display: grid;
    gap: 0;
  }
  .standing-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    padding: 11px 0;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 9%, transparent);
  }
  .standing-row:first-child { border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 9%, transparent); }
  .standing-row dt {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
  }
  .standing-row dd {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(17px, 1.4vw, 21px);
    color: var(--color-ink-primary, #34251c);
  }
  .standing-count {
    color: var(--color-ember-deep, #6f3b24);
    font-style: italic;
  }

  /* ── signing form ───────────────────────────────────────────── */
  .sign-form {
    position: relative;
    width: 100%;
    display: grid;
    gap: 16px;
  }

  .field { display: grid; gap: 6px; }
  .field-label {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
  }
  /* inputs are ruled lines you sign on, not boxed shop fields */
  .field input {
    width: 100%;
    padding: 6px 2px 8px;
    border: none;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 28%, transparent);
    background: transparent;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(17px, 1.5vw, 21px);
    color: var(--color-ink-primary, #34251c);
    transition: border-color 0.2s ease;
  }
  .field input::placeholder {
    color: color-mix(in srgb, var(--color-ink-primary) 36%, transparent);
    font-style: italic;
  }
  .field input:focus {
    outline: none;
    border-bottom-color: var(--color-ember, #c65f3c);
  }
  .field input[aria-invalid='true'] {
    border-bottom-color: #b0472c;
  }

  /* honeypot — kept off-screen for everyone, traps bots */
  .hp {
    position: absolute;
    left: -9999px;
    width: 1px;
    height: 1px;
    overflow: hidden;
  }

  .form-error {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: 15px;
    color: #a23c22;
  }
  .form-note {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: 15px;
    color: var(--color-ink-secondary, #5f4636);
  }

  .sign-btn {
    justify-self: start;
    display: inline-flex;
    align-items: center;
    gap: 12px;
    height: 42px;
    margin-top: 4px;
    padding: 0 22px;
    border: none;
    background: var(--color-ink-primary, #34251c);
    color: var(--color-canvas-raised, #fffaf2);
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    cursor: pointer;
    clip-path: polygon(0 0, calc(100% - 7px) 0, 100% 7px, 100% 100%, 7px 100%, 0 calc(100% - 7px));
    transition: background 0.22s ease, box-shadow 0.22s ease, transform 0.12s ease;
  }
  .sign-btn svg { transition: transform 0.22s ease; }
  .sign-btn:hover:not(:disabled) {
    background: var(--color-ember-deep, #6f3b24);
    box-shadow: 0 10px 24px rgba(68, 37, 20, 0.16);
  }
  .sign-btn:hover:not(:disabled) svg { transform: translateX(4px); }
  .sign-btn:active:not(:disabled) { transform: translateY(1px); }
  .sign-btn:disabled { opacity: 0.55; cursor: default; }

  .privacy {
    margin: 2px 0 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11.5px;
    letter-spacing: 0.01em;
    color: var(--color-ink-tertiary);
  }

  .consent {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    cursor: pointer;
  }
  .consent input {
    margin-top: 2px;
    width: 15px;
    height: 15px;
    accent-color: var(--color-ember, #c65f3c);
    flex-shrink: 0;
    cursor: pointer;
  }
  .consent span {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--color-ink-tertiary);
  }

  /* ── the seal being pressed (transient) ─────────────────────── */
  .seal-press {
    position: absolute;
    inset: 0;
    z-index: 4;
    display: grid;
    place-items: center;
    pointer-events: none;
  }

  /* ── signed state (right page) ──────────────────────────────── */
  .signed {
    width: 100%;
    display: grid;
    justify-items: start;
    gap: 16px;
  }
  .first-look-cta {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    height: 42px;
    padding: 0 22px;
    background: var(--color-ink-primary, #34251c);
    color: var(--color-canvas-raised, #fffaf2);
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    text-decoration: none;
    clip-path: polygon(0 0, calc(100% - 7px) 0, 100% 7px, 100% 100%, 7px 100%, 0 calc(100% - 7px));
    transition: background 0.22s ease, box-shadow 0.22s ease, transform 0.12s ease;
  }
  .first-look-cta svg { transition: transform 0.22s ease; }
  .first-look-cta:hover {
    background: var(--color-ember-deep, #6f3b24);
    box-shadow: 0 10px 24px rgba(68, 37, 20, 0.16);
  }
  .first-look-cta:hover svg { transform: translateX(4px); }
  .first-look-cta:active { transform: translateY(1px); }

  .signed-as {
    margin: 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    letter-spacing: 0.04em;
    line-height: 1.6;
    color: var(--color-ink-tertiary);
  }
  .signed-as strong {
    font-size: 14px;
    color: var(--color-ink-primary, #34251c);
  }
  .leave-link {
    padding: 0 0 2px;
    border: none;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 22%, transparent);
    background: transparent;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
    cursor: pointer;
    transition: color 0.2s ease, border-color 0.2s ease;
  }
  .leave-link:hover:not(:disabled) {
    color: var(--color-ember, #c65f3c);
    border-color: color-mix(in srgb, var(--color-ember) 50%, transparent);
  }
  .leave-link:disabled { opacity: 0.5; cursor: default; }

  /* ── wax seal ───────────────────────────────────────────────── */
  .wax {
    position: relative;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: radial-gradient(circle at 38% 32%, #d2683f, #8a2f1c 62%, #5f2012);
    box-shadow:
      0 2px 8px rgba(10, 4, 2, 0.45),
      inset 0 1px 0 rgba(255, 200, 170, 0.5);
  }
  .wax-mark {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 26px;
    color: rgba(40, 12, 6, 0.5);
  }
  .wax-pressing {
    animation: press 0.5s cubic-bezier(0.34, 1.4, 0.64, 1) both;
  }
  @keyframes press {
    0% { transform: scale(1.6); opacity: 0; }
    60% { transform: scale(0.92); opacity: 1; }
    100% { transform: scale(1); opacity: 1; }
  }

  /* ── responsive ─────────────────────────────────────────────── */
  @media (max-width: 820px) {
    .leaf { grid-template-columns: 1fr; }
    .page-sign {
      border-left: none;
      border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 12%, transparent);
      background: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .wax-pressing { animation: none; }
    .sign-btn svg, .first-look-cta svg { transition: none; }
  }
</style>
