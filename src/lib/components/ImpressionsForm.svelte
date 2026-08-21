<script lang="ts">
  /**
   * ImpressionsForm — the "Book of Impressions": a single-page ledger where a
   * visitor leaves a free-form reaction to the exhibition as a whole (not tied
   * to one figurine, unlike FigurineComments). No email field — an impression
   * asks nothing of the visitor in return, unlike the newsletter "house book"
   * (VisitorBook.svelte). Curator moderates before anything is shown publicly;
   * approved entries may later be featured as quotes elsewhere on the site.
   *
   * Mood is an optional single-word chip, not a star rating — stars read as a
   * shop review widget and clash with the museum aesthetic; a single evocative
   * word fits the letterpress/ledger register instead.
   */
  import { scale } from 'svelte/transition';
  import { api } from '$lib/api';
  import { t, type TranslationKey } from '$lib/i18n';

  const MOODS = ['quiet', 'haunting', 'nostalgic', 'meticulous', 'uneasy', 'moved'] as const;
  type Mood = (typeof MOODS)[number];

  const MOOD_LABEL_KEYS: Record<Mood, TranslationKey> = {
    quiet: 'impressionsMood_quiet',
    haunting: 'impressionsMood_haunting',
    nostalgic: 'impressionsMood_nostalgic',
    meticulous: 'impressionsMood_meticulous',
    uneasy: 'impressionsMood_uneasy',
    moved: 'impressionsMood_moved',
  };

  let message = $state('');
  let name = $state('');
  let mood = $state<Mood | ''>('');
  let honeypot = $state(''); // bots fill this; humans never see it
  let sealing = $state(false);
  let submitting = $state(false);
  let error = $state('');
  let left = $state(false);

  const MAX_LEN = 400;

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (submitting) return;
    error = '';
    if (honeypot.trim()) return; // silently drop bots

    const trimmed = message.trim();
    if (!trimmed) {
      error = $t('impressionsErrorEmpty');
      return;
    }
    if (trimmed.length > MAX_LEN) {
      error = $t('impressionsErrorTooLong');
      return;
    }

    submitting = true;
    try {
      await api.submitImpression({
        message: trimmed,
        authorName: name.trim() || undefined,
        mood: mood || undefined,
      });
      sealing = true;
      setTimeout(() => {
        sealing = false;
        left = true;
      }, 900);
    } catch {
      error = $t('impressionsError');
    } finally {
      submitting = false;
    }
  }

  function leaveAnother() {
    message = '';
    name = '';
    mood = '';
    left = false;
  }
</script>

<section class="impressions" aria-labelledby="impressions-title">
  <div class="leaf">
    {#if left}
      <div class="page page-thanks" in:scale={{ start: 0.94, duration: 400 }}>
        <span class="wax" aria-hidden="true"><span class="wax-mark">❧</span></span>
        <h2 class="thanks-title">{$t('impressionsThanksTitle')}</h2>
        <p class="thanks-lead">{$t('impressionsThanksLead')}</p>
        <button class="another-link" type="button" onclick={leaveAnother}>
          {$t('impressionsAnother')}
        </button>
      </div>
    {:else}
      <div class="page page-invite">
        <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('impressionsEyebrow')}</p>
        <h2 id="impressions-title" class="impressions-title">{$t('impressionsTitle')}</h2>
        <p class="impressions-lead">{$t('impressionsLead')}</p>
      </div>

      <div class="page page-form">
        <form class="leave-form" onsubmit={submit} novalidate>
          {#if sealing}
            <div class="seal-press" in:scale={{ start: 0.4, duration: 400 }} aria-hidden="true">
              <span class="wax wax-pressing"><span class="wax-mark">❧</span></span>
            </div>
          {/if}

          <label class="field field-message">
            <span class="field-label">{$t('impressionsMessageLabel')}</span>
            <textarea
              bind:value={message}
              placeholder={$t('impressionsMessagePlaceholder')}
              maxlength={MAX_LEN}
              rows="4"
              required
              aria-invalid={Boolean(error)}
              disabled={submitting || sealing}
            ></textarea>
            <span class="char-count">{message.length} / {MAX_LEN}</span>
          </label>

          <fieldset class="field field-mood">
            <legend class="field-label">{$t('impressionsMoodLabel')}</legend>
            <div class="mood-chips">
              {#each MOODS as m (m)}
                <button
                  type="button"
                  class="mood-chip"
                  class:selected={mood === m}
                  disabled={submitting || sealing}
                  onclick={() => (mood = mood === m ? '' : m)}
                >
                  {$t(MOOD_LABEL_KEYS[m])}
                </button>
              {/each}
            </div>
          </fieldset>

          <label class="field field-name">
            <span class="field-label">{$t('impressionsNameLabel')}</span>
            <input
              id="impressions-name"
              name="name"
              type="text"
              bind:value={name}
              placeholder={$t('impressionsNamePlaceholder')}
              autocomplete="name"
              maxlength="100"
              disabled={submitting || sealing}
            />
          </label>

          <!-- honeypot: visually hidden, off-screen; real users never fill it -->
          <div class="hp" aria-hidden="true">
            <label>Leave this empty<input id="impressions-hp" name="website" type="text" bind:value={honeypot} tabindex="-1" autocomplete="off" /></label>
          </div>

          {#if error}<p class="form-error" role="alert">{error}</p>{/if}

          <button class="submit-btn" type="submit" disabled={submitting || sealing}>
            {submitting || sealing ? $t('impressionsSubmitting') : $t('impressionsSubmit')}
            <svg width="18" height="9" viewBox="0 0 18 9" fill="none" aria-hidden="true">
              <path d="M0 4.5H17M17 4.5L12.5 1M17 4.5L12.5 8" stroke="currentColor" stroke-width="1"/>
            </svg>
          </button>

          <p class="privacy">{$t('impressionsPrivacy')}</p>
        </form>
      </div>
    {/if}
  </div>
</section>

<style>
  .impressions {
    max-width: 1080px;
    margin: 0 auto;
    padding: clamp(28px, 4vw, 60px) clamp(20px, 4.5vw, 64px) clamp(48px, 6vw, 88px);
  }

  .leaf {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.1fr);
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

  .page { padding: clamp(24px, 3vw, 46px); }
  .page-form {
    border-left: 1px solid color-mix(in srgb, var(--color-ink-primary) 12%, transparent);
    background: linear-gradient(90deg, rgba(60, 36, 22, 0.05) 0%, transparent 4%), transparent;
    display: flex;
    align-items: center;
  }
  .page-thanks {
    grid-column: 1 / -1;
    display: grid;
    justify-items: start;
    gap: 14px;
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

  .impressions-title, .thanks-title {
    margin: 0 0 14px;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(30px, 3vw, 46px);
    font-weight: 300;
    line-height: 0.98;
    color: var(--color-ink-primary, #34251c);
  }

  .impressions-lead, .thanks-lead {
    margin: 0;
    max-width: 42ch;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(16px, 1.4vw, 20px);
    font-style: italic;
    font-weight: 300;
    line-height: 1.45;
    color: var(--color-ink-secondary, #5f4636);
  }

  .leave-form {
    position: relative;
    width: 100%;
    display: grid;
    gap: 18px;
  }

  .field { display: grid; gap: 6px; position: relative; }
  .field-label {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
    padding: 0;
    border: none;
  }

  .field-message textarea, .field-name input {
    width: 100%;
    padding: 6px 2px 8px;
    border: none;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 28%, transparent);
    background: transparent;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(17px, 1.5vw, 21px);
    color: var(--color-ink-primary, #34251c);
    resize: vertical;
    transition: border-color 0.2s ease;
  }
  .field-message textarea::placeholder, .field-name input::placeholder {
    color: color-mix(in srgb, var(--color-ink-primary) 36%, transparent);
    font-style: italic;
  }
  .field-message textarea:focus, .field-name input:focus {
    outline: none;
    border-bottom-color: var(--color-ember, #c65f3c);
  }
  .field-message textarea[aria-invalid='true'] { border-bottom-color: #b0472c; }

  .char-count {
    justify-self: end;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 10px;
    color: var(--color-ink-tertiary);
  }

  .field-mood { margin: 0; padding: 0; border: none; display: grid; gap: 8px; }
  .mood-chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .mood-chip {
    padding: 6px 14px;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 24%, transparent);
    background: transparent;
    border-radius: 999px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11.5px;
    letter-spacing: 0.03em;
    color: var(--color-ink-secondary, #5f4636);
    cursor: pointer;
    transition: background 0.18s ease, border-color 0.18s ease, color 0.18s ease;
  }
  .mood-chip:hover:not(:disabled) { border-color: var(--color-ember, #c65f3c); }
  .mood-chip.selected {
    background: var(--color-ink-primary, #34251c);
    border-color: var(--color-ink-primary, #34251c);
    color: var(--color-canvas-raised, #fffaf2);
  }
  .mood-chip:disabled { opacity: 0.55; cursor: default; }

  /* honeypot — kept off-screen for everyone, traps bots */
  .hp { position: absolute; left: -9999px; width: 1px; height: 1px; overflow: hidden; }

  .form-error {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: 15px;
    color: #a23c22;
  }

  .submit-btn {
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
  .submit-btn svg { transition: transform 0.22s ease; }
  .submit-btn:hover:not(:disabled) {
    background: var(--color-ember-deep, #6f3b24);
    box-shadow: 0 10px 24px rgba(68, 37, 20, 0.16);
  }
  .submit-btn:hover:not(:disabled) svg { transform: translateX(4px); }
  .submit-btn:active:not(:disabled) { transform: translateY(1px); }
  .submit-btn:disabled { opacity: 0.55; cursor: default; }

  .privacy {
    margin: 2px 0 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11.5px;
    letter-spacing: 0.01em;
    color: var(--color-ink-tertiary);
  }

  .another-link {
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
  .another-link:hover {
    color: var(--color-ember, #c65f3c);
    border-color: color-mix(in srgb, var(--color-ember) 50%, transparent);
  }

  /* ── seal ───────────────────────────────────────────────────── */
  .seal-press { position: absolute; inset: 0; z-index: 4; display: grid; place-items: center; pointer-events: none; }
  .wax {
    position: relative;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: radial-gradient(circle at 38% 32%, #d2683f, #8a2f1c 62%, #5f2012);
    box-shadow: 0 2px 8px rgba(10, 4, 2, 0.45), inset 0 1px 0 rgba(255, 200, 170, 0.5);
  }
  .wax-mark {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 26px;
    color: rgba(40, 12, 6, 0.5);
  }
  .wax-pressing { animation: press 0.5s cubic-bezier(0.34, 1.4, 0.64, 1) both; }
  @keyframes press {
    0% { transform: scale(1.6); opacity: 0; }
    60% { transform: scale(0.92); opacity: 1; }
    100% { transform: scale(1); opacity: 1; }
  }

  @media (max-width: 820px) {
    .leaf { grid-template-columns: 1fr; }
    .page-form {
      border-left: none;
      border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 12%, transparent);
      background: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .wax-pressing { animation: none; }
    .submit-btn svg { transition: none; }
  }
</style>
