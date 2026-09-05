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
  import { replaceState } from '$app/navigation';
  import { page } from '$app/state';
  import { t, lang, brandName } from '$lib/i18n';
  import { SITE_URL } from '$lib/site';
  import { jsonLdSafe } from '$lib/jsonld';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { cardCopy, pricesOf, workHref } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import BattleSheet from '$lib/components/BattleSheet.svelte';
  import BattleTaking from '$lib/components/BattleTaking.svelte';
  import BattleGreeting from '$lib/components/BattleGreeting.svelte';
  import BattleDoor from '$lib/components/BattleDoor.svelte';
  import type {
    BattleCard as BattleCardDto,
    BattleErrand,
    BattleMe,
    BattleWelcomeGift,
  } from '$lib/types/api';

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
  /** Карта, которую читают до покупки: лист, не витрина. */
  let sheet = $state<BattleCardDto | null>(null);

  // ── Сколько осело с прошлого раза ────────────────────────────────────────
  //
  // Пыль капает с маяков на работах и небылицах, и до сих пор человек никогда
  // не узнавал, за что выросло число. Сказано это ЗДЕСЬ, на полке, а не там,
  // где капает: всплывающее «+1» над работой превратило бы разглядывание
  // вещи в добычу — ровно то, против чего построена комната. Полка — место,
  // где пыль тратят; там ей и место быть посчитанной.
  //
  // Отметка на полях: только прибыль, только один раз за визит, и на первом
  // заходе — молчание, а не «осело 0». Тот же счёт, что у книги посещений:
  // не с чем сравнить — значит нечего сказать.
  const PURSE_KEY = 'gotiga_battle_purse_';

  let settled = $state<number | null>(null);
  /** Разница считается один раз за визит: покупка карты — не «осело меньше». */
  let counted = false;

  /** Задания — названное заранее то, что сайт и так считает. */
  let errands = $state<BattleErrand[]>([]);

  /**
   * Встреча. Существует только когда есть что сказать: окно, открывающееся на
   * каждый заход и сообщающее ноль, закрывают не читая — а через неделю
   * закрывают вместе с настоящей новостью.
   */
  let greeting = $state<{
    gift: BattleWelcomeGift | null;
    works: number;
    dust: number;
    paid: BattleErrand[];
  } | null>(null);

  /**
   * @param dust     — баланс, который лёг в книгу.
   * @param arrivedNow — сколько из него пришло прямо сейчас, при входе.
   *
   * Пришедшее при входе вычитается из сравнения и НЕ вычитается из отметки:
   * иначе проявка сказала бы о себе дважды — своей строкой и строкой «осело с
   * прошлого раза», — а на следующий визит дом стал бы догонять собственную
   * засечку молча.
   */
  function markPurse(dust: number, arrivedNow = 0) {
    const who = authStore.user?.id;
    if (!who) return;
    const key = PURSE_KEY + who;
    let before: number | null = null;
    try {
      const raw = localStorage.getItem(key);
      before = raw === null ? null : Number(raw);
    } catch {
      // Приватное окно, запрещённые site data — отметки просто не будет.
    }
    if (!counted) {
      counted = true;
      const earlier = dust - arrivedNow;
      if (before !== null && Number.isFinite(before) && earlier > before) {
        settled = earlier - before;
      }
    }
    // Записывается на каждое чтение книги, а не только на первое: иначе
    // потраченное на карту оставило бы засечку выше баланса, и следующий приход
    // пыли пришлось бы догонять до неё молча.
    try {
      localStorage.setItem(key, String(dust));
    } catch {
      // См. выше: отметка — удобство, а не учёт.
    }
  }

  async function readWallet() {
    const token = authStore.token;
    if (!token) {
      me = null;
      return;
    }
    try {
      me = await api.getBattleMe(token);
      markPurse(me.dust);
      // Лист перечитывается вместе с кошельком: купленная карта закрывает
      // поручение на сервере, и лист, оставшийся прежним, врал бы про уже
      // заплаченное.
      try {
        errands = await api.getBattleErrands(token);
      } catch {
        // Лист — не полка: не прочитался, останется прежним до следующего раза.
      }
    } catch {
      // Полка от этого не ломается: без книги она просто вся в пыли.
      me = null;
    }
  }

  /**
   * Работы, которые браузер помнит просмотренными.
   *
   * Тот же список, по которому архив рисует пометку «смотрели»: дом уже сегодня
   * называет эти работы просмотренными, и проявка не выдумывает новое
   * определение внимания, а берёт готовое.
   */
  function viewedWorks(): string[] {
    try {
      const raw: unknown = JSON.parse(localStorage.getItem('gotiga_viewed') ?? '[]');
      if (!Array.isArray(raw)) return [];
      // Отсеивается здесь ТОЖЕ, хотя сервер и сам терпелив: список копится
      // годами, и в нём лежат слаги, записанные до перехода на слаги. Слать
      // заведомый мусор через сеть незачем.
      const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
      return raw.filter((v): v is string => typeof v === 'string' && uuid.test(v));
    } catch {
      // Приватное окно — проявлять просто нечего.
      return [];
    }
  }

  /**
   * Войти в комнату: дар первого входа и проявка.
   *
   * Ноль на первом входе — почти всегда ложь: человек, дошедший до полки, почти
   * наверняка уже смотрел работы, и маячок за них не сработал, потому что тогда
   * он был без имени. Сервер проверяет каждую работу по своей базе и платит за
   * неё однажды, так что войти можно сколько угодно раз.
   */
  async function enterRoom() {
    const token = authStore.token;
    if (!token) {
      me = null;
      return;
    }
    try {
      const entered = await api.enterBattleRoom(token, viewedWorks());
      me = entered.me;
      errands = entered.errands ?? [];
      const paid = entered.paid ?? [];
      // Один повод — уже разговор. Ни одного — тишина, и это не оплошность.
      if (entered.gift || entered.developedWorks > 0 || paid.length) {
        greeting = {
          gift: entered.gift,
          works: entered.developedWorks,
          dust: entered.developedDust,
          paid,
        };
      }
      const paidDust = paid.reduce((sum, e) => sum + (e.currency === 'dust' ? e.amount : 0), 0);
      markPurse(
        me.dust,
        (entered.developedDust ?? 0) + (entered.gift?.dust ?? 0) + paidDust,
      );
    } catch {
      // Вход не удался — полка от этого не закрывается. Кошелёк читается
      // отдельно: дар и проявка ничего не теряют и подождут следующего раза.
      await readWallet();
    }
  }

  onMount(enterRoom);

  let ownedBy = $derived(new Map((me?.owned ?? []).map((o) => [o.cardId, o])));
  const holdingOf = (id: string) => ownedBy.get(id) ?? null;

  function loginFromSheet(card: BattleCardDto): string {
    return `/login?from=${encodeURIComponent(`/battles?card=${card.id}`)}`;
  }

  function lookAt(card: BattleCardDto) {
    sheet = card;
    if (typeof window === 'undefined') return;
    const url = new URL(window.location.href);
    url.searchParams.set('card', card.id);
    replaceState(url, {});
  }

  function closeSheet() {
    sheet = null;
    if (typeof window === 'undefined') return;
    const url = new URL(window.location.href);
    if (!url.searchParams.has('card')) return;
    url.searchParams.delete('card');
    replaceState(url, {});
  }

  function openFromUrl(id: string | null) {
    if (!id) return;
    const found = cards.find((c) => c.id === id);
    if (found) sheet = found;
  }

  $effect(() => {
    openFromUrl(page.url.searchParams.get('card'));
  });

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
      if (res.takenNow) {
        closeSheet();
        taking = card;
      }
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
    <!-- Три комнаты одной дверью. Кошелёк и задания висят на ней, не в теле
         полки: полка показывает карты, а не объясняет, где ещё можно быть. -->
    <BattleDoor {me} {settled} {errands} />

    <header class="masthead" in:fly={{ x: -20, duration: 900, delay: 80, easing: cubicOut }}>
      <p class="eyebrow">
        <span class="eyebrow-rule"></span>
        {$t('battlesPageKicker')}
      </p>
      <h1 class="page-title">{$t('battlesPageTitle')}</h1>
      <p class="page-rule">{$t('battlesPageRule')}</p>
    </header>

    {#if complaint}
      <p class="complaint" transition:fade={{ duration: 200 }}>{complaint}</p>
    {/if}

    {#if !cards.length}
      <p class="empty" in:fade={{ duration: 700, delay: 160 }}>{$t('battlesEmpty')}</p>
    {:else}
      <div class="shelf" in:fade={{ duration: 700, delay: 240 }}>
        {#each cards as card (card.id)}
          {@const copy = cardCopy(card, $lang)}
          {@const prices = pricesOf(card)}
          {@const href = workHref(card)}
          {@const held = holdingOf(card.id)}
          <figure class="stand" class:stand--raised={justRaised === card.id} use:firstLook={card.id}>
            <!-- The shelf is a showcase. A back would hide the work, and a
                 person would have to remember what they had not been shown.
                 Face always; "yours" is the caption and the pips, not a
                 turned card. Get it lives on the sheet, where the body is. -->
            <div
              class="stand-face"
              role="button"
              tabindex="0"
              aria-haspopup="dialog"
              aria-label={copy.title || card.figurineName || ''}
              onclick={() => lookAt(card)}
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  lookAt(card);
                }
              }}
            >
              <BattleCard
                {card}
                {frames}
                owned={true}
                level={held?.level ?? null}
                isNew={held?.isNew ?? false}
                interactive={false}
              />
            </div>
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

{#if sheet}
  <BattleSheet
    card={sheet}
    {frames}
    signedIn={signedIn}
    owned={!!holdingOf(sheet.id)}
    busy={busyCard === sheet.id}
    canAfford={affordable}
    loginHref={loginFromSheet(sheet)}
    ontake={(coin, amount) => take(sheet!, coin, amount)}
    onclose={closeSheet}
    complaint={complaint}
  />
{/if}

{#if taking}
  <BattleTaking card={taking} {frames} onclose={() => (taking = null)} />
{/if}

<!-- Встреча идёт ПОСЛЕ церемонии в разметке и потому лежит поверх неё. Совпасть
     они могут только одним путём — человек взял карту и тут же перезагрузил
     страницу, — и тогда поверх должно быть то, что новее. -->
{#if greeting}
  <BattleGreeting
    gift={greeting.gift}
    developedWorks={greeting.works}
    developedDust={greeting.dust}
    paid={greeting.paid}
    {errands}
    onclose={() => (greeting = null)}
  />
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

  .empty {
    margin: 4rem 0 0;
    font-family: Georgia, 'Fraunces', serif;
    font-style: italic;
    opacity: 0.6;
  }

  .shelf {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 2.6rem 1.8rem;
    margin-top: 2.2rem;
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

  .stand-face {
    display: block;
    width: 100%;
    padding: 0;
    margin: 0;
    cursor: pointer;
    color: inherit;
    background: none;
    border: none;
    text-align: inherit;
  }

  .stand-face:focus-visible {
    outline: 1px solid #6f3b24;
    outline-offset: 6px;
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
