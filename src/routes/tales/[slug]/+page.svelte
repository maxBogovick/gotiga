<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { t, lang, brandName } from '$lib/i18n';
  import { SITE_URL } from '$lib/site';
  import { jsonLdSafe } from '$lib/jsonld';
  import { leafCopy, leafCoverUrl, neighborTitle, workHref } from '$lib/gazette';
  import { renderTale, ORNAMENT } from '$lib/tales';
  import AppImage from '$lib/components/AppImage.svelte';
  import NotFound from '$lib/components/NotFound.svelte';

  let { data } = $props();

  let copy = $derived(data.leaf ? leafCopy(data.leaf, $lang) : null);
  let blocks = $derived(renderTale(copy?.body));
  // The drop cap belongs to the first paragraph, which need not be the first
  // block — a tale may open on an ornament.
  let firstPara = $derived(blocks.findIndex((b) => b.kind === 'p'));
  let plate = $derived(data.leaf ? leafCoverUrl(data.leaf) : '');
  let work = $derived(data.leaf ? workHref(data.leaf, 'tale') : null);
  // One photograph on the page, so the morph into the work has a unique name.
  let morph = $derived(data.leaf?.figurineId ? `figurine-${data.leaf.figurineId}` : '');
  let ogLocale = $derived($lang === 'ru' ? 'ru_RU' : 'en_US');

  /**
   * Пыль за прочитанную небылицу — тому, кто дочитал до конца, а не тому, кто
   * открыл. Конец засчитывается буквально: последняя строка попала на экран.
   *
   * Действие на самом низе текста, а не таймер: небылицы разной длины, и
   * секунды сказали бы про длинную то же, что про короткую.
   */
  function lastLine(node: HTMLElement) {
    const token = authStore.token;
    const id = data.leaf?.id;
    if (!token || !id) return;
    const watcher = new IntersectionObserver((entries) => {
      if (!entries.some((e) => e.isIntersecting)) return;
      watcher.disconnect();
      void api.grantBattleAttention(token, 'read', id);
    });
    watcher.observe(node);
    return { destroy: () => watcher.disconnect() };
  }

  let jsonLd = $derived(
    data.leaf && copy
      ? jsonLdSafe({
          '@context': 'https://schema.org',
          '@type': 'Article',
          headline: copy.title,
          description: copy.dek || $t('talesPageRule'),
          url: `${SITE_URL}/tales/${data.leaf.slug}`,
          datePublished: data.leaf.publishedAt ?? data.leaf.createdAt,
          inLanguage: $lang === 'ru' ? 'ru' : 'en',
          image: plate || undefined,
          author: { '@type': 'Organization', name: $brandName },
          isPartOf: { '@type': 'WebSite', name: $brandName, url: SITE_URL },
        })
      : '',
  );
</script>

<svelte:head>
  {#if data.leaf}
    <title>{copy?.title ?? $t('talesPageTitle')} — {$brandName}</title>
    <meta name="description" content={copy?.dek || $t('talesPageRule')} />
    <link rel="canonical" href="{SITE_URL}/tales/{data.leaf.slug}" />
    <meta property="og:site_name" content={$brandName} />
    <meta property="og:locale" content={ogLocale} />
    <meta property="og:type" content="article" />
    <meta property="og:title" content="{copy?.title ?? $t('talesPageTitle')} — {$brandName}" />
    <meta property="og:description" content={copy?.dek || $t('talesPageRule')} />
    <meta property="og:url" content="{SITE_URL}/tales/{data.leaf.slug}" />
    <meta property="og:image" content={plate || `${SITE_URL}/images/cabinet-bg.jpeg`} />
    {#if jsonLd}{@html `<script type="application/ld+json">${jsonLd}<\/script>`}{/if}
  {/if}
</svelte:head>

{#if data.loadError}
  <NotFound
    title={$t('loadErrorTitle')}
    message={$t('talesLoadError')}
    backHref="/tales"
    backLabel={$t('talesBackShelf')}
  />
{:else if !data.leaf || !copy}
  <NotFound backHref="/tales" backLabel={$t('talesBackShelf')} />
{:else}
  <div class="root">
    <div class="grain" aria-hidden="true"></div>
    <article class="page">
      <nav class="back-nav" in:fade={{ duration: 600 }}>
        <a href="/tales" class="back-link">{$t('talesBackShelf')}</a>
      </nav>

      <header class="masthead" in:fly={{ x: -20, duration: 900, delay: 80, easing: cubicOut }}>
        <p class="eyebrow">
          <span class="eyebrow-rule"></span>
          {$t('talesKicker')}
        </p>
        <h1 class="title">{copy.title}</h1>
        {#if copy.dek}<p class="epigraph">{copy.dek}</p>{/if}
      </header>

      <div class="leaf" in:fade={{ duration: 700, delay: 160 }}>
        {#each blocks as block, i}
          {#if block.kind === 'ornament'}
            <p class="ornament" aria-hidden="true">{ORNAMENT}</p>
          {:else}
            <p class="para" class:opening={i === firstPara}>{block.text}</p>
          {/if}

          {#if i === 0 && plate}
            <!-- Pinned in the margin on a wide screen, and dropped into the
                 prose right here once the margin is gone. -->
            <aside class="margin" style="--span: {blocks.length}">
              <svelte:element
                this={work ? 'a' : 'div'}
                class="margin-plate"
                href={work || undefined}
                style={morph ? `view-transition-name: ${morph}` : undefined}
              >
                <AppImage src={plate} alt="" class="margin-img" sizes="168px" />
              </svelte:element>
              {#if data.leaf.figurineName}
                <p class="margin-name">{data.leaf.figurineName}</p>
              {/if}
            </aside>
          {/if}
        {/each}

        {#if plate && blocks.length === 0}
          <aside class="margin">
            <svelte:element
              this={work ? 'a' : 'div'}
              class="margin-plate"
              href={work || undefined}
              style={morph ? `view-transition-name: ${morph}` : undefined}
            >
              <AppImage src={plate} alt="" class="margin-img" sizes="168px" />
            </svelte:element>
          </aside>
        {/if}
      </div>

      <!-- Дно текста. Ничего не показывает — только отмечает, что дочитано. -->
      <span class="bottom" use:lastLine aria-hidden="true"></span>

      {#if work}
        <footer class="stands" in:fade={{ duration: 500, delay: 220 }}>
          <a href={work}>{$t('talesWorkHere')} →</a>
        </footer>
      {/if}

      {#if data.leaf.prev || data.leaf.next}
        <nav class="neighbors" aria-label={$t('talesNearby')} in:fade={{ duration: 500, delay: 240 }}>
          {#if data.leaf.prev}
            <a class="neighbor" href="/tales/{data.leaf.prev.slug}">
              <span class="neighbor-kicker">{$t('talesLeft')}</span>
              <span class="neighbor-title">{neighborTitle(data.leaf.prev, $lang)}</span>
            </a>
          {:else}
            <span class="neighbor empty"></span>
          {/if}
          {#if data.leaf.next}
            <a class="neighbor next" href="/tales/{data.leaf.next.slug}">
              <span class="neighbor-kicker">{$t('talesRight')}</span>
              <span class="neighbor-title">{neighborTitle(data.leaf.next, $lang)}</span>
            </a>
          {/if}
        </nav>
      {/if}
    </article>
  </div>
{/if}

<style>
  /* Ни высоты, ни цвета: это отметка дна, а не элемент страницы. */
  .bottom {
    display: block;
    height: 1px;
  }

  .root {
    width: 100%;
    min-height: 100svh;
    background:
      radial-gradient(ellipse 70% 55% at 72% 38%, rgba(198, 95, 60, 0.06) 0%, transparent 65%),
      var(--cream, #f8f1e7);
    position: relative;
    /* `clip`, not `hidden`: hidden computes overflow-y to `auto`, which makes
       this element the scrollport the margin plate sticks to — and since it
       never scrolls itself, the plate would simply never stick. `clip` cuts
       the same overflow without creating a scroll container. */
    overflow-x: clip;
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
    max-width: 940px;
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

  .masthead { max-width: 30em; margin-bottom: clamp(34px, 5vw, 56px); }

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

  .title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(36px, 5.2vw, 68px);
    font-weight: 300;
    line-height: 1;
    letter-spacing: -0.012em;
    color: var(--ink, #34251c);
    margin: 0 0 16px;
  }

  .epigraph {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(17px, 1.8vw, 21px);
    font-weight: 300;
    font-style: italic;
    line-height: 1.5;
    color: var(--muted, #5f4636);
    margin: 0;
  }

  /* ── The tale ──────────────────────────────────────────────────────────────
     One grid: the prose walks down column one, the work stands in column two
     and stays there while you read. Below 1100px the second column is gone and
     the plate falls back into the prose exactly where it sits in the markup —
     after the opening paragraph. */

  .leaf {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 168px;
    column-gap: clamp(32px, 4.5vw, 64px);
    align-items: start;
  }

  .para,
  .ornament { grid-column: 1; }

  .para {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(18px, 1.9vw, 21px);
    font-weight: 400;
    line-height: 1.72;
    color: var(--ink, #34251c);
    max-width: 62ch;
    margin: 0 0 1.15em;
  }

  .para.opening::first-letter {
    float: left;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 3.5em;
    line-height: 0.82;
    padding: 0.06em 0.09em 0 0;
    color: var(--deep, #6f3b24);
  }

  .ornament {
    max-width: 62ch;
    margin: 0.5em 0 1.4em;
    text-align: center;
    font-size: 13px;
    letter-spacing: 0.5em;
    color: var(--copper, #c65f3c);
    opacity: 0.55;
  }

  .margin {
    grid-column: 2;
    /* Spans every row of prose, so the sticky plate has the whole tale to
       travel down. `1 / -1` cannot do this: -1 names the last line of the
       EXPLICIT grid, and these rows are all implicit — the plate would take
       one row and stretch it to its own height, tearing a hole after the
       opening paragraph. The exact count is known, so it is passed in. */
    grid-row: 1 / span var(--span, 1);
    align-self: start;
    position: sticky;
    top: 12vh;
  }

  .margin-plate {
    display: block;
    width: 168px;
    height: 210px;
    overflow: hidden;
    background: #1a120e;
    border: 1px solid var(--line, #d8c6b1);
    box-shadow: 0 2px 10px rgba(52, 37, 28, 0.14);
    transition: border-color 0.25s ease;
  }
  a.margin-plate:hover { border-color: rgba(198, 95, 60, 0.5); }
  .margin-plate :global(.app-image-wrap),
  .margin-plate :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .margin-name {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 14px;
    font-style: italic;
    color: var(--muted, #5f4636);
    margin: 10px 0 0;
  }

  .stands {
    margin: clamp(28px, 4vw, 48px) 0 0;
    max-width: 62ch;
  }
  .stands a {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--copper, #c65f3c);
    text-decoration: none;
    transition: color 0.25s;
  }
  .stands a:hover { color: var(--deep, #6f3b24); }

  .neighbors {
    display: flex;
    justify-content: space-between;
    gap: 24px;
    margin-top: clamp(48px, 7vw, 88px);
    padding-top: 22px;
    border-top: 1px solid rgba(52, 37, 28, 0.12);
  }

  .neighbor {
    display: grid;
    gap: 5px;
    max-width: 20em;
    text-decoration: none;
    color: inherit;
  }
  .neighbor.next { text-align: right; }
  .neighbor.empty { visibility: hidden; }

  .neighbor-kicker {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--muted2, #5f4636);
  }

  .neighbor-title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 19px;
    line-height: 1.2;
    color: var(--ink, #34251c);
    transition: color 0.25s;
  }
  .neighbor:hover .neighbor-title { color: var(--deep, #6f3b24); }

  @media (max-width: 1099px) {
    .leaf { grid-template-columns: 1fr; }
    .para,
    .ornament,
    .margin { grid-column: auto; }
    .margin {
      grid-row: auto;
      position: static;
      margin: 6px 0 1.6em;
    }
    .margin-plate { width: 100%; height: clamp(200px, 46vw, 300px); }
  }

  @media (max-width: 560px) {
    .neighbors { flex-direction: column; }
    .neighbor.next { text-align: left; }
    .neighbor.empty { display: none; }
  }
</style>
