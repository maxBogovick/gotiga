<script lang="ts">
  // Скромные эпические битвы — the shelf of cards.
  //
  // Not a shop window, whatever the presence of prices might suggest. The room
  // is a shelf of the house's own cards with what each would cost written on the
  // shelf beneath it, the way a museum writes a card for a case. Nothing here
  // can be bought with money, and nothing is decided by chance.
  import { onMount } from 'svelte';
  import { fade, fly, slide } from 'svelte/transition';
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
  import BattleErrandList from '$lib/components/BattleErrandList.svelte';
  import BattleWallet from '$lib/components/BattleWallet.svelte';
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

  /**
   * Что дом досчитал при входе: работы, которые человек смотрел до того, как
   * комната для него открылась, и пыль за них.
   *
   * Числа врозь намеренно. Человеку говорят про РАБОТЫ — это единственное, что
   * он узнаёт про себя, — а в кошелёк ложится пыль по ставке, которую правит
   * хранитель, и одно из другого не выводится.
   */
  let developed = $state<{ works: number; dust: number } | null>(null);

  /** Задания — названное заранее то, что сайт и так считает. */
  let errands = $state<BattleErrand[]>([]);

  /** Окно «Что это такое?»: то же, что при первом входе, но по требованию. */
  let about = $state(false);

  /** Журнал начислений: нужен редко и по конкретному поводу. */
  let historyOpen = $state(false);

  /** Метка сразу под верхней строкой и то, ушла ли она вверх: по ней строка
   *  понимает, что она уже закреплена, и берёт фон. */
  let walletHead = $state<HTMLElement | null>(null);
  let stuck = $state(false);

  // Наблюдатель, а не слушатель прокрутки: положение шапки считает браузер,
  // а не обработчик, который срабатывает по сорок раз на движение колеса.
  $effect(() => {
    const head = walletHead;
    if (!head || typeof IntersectionObserver === 'undefined') return;
    const watcher = new IntersectionObserver(
      ([entry]) => {
        // Уехала ВВЕРХ, а не просто скрылась: при прокрутке к началу страницы
        // шапка тоже вне экрана, и полоска там не нужна.
        stuck = !entry.isIntersecting && entry.boundingClientRect.top < 0;
        if (!stuck) historyOpen = false;
      },
      { threshold: 0 },
    );
    watcher.observe(head);
    return () => watcher.disconnect();
  });

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
      if (entered.developedWorks > 0) {
        developed = { works: entered.developedWorks, dust: entered.developedDust };
      }
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
    <!-- Верхняя строка — она же кошелёк.
         Кошелёк стоял в шапке карточки заданий И повторялся закреплённой
         полоской: одно и то же число было нарисовано дважды, и какое из них
         видит человек, решала прокрутка. Здесь он один, виден сразу при
         открытии, не требует прокрутки вниз и остаётся на месте, пока идёшь
         вдоль полки с ценами. А строка «На главную» до сих пор занимала целую
         полосу ради одной короткой ссылки. -->
    <div class="topbar" class:topbar--stuck={stuck} in:fade={{ duration: 600 }}>
      <nav class="back-nav">
        <a href="/" class="back-link">{$t('battlesBack')}</a>
        {#if me}
          <span class="nav-grow"></span>
          <BattleWallet dust={me.dust} feed={me.feed} {settled} compact={true} />
          {#if me.gifts.length}
            <button
              type="button"
              class="quiet"
              onclick={() => (historyOpen = !historyOpen)}
              aria-expanded={historyOpen}>{$t('battlesWalletHistory')}</button
            >
          {/if}
          <button type="button" class="quiet" onclick={() => (about = true)}
            >{$t('battlesWalletAbout')}</button
          >
        {/if}
      </nav>

      <!-- Журнал раскрывается ВНУТРИ закреплённой строки: снаружи он уехал бы
           вверх вместе со страницей, а кошелёк остался бы на месте. -->
      {#if historyOpen && me?.gifts.length}
        <ul class="history" transition:slide={{ duration: 200 }}>
          {#each me.gifts as g (g.at)}
            <li class="gift">
              <span class="gift-why">
                {g.reason === 'welcome' ? $t('battlesGiftWelcome') : (g.note ?? '')}
              </span>
              <span class="gift-sum">
                <i class="gift-dot gift-dot--{g.currency}" aria-hidden="true"></i>{g.amount}
              </span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
    <div bind:this={walletHead} class="topbar-mark" aria-hidden="true"></div>

    <header class="masthead" in:fly={{ x: -20, duration: 900, delay: 80, easing: cubicOut }}>
      <p class="eyebrow">
        <span class="eyebrow-rule"></span>
        {$t('battlesPageKicker')}
      </p>
      <h1 class="page-title">{$t('battlesPageTitle')}</h1>
      <p class="page-rule">{$t('battlesPageRule')}</p>
      <!-- Одна строка, объясняющая, что здесь вообще можно делать. Без неё
           страница показывала карты с ценами и не говорила, зачем они. -->
      <p class="page-rule">{$t('battlesPageWhat')}</p>
      <!-- Соседние комнаты: где карты лежат разложенными и где они играют.
           Стол идёт первым — своё собрание смотрят раньше, чем садятся играть. -->
      <p class="page-rule">
        <a href="/battles/table" class="study-link">{$t('battlesTableTitle')} →</a>
      </p>
      <p class="page-rule"><a href="/battles/etude" class="study-link">{$t('battleStudies')} →</a></p>
    </header>

    <!-- Задания. Своё место — сразу под шапкой, до цен и до карт: сначала было
         строкой-раскладушкой в сноске под кошельком, и посмотреть, что осталось
         на сегодня, стоило двух попаданий по мелкой ссылке. Это первое, зачем
         сюда возвращаются, — значит и стоять должно первым. -->
    {#if me && errands.length}
      <BattleErrandList {errands} />
    {/if}

    <!-- Две монеты названы, объяснение — по ссылке.
         Раньше здесь стояли два абзаца, которые показывались каждый визит
         навсегда: человек прочитал их один раз в окне при первом входе, а
         дальше они были самым крупным текстом на экране, объясняющим самое
         мелкое (баланс). Легенда осталась, объяснение уехало в то же окно. -->
    <aside class="coins" in:fade={{ duration: 700, delay: 200 }}>
      <p class="coins-line">
        <span class="coin coin--dust"></span>{$t('battlesCoinDust')}
        <span class="coin-sep">·</span>
        <span class="coin coin--feed"></span>{$t('battlesCoinFeed')}
        <span class="coin-sep">·</span>
        <button type="button" class="coins-about" onclick={() => (about = true)}>
          {$t('battlesWalletAbout')}
        </button>
      </p>
      {#if developed}
        <p class="settled" in:fade={{ duration: 600 }}>
          {$t('battlesDeveloped')} <span class="purse-num">{developed.works}</span>
          <span class="coin-sep">·</span>
          {$t('battlesDevelopedDust')} <span class="purse-num">{developed.dust}</span>
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
        <p class="not-yet" in:fade={{ duration: 700, delay: 200 }}>
          <a href="/login?from={encodeURIComponent('/battles')}">{$t('battlesSignInToTake')}</a>
        </p>
      {/if}
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
{#if about}
  <BattleGreeting mode="about" {errands} onclose={() => (about = false)} />
{/if}

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

  /* Верхняя строка липнет под шапкой сайта: та `position: fixed` и в
     прокрученном виде ровно 54 пикселя. */
  .topbar {
    position: sticky;
    top: 54px;
    z-index: 40;
    margin: 0 -1.5rem 2.5rem;
    padding: 0 1.5rem;
  }

  /* Фон берётся только когда строка уже прилипла: поверх пергамента она должна
     быть невидимой, а поверх карт — читаемой. */
  .topbar--stuck {
    background: rgba(248, 241, 231, 0.94);
    backdrop-filter: blur(6px);
    border-bottom: 1px solid #d8c6b1;
    box-shadow: 0 6px 16px rgba(52, 37, 28, 0.05);
  }

  .topbar-mark {
    height: 1px;
    margin-top: -2.5rem;
  }

  .back-nav {
    display: flex;
    align-items: center;
    gap: 1.1rem;
    min-height: 2.6rem;
  }

  .nav-grow {
    flex: 1 1 auto;
  }

  .quiet {
    padding: 0 0 1px;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #8a6a55;
    background: none;
    border: none;
    border-bottom: 1px solid rgba(138, 106, 85, 0.35);
    cursor: pointer;
    white-space: nowrap;
  }

  .quiet:hover {
    color: #c65f3c;
    border-bottom-color: rgba(198, 95, 60, 0.5);
  }

  /* Журнал: причина первой, число вторым, валюта — только точкой цвета.
     Раньше строка начиналась с названия валюты капсом, то есть с наименее
     важного, а «за что» стояло последним и мельче всего. */
  .history {
    margin: 0;
    padding: 0.5rem 0 0.7rem;
    list-style: none;
    max-width: 34rem;
  }

  .gift {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.18rem 0;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.78rem;
    color: #6f3b24;
  }

  .gift-why {
    color: #8a6a55;
  }

  .gift-sum {
    display: inline-flex;
    align-items: baseline;
    gap: 0.35rem;
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }

  .gift-dot {
    width: 0.42rem;
    height: 0.42rem;
    border-radius: 50%;
    align-self: center;
  }

  .gift-dot--dust {
    background: #cbb79c;
  }

  .gift-dot--feed {
    background: #6f3b24;
  }

  /* На узком экране в первую строку идут ссылка и числа — то, ради чего строка
     и нужна, — а две мелкие ссылки переносятся во вторую. Раньше `flex-basis:
     100%` уводил числа вниз, и закреплённая полоса занимала на телефоне три
     строки: столько экрана служебная строка стоить не может. */
  @media (max-width: 40rem) {
    .topbar {
      top: 52px;
    }

    .back-nav {
      flex-wrap: wrap;
      gap: 0.35rem 0.8rem;
      padding: 0.3rem 0;
      min-height: 0;
    }

    .nav-grow {
      order: 1;
      flex-basis: 100%;
      height: 0;
    }

    .quiet {
      order: 2;
      font-size: 0.62rem;
    }

    /* Прилипнув на телефоне, строка оставляет только то, ради чего прилипает, —
       числа. «На главную» есть в шапке сайта, а журнал и объяснение открывают
       не на бегу вдоль полки. Иначе служебная строка съедает три строки экрана
       из десяти. */
    .topbar--stuck .back-link,
    .topbar--stuck .quiet {
      display: none;
    }

    .topbar--stuck .nav-grow {
      display: none;
    }
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

  .not-yet a {
    color: inherit;
    text-decoration: none;
    border-bottom: 1px solid rgba(111, 59, 36, 0.35);
  }

  .not-yet a:hover {
    color: #6f3b24;
    opacity: 1;
    border-bottom-color: #6f3b24;
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

  /* Кошелёк на полях — тем же голосом, что и названия монет над ним. */
  .coins-about {
    padding: 0;
    font-family: inherit;
    font-size: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    color: #c65f3c;
    background: none;
    border: none;
    border-bottom: 1px solid rgba(198, 95, 60, 0.35);
    cursor: pointer;
  }

  .settled {
    margin: 0.25rem 0 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.82rem;
    font-style: italic;
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
