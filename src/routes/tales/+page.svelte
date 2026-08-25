<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { t, lang, brandName } from '$lib/i18n';
  import { SITE_URL } from '$lib/site';
  import { jsonLdSafe } from '$lib/jsonld';
  import { leafCopy, leafCoverUrl, leafHref } from '$lib/gazette';
  import { leadTale, spineHeight } from '$lib/tales';
  import AppImage from '$lib/components/AppImage.svelte';

  let { data } = $props();

  let tales = $derived(data.tales ?? []);
  let lead = $derived(leadTale(tales));
  let shelf = $derived(tales.filter((tale) => tale.id !== lead?.id));
  let leadCopy = $derived(lead ? leafCopy(lead, $lang) : null);
  let leadCover = $derived(lead ? leafCoverUrl(lead) : '');

  // Picked after the page is alive, never during load: this room prerenders,
  // so a build-time pick would freeze one tale as "random" forever.
  let randomHref = $state('');
  onMount(() => {
    if (!shelf.length) return;
    randomHref = leafHref(shelf[Math.floor(Math.random() * shelf.length)], 'tales_random');
  });

  // The spines are ordinary links, so Tab already walks them. The arrows are
  // for the reader who is already standing at the shelf — and, like the
  // gallery's, they are not advertised anywhere.
  function walkShelf(event: KeyboardEvent & { currentTarget: HTMLElement }) {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
    const spines = [...event.currentTarget.querySelectorAll<HTMLAnchorElement>('.spine')];
    const at = spines.indexOf(document.activeElement as HTMLAnchorElement);
    if (at < 0) return;
    const to = spines[at + (event.key === 'ArrowRight' ? 1 : -1)];
    if (!to) return;
    event.preventDefault();
    to.focus();
  }

  let ogLocale = $derived($lang === 'ru' ? 'ru_RU' : 'en_US');
  let jsonLd = $derived(jsonLdSafe({
    '@context': 'https://schema.org',
    '@type': 'CollectionPage',
    name: $t('talesPageTitle'),
    description: $t('talesPageRule'),
    url: `${SITE_URL}/tales`,
    inLanguage: $lang === 'ru' ? 'ru' : 'en',
    isPartOf: { '@type': 'WebSite', name: $brandName, url: SITE_URL },
  }));
</script>

<svelte:head>
  <title>{$t('talesPageTitle')} — {$brandName}</title>
  <meta name="description" content={$t('talesPageRule')} />
  <link rel="canonical" href="{SITE_URL}/tales" />
  <meta property="og:site_name" content={$brandName} />
  <meta property="og:locale" content={ogLocale} />
  <meta property="og:type" content="website" />
  <meta property="og:title" content="{$t('talesPageTitle')} — {$brandName}" />
  <meta property="og:description" content={$t('talesPageRule')} />
  <meta property="og:url" content="{SITE_URL}/tales" />
  <meta property="og:image" content="{SITE_URL}/images/cabinet-bg.jpeg" />
  {@html `<script type="application/ld+json">${jsonLd}<\/script>`}
</svelte:head>

<div class="root">
  <div class="grain" aria-hidden="true"></div>
  <div class="page">
    <nav class="back-nav" in:fade={{ duration: 600 }}>
      <a href="/" class="back-link">{$t('talesBack')}</a>
    </nav>

    <header class="masthead" in:fly={{ x: -20, duration: 900, delay: 80, easing: cubicOut }}>
      <p class="eyebrow">
        <span class="eyebrow-rule"></span>
        {$t('talesPageKicker')}
      </p>
      <h1 class="page-title">{$t('talesPageTitle')}</h1>
      <p class="page-rule">{$t('talesPageRule')}</p>
    </header>

    {#if !lead || !leadCopy}
      <p class="empty" in:fade={{ duration: 700, delay: 160 }}>{$t('talesEmpty')}</p>
    {:else}
      <a
        class="lead"
        href={leafHref(lead, 'tales_lead')}
        in:fly={{ y: 16, duration: 700, delay: 120, easing: cubicOut }}
      >
        {#if leadCover}
          <span class="lead-face">
            <AppImage src={leadCover} alt="" class="lead-img" sizes="160px" />
          </span>
        {/if}
        <span class="lead-copy">
          <span class="lead-title">{leadCopy.title}</span>
          {#if leadCopy.dek}<span class="lead-dek">{leadCopy.dek}</span>{/if}
          <span class="lead-read">{$t('talesRead')} →</span>
        </span>
      </a>

      {#if shelf.length}
        <section class="shelf-block" in:fade={{ duration: 700, delay: 220 }}>
          <p class="shelf-label">{$t('talesShelf')}</p>
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <ul class="shelf" onkeydown={walkShelf}>
            {#each shelf as tale (tale.id)}
              {@const copy = leafCopy(tale, $lang)}
              {@const cover = leafCoverUrl(tale)}
              <li class="slot">
                <a
                  class="spine"
                  href={leafHref(tale, 'tales_shelf')}
                  style="height: {spineHeight(tale.slug)}px"
                >
                  <span class="spine-title">{copy.title}</span>
                  {#if cover}
                    <span class="spine-face" aria-hidden="true">
                      <AppImage src={cover} alt="" class="spine-img" sizes="54px" />
                    </span>
                  {/if}
                </a>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      {#if randomHref}
        <p class="chance" in:fade={{ duration: 600 }}>
          <a href={randomHref}>{$t('talesRandom')}</a>
        </p>
      {/if}
    {/if}
  </div>
</div>

<style>
  .root {
    width: 100%;
    min-height: 100svh;
    background:
      radial-gradient(ellipse 70% 55% at 72% 38%, rgba(198, 95, 60, 0.06) 0%, transparent 65%),
      var(--cream, #f8f1e7);
    position: relative;
    overflow-x: hidden;
  }

  .grain {
    position: fixed;
    inset: -50%;
    width: 200%;
    height: 200%;
    opacity: 0.028;
    pointer-events: none;
    z-index: 500;
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
  }

  .page {
    max-width: 1120px;
    margin: 0 auto;
    padding: clamp(80px, 10vw, 140px) clamp(20px, 5vw, 64px) clamp(72px, 10vw, 120px);
    position: relative;
    z-index: 1;
  }

  .back-nav { margin-bottom: clamp(28px, 4vw, 56px); }

  .back-link {
    font-size: 9px;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: var(--muted2, #5f4636);
    text-decoration: none;
    transition: color 0.25s;
  }
  .back-link:hover { color: var(--brown, #34251c); }

  .masthead { max-width: 38em; margin-bottom: clamp(36px, 5vw, 64px); }

  .eyebrow {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: var(--muted2, #5f4636);
    margin: 0 0 18px;
  }
  .eyebrow-rule {
    display: inline-block;
    width: 26px;
    height: 1px;
    background: var(--copper, #c65f3c);
    opacity: 0.65;
  }

  .page-title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(44px, 6.4vw, 88px);
    font-weight: 300;
    line-height: 0.92;
    letter-spacing: -0.015em;
    color: var(--ink, #34251c);
    margin: 0 0 14px;
  }

  .page-rule {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(16px, 1.7vw, 20px);
    font-weight: 300;
    font-style: italic;
    line-height: 1.5;
    color: var(--muted, #5f4636);
    margin: 0;
  }

  .empty {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 18px;
    font-style: italic;
    color: var(--muted, #5f4636);
  }

  /* ── The one tale the room shows large ─────────────────────────────────── */

  .lead {
    display: grid;
    grid-template-columns: 1fr;
    gap: clamp(18px, 3vw, 32px);
    align-items: start;
    padding: 24px 26px;
    margin-bottom: clamp(48px, 7vw, 88px);
    text-decoration: none;
    color: inherit;
    border-radius: 6px;
    background: linear-gradient(165deg, #fbf4e8 0%, #f3e6d0 100%);
    border: 1px solid rgba(52, 37, 28, 0.12);
    border-left: 2px solid var(--copper, #c65f3c);
    box-shadow: inset 0 1px 0 rgba(255, 247, 234, 0.7);
    transition: border-color 0.22s ease, transform 0.22s ease;
  }
  .lead:has(.lead-face) { grid-template-columns: auto 1fr; }
  .lead:hover,
  .lead:focus-visible {
    border-color: rgba(198, 95, 60, 0.4);
    transform: translateY(-1px);
    outline: none;
  }

  .lead-face {
    display: block;
    width: 148px;
    height: 148px;
    overflow: hidden;
    background: #1a120e;
    border: 1px solid var(--line, #d8c6b1);
    flex-shrink: 0;
  }
  .lead-face :global(.app-image-wrap),
  .lead-face :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .lead-copy { display: grid; gap: 10px; min-width: 0; }

  .lead-title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(30px, 3.6vw, 46px);
    font-weight: 400;
    line-height: 1.06;
    color: var(--ink, #34251c);
  }

  .lead-dek {
    font-size: 16px;
    line-height: 1.55;
    color: var(--muted, #5f4636);
    max-width: 38em;
  }

  .lead-read {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--copper, #c65f3c);
  }

  /* ── The shelf ─────────────────────────────────────────────────────────────
     Every slot is exactly as tall as the tallest spine may stand, so each
     wrapped row lands on the same grid — which is what lets one repeating
     gradient draw a board under every row without knowing where they break. */

  .shelf-block { margin-bottom: clamp(40px, 6vw, 72px); }

  .shelf-label {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--copper, #c65f3c);
    margin: 0 0 22px;
  }

  .shelf {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 56px 12px;
    margin: 0;
    padding: 0;
    list-style: none;
    background-image: repeating-linear-gradient(
      to bottom,
      transparent 0,
      transparent 338px,
      rgba(52, 37, 28, 0.18) 338px,
      rgba(52, 37, 28, 0.18) 340px,
      rgba(52, 37, 28, 0.05) 340px,
      rgba(52, 37, 28, 0.05) 343px,
      transparent 343px,
      transparent 396px
    );
  }

  .slot {
    height: 340px;
    display: flex;
    align-items: flex-end;
  }

  .spine {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    width: 54px;
    padding: 16px 0 0;
    text-decoration: none;
    color: inherit;
    overflow: hidden;
    background: linear-gradient(100deg, #f7eddd 0%, #efe0c8 52%, #e6d3b8 100%);
    border: 1px solid rgba(52, 37, 28, 0.14);
    border-left: 2px solid rgba(111, 59, 36, 0.5);
    border-radius: 2px 3px 3px 2px;
    box-shadow:
      inset 1px 0 0 rgba(255, 247, 234, 0.6),
      1px 2px 4px rgba(52, 37, 28, 0.12);
    transition: transform 0.28s cubic-bezier(0.2, 0.8, 0.3, 1), box-shadow 0.28s ease,
      border-color 0.28s ease;
  }
  .spine:hover,
  .spine:focus-visible {
    transform: translateY(-10px) rotate(-1deg);
    border-color: rgba(198, 95, 60, 0.45);
    box-shadow:
      inset 1px 0 0 rgba(255, 247, 234, 0.6),
      2px 8px 16px rgba(52, 37, 28, 0.2);
    outline: none;
  }

  .spine-title {
    writing-mode: vertical-rl;
    text-orientation: mixed;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 16px;
    font-weight: 400;
    letter-spacing: 0.04em;
    line-height: 1;
    color: var(--ink, #34251c);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-height: 100%;
    min-height: 0;
  }

  .spine-face {
    display: block;
    width: 100%;
    height: 40px;
    flex-shrink: 0;
    margin-top: 12px;
    overflow: hidden;
    background: #1a120e;
    border-top: 1px solid rgba(52, 37, 28, 0.18);
  }
  .spine-face :global(.app-image-wrap),
  .spine-face :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: sepia(0.4) contrast(0.9);
    opacity: 0.55;
    transition: opacity 0.28s ease;
  }
  .spine:hover .spine-face :global(img),
  .spine:focus-visible .spine-face :global(img) { opacity: 0.85; }

  .chance {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 17px;
    font-style: italic;
  }
  .chance a {
    color: var(--muted, #5f4636);
    text-decoration: none;
    border-bottom: 1px solid rgba(198, 95, 60, 0.35);
    padding-bottom: 2px;
    transition: color 0.25s, border-color 0.25s;
  }
  .chance a:hover {
    color: var(--brown, #34251c);
    border-color: var(--copper, #c65f3c);
  }

  /* One long shelf you walk along, rather than a grid of stubs. */
  @media (max-width: 639px) {
    .shelf {
      flex-wrap: nowrap;
      overflow-x: auto;
      padding-bottom: 8px;
      scrollbar-width: thin;
    }
    .lead { padding: 20px; }
    .lead-face { width: 108px; height: 108px; }
  }

  @media (prefers-reduced-motion: reduce) {
    .spine,
    .lead { transition: border-color 0.28s ease, box-shadow 0.28s ease; }
    .spine:hover,
    .spine:focus-visible,
    .lead:hover,
    .lead:focus-visible { transform: none; }
  }
</style>
