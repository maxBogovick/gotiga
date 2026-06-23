<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { t, brandName } from '$lib/i18n';

  const numerals = ['I', 'II', 'III', 'IV', 'V'];

  let steps = $derived([
    { title: $t('acquireStep1Title'), body: $t('acquireStep1Body') },
    { title: $t('acquireStep2Title'), body: $t('acquireStep2Body') },
    { title: $t('acquireStep3Title'), body: $t('acquireStep3Body') },
    { title: $t('acquireStep4Title'), body: $t('acquireStep4Body') },
    { title: $t('acquireStep5Title'), body: $t('acquireStep5Body') },
  ]);
</script>

<svelte:head>
  <title>{$t('acquireTitle')} — {$brandName}</title>
  <meta name="description" content={$t('acquireSubtitle')} />
</svelte:head>

<div class="root">
  <div class="grain" aria-hidden="true"></div>

  <div class="page">
    <header class="page-header" in:fly={{ x: -20, duration: 900, delay: 100, easing: cubicOut }}>
      <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('acquireKicker')}</p>
      <h1 class="page-title">{$t('acquireTitle')}</h1>
      <p class="page-subtitle">{$t('acquireSubtitle')}</p>
    </header>

    <ol class="ledger">
      {#each steps as step, i (step.title)}
        <li class="entry" in:fade={{ duration: 500, delay: 150 + i * 90 }}>
          <span class="numeral" aria-hidden="true">{numerals[i]}</span>
          <div class="entry-copy">
            <h2 class="entry-title">{step.title}</h2>
            <p class="entry-body">{step.body}</p>
          </div>
        </li>
      {/each}
    </ol>

    <div class="closing" in:fade={{ duration: 600, delay: 650 }}>
      <span class="seal" aria-hidden="true">✦</span>
      <p>{$t('acquireClosing')}</p>
      <div class="actions">
        <a class="cta cta--primary" href="/commission">{$t('acquireCommissionCta')} →</a>
        <a class="cta cta--ghost" href="/figurines">{$t('acquireBrowseCta')}</a>
      </div>
    </div>
  </div>
</div>

<style>
  .root {
    position: relative;
    min-height: 100vh;
    background: #f8f1e7;
    color: #34251c;
    font-family: 'Instrument Sans', sans-serif;
    overflow-x: hidden;
  }
  .grain {
    position: fixed; inset: 0; pointer-events: none; opacity: 0.4; z-index: 1;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='120'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)' opacity='0.5'/%3E%3C/svg%3E");
  }
  .page { position: relative; z-index: 2; max-width: 680px; margin: 0 auto; padding: 3.5rem 1.5rem 5rem; }

  .eyebrow { display: flex; align-items: center; gap: 0.75rem; font-size: 0.7rem; letter-spacing: 0.22em; text-transform: uppercase; color: #c65f3c; margin-bottom: 1rem; }
  .eyebrow-rule { width: 2.5rem; height: 1px; background: #c65f3c; }
  .page-title { font-family: 'Fraunces', Georgia, serif; font-size: clamp(2rem, 6vw, 3rem); font-weight: 400; line-height: 1.05; margin: 0 0 0.9rem; }
  .page-subtitle { font-family: 'Cormorant Garamond', Georgia, serif; font-size: 1.25rem; font-style: italic; color: #6f3b24; max-width: 44ch; line-height: 1.5; }

  .ledger { list-style: none; margin: 3rem 0 0; padding: 0; }
  .entry { display: grid; grid-template-columns: 3.2rem 1fr; gap: 1.1rem; padding: 1.6rem 0; border-top: 1px solid #d8c6b1; }
  .entry:last-child { border-bottom: 1px solid #d8c6b1; }
  .numeral { font-family: 'Fraunces', Georgia, serif; font-size: 1.5rem; color: #c65f3c; line-height: 1.1; letter-spacing: 0.04em; }
  .entry-title { font-family: 'Fraunces', Georgia, serif; font-size: 1.3rem; font-weight: 500; margin: 0 0 0.5rem; color: #34251c; }
  .entry-body { font-family: 'Cormorant Garamond', Georgia, serif; font-size: 1.15rem; line-height: 1.6; color: #5f4636; margin: 0; }

  .closing { margin-top: 3rem; text-align: center; }
  .seal { display: inline-grid; place-items: center; width: 3.4rem; height: 3.4rem; margin-bottom: 1rem; border-radius: 48% 52% 45% 55%; background: radial-gradient(circle at 35% 30%, #d8714f, #8f2e1a); color: #f8f1e7; font-size: 1.2rem; box-shadow: 0 10px 24px -12px rgba(143,46,26,0.7); }
  .closing p { font-family: 'Cormorant Garamond', Georgia, serif; font-size: 1.2rem; font-style: italic; color: #6f3b24; max-width: 40ch; margin: 0 auto 1.6rem; line-height: 1.55; }
  .actions { display: flex; gap: 0.75rem; justify-content: center; flex-wrap: wrap; }
  .cta { display: inline-block; padding: 0.7rem 1.6rem; font-size: 0.85rem; letter-spacing: 0.08em; text-transform: uppercase; text-decoration: none; transition: background 0.2s, border-color 0.2s, color 0.2s; }
  .cta--primary { background: #6f3b24; color: #f8f1e7; }
  .cta--primary:hover { background: #c65f3c; }
  .cta--ghost { border: 1px solid #d8c6b1; color: #6f3b24; }
  .cta--ghost:hover { background: #f0e6d6; border-color: #c65f3c; }

  @media (max-width: 520px) {
    .entry { grid-template-columns: 2.4rem 1fr; gap: 0.8rem; }
    .numeral { font-size: 1.2rem; }
  }
</style>
