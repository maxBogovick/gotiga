<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { t, lang, brandName } from '$lib/i18n';
  import { SITE_URL } from '$lib/site';
  import { jsonLdSafe } from '$lib/jsonld';
  import { GAZETTE_KIND_KEY, leafCopy, quietDate, workHref } from '$lib/gazette';
  import NotFound from '$lib/components/NotFound.svelte';
  import AppImage from '$lib/components/AppImage.svelte';

  let { data } = $props();
  let copy = $derived(data.leaf ? leafCopy(data.leaf, $lang) : null);
  let work = $derived(data.leaf ? workHref(data.leaf, 'gazette_leaf') : null);
  let date = $derived(
    data.leaf ? quietDate(data.leaf.publishedAt ?? data.leaf.createdAt, $lang) : '',
  );
  let outside = $derived(data.leaf?.href?.startsWith('http') ? data.leaf.href : null);

  let jsonLd = $derived(
    data.leaf && copy
      ? jsonLdSafe({
          '@context': 'https://schema.org',
          '@type': 'Article',
          headline: copy.title,
          description: copy.dek || $t('gazettePageSubtitle'),
          url: `${SITE_URL}/gazette/${data.leaf.slug}`,
          datePublished: data.leaf.publishedAt ?? data.leaf.createdAt,
          isPartOf: { '@type': 'WebSite', name: $brandName, url: SITE_URL },
        })
      : '',
  );
</script>

<svelte:head>
  <title>{copy?.title ?? $t('gazettePageTitle')} — {$brandName}</title>
  <meta name="description" content={copy?.dek || $t('gazettePageSubtitle')} />
  <meta property="og:site_name" content={$brandName} />
  <meta property="og:locale" content="en_US" />
  <meta property="og:type" content="article" />
  <meta property="og:title" content="{copy?.title ?? $t('gazettePageTitle')} — {$brandName}" />
  <meta property="og:description" content={copy?.dek || $t('gazettePageSubtitle')} />
  <meta property="og:url" content="{SITE_URL}/gazette/{data.leaf?.slug ?? ''}" />
  {#if data.leaf?.imageUrl}
    <meta property="og:image" content={data.leaf.imageUrl} />
  {:else}
    <meta property="og:image" content="{SITE_URL}/images/cabinet-bg.jpeg" />
  {/if}
  {#if jsonLd}{@html `<script type="application/ld+json">${jsonLd}<\/script>`}{/if}
</svelte:head>

{#if !data.leaf}
  <NotFound />
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

      {#if data.leaf.imageUrl}
        <div class="plate" in:fade={{ duration: 700, delay: 120 }}>
          <AppImage src={data.leaf.imageUrl} alt="" class="plate-img" sizes="280px" />
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

  .plate {
    width: min(280px, 100%);
    margin: 0 0 clamp(28px, 4vw, 40px);
    border: 1px solid #d8c6b1;
    background: #1a120e;
    overflow: hidden;
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
</style>
