<script lang="ts">
  /**
   * The reusable core of "write to the author": email + message, nothing
   * else. Anonymous, not tied to a figurine (unlike OrderModal) — used both
   * embedded in CorrespondenceInvite (home page) and inside the header's
   * quill panel (SiteHeader), via the `compact` prop.
   */
  import { scale } from 'svelte/transition';
  import { elasticOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import { t, lang, brandName } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';
  import { isValidEmail } from '$lib/validation';

  let {
    source = 'home' as 'home' | 'header',
    compact = false,
  }: { source?: 'home' | 'header'; compact?: boolean } = $props();

  const uid = $props.id();

  let email = $state(authStore.user?.email ?? '');
  let message = $state('');
  let submitting = $state(false);
  let sealed = $state(false);
  let error = $state('');

  let sealInitial = $derived($brandName.charAt(0).toUpperCase());

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    error = '';
    const trimmedEmail = email.trim();
    const trimmedMessage = message.trim();
    if (!trimmedEmail || !trimmedMessage) {
      error = $t('contactFormFillFields');
      return;
    }
    if (!isValidEmail(trimmedEmail)) {
      error = $t('contactFormInvalidEmail');
      return;
    }
    submitting = true;
    try {
      await api.submitContactMessage({
        email: trimmedEmail,
        message: trimmedMessage,
        source,
        lang: $lang,
      });
      sealed = true;
    } catch {
      error = $t('contactFormError');
    } finally {
      submitting = false;
    }
  }

  function writeAnother() {
    sealed = false;
    message = '';
  }
</script>

<form class="cmf" class:cmf-compact={compact} onsubmit={handleSubmit}>
  {#if !sealed}
    <label class="cmf-field">
      <span class="cmf-label">{$t('contactFormEmailLabel')}</span>
      <input
        id="contact-email-{uid}"
        name="email"
        type="email"
        bind:value={email}
        placeholder={$t('contactFormEmailPlaceholder')}
        autocomplete="email"
        required
      />
    </label>

    <label class="cmf-field">
      <span class="cmf-label">{$t('contactFormMessageLabel')}</span>
      <textarea
        id="contact-message-{uid}"
        name="message"
        bind:value={message}
        rows={compact ? 3 : 4}
        placeholder={$t('contactFormMessagePlaceholder')}
        required
      ></textarea>
    </label>

    {#if error}<p class="cmf-error">{error}</p>{/if}

    <button type="submit" class="cmf-submit" disabled={submitting}>
      {#if submitting}
        <span class="cmf-spinner" aria-hidden="true"></span>
        <span>{$t('contactFormSending')}</span>
      {:else}
        <span>{$t('contactFormSubmit')}</span>
        <span class="cmf-quill" aria-hidden="true">✒</span>
      {/if}
    </button>
  {:else}
    <div class="cmf-sealed" in:scale={{ duration: 500, start: 0.92, easing: elasticOut }}>
      <div class="cmf-seal" aria-hidden="true">
        <span class="cmf-seal-mark">{sealInitial}</span>
      </div>
      <p class="cmf-sealed-title">{$t('contactFormSuccessTitle')}</p>
      <p class="cmf-sealed-text">{$t('contactFormSuccessText')}</p>
      <button type="button" class="cmf-again" onclick={writeAnother}>
        {$t('contactFormWriteAnother')}
      </button>
    </div>
  {/if}
</form>

<style>
  .cmf {
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 100%;
  }

  .cmf-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .cmf-label {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary, #6f5a48);
  }
  .cmf input,
  .cmf textarea {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 14px;
    color: var(--color-ink-primary, #34251c);
    background: color-mix(in srgb, var(--color-canvas-base, #f8f1e7) 60%, white 40%);
    border: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 20%, transparent);
    border-radius: 2px;
    padding: 10px 12px;
    outline: none;
    resize: vertical;
    transition: border-color 0.25s ease, background 0.25s ease;
  }
  .cmf input::placeholder,
  .cmf textarea::placeholder {
    color: color-mix(in srgb, var(--color-ink-tertiary, #6f5a48) 65%, transparent);
  }
  .cmf input:focus,
  .cmf textarea:focus {
    border-color: var(--color-ember, #c65f3c);
    background: var(--color-canvas-base, #f8f1e7);
  }

  .cmf-error {
    margin: 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 12px;
    color: #a6392a;
  }

  .cmf-submit {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    align-self: flex-start;
    padding: 12px 22px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #fff9f0;
    background: var(--color-ember, #c65f3c);
    border: none;
    border-radius: 2px;
    cursor: pointer;
    transition: background 0.25s ease, transform 0.25s ease;
  }
  .cmf-submit:hover:not(:disabled) {
    background: color-mix(in srgb, var(--color-ember, #c65f3c) 85%, black 15%);
    transform: translateY(-1px);
  }
  .cmf-submit:disabled { opacity: 0.6; cursor: default; }
  .cmf-quill { font-size: 14px; opacity: 0.85; }
  .cmf-spinner {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(255,249,240,0.4);
    border-top-color: #fff9f0;
    border-radius: 50%;
    animation: cmf-spin 0.7s linear infinite;
  }
  @keyframes cmf-spin { to { transform: rotate(360deg); } }

  /* ── Sealed / success state ─────────────────── */
  .cmf-sealed {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 6px;
    padding: 8px 0 4px;
  }
  .cmf-seal {
    position: relative;
    width: 56px;
    height: 56px;
    margin-bottom: 8px;
    border-radius: 45% 55% 48% 52% / 51% 46% 54% 49%;
    background: linear-gradient(135deg, #c65f3c 0%, #a86124 55%, #9e452d 100%);
    border: 3px solid rgba(111,59,36,0.2);
    box-shadow: inset 2px 2px 10px rgba(111,59,36,0.18), inset -2px -2px 8px rgba(255,255,255,0.12);
    display: flex;
    align-items: center;
    justify-content: center;
    transform: rotate(-6deg);
  }
  .cmf-seal-mark {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 24px;
    color: #fff9f0;
    opacity: 0.9;
  }
  .cmf-sealed-title {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 20px;
    font-weight: 500;
    color: var(--color-ink-primary, #34251c);
  }
  .cmf-sealed-text {
    margin: 0;
    max-width: 32ch;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 13px;
    line-height: 1.6;
    color: var(--color-ink-secondary, #5f4636);
  }
  .cmf-again {
    margin-top: 10px;
    background: none;
    border: none;
    padding: 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary, #6f5a48);
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }
  .cmf-again:hover { color: var(--color-ember, #c65f3c); }

  /* ── Compact (header panel) tuning ──────────── */
  .cmf-compact { gap: 12px; }
  .cmf-compact .cmf-submit { align-self: stretch; justify-content: center; }
  .cmf-compact .cmf-seal { width: 44px; height: 44px; }
  .cmf-compact .cmf-seal-mark { font-size: 18px; }
  .cmf-compact .cmf-sealed-title { font-size: 16px; }
</style>
