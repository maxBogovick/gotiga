<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { fade } from 'svelte/transition';
  import { t, brandName } from '$lib/i18n';
  import { api } from '$lib/api';

  let token = $derived(page.params.token ?? '');

  type Phase = 'loading' | 'found' | 'not_found' | 'done' | 'error';

  let phase = $state<Phase>('loading');
  let email = $state('');
  let leaving = $state(false);

  const VISITOR_BOOK_KEY = 'gotiga_visitor_book';

  onMount(async () => {
    if (!token) { phase = 'not_found'; return; }
    try {
      const info = await api.getSubscriptionByToken(token);
      if (!info) { phase = 'not_found'; return; }
      email = info.email;
      phase = 'found';
    } catch {
      phase = 'error';
    }
  });

  async function confirm() {
    if (leaving) return;
    leaving = true;
    try {
      await api.unsubscribeByToken(token);
      // Clear the home-page "already signed" state if it belongs to this token.
      if (typeof localStorage !== 'undefined') {
        try {
          const raw = localStorage.getItem(VISITOR_BOOK_KEY);
          if (raw && JSON.parse(raw)?.token === token) localStorage.removeItem(VISITOR_BOOK_KEY);
        } catch { /* ignore */ }
      }
      phase = 'done';
    } catch {
      phase = 'error';
    } finally {
      leaving = false;
    }
  }
</script>

<svelte:head>
  <title>{$t('unsubscribeTitle')} — {$brandName}</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<main class="wrap" in:fade={{ duration: 400 }}>
  <div class="card">
    <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('visitorBookEyebrow')}</p>

    {#if phase === 'loading'}
      <p class="lead">{$t('unsubscribeLoading')}</p>
    {:else if phase === 'not_found'}
      <h1 class="title">{$t('unsubscribeNotFoundTitle')}</h1>
      <p class="lead">{$t('unsubscribeNotFound')}</p>
      <a class="back" href="/">{$t('unsubscribeBack')}</a>
    {:else if phase === 'error'}
      <h1 class="title">{$t('unsubscribeErrorTitle')}</h1>
      <p class="lead">{$t('visitorBookError')}</p>
      <a class="back" href="/">{$t('unsubscribeBack')}</a>
    {:else if phase === 'done'}
      <span class="wax" aria-hidden="true"><span class="wax-mark">❧</span></span>
      <h1 class="title">{$t('unsubscribeDoneTitle')}</h1>
      <p class="lead">{$t('unsubscribeDone')}</p>
      <a class="back" href="/">{$t('unsubscribeBack')}</a>
    {:else}
      <h1 class="title">{$t('unsubscribeTitle')}</h1>
      <p class="lead">{$t('unsubscribeText')}</p>
      {#if email}<p class="email">{email}</p>{/if}
      <div class="actions">
        <button class="confirm" type="button" onclick={confirm} disabled={leaving}>
          {leaving ? $t('visitorBookLeaving') : $t('unsubscribeConfirm')}
        </button>
        <a class="stay" href="/">{$t('unsubscribeStay')}</a>
      </div>
    {/if}
  </div>
</main>

<style>
  .wrap {
    min-height: 100svh;
    display: grid;
    place-items: center;
    padding: clamp(40px, 8vw, 120px) clamp(20px, 5vw, 64px);
    background:
      radial-gradient(ellipse 60% 50% at 30% 20%, rgba(198, 95, 60, 0.05) 0%, transparent 60%),
      var(--color-canvas-base, #f8f1e7);
  }
  .card {
    width: 100%;
    max-width: 480px;
    display: grid;
    justify-items: start;
    gap: 14px;
    padding: clamp(28px, 4vw, 48px);
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 14%, transparent);
    border-radius: 4px;
    background: linear-gradient(180deg, var(--color-canvas-raised, #fffaf2), var(--color-canvas-base, #f8f1e7));
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.7), 0 14px 40px rgba(60, 36, 22, 0.10);
  }
  .eyebrow {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
  }
  .eyebrow-rule { display: inline-block; width: 26px; height: 1px; background: var(--color-ember, #c65f3c); opacity: 0.65; }
  .title {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(26px, 2.6vw, 38px);
    font-weight: 300;
    line-height: 1;
    color: var(--color-ink-primary, #34251c);
  }
  .lead {
    margin: 0;
    max-width: 42ch;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: clamp(16px, 1.4vw, 20px);
    line-height: 1.45;
    color: var(--color-ink-secondary, #5f4636);
  }
  .email {
    margin: 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 14px;
    letter-spacing: 0.02em;
    color: var(--color-ink-primary, #34251c);
  }
  .actions { display: flex; align-items: center; gap: 18px; margin-top: 8px; flex-wrap: wrap; }
  .confirm {
    display: inline-flex;
    align-items: center;
    height: 42px;
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
    transition: background 0.22s ease, transform 0.12s ease;
  }
  .confirm:hover:not(:disabled) { background: var(--color-ember-deep, #6f3b24); }
  .confirm:active:not(:disabled) { transform: translateY(1px); }
  .confirm:disabled { opacity: 0.55; cursor: default; }
  .stay, .back {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    text-decoration: none;
    color: var(--color-ink-tertiary);
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 22%, transparent);
    padding-bottom: 2px;
    transition: color 0.2s ease, border-color 0.2s ease;
  }
  .stay:hover, .back:hover { color: var(--color-ember, #c65f3c); border-color: color-mix(in srgb, var(--color-ember) 50%, transparent); }
  .back { margin-top: 6px; }
  .wax {
    position: relative;
    width: 52px;
    height: 52px;
    border-radius: 50%;
    background: radial-gradient(circle at 38% 32%, #d2683f, #8a2f1c 62%, #5f2012);
    box-shadow: 0 2px 8px rgba(10, 4, 2, 0.45), inset 0 1px 0 rgba(255, 200, 170, 0.5);
  }
  .wax-mark { position: absolute; inset: 0; display: grid; place-items: center; font-size: 24px; color: rgba(40, 12, 6, 0.5); }
</style>
