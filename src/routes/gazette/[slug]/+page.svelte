<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { t, lang, brandName } from '$lib/i18n';
  import { SITE_URL } from '$lib/site';
  import { jsonLdSafe } from '$lib/jsonld';
  import {
    GAZETTE_KIND_KEY,
    leafCopy,
    neighborTitle,
    quietDate,
    workHref,
    leafCoverUrl,
    leafImageList,
    expectedWhisper,
  } from '$lib/gazette';
  import NotFound from '$lib/components/NotFound.svelte';
  import AppImage from '$lib/components/AppImage.svelte';
  import GazetteRoom from '$lib/components/GazetteRoom.svelte';
  import GazetteWatchSeal from '$lib/components/GazetteWatchSeal.svelte';

  let { data } = $props();
  let copy = $derived(data.leaf ? leafCopy(data.leaf, $lang) : null);
  let work = $derived(data.leaf ? workHref(data.leaf, 'gazette_leaf') : null);
  let expected = $derived(
    data.leaf
      ? expectedWhisper(
          data.leaf,
          $lang,
          (d) => $t('gazetteExpectedAround').replace('{date}', d),
          (a, b) => $t('gazetteExpectedRange').replace('{from}', a).replace('{to}', b),
        )
      : '',
  );
  let date = $derived(
    expected ||
      (data.leaf ? quietDate(data.leaf.publishedAt ?? data.leaf.createdAt, $lang) : ''),
  );
  let outside = $derived(data.leaf?.href?.startsWith('http') ? data.leaf.href : null);
  let ogLocale = $derived($lang === 'ru' ? 'ru_RU' : 'en_US');
  let prevTitle = $derived(
    data.leaf?.prev ? neighborTitle(data.leaf.prev, $lang) : '',
  );
  let nextTitle = $derived(
    data.leaf?.next ? neighborTitle(data.leaf.next, $lang) : '',
  );
  let plates = $derived(data.leaf ? leafImageList(data.leaf) : []);
  let cover = $derived(data.leaf ? leafCoverUrl(data.leaf) : '');

  let jsonLd = $derived(
    data.leaf && copy
      ? jsonLdSafe({
          '@context': 'https://schema.org',
          '@type': 'Article',
          headline: copy.title,
          description: copy.dek || $t('gazettePageSubtitle'),
          url: `${SITE_URL}/gazette/${data.leaf.slug}`,
          datePublished: data.leaf.publishedAt ?? data.leaf.createdAt,
          inLanguage: $lang === 'ru' ? 'ru' : 'en',
          image: cover || undefined,
          author: { '@type': 'Organization', name: $brandName },
          isPartOf: { '@type': 'WebSite', name: $brandName, url: SITE_URL },
        })
      : '',
  );
</script>

<svelte:head>
  {#if data.mode === 'leaf' && data.leaf}
    <title>{copy?.title ?? $t('gazettePageTitle')} — {$brandName}</title>
    <meta name="description" content={copy?.dek || $t('gazettePageSubtitle')} />
    <link rel="canonical" href="{SITE_URL}/gazette/{data.leaf.slug}" />
    <link rel="alternate" type="application/rss+xml" title={$t('gazetteRssTitle')} href="{SITE_URL}/gazette/feed.xml" />
    <meta property="og:site_name" content={$brandName} />
    <meta property="og:locale" content={ogLocale} />
    <meta property="og:type" content="article" />
    <meta property="og:title" content="{copy?.title ?? $t('gazettePageTitle')} — {$brandName}" />
    <meta property="og:description" content={copy?.dek || $t('gazettePageSubtitle')} />
    <meta property="og:url" content="{SITE_URL}/gazette/{data.leaf.slug}" />
    {#if cover}
      <meta property="og:image" content={cover} />
    {:else}
      <meta property="og:image" content="{SITE_URL}/images/cabinet-bg.jpeg" />
    {/if}
    {#if jsonLd}{@html `<script type="application/ld+json">${jsonLd}<\/script>`}{/if}
  {/if}
</svelte:head>

{#if data.mode === 'year' && data.room}
  <GazetteRoom
    year={data.room.year}
    years={data.room.years}
    leaves={data.room.leaves}
    cuttings={data.room.cuttings}
  />
{:else if data.loadError}
  <NotFound
    title={$t('loadErrorTitle')}
    message={$t('gazetteLoadError')}
    backHref="/gazette"
    backLabel={$t('gazetteBackLeaves')}
  />
{:else if !data.leaf}
  <NotFound
    backHref="/gazette"
    backLabel={$t('gazetteBackLeaves')}
  />
{:else}
  <div class="root">
    <div class="grain" aria-hidden="true"></div>
    <article class="page">
      <nav class="back-nav" in:fade={{ duration: 600 }}>
        <a href="/gazette" class="back-link">{$t('gazetteBackLeaves')}</a>
      </nav>

      <header class="masthead" in:fly={{ x: -20, duration: 900, delay: 80, easing: cubicOut }}>
        <p class="eyebrow">
          <span class="eyebrow-rule"></span>
          {$t(GAZETTE_KIND_KEY[data.leaf.kind])}
          {#if date}<span class="date">{date}</span>{/if}
        </p>
        <h1 class="title">{copy?.title}</h1>
        {#if copy?.dek}<p class="dek">{copy.dek}</p>{/if}
      </header>

      {#if data.leaf.kind === 'sketch'}
        <div class="watch" in:fade={{ duration: 500, delay: 140 }}>
          <GazetteWatchSeal leaf={data.leaf} showCopy />
        </div>
      {/if}

      {#if plates.length > 0}
        <div class="plates" class:many={plates.length > 1} in:fade={{ duration: 700, delay: 120 }}>
          {#each plates as src, i (src)}
            <div class="plate" style="--tilt: {(i % 2 === 0 ? -1.2 : 0.9)}deg">
              <AppImage {src} alt="" class="plate-img" sizes={i === 0 ? '280px' : '160px'} />
            </div>
          {/each}
        </div>
      {/if}

      {#if copy?.body}
        <div class="body" in:fade={{ duration: 700, delay: 180 }}>
          {#each copy.body.split(/\n\n+/) as para}
            <p>{para}</p>
          {/each}
        </div>
      {/if}

      {#if work || outside}
        <footer class="cta" in:fade={{ duration: 500, delay: 220 }}>
          {#if work}
            <a href={work}>{$t('gazetteOpenWork')} →</a>
          {:else if outside}
            <a href={outside} target="_blank" rel="noopener noreferrer">
              {data.leaf.sourceName || $t('gazetteLeavesHouse')} →
            </a>
          {/if}
        </footer>
      {/if}

      {#if data.leaf.prev || data.leaf.next}
        <nav class="neighbors" aria-label={$t('gazettePageKicker')} in:fade={{ duration: 500, delay: 240 }}>
          {#if data.leaf.prev}
            <a class="neighbor" href="/gazette/{data.leaf.prev.slug}">
              <span class="neighbor-kicker">{$t('gazetteLeafAbove')}</span>
              <span class="neighbor-title">{prevTitle}</span>
            </a>
          {:else}
            <span class="neighbor empty"></span>
          {/if}
          {#if data.leaf.next}
            <a class="neighbor next" href="/gazette/{data.leaf.next.slug}">
              <span class="neighbor-kicker">{$t('gazetteLeafBelow')}</span>
              <span class="neighbor-title">{nextTitle}</span>
            </a>
          {/if}
        </nav>
      {/if}
    </article>
  </div>
{/if}

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
    max-width: 720px;
    margin: 0 auto;
    padding: clamp(80px, 10vw, 140px) clamp(20px, 5vw, 64px) clamp(72px, 10vw, 120px);
    position: relative;
    z-index: 1;
  }

  .back-nav { margin-bottom: clamp(28px, 4vw, 48px); }

  .back-link {
    font-size: 9px;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: var(--muted2, #5f4636);
    text-decoration: none;
    transition: color 0.25s;
  }
  .back-link:hover { color: var(--brown, #34251c); }

  .masthead {
    padding-left: 18px;
    border-left: 2px solid var(--copper, #c65f3c);
    margin-bottom: clamp(28px, 4vw, 44px);
  }

  .eyebrow {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px 14px;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #6f3b24;
    margin: 0 0 14px;
  }
  .eyebrow-rule {
    width: 26px;
    height: 1px;
    background: var(--copper, #c65f3c);
    opacity: 0.65;
  }
  .date { color: #8a6a55; font-weight: 500; }

  .title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(36px, 5.2vw, 56px);
    font-weight: 300;
    line-height: 1.05;
    letter-spacing: -0.015em;
    color: #34251c;
    margin: 0 0 14px;
  }

  .dek {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(18px, 2vw, 22px);
    font-weight: 300;
    font-style: italic;
    line-height: 1.45;
    color: #5f4636;
    margin: 0;
  }

  .watch { margin: 0 0 clamp(24px, 3vw, 36px); }

  .plates {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin: 0 0 clamp(28px, 4vw, 40px);
    align-items: flex-end;
  }
  .plate {
    width: min(280px, 100%);
    margin: 0;
    border: 1px solid #d8c6b1;
    background: #1a120e;
    overflow: hidden;
    transform: rotate(var(--tilt, 0deg));
    transform-origin: 20% 0;
  }
  .plates.many .plate:first-child {
    width: min(280px, 100%);
  }
  .plates.many .plate:not(:first-child) {
    width: min(160px, 42%);
  }
  .plate :global(.app-image-wrap),
  .plate :global(img) {
    width: 100%;
    display: block;
  }

  .body {
    font-size: 17.5px;
    line-height: 1.72;
    color: #34251c;
    max-width: 38em;
  }
  .body p {
    margin: 0 0 1.1em;
    white-space: pre-wrap;
  }

  .cta { margin-top: 36px; }
  .cta a {
    font-size: 13px;
    letter-spacing: 0.04em;
    color: #6f3b24;
    text-decoration: none;
    border-bottom: 1px solid #d8c6b1;
    transition: border-color 0.2s ease, color 0.2s ease;
  }
  .cta a:hover { border-bottom-color: #c65f3c; color: #34251c; }

  .neighbors {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 24px;
    margin-top: 48px;
    padding-top: 22px;
    border-top: 1px solid rgba(216, 198, 177, 0.75);
  }
  .neighbor {
    display: grid;
    gap: 6px;
    text-decoration: none;
    color: inherit;
    min-width: 0;
  }
  .neighbor.next { text-align: right; }
  .neighbor.empty { visibility: hidden; }
  .neighbor-kicker {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
  }
  .neighbor-title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 18px;
    line-height: 1.25;
    color: #34251c;
  }
  .neighbor:hover .neighbor-title { color: #6f3b24; }

  @media (max-width: 560px) {
    .neighbors { grid-template-columns: 1fr; }
    .neighbor.next { text-align: left; }
  }
</style>
