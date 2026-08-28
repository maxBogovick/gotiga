<script lang="ts">
  // Скромные эпические битвы — the shelf of cards.
  //
  // Not a shop window, whatever the presence of prices might suggest. The room
  // is a shelf of the house's own cards with what each would cost written on the
  // shelf beneath it, the way a museum writes a card for a case. Nothing here
  // can be bought with money, and nothing is decided by chance.
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { t, lang, brandName } from '$lib/i18n';
  import { SITE_URL } from '$lib/site';
  import { jsonLdSafe } from '$lib/jsonld';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { cardCopy, pricesOf, workHref } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import BattleTaking from '$lib/components/BattleTaking.svelte';
  import type { BattleCard as BattleCardDto, BattleMe } from '$lib/types/api';

  let { data } = $props();

  let cards = $derived(data.cards ?? []);
  let frames = $derived(data.frames ?? []);

  // ── Кошелёк и владение ────────────────────────────────────────────────────
  //
  // Полка видна всем; своё видно только под именем. Гость видит цены целиком —
  // полка сама себе прейскурант, и отдельной витрины не нужно.
  let me = $state<BattleMe | null>(null);
  let signedIn = $derived(authStore.isLoggedIn);
  let busyCard = $state<string | null>(null);
  let complaint = $state<string | null>(null);
  /** Карта, которую сейчас принимают: церемония играет над полкой. */
  let taking = $state<BattleCardDto | null>(null);

  async function readWallet() {
    const token = authStore.token;
    if (!token) {
      me = null;
      return;
    }
    try {
      me = await api.getBattleMe(token);
    } catch {
      // Полка от этого не ломается: без книги она просто вся в пыли.
      me = null;
    }
  }

  onMount(readWallet);

  let ownedBy = $derived(new Map((me?.owned ?? []).map((o) => [o.cardId, o])));
  const holdingOf = (id: string) => ownedBy.get(id) ?? null;

  /** Хватает ли на эту цену. Ответ сервера всё равно главнее — здесь только
   *  то, что позволяет не предлагать несбыточное. */
  function affordable(coin: 'dust' | 'feed', amount: number): boolean {
    if (!me) return false;
    return (coin === 'dust' ? me.dust : me.feed) >= amount;
  }

  async function take(card: BattleCardDto, coin: 'dust' | 'feed', amount: number) {
    const token = authStore.token;
    if (!token || busyCard) return;
    busyCard = card.id;
    complaint = null;
    try {
      const res = await api.buyBattleCard(token, {
        cardId: card.id,
        currency: coin,
        expectedPrice: amount,
      });
      await readWallet();
      // Церемония играет однажды. Повтор — это уже своя карта, и праздновать
      // её второй раз значит праздновать двойной щелчок.
      if (res.takenNow) taking = card;
    } catch (e) {
      complaint = String(e).includes('price')
        ? $t('battlesPriceChanged')
        : $t('battlesTakeFailed');
      await readWallet();
    } finally {
      busyCard = null;
    }
  }

  /**
   * Цена следующей ступени — или `null`, если лестницы нет или она пройдена.
   *
   * Ступени всегда четыре: 1→2, 2→3, 3→4, 4→5. Уровень пятый — потолок, и
   * дальше не за что платить.
   */
  function nextRung(card: BattleCardDto, level: number): number | null {
    const ladder = card.levelPriceDust;
    if (!ladder || ladder.length !== 4 || level >= 5) return null;
    return ladder[level - 1] ?? null;
  }

  /** Карта, у которой только что зажглась засечка: блик проходит один раз. */
  let justRaised = $state<string | null>(null);

  async function raise(card: BattleCardDto, price: number) {
    const token = authStore.token;
    if (!token || busyCard) return;
    busyCard = card.id;
    complaint = null;
    try {
      const res = await api.raiseBattleCard(token, {
        cardId: card.id,
        expectedPrice: price,
      });
      await readWallet();
      // Засечка загорается один раз. Повтор — это уже поднятая ступень, и
      // праздновать её второй раз значит праздновать двойной щелчок.
      if (res.raisedNow) {
        justRaised = card.id;
        setTimeout(() => {
          if (justRaised === card.id) justRaised = null;
        }, 1600);
      }
    } catch (e) {
      complaint = String(e).includes('price')
        ? $t('battlesPriceChanged')
        : $t('battlesRaiseFailed');
      await readWallet();
    } finally {
      busyCard = null;
    }
  }

  /**
   * Пометка «новая» держится до первого взгляда — буквально: она снимается,
   * когда карта попала на экран, а не когда страница открылась. Карта, до
   * которой не долистали, остаётся новой.
   */
  function firstLook(node: HTMLElement, cardId: string) {
    let watcher: IntersectionObserver | null = null;
    const arm = (id: string) => {
      watcher?.disconnect();
      if (!holdingOf(id)?.isNew) return;
      watcher = new IntersectionObserver(
        (entries) => {
          if (!entries.some((e) => e.isIntersecting)) return;
          watcher?.disconnect();
          const token = authStore.token;
          if (token) void api.markBattleCardSeen(token, id).catch(() => {});
        },
        { threshold: 0.5 },
      );
      watcher.observe(node);
    };
    arm(cardId);
    return {
      update: (id: string) => arm(id),
      destroy: () => watcher?.disconnect(),
    };
  }

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
      <!-- Соседняя комната: те же карты, но расставленные и играющие. -->
      <p class="page-rule"><a href="/battles/etude" class="study-link">{$t('battleStudies')} →</a></p>
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
      <!-- Баланс — две мелкие отметки на полях, а не шапка магазина. -->
      {#if me}
        <p class="purse" in:fade={{ duration: 400 }}>
          {$t('battlesYourDust')} <span class="purse-num">{me.dust}</span>
          <span class="coin-sep">·</span>
          {$t('battlesYourFeed')} <span class="purse-num">{me.feed}</span>
        </p>
      {/if}
    </aside>

    {#if complaint}
      <p class="complaint" transition:fade={{ duration: 200 }}>{complaint}</p>
    {/if}

    {#if !cards.length}
      <p class="empty" in:fade={{ duration: 700, delay: 160 }}>{$t('battlesEmpty')}</p>
    {:else}
      {#if !signedIn}
        <p class="not-yet" in:fade={{ duration: 700, delay: 200 }}>{$t('battlesSignInToTake')}</p>
      {/if}
      <div class="shelf" in:fade={{ duration: 700, delay: 240 }}>
        {#each cards as card (card.id)}
          {@const copy = cardCopy(card, $lang)}
          {@const prices = pricesOf(card)}
          {@const href = workHref(card)}
          {@const held = holdingOf(card.id)}
          <figure class="stand" class:stand--raised={justRaised === card.id} use:firstLook={card.id}>
            <!-- Полка стоит лицом: каждая карта показывает, что она есть, а
                 цена — своей ли, чужой ли — читается в подписи под ней. -->
            <BattleCard
              {card}
              {frames}
              owned={true}
              level={held?.level ?? null}
              isNew={held?.isNew ?? false}
            />
            <!-- The price belongs to the shelf, not to the card: a card is a
                 thing the house made, a price is a note pinned under it. -->
            <figcaption class="label">
              {#if held}
                {@const rung = nextRung(card, held.level)}
                <span class="label-prices">
                  <span class="label-yours">{$t('battlesYours')}</span>
                  <!-- Ступень: цена и слово рядом, тем же голосом, что и цена
                       карты. Уровень ничего не даёт в бою — он засечка. -->
                  {#if rung !== null}
                    <span class="label-price">
                      <span class="label-amount">{rung}</span>
                      <span class="label-coin">{$t('battlesCoinDust')}</span>
                      <button
                        type="button"
                        class="label-take"
                        disabled={busyCard !== null || !affordable('dust', rung)}
                        onclick={() => raise(card, rung)}
                      >
                        {busyCard === card.id
                          ? $t('battlesRaising')
                          : affordable('dust', rung)
                            ? $t('battlesRaise')
                            : $t('battlesNotEnough')}
                      </button>
                    </span>
                  {/if}
                </span>
              {:else}
                <span class="label-prices">
                  {#each prices as price (price.coin)}
                    <span class="label-price">
                      <span class="label-amount">{price.amount}</span>
                      <span class="label-coin">
                        {price.coin === 'dust' ? $t('battlesCoinDust') : $t('battlesCoinFeed')}
                      </span>
                      {#if signedIn}
                        <button
                          type="button"
                          class="label-take"
                          disabled={busyCard !== null || !affordable(price.coin, price.amount)}
                          onclick={() => take(card, price.coin, price.amount)}
                        >
                          {busyCard === card.id
                            ? $t('battlesTaking')
                            : affordable(price.coin, price.amount)
                              ? $t('battlesTake')
                              : $t('battlesNotEnough')}
                        </button>
                      {/if}
                    </span>
                  {/each}
                </span>
              {/if}
              {#if href}
                <a class="label-work" {href}>{card.figurineName || copy.title}</a>
              {/if}
            </figcaption>
          </figure>
        {/each}
      </div>
    {/if}
  </div>
</div>

{#if taking}
  <BattleTaking card={taking} {frames} onclose={() => (taking = null)} />
{/if}

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

  .study-link {
    color: #6f3b24;
    text-decoration: none;
    border-bottom: 1px solid rgba(111, 59, 36, 0.3);
  }
  .study-link:hover {
    color: #c65f3c;
    border-bottom-color: #c65f3c;
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
    margin: 2.5rem 0 0;
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
    /* Each label sits under its own card rather than on a shared line: a frame
       carries its own ratio, so a dressed card is shorter than a bare one
       beside it, and pretending otherwise would either stretch a card out of
       its proportions or leave a hole under it. A shelf of different cards is
       uneven; that is what a shelf looks like. */
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

  /* «Взять» — не кнопка лавки: подчёркнутое слово рядом с ценой, как приписка
     на карточке под витриной, а не призыв. */
  .label-take {
    padding: 0;
    font: inherit;
    font-size: 0.72rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #6f3b24;
    background: none;
    border: none;
    border-bottom: 1px solid rgba(111, 59, 36, 0.35);
    cursor: pointer;
  }

  .label-take:hover:not(:disabled) {
    color: #c65f3c;
    border-bottom-color: #c65f3c;
  }

  .label-take:disabled {
    color: #8a6a55;
    border-bottom-color: transparent;
    cursor: default;
  }

  /* Ступень поднята: по карте один раз проходит блик фольги — то самое, чем
     уровень и является. Ни модалки, ни печати: печать — про получение, а это
     про то, что вещь стала твоей чуть больше. */
  .stand--raised {
    position: relative;
  }

  .stand--raised::after {
    content: '';
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: linear-gradient(
      105deg,
      transparent 40%,
      rgba(255, 249, 240, 0.55) 48%,
      rgba(255, 249, 240, 0.14) 55%,
      transparent 63%
    );
    background-size: 260% 100%;
    animation: rungSweep 1400ms ease-out both;
  }

  @keyframes rungSweep {
    from { background-position: 160% 0; }
    to { background-position: -60% 0; }
  }

  @media (prefers-reduced-motion: reduce) {
    .stand--raised::after {
      animation: none;
      opacity: 0;
    }
  }

  .label-yours {
    font-size: 0.72rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  /* Кошелёк на полях — тем же голосом, что и названия монет над ним. */
  .purse {
    margin: 0.4rem 0 0;
    font-size: 0.78rem;
    letter-spacing: 0.04em;
    color: #8a6a55;
  }

  .purse-num {
    font-variant-numeric: tabular-nums;
    color: #34251c;
  }

  .complaint {
    margin: 1.5rem 0 0;
    font-size: 0.85rem;
    color: #8f2f22;
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
