<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { t, lang, brandName } from '$lib/i18n';
  import { SITE_URL } from '$lib/site';
  import { jsonLdSafe } from '$lib/jsonld';
  import AppImage from '$lib/components/AppImage.svelte';
  import { decodeEntities, GAZETTE_KIND_KEY, leafCopy, leafHref, quietDate } from '$lib/gazette';

  let { data } = $props();
  let leaves = $derived(data.page.items);
  let cuttings = $derived(data.cuttings);
  let lead = $derived(leaves[0] ?? null);
  let rest = $derived(leaves.slice(1));
  let leadCopy = $derived(lead ? leafCopy(lead, $lang) : null);
  let empty = $derived(leaves.length === 0 && cuttings.length === 0);

  let jsonLd = $derived(jsonLdSafe({
    '@context': 'https://schema.org',
    '@type': 'CollectionPage',
    name: $t('gazettePageTitle'),
    description: $t('gazettePageSubtitle'),
    url: `${SITE_URL}/gazette`,
    isPartOf: { '@type': 'WebSite', name: $brandName, url: SITE_URL },
  }));
</script>

<svelte:head>
  <title>{$t('gazettePageTitle')} — {$brandName}</title>
  <meta name="description" content={$t('gazettePageSubtitle')} />
  <meta property="og:site_name" content={$brandName} />
  <meta property="og:locale" content="en_US" />
  <meta property="og:type" content="website" />
  <meta property="og:title" content="{$t('gazettePageTitle')} — {$brandName}" />
  <meta property="og:description" content={$t('gazettePageSubtitle')} />
  <meta property="og:url" content="{SITE_URL}/gazette" />
  <meta property="og:image" content="{SITE_URL}/images/cabinet-bg.jpeg" />
  {@html `<script type="application/ld+json">${jsonLd}<\/script>`}
</svelte:head>

<div class="root">
  <div class="grain" aria-hidden="true"></div>
  <div class="page">
    <nav class="back-nav" in:fade={{ duration: 600 }}>
      <a href="/" class="back-link">{$t('gazetteBack')}</a>
    </nav>

    <header class="masthead" in:fly={{ x: -20, duration: 900, delay: 80, easing: cubicOut }}>
      <p class="eyebrow">
        <span class="eyebrow-rule"></span>
        {$t('gazettePageKicker')}
      </p>
      <h1 class="page-title">{$t('gazettePageTitle')}</h1>
      <p class="page-subtitle">{$t('gazettePageSubtitle')}</p>
    </header>

    {#if empty}
      <p class="empty" in:fade={{ duration: 700, delay: 160 }}>{$t('gazetteEmpty')}</p>
    {:else}
      {#if lead && leadCopy}
        <a
          class="lead"
          href={leafHref(lead, 'gazette')}
          in:fly={{ y: 16, duration: 700, delay: 120, easing: cubicOut }}
        >
          {#if lead.imageUrl}
            <span class="lead-face">
              <AppImage src={lead.imageUrl} alt="" class="lead-img" sizes="160px" />
            </span>
          {/if}
          <span class="lead-copy">
            <span class="meta">
              <span>{$t(GAZETTE_KIND_KEY[lead.kind])}</span>
              {#if quietDate(lead.publishedAt ?? lead.createdAt, $lang)}
                <span>{quietDate(lead.publishedAt ?? lead.createdAt, $lang)}</span>
              {/if}
            </span>
            <span class="lead-title">{leadCopy.title}</span>
            {#if leadCopy.dek}
              <span class="lead-dek">{leadCopy.dek}</span>
            {/if}
          </span>
        </a>
      {/if}

      {#if rest.length > 0 || cuttings.length > 0}
      <div
        class="board"
        class:solo={rest.length === 0 || cuttings.length === 0}
        in:fade={{ duration: 600, delay: 180 }}
      >
        {#if rest.length > 0}
          <section class="col" aria-labelledby="gz-house">
            <h2 id="gz-house" class="col-label">{$t('gazetteHouseCol')}</h2>
            <ol class="ledger">
              {#each rest as leaf, i (leaf.id)}
                {@const copy = leafCopy(leaf, $lang)}
                <li in:fly={{ y: 12, duration: 560, delay: 40 * i, easing: cubicOut }}>
                  <a class="row" href={leafHref(leaf, 'gazette')}>
                    <span class="meta">
                      <span>{$t(GAZETTE_KIND_KEY[leaf.kind])}</span>
                      {#if quietDate(leaf.publishedAt ?? leaf.createdAt, $lang)}
                        <span>{quietDate(leaf.publishedAt ?? leaf.createdAt, $lang)}</span>
                      {/if}
                    </span>
                    <span class="row-title">{copy.title}</span>
                    {#if copy.dek}
                      <span class="row-dek">{copy.dek}</span>
                    {/if}
                  </a>
                </li>
              {/each}
            </ol>
          </section>
        {/if}

        {#if cuttings.length > 0}
          <section class="col col-world" aria-labelledby="gz-world">
            <h2 id="gz-world" class="col-label">{$t('gazetteWorldCol')}</h2>
            <ul class="ledger">
              {#each cuttings as cut, i (cut.id)}
                <li in:fly={{ y: 12, duration: 560, delay: 40 * i, easing: cubicOut }}>
                  <a
                    class="row cut"
                    href={cut.url}
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    <span class="meta">
                      <span>{$t('gazetteSource')} {cut.sourceName}</span>
                    </span>
                    <span class="row-title">{decodeEntities(cut.title)}</span>
                    <span class="cut-out">{$t('gazetteLeavesHouse')} →</span>
                  </a>
                </li>
              {/each}
            </ul>
          </section>
        {/if}
      </div>
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
    margin: 0 0 18px;
  }

  .page-subtitle {
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

  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 14px;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .lead {
    display: grid;
    grid-template-columns: 1fr;
    gap: clamp(18px, 3vw, 32px);
    align-items: start;
    margin-bottom: clamp(40px, 6vw, 72px);
    padding: 22px 24px 22px 22px;
    text-decoration: none;
    color: inherit;
    border-radius: 6px;
    background:
      linear-gradient(165deg, #fbf4e8 0%, #f3e6d0 100%);
    border: 1px solid rgba(52, 37, 28, 0.12);
    border-left: 2px solid var(--copper, #c65f3c);
    box-shadow: inset 0 1px 0 rgba(255, 247, 234, 0.7);
    transition: border-color 0.22s ease, transform 0.22s ease;
  }
  .lead:has(.lead-face) {
    grid-template-columns: auto 1fr;
  }
  .lead:hover,
  .lead:focus-visible {
    border-color: rgba(198, 95, 60, 0.4);
    transform: translateY(-1px);
    outline: none;
  }

  .lead-face {
    display: block;
    width: 132px;
    height: 132px;
    overflow: hidden;
    background: #1a120e;
    border: 1px solid #d8c6b1;
    flex-shrink: 0;
  }
  .lead-face :global(.app-image-wrap),
  .lead-face :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .lead-copy { display: grid; gap: 8px; min-width: 0; }

  .lead-title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(28px, 3.4vw, 42px);
    font-weight: 400;
    line-height: 1.08;
    color: #34251c;
  }

  .lead-dek {
    font-size: 16px;
    line-height: 1.5;
    color: #5f4636;
    max-width: 40em;
  }

  .board {
    display: grid;
    grid-template-columns: 1.15fr 0.85fr;
    gap: clamp(36px, 5vw, 72px);
    align-items: start;
  }
  .board.solo { grid-template-columns: 1fr; max-width: 640px; }

  .col-label {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--copper, #c65f3c);
    margin: 0 0 6px;
  }

  .ledger {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .row {
    display: grid;
    gap: 5px;
    padding: 16px 0;
    border-top: 1px solid rgba(216, 198, 177, 0.7);
    text-decoration: none;
    color: inherit;
    transition: color 0.2s ease;
  }
  .ledger li:first-child .row { border-top: 1px solid rgba(216, 198, 177, 0.95); }

  .row-title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 22px;
    line-height: 1.2;
    color: #34251c;
  }
  .row-dek {
    font-size: 14px;
    line-height: 1.45;
    color: #5f4636;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }
  .row:hover .row-title,
  .row:focus-visible .row-title { color: #6f3b24; }
  .row:focus-visible { outline: none; }

  .cut .row-title { font-size: 18px; }
  .cut-out {
    font-size: 11px;
    letter-spacing: 0.04em;
    color: #8a5a3a;
  }

  @media (max-width: 860px) {
    .board { grid-template-columns: 1fr; }
    .lead { grid-template-columns: 1fr; }
    .lead-face { width: 96px; height: 96px; }
  }

  @media (prefers-reduced-motion: reduce) {
    .lead { transition: none; }
    .lead:hover,
    .lead:focus-visible { transform: none; }
  }
</style>
