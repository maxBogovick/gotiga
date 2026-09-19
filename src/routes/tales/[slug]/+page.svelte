<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { api, resolveLargestImageUrl } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { t, lang, brandName } from '$lib/i18n';
  import { SITE_URL, toAbsoluteUrl } from '$lib/site';
  import { jsonLdSafe } from '$lib/jsonld';
  import { leafCopy, leafCoverUrl, neighborTitle, workHref } from '$lib/gazette';
  import { renderTale, taleMorphNames, ORNAMENT } from '$lib/tales';
  import type { GazetteLeaf } from '$lib/types/api';
  import AppImage from '$lib/components/AppImage.svelte';
  import ArchClip from '$lib/components/ArchClip.svelte';
  import NotFound from '$lib/components/NotFound.svelte';

  let { data } = $props();

  let copy = $derived(data.leaf ? leafCopy(data.leaf, $lang) : null);
  let blocks = $derived(renderTale(copy?.body));
  // The drop cap belongs to the first paragraph, which need not be the first
  // block — a tale may open on an ornament.
  let firstPara = $derived(blocks.findIndex((b) => b.kind === 'p'));
  let plate = $derived(data.leaf ? leafCoverUrl(data.leaf) : '');
  let work = $derived(data.leaf ? workHref(data.leaf, 'tale') : null);
  /**
   * The shelf, fetched by the page itself.
   *
   * `leaf.next` already names the next tale in shelf order — the server walks
   * tales by `shelf_order` and not by date — but it carries a slug and a title
   * and no photograph. The arch needs the photograph, so the shelf is read
   * here rather than widened on the server: the same thing HeroTalesPlate
   * does, and one request for six rows.
   *
   * Client-side only. This route prerenders, and a shelf baked in at build
   * time would freeze the neighbour's photograph into the HTML.
   */
  let shelf = $state<GazetteLeaf[]>([]);
  onMount(async () => {
    if (!data.leaf?.next) return;
    try {
      shelf = await api.getTales();
    } catch {
      // The invitation simply arrives without its arch. A tale that cannot
      // reach the shelf is still a tale.
      shelf = [];
    }
  });

  let nextLeaf = $derived(
    data.leaf?.next ? shelf.find((tale) => tale.slug === data.leaf?.next?.slug) ?? null : null,
  );
  let nextCover = $derived(nextLeaf ? leafCoverUrl(nextLeaf) : '');

  // Both photographs on this page are named at once, and for the same reason
  // the shelf names its whole arcade at once: two tales about one work carry
  // one name, and two identical `view-transition-name`s abort the transition
  // for the page. Here that pair is this tale and the next one.
  let morphs = $derived(
    data.leaf ? taleMorphNames(nextLeaf ? [data.leaf, nextLeaf] : [data.leaf]) : new Map<string, string>(),
  );
  let morph = $derived(data.leaf ? morphs.get(data.leaf.id) ?? '' : '');
  // Worn by the invitation's arch and by nothing else on the page: the small
  // arch at the foot of the thread shows the same photograph, and naming both
  // would be the duplicate this map exists to prevent.
  let nextMorph = $derived(nextLeaf ? morphs.get(nextLeaf.id) ?? '' : '');
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

  let taleUrl = $derived(data.leaf ? `${SITE_URL}/tales/${data.leaf.slug}` : SITE_URL);
  let pageTitle = $derived(`${copy?.title ?? $t('talesPageTitle')} — ${$brandName}`);
  let description = $derived(copy?.dek || $t('talesPageRule'));
  // `plate` is the stored path — `/static/images/…`. og:image and schema.org
  // image both require an absolute URL; a relative one is simply dropped, which
  // is how a tale with a photograph shares as a bare grey card.
  let plateAbsolute = $derived(
    toAbsoluteUrl(resolveLargestImageUrl(plate) ?? plate ?? null),
  );
  let ogImage = $derived(plateAbsolute ?? `${SITE_URL}/images/cabinet-bg.jpeg`);
  // The work the tale is about, as an entity rather than a link. This is what
  // ties the prose to the piece for anything reading the graph: the same URL the
  // figurine's own page declares as a VisualArtwork.
  let workUrl = $derived(
    data.leaf?.figurineId
      ? `${SITE_URL}/figurines/${data.leaf.figurineSlug || data.leaf.figurineId}`
      : null,
  );
  let wordCount = $derived(
    blocks.reduce((n, b) => (b.kind === 'p' ? n + b.text.split(/\s+/).filter(Boolean).length : n), 0),
  );

  let jsonLd = $derived(
    data.leaf && copy
      ? jsonLdSafe({
          '@context': 'https://schema.org',
          '@type': 'Article',
          headline: copy.title,
          description,
          url: taleUrl,
          mainEntityOfPage: { '@type': 'WebPage', '@id': taleUrl },
          datePublished: data.leaf.publishedAt ?? data.leaf.createdAt,
          dateModified: data.leaf.updatedAt ?? data.leaf.publishedAt ?? data.leaf.createdAt,
          inLanguage: $lang === 'ru' ? 'ru' : 'en',
          image: plateAbsolute ?? undefined,
          articleSection: $t('talesPageTitle'),
          ...(wordCount ? { wordCount } : {}),
          author: { '@type': 'Organization', name: $brandName, url: SITE_URL },
          publisher: { '@type': 'Organization', name: $brandName, url: SITE_URL },
          isPartOf: { '@type': 'WebSite', name: $brandName, url: SITE_URL },
          ...(workUrl && data.leaf.figurineName
            ? {
                about: {
                  '@type': 'VisualArtwork',
                  '@id': workUrl,
                  name: data.leaf.figurineName,
                  url: workUrl,
                },
              }
            : {}),
        })
      : '',
  );

  // Home › Tall tales › this tale.
  let breadcrumbJsonLd = $derived(
    data.leaf && copy
      ? jsonLdSafe({
          '@context': 'https://schema.org',
          '@type': 'BreadcrumbList',
          itemListElement: [
            { '@type': 'ListItem', position: 1, name: $brandName, item: SITE_URL },
            { '@type': 'ListItem', position: 2, name: $t('talesPageTitle'), item: `${SITE_URL}/tales` },
            { '@type': 'ListItem', position: 3, name: copy.title, item: taleUrl },
          ],
        })
      : '',
  );
</script>

<svelte:head>
  {#if data.leaf}
    <title>{pageTitle}</title>
    <meta name="description" content={description} />
    <!-- canonical is emitted once, globally, in +layout.svelte. -->
    <link
      rel="alternate"
      type="application/rss+xml"
      title={$t('gazetteRssTitle')}
      href="{SITE_URL}/gazette/feed.xml"
    />
    <meta property="og:site_name" content={$brandName} />
    <meta property="og:locale" content={ogLocale} />
    <meta property="og:type" content="article" />
    <meta property="og:title" content={pageTitle} />
    <meta property="og:description" content={description} />
    <meta property="og:url" content={taleUrl} />
    <meta property="og:image" content={ogImage} />
    <meta
      property="article:published_time"
      content={data.leaf.publishedAt ?? data.leaf.createdAt}
    />
    <meta name="twitter:card" content="summary_large_image" />
    <meta name="twitter:title" content={pageTitle} />
    <meta name="twitter:description" content={description} />
    <meta name="twitter:image" content={ogImage} />
    {#if jsonLd}{@html `<script type="application/ld+json">${jsonLd}<\/script>`}{/if}
    {#if breadcrumbJsonLd}{@html `<script type="application/ld+json">${breadcrumbJsonLd}<\/script>`}{/if}
  {:else}
    <!-- Nothing on the shelf under this name. The address can still be reached —
         a stale link, a guessed slug — and nginx answers a miss with the SPA
         shell, so say plainly that this page is not to be kept. -->
    <title>{$t('talesPageTitle')} — {$brandName}</title>
    <meta name="robots" content="noindex, follow" />
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
    <ArchClip />
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
        {#if plate}
          <!-- Полка, натянутая вдоль небылицы: наверху арка, из которой сюда
               приехали, внизу — следующая по полке. Обе без имени перехода:
               ту, что наверху, уже носит плита в поле, а ту, что внизу, —
               приглашение под текстом, и второе такое же имя прервало бы
               переход всей страницы. -->
          <aside class="spine" style="--span: {blocks.length}" aria-hidden="true">
            <span class="spine-arch spine-arch--here">
              <AppImage src={plate} alt="" class="spine-img" sizes="26px" />
            </span>
            <span class="spine-thread"><span class="spine-fill"></span></span>
            {#if nextCover}
              <span class="spine-arch spine-arch--next">
                <AppImage src={nextCover} alt="" class="spine-img" sizes="26px" />
              </span>
            {/if}
          </aside>
        {/if}
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
                <AppImage src={plate} alt={data.leaf.figurineName ?? ''} class="margin-img" sizes="168px" />
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
              <AppImage src={plate} alt={data.leaf.figurineName ?? ''} class="margin-img" sizes="168px" />
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

      {#if data.leaf.next}
        <a
          class="onward"
          href="/tales/{data.leaf.next.slug}"
          in:fade={{ duration: 500, delay: 240 }}
        >
          {#if nextCover}
            <span
              class="onward-arch"
              style={nextMorph ? `view-transition-name: ${nextMorph}` : undefined}
            >
              <AppImage src={nextCover} alt="" class="onward-img" sizes="92px" />
            </span>
          {/if}
          <span class="onward-copy">
            <span class="onward-kicker">{$t('talesNextOnShelf')}</span>
            <span class="onward-title">{neighborTitle(data.leaf.next, $lang)}</span>
          </span>
        </a>
      {/if}

      {#if data.leaf.prev}
        <nav class="neighbors" aria-label={$t('talesNearby')} in:fade={{ duration: 500, delay: 260 }}>
          <a class="neighbor" href="/tales/{data.leaf.prev.slug}">
            <span class="neighbor-kicker">{$t('talesLeft')}</span>
            <span class="neighbor-title">{neighborTitle(data.leaf.prev, $lang)}</span>
          </a>
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
    --spine: 26px;
    --spine-gap: clamp(22px, 3.4vw, 46px);
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
    grid-template-columns: var(--spine) minmax(0, 1fr) 168px;
    column-gap: var(--spine-gap);
    align-items: start;
    /* The thread reads its progress from the leaf itself, not from the page:
       the masthead above it and the neighbours below it are not the tale, and
       a thread that fills while you scroll past them would be measuring the
       wrong thing. */
    view-timeline-name: --tale;
    view-timeline-axis: block;
  }

  .para,
  .ornament { grid-column: 2; }

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
    grid-column: 3;
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
    margin: clamp(28px, 4vw, 48px) 0 0 calc(var(--spine) + var(--spine-gap));
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
    margin: clamp(48px, 7vw, 88px) 0 0 calc(var(--spine) + var(--spine-gap));
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

  /* ── The thread, and the shelf at both ends of it ──────────────────────
     The arch you arrived through stands at the top, the next tale on the
     shelf at the foot. The thread between them fills as the leaf passes
     through the window, and the lower arch — grey while you read — wakes over
     the last fifth. Nothing here is a control: it is the shelf, held along
     the tale you are reading. */

  .spine {
    grid-column: 1;
    /* Same reason the margin plate spans explicitly: `1 / -1` names the last
       line of the EXPLICIT grid, and every row here is implicit. */
    grid-row: 1 / span var(--span, 1);
    align-self: stretch;
    position: relative;
    justify-self: center;
    width: var(--spine);
  }

  .spine-arch {
    position: absolute;
    left: 0;
    display: block;
    width: 26px;
    aspect-ratio: 3 / 4;
    overflow: hidden;
    background: #1a120e;
    clip-path: url(#tale-arch);
  }
  .spine-arch--here { top: 0; }
  .spine-arch--next { bottom: 0; filter: grayscale(1) brightness(0.82); }
  .spine-arch :global(.app-image-wrap),
  .spine-arch :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .spine-thread {
    position: absolute;
    left: 50%;
    top: 46px;
    bottom: 46px;
    width: 1px;
    margin-left: -0.5px;
    background: var(--line, #d8c6b1);
  }
  .spine-fill {
    position: absolute;
    inset: 0;
    background: var(--copper, #c65f3c);
    transform: scaleY(0);
    transform-origin: top;
  }

  @supports (animation-timeline: view()) {
    @media (prefers-reduced-motion: no-preference) {
      .spine-fill {
        animation: tale-thread linear both;
        animation-timeline: --tale;
        animation-range: entry 0% entry 100%;
      }
      .spine-arch--next {
        animation: tale-wake linear both;
        animation-timeline: --tale;
        animation-range: entry 80% entry 100%;
      }
    }
  }
  @keyframes tale-thread {
    from { transform: scaleY(0); }
    to { transform: scaleY(1); }
  }
  @keyframes tale-wake {
    from { filter: grayscale(1) brightness(0.82); }
    to { filter: none; }
  }

  /* ── The invitation ───────────────────────────────────────────────────
     Printed where the tale ends, so finishing one and choosing the next is
     one movement. It is also the only place the next tale is named: the
     neighbours strip below keeps the previous one and nothing else, because
     the same tale printed twice on one page is two answers to one question. */

  .onward {
    display: flex;
    align-items: center;
    gap: clamp(16px, 2.4vw, 26px);
    margin: clamp(40px, 6vw, 72px) 0 0 calc(var(--spine) + var(--spine-gap));
    padding-top: 24px;
    border-top: 1px solid var(--line, #d8c6b1);
    text-decoration: none;
    color: inherit;
  }
  .onward-arch {
    display: block;
    width: 92px;
    aspect-ratio: 3 / 4;
    flex-shrink: 0;
    overflow: hidden;
    background: #1a120e;
    clip-path: url(#tale-arch);
    transition: transform 0.35s cubic-bezier(0.2, 0.8, 0.3, 1);
  }
  .onward:hover .onward-arch,
  .onward:focus-visible .onward-arch { transform: translateY(-4px); }
  .onward-arch :global(.app-image-wrap),
  .onward-arch :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .onward-copy { min-width: 0; }
  .onward-kicker {
    display: block;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--copper, #c65f3c);
    margin-bottom: 7px;
  }
  .onward-title {
    display: block;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(24px, 3vw, 34px);
    font-weight: 400;
    line-height: 1.1;
    text-wrap: balance;
    color: var(--ink, #34251c);
    transition: color 0.25s;
  }
  .onward:hover .onward-title { color: var(--copper, #c65f3c); }

  @media (max-width: 1099px) {
    /* The thread is a wide-screen affordance: at this width the margin plate
       has already fallen into the prose, and 26 px of gutter buys nothing the
       invitation does not already say. */
    .page { --spine: 0px; --spine-gap: 0px; }
    .leaf { grid-template-columns: 1fr; column-gap: 0; }
    .spine { display: none; }
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
  }
</style>
