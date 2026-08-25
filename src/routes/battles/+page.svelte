<script lang="ts">
  // Скромные эпические битвы — the shelf of cards.
  //
  // Not a shop window, whatever the presence of prices might suggest. The room
  // is a shelf of the house's own cards with what each would cost written on the
  // shelf beneath it, the way a museum writes a card for a case. Nothing here
  // can be bought with money, and nothing is decided by chance.
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { t, lang, brandName } from '$lib/i18n';
  import { SITE_URL } from '$lib/site';
  import { jsonLdSafe } from '$lib/jsonld';
  import { cardCopy, pricesOf, workHref } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';

  let { data } = $props();

  let cards = $derived(data.cards ?? []);
  let frames = $derived(data.frames ?? []);

  let ogLocale = $derived($lang === 'ru' ? 'ru_RU' : 'en_US');
  let jsonLd = $derived(jsonLdSafe({
    '@context': 'https://schema.org',
    '@type': 'CollectionPage',
    name: $t('battlesPageTitle'),
    description: $t('battlesPageRule'),
    url: `${SITE_URL}/battles`,
    inLanguage: $lang === 'ru' ? 'ru' : 'en',
    isPartOf: { '@type': 'WebSite', name: $brandName, url: SITE_URL },
  }));
</script>

<svelte:head>
  <title>{$t('battlesPageTitle')} — {$brandName}</title>
  <meta name="description" content={$t('battlesPageRule')} />
  <link rel="canonical" href="{SITE_URL}/battles" />
  <meta property="og:site_name" content={$brandName} />
  <meta property="og:locale" content={ogLocale} />
  <meta property="og:type" content="website" />
  <meta property="og:title" content="{$t('battlesPageTitle')} — {$brandName}" />
  <meta property="og:description" content={$t('battlesPageRule')} />
  <meta property="og:url" content="{SITE_URL}/battles" />
  <meta property="og:image" content="{SITE_URL}/images/cabinet-bg.jpeg" />
  {@html `<script type="application/ld+json">${jsonLd}<\/script>`}
</svelte:head>

<div class="root">
  <div class="grain" aria-hidden="true"></div>
  <div class="page">
    <nav class="back-nav" in:fade={{ duration: 600 }}>
      <a href="/" class="back-link">{$t('battlesBack')}</a>
    </nav>

    <header class="masthead" in:fly={{ x: -20, duration: 900, delay: 80, easing: cubicOut }}>
      <p class="eyebrow">
        <span class="eyebrow-rule"></span>
        {$t('battlesPageKicker')}
      </p>
      <h1 class="page-title">{$t('battlesPageTitle')}</h1>
      <p class="page-rule">{$t('battlesPageRule')}</p>
    </header>

    <!-- The two coins, named before any price is shown. A price in a currency
         nobody explained is the oldest trick in the free-to-play book, and the
         explanation is one sentence long. -->
    <aside class="coins" in:fade={{ duration: 700, delay: 200 }}>
      <p class="coins-line">
        <span class="coin coin--dust"></span>{$t('battlesCoinDust')}
        <span class="coin-sep">·</span>
        <span class="coin coin--feed"></span>{$t('battlesCoinFeed')}
      </p>
      <p class="coins-note">{$t('battlesCoinsNote')}</p>
    </aside>

    {#if !cards.length}
      <p class="empty" in:fade={{ duration: 700, delay: 160 }}>{$t('battlesEmpty')}</p>
    {:else}
      <div class="shelf" in:fade={{ duration: 700, delay: 240 }}>
        {#each cards as card, index (card.id)}
          {@const copy = cardCopy(card, $lang)}
          {@const prices = pricesOf(card)}
          {@const href = workHref(card)}
          <figure class="stand">
            <BattleCard {card} {frames} owned={true} />
            <!-- The price belongs to the shelf, not to the card: a card is a
                 thing the house made, a price is a note pinned under it. -->
            <figcaption class="label">
              <span class="label-prices">
                {#each prices as price (price.coin)}
                  <span class="label-price">
                    <span class="label-amount">{price.amount}</span>
                    <span class="label-coin">
                      {price.coin === 'dust' ? $t('battlesCoinDust') : $t('battlesCoinFeed')}
                    </span>
                  </span>
                {/each}
              </span>
              {#if href}
                <a class="label-work" {href}>{card.figurineName || copy.title}</a>
              {/if}
            </figcaption>
          </figure>
          {#if index === 0}
            <!-- Said once, at the top of the shelf, and never repeated per card. -->
            <p class="not-yet">{$t('battlesNotYetTakeable')}</p>
          {/if}
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .root {
    position: relative;
    min-height: 100vh;
    background: #f8f1e7;
    color: #34251c;
  }

  .grain {
    position: fixed;
    inset: 0;
    pointer-events: none;
    opacity: 0.05;
    background-image: radial-gradient(#34251c 0.5px, transparent 0.5px);
    background-size: 4px 4px;
  }

  .page {
    position: relative;
    max-width: 1180px;
    margin: 0 auto;
    padding: 3rem 1.5rem 6rem;
  }

  .back-nav {
    margin-bottom: 2.5rem;
  }

  .back-link {
    font-size: 0.72rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #6f3b24;
    text-decoration: none;
  }

  .back-link:hover {
    color: #c65f3c;
  }

  .eyebrow {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    margin: 0 0 1rem;
    font-size: 0.68rem;
    letter-spacing: 0.28em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .eyebrow-rule {
    width: 3.5rem;
    height: 1px;
    background: #d8c6b1;
  }

  .page-title {
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: clamp(2rem, 5vw, 3.2rem);
    font-weight: 400;
    line-height: 1.1;
  }

  .page-rule {
    max-width: 42ch;
    margin: 1rem 0 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1rem;
    line-height: 1.6;
    opacity: 0.74;
  }

  .coins {
    margin: 2.5rem 0 0;
    padding-top: 1.4rem;
    border-top: 1px solid #d8c6b1;
  }

  .coins-line {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0;
    font-size: 0.72rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .coin {
    width: 0.55rem;
    height: 0.55rem;
    border-radius: 50%;
  }

  .coin--dust {
    background: #c3ad93;
  }

  .coin--feed {
    background: #6f3b24;
  }

  .coin-sep {
    opacity: 0.4;
  }

  .coins-note {
    max-width: 56ch;
    margin: 0.7rem 0 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.9rem;
    line-height: 1.6;
    opacity: 0.66;
  }

  .empty {
    margin: 4rem 0 0;
    font-family: Georgia, 'Fraunces', serif;
    font-style: italic;
    opacity: 0.6;
  }

  .not-yet {
    grid-column: 1 / -1;
    order: -1;
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.88rem;
    font-style: italic;
    opacity: 0.6;
  }

  .shelf {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 2.6rem 1.8rem;
    margin-top: 3rem;
  }

  .stand {
    /* A long shelf skips the layout and paint of what is not on screen. Cheaper
       than virtualising it in JS, and it does not break find-in-page. */
    content-visibility: auto;
    contain-intrinsic-size: auto 320px;
    margin: 0;
  }

  .label {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-top: 0.9rem;
    text-align: center;
  }

  .label-prices {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: center;
    gap: 0.2rem 0.9rem;
    font-size: 0.66rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  /* One price is one thing: it may drop to its own line, never split across two. */
  .label-price {
    display: inline-flex;
    align-items: baseline;
    gap: 0.35rem;
    white-space: nowrap;
  }

  .label-amount {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.95rem;
    letter-spacing: 0;
  }

  .label-work {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.85rem;
    color: #34251c;
    opacity: 0.6;
    text-decoration: none;
    border-bottom: 1px solid transparent;
  }

  .label-work:hover {
    opacity: 1;
    border-bottom-color: #d8c6b1;
  }
</style>
