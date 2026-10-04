<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { t, lang, brandName } from '$lib/i18n';
  import { resolveLargestImageUrl } from '$lib/api';
  import { SITE_URL, toAbsoluteUrl } from '$lib/site';
  import { jsonLdSafe } from '$lib/jsonld';
  import { leafCopy, leafCoverUrl, leafHref } from '$lib/gazette';
  import { leadTale, minutesKey, readingMinutes, taleMorphNames } from '$lib/tales';
  import { readTales } from '$lib/talesRead';
  import { api } from '$lib/api';
  import AppImage from '$lib/components/AppImage.svelte';
  import ArchClip from '$lib/components/ArchClip.svelte';
  import NotFound from '$lib/components/NotFound.svelte';
  import TalePoll from '$lib/components/TalePoll.svelte';
  import TaleSeal from '$lib/components/TaleSeal.svelte';

  let { data } = $props();

  let tales = $derived(data.tales ?? []);
  let lead = $derived(leadTale(tales));
  // The arcade in the order it is walked: the keeper's pin first, then the
  // shelf as the keeper arranged it. One list, not a lead plus a remainder —
  // every arch is the same arch.
  let arcade = $derived(lead ? [lead, ...tales.filter((tale) => tale.id !== lead.id)] : tales);
  // Names computed for the whole shelf at once, because uniqueness is a
  // property of the page and not of any one tale. See taleMorphNames.
  let morphs = $derived(taleMorphNames(arcade));
  let leadCover = $derived(lead ? leafCoverUrl(lead) : '');

  // Picked after the page is alive, never during load: this room prerenders,
  // so a build-time pick would freeze one tale as "random" forever.
  let randomHref = $state('');
  /**
   * Прочитанное — печать на арке. Читается после монтирования: полка
   * пререндерится, а что прочёл этот читатель, знает только его браузер.
   */
  let read = $state<Set<string>>(new Set());
  let letters = $state(false);
  onMount(() => {
    read = readTales();
    void api.getTaleDoors().then((doors) => (letters = doors.letters));
    if (arcade.length < 2) return;
    randomHref = leafHref(arcade[Math.floor(Math.random() * arcade.length)], 'tales_random');
  });

  // The arches are ordinary links, so Tab already walks them. The arrows are
  // for the reader who is already standing in the arcade — and, like the
  // gallery's, they are not advertised anywhere.
  function walkArcade(event: KeyboardEvent & { currentTarget: HTMLElement }) {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
    const arches = [...event.currentTarget.querySelectorAll<HTMLAnchorElement>('.arch')];
    const at = arches.indexOf(document.activeElement as HTMLAnchorElement);
    if (at < 0) return;
    const to = arches[at + (event.key === 'ArrowRight' ? 1 : -1)];
    if (!to) return;
    event.preventDefault();
    to.focus();
  }

  let ogLocale = $derived($lang === 'ru' ? 'ru_RU' : 'en_US');
  let pageTitle = $derived(`${$t('talesPageTitle')} — ${$brandName}`);
  // The shelf shares as its own lead photograph when it has one — a room whose
  // share card is the house's generic background looks like every other room.
  let ogImage = $derived(
    toAbsoluteUrl(resolveLargestImageUrl(leadCover) ?? leadCover ?? null) ??
      `${SITE_URL}/images/cabinet-bg.jpeg`,
  );

  // The shelf in machine-readable form. A CollectionPage that names no parts
  // says only that a collection exists; the ItemList is what lets a crawler or
  // an agent read the shelf itself — every tale, in the keeper's own order,
  // with its title, its dek and its address — without running the page's JS.
  // Clean addresses, deliberately: the visible links carry a `?src=` mark for
  // the ledger, and that mark is not part of any tale's identity.
  let jsonLd = $derived(jsonLdSafe({
    '@context': 'https://schema.org',
    '@type': 'CollectionPage',
    name: $t('talesPageTitle'),
    description: $t('talesPageRule'),
    url: `${SITE_URL}/tales`,
    inLanguage: $lang === 'ru' ? 'ru' : 'en',
    isPartOf: { '@type': 'WebSite', name: $brandName, url: SITE_URL },
    mainEntity: {
      '@type': 'ItemList',
      name: $t('talesPageTitle'),
      numberOfItems: arcade.length,
      itemListOrder: 'https://schema.org/ItemListOrderAscending',
      itemListElement: arcade.map((tale, i) => {
        const copy = leafCopy(tale, $lang);
        const raw = leafCoverUrl(tale);
        const cover = toAbsoluteUrl(resolveLargestImageUrl(raw) ?? raw ?? null);
        return {
          '@type': 'ListItem',
          position: i + 1,
          url: `${SITE_URL}/tales/${tale.slug}`,
          item: {
            '@type': 'Article',
            '@id': `${SITE_URL}/tales/${tale.slug}`,
            headline: copy.title,
            url: `${SITE_URL}/tales/${tale.slug}`,
            ...(copy.dek ? { description: copy.dek } : {}),
            ...(cover ? { image: cover } : {}),
            datePublished: tale.publishedAt ?? tale.createdAt,
          },
        };
      }),
    },
  }));

  // Home › Tall tales. Google renders the trail in place of the bare URL, and
  // it is the cheapest way to say where this room stands in the house.
  let breadcrumbJsonLd = $derived(jsonLdSafe({
    '@context': 'https://schema.org',
    '@type': 'BreadcrumbList',
    itemListElement: [
      { '@type': 'ListItem', position: 1, name: $brandName, item: SITE_URL },
      { '@type': 'ListItem', position: 2, name: $t('talesPageTitle'), item: `${SITE_URL}/tales` },
    ],
  }));
</script>

<svelte:head>
  <title>{pageTitle}</title>
  <meta name="description" content={$t('talesPageRule')} />
  <!-- canonical is emitted once, globally, in +layout.svelte — a second one here
       (the same URL, printed twice) is a conflicting signal to look at, not a
       stronger one. -->
  <link
    rel="alternate"
    type="application/rss+xml"
    title={$t('gazetteRssTitle')}
    href="{SITE_URL}/gazette/feed.xml"
  />
  <meta property="og:site_name" content={$brandName} />
  <meta property="og:locale" content={ogLocale} />
  <meta property="og:type" content="website" />
  <meta property="og:title" content={pageTitle} />
  <meta property="og:description" content={$t('talesPageRule')} />
  <meta property="og:url" content="{SITE_URL}/tales" />
  <meta property="og:image" content={ogImage} />
  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:title" content={pageTitle} />
  <meta name="twitter:description" content={$t('talesPageRule')} />
  <meta name="twitter:image" content={ogImage} />
  {@html `<script type="application/ld+json">${jsonLd}<\/script>`}
  {@html `<script type="application/ld+json">${breadcrumbJsonLd}<\/script>`}
</svelte:head>

{#if data.loadError}
  <NotFound
    title={$t('loadErrorTitle')}
    message={$t('talesLoadError')}
    backHref="/"
    backLabel={$t('talesBack')}
  />
{:else}
<div class="root">
  <div class="grain" aria-hidden="true"></div>

  <ArchClip />

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

    {#if !arcade.length}
      <p class="empty" in:fade={{ duration: 700, delay: 160 }}>{$t('talesEmpty')}</p>
    {:else}
      <!-- No entrance transition on the arcade, deliberately: the arch a
           visitor arrives through is the one that just travelled here from the
           tale, and a fade starting at zero would hide the very thing the
           morph is moving. -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <ul class="arcade" onkeydown={walkArcade}>
        {#each arcade as tale (tale.id)}
          {@const copy = leafCopy(tale, $lang)}
          {@const cover = leafCoverUrl(tale)}
          {@const mins = readingMinutes(copy.body)}
          {@const morph = morphs.get(tale.id)}
          <li>
            <a class="arch" href={leafHref(tale, 'tales_arcade')}>
              <span
                class="arch-frame"
                style={morph ? `view-transition-name: ${morph}` : undefined}
              >
                {#if cover}
                  <!-- The work in the photograph, not the tale's title: the
                       title is already the link's own text, and repeating it
                       here would say the same thing twice to a screen reader
                       while telling image search nothing about what is pictured. -->
                  <AppImage
                    src={cover}
                    alt={tale.figurineName ?? ''}
                    class="arch-img"
                    sizes="(max-width: 480px) 88vw, (max-width: 860px) 44vw, 320px"
                  />
                {:else}
                  <span class="arch-empty" aria-hidden="true">✦</span>
                {/if}
              </span>
              <span class="arch-plinth">
                <!-- Печать на цоколе, а не в арке: арка вырезана из
                     фотографии, и всё, что лежит внутри, обрезается по ней. -->
                {#if read.has(tale.id)}
                  <span class="arch-seal"><TaleSeal size={30} label={$t('talesReadSeal')} /></span>
                {/if}
                <span class="arch-title">{copy.title}</span>
                {#if copy.dek}<span class="arch-dek">{copy.dek}</span>{/if}
                {#if mins}<span class="arch-meta">{mins}&nbsp;{$t(minutesKey(mins))}</span>{/if}
              </span>
            </a>
          </li>
        {/each}
      </ul>

      {#if randomHref}
        <p class="chance" in:fade={{ duration: 600 }}>
          <a href={randomHref}>{$t('talesRandom')}</a>
        </p>
      {/if}
    {/if}

    <!-- Голосование «о ком следующая» стоит и под пустой полкой: выбрать, с
         кого начать, — тоже выбор. -->
    <TalePoll place="shelf" {letters} />
  </div>
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

  .masthead { max-width: 38em; margin-bottom: clamp(44px, 6vw, 76px); }

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

  /* ── The arcade ───────────────────────────────────────────────────────────
     Every span the same, because that is what makes a row of arches an arcade.
     Which tale stands first is the keeper's pin; nothing else ranks them. */

  .arcade {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: clamp(30px, 4vw, 54px) clamp(18px, 2.4vw, 34px);
    margin: 0 0 clamp(44px, 6vw, 72px);
    padding: 0;
    list-style: none;
  }
  @media (max-width: 860px) { .arcade { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
  @media (max-width: 480px) { .arcade { grid-template-columns: 1fr; } }

  .arch {
    display: block;
    text-decoration: none;
    color: inherit;
  }
  .arch:focus-visible { outline: 2px solid rgba(198, 95, 60, 0.6); outline-offset: 6px; }

  /* The shape is cut out of the photograph rather than drawn over it: a frame
     laid on top would have to be redrawn for every crop, and would sit above
     the picture during the morph instead of travelling with it. */
  .arch-frame {
    display: flex;
    align-items: center;
    justify-content: center;
    aspect-ratio: 3 / 4;
    overflow: hidden;
    background: #1a120e;
    clip-path: url(#tale-arch);
    transition: transform 0.4s cubic-bezier(0.2, 0.8, 0.3, 1);
  }
  .arch-frame :global(.app-image-wrap),
  .arch-frame :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .arch-frame :global(img) {
    filter: sepia(0.28) contrast(0.95) brightness(0.94);
    transition: filter 0.45s ease, transform 0.6s cubic-bezier(0.2, 0.8, 0.3, 1);
  }
  .arch:hover .arch-frame { transform: translateY(-6px); }
  .arch:hover .arch-frame :global(img),
  .arch:focus-visible .arch-frame :global(img) { filter: none; transform: scale(1.04); }

  .arch-empty {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 30px;
    color: rgba(216, 198, 177, 0.5);
  }

  /* The plinth the arch stands on. Heavier than a hairline on purpose — a
     1px rule under a cut shape reads as the edge of the photograph. */
  .arch-plinth {
    position: relative;
    display: block;
    margin-top: 13px;
    padding-top: 10px;
    border-top: 2px solid var(--brown, #6f3b24);
  }

  /* Печать лежит на черте цоколя, как сургуч на краю листа, — чуть набок. */
  .arch-seal {
    position: absolute;
    top: -16px;
    right: 8px;
    transform: rotate(-9deg);
  }
  .arch-plinth:has(.arch-seal) .arch-title { padding-right: 34px; }

  .arch-title {
    display: block;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(20px, 2.1vw, 25px);
    font-weight: 400;
    line-height: 1.12;
    letter-spacing: -0.008em;
    text-wrap: balance;
    color: var(--ink, #34251c);
    margin-bottom: 6px;
    transition: color 0.25s ease;
  }
  .arch:hover .arch-title { color: var(--copper, #c65f3c); }

  /* Two lines, and the reason is the whole point of the change: a title alone
     does not let anyone decide whether to read. */
  .arch-dek {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-size: 13.5px;
    line-height: 1.5;
    color: var(--muted, #5f4636);
    margin-bottom: 9px;
  }

  .arch-meta {
    display: block;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--copper, #c65f3c);
    font-variant-numeric: tabular-nums;
  }

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

  /* The arches take their place as they come into view. `both` is what keeps
     this honest: an arch already on screen when the page loads is past the
     range and renders at its end state, so nothing waits on a scroll that may
     never happen. */
  @supports (animation-timeline: view()) {
    @media (prefers-reduced-motion: no-preference) {
      .arcade li {
        animation: arch-rise linear both;
        animation-timeline: view();
        animation-range: entry 6% cover 30%;
      }
    }
  }
  @keyframes arch-rise {
    from { opacity: 0.55; transform: translateY(20px); }
    to { opacity: 1; transform: none; }
  }

  @media (prefers-reduced-motion: reduce) {
    .arcade li { opacity: 1; transform: none; }
    .arch-frame,
    .arch-frame :global(img) { transition: filter 0.3s ease; }
    .arch:hover .arch-frame { transform: none; }
    .arch:hover .arch-frame :global(img) { transform: none; }
  }
</style>
