<script lang="ts">
  // Лавка авторов: чужие лицензии, выставленные на продажу, и свои — на полке
  // рядом. Одна комната, а не две: «купить» и «продать» — две стороны одного
  // прилавка, и человек, пришедший продать, обязан видеть, почём стоят соседи.
  //
  // Смотреть можно без имени: сюда приходят по ссылке. Покупать — со входом.
  //
  // Лицензия показана НАСТОЯЩЕЙ картой в её раме, тем же отрисовщиком, каким
  // она встанет в игру: второй облик однажды соврёт.
  import { onMount } from 'svelte';
  import { t, lang } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { emptyBattleCard } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import StudioAsk from '$lib/components/studio/StudioAsk.svelte';
  import '$lib/components/studio/studio-room.css';
  import type {
    BattleCard as BattleCardDto,
    CopyListing,
    MyCopy,
    Auction,
    Trade,
    StudioLicence,
    StudioListing,
    StudioSettings,
  } from '$lib/types/api';

  let listings = $state<StudioListing[]>([]);
  let copies = $state<CopyListing[]>([]);
  /** Карты дома — чтобы показать выставленный экземпляр НАСТОЯЩЕЙ картой.
   *  Второго облика у карты нет и не будет. */
  let shelf = $state<BattleCardDto[]>([]);
  let frames = $state<import('$lib/types/api').BattleFrame[] | null>(null);
  let mine = $state<StudioLicence[]>([]);
  let myCopies = $state<MyCopy[]>([]);
  let trades = $state<Trade[]>([]);
  let auctions = $state<Auction[]>([]);
  /** Какому объявлению предлагают мену и что в неё кладут. Одна за раз: мена —
   *  решение, а не поле в таблице. */
  let trading = $state<string | null>(null);
  let picked = $state<Set<string>>(new Set());
  let settings = $state<StudioSettings | null>(null);
  let dust = $state<number | null>(null);
  let loading = $state(true);
  let said = $state<string | null>(null);
  /** Какую лицензию сейчас оценивают. Одна за раз: цена — решение, а не поле
   *  в таблице, и десять открытых полей превращают её в таблицу. */
  let pricing = $state<string | null>(null);
  let price = $state(0);
  let busy = $state<string | null>(null);
  /** О чём спрашивает дом сейчас. Своим окном, а не системным: серое окно
   *  посреди пергамента — самое громкое, что было в этой комнате, и в него
   *  нельзя было написать, из чего выбирают (коридор цены человек узнавал
   *  только из отказа сервера). */
  let ask = $state<
    | null
    | { kind: 'sellCopy'; copy: MyCopy }
    | { kind: 'hammer'; copy: MyCopy }
    | { kind: 'bid'; lot: Auction }
    | { kind: 'buy'; listing: string; name: string; price: number }
  >(null);

  let token = $derived(authStore.token);

  /** Манекен один на всю лавку: продаётся рама, и разные карты сравнивали бы
   *  не то. Тот же, что в галерее и в сезоне. */
  const sample: BattleCardDto = {
    ...emptyBattleCard(),
    tier: 1,
    titleEn: 'The Keeper of the Key',
    titleRu: 'Хранительница Ключа',
    effectRu: 'Вихрь Души: каждое третье заклинание создаёт копию эффекта.',
    effectEn: 'Wind of Soul: every third spell makes a copy of its effect.',
    cost: 5,
    power: 10,
  };

  onMount(reload);

  async function reload() {
    loading = true;
    const [counter, onSale, hammer, cards, dress] = await Promise.all([
      api.getStudioMarket().catch(() => []),
      api.getMarketCopies().catch(() => []),
      api.getAuctions().catch(() => []),
      api.getBattleCards().catch(() => []),
      api.getBattleFrames().catch(() => null),
    ]);
    listings = counter;
    copies = onSale;
    auctions = hammer;
    shelf = cards;
    frames = dress?.frames ?? null;
    if (token) {
      const [licences, studio, me, own] = await Promise.all([
        api.getMyLicences(token).catch(() => []),
        api.getStudio(token).catch(() => null),
        api.getBattleMe(token).catch(() => null),
        api.getMyCopies(token).catch(() => []),
      ]);
      trades = await api.getTrades(token).catch(() => []);
      mine = licences;
      myCopies = own;
      settings = studio?.settings ?? null;
      dust = me?.dust ?? null;
    }
    loading = false;
  }

  function flash(text: string, ms = 6000) {
    said = text;
    setTimeout(() => (said = null), ms);
  }

  function openPrice(licence: StudioLicence) {
    pricing = licence.id;
    price = settings?.priceFloor ?? 50;
  }

  async function sell(licence: StudioLicence) {
    if (!token) return;
    busy = licence.id;
    try {
      await api.listThing(token, 'license', licence.id, Math.round(price), 'dust');
      pricing = null;
      await reload();
      flash($t('studioSellDone'));
    } catch (e) {
      flash(String(e));
    }
    busy = null;
  }

  // Снимают и покупают ПО ОБЪЯВЛЕНИЮ, а не по виду вещи: сделка у лицензии и
  // у экземпляра одна, и две одинаковые пары кнопок разошлись бы на первой же
  // правке.
  async function withdrawId(listing: string) {
    if (!token) return;
    busy = listing;
    try {
      await api.withdrawListing(token, listing);
      await reload();
      flash($t('studioWithdrawDone'));
    } catch (e) {
      flash(String(e));
    }
    busy = null;
  }

  const withdraw = (listing: StudioListing) => withdrawId(listing.id);

  // Покупка — трата, и спрашивают о ней один раз, словами и с ценой; спрашивает
  // окно дома, а согласие приходит сюда.
  async function buyId(listing: string, name: string, price: number) {
    if (!token) return;
    busy = listing;
    try {
      const done = await api.buyListing(token, listing);
      dust = done.balance;
      await reload();
      flash($t('studioBuyDone').replace('{name}', name));
    } catch (e) {
      flash(String(e));
    }
    busy = null;
  }

  const buy = (listing: StudioListing) =>
    (ask = { kind: 'buy', listing: listing.id, name: listing.name, price: listing.price });

  /** Что дом спрашивает у окна и что делает с ответом. Одним местом: четыре
   *  окна с четырьмя своими кнопками разошлись бы на первой же правке. */
  const askTitle = $derived.by(() => {
    if (!ask) return '';
    if (ask.kind === 'buy') return $t('studioBuy');
    if (ask.kind === 'bid') return $t('studioBid');
    if (ask.kind === 'hammer') return $t('studioToHammer');
    return $t('studioSell');
  });

  function answered(said: string) {
    const now = ask;
    ask = null;
    if (!now) return;
    if (now.kind === 'buy') void buyId(now.listing, now.name, now.price);
    else if (now.kind === 'bid') void bid(now.lot, said);
    else if (now.kind === 'hammer') void toHammer(now.copy, said);
    else void sellCopy(now.copy, said);
  }

  /** Сколько получит продавец: доля дома сгорает, и молчать об этом нельзя —
   *  человек ставит цену, а получает меньше. */
  let takeHome = $derived.by(() => {
    const cut = settings?.commissionPercent ?? 0;
    return Math.max(0, Math.round(price) - Math.floor((Math.round(price) * cut) / 100));
  });

  // ── Молоток ───────────────────────────────────────────────────────────

  /** Сколько осталось торгу — днями и часами, а не бегущими секундами: секунды
   *  превращают комнату в таймер, которого дом не держит. */
  function timeLeft(when: string): string {
    const left = new Date(when).getTime() - Date.now();
    if (left <= 0) return $t('studioLotOver');
    const hours = Math.ceil(left / 3600000);
    return hours >= 24
      ? $t('studioLotDays').replace('{n}', String(Math.ceil(hours / 24)))
      : $t('studioLotHours').replace('{n}', String(hours));
  }

  /** Сколько дать, чтобы перебить: домашний шаг назван вслух, иначе отказ
   *  «мало» приходит без числа. */
  const leastBid = (lot: Auction) => (lot.topBid ? lot.topBid + 10 : lot.startPrice);

  async function bid(lot: Auction, asked: string) {
    if (!token) return;
    busy = lot.id;
    try {
      await api.placeBid(token, lot.id, Math.round(Number(asked)));
      await reload();
      flash($t('studioBidDone'));
    } catch (e) {
      flash(String(e));
    }
    busy = null;
  }

  async function toHammer(copy: MyCopy, asked: string) {
    if (!token) return;
    busy = copy.id;
    try {
      await api.startAuction(token, 'copy', copy.id, Math.round(Number(asked)), 'dust');
      await reload();
      flash($t('studioLotStarted'));
    } catch (e) {
      flash(String(e));
    }
    busy = null;
  }

  // ── Мена ──────────────────────────────────────────────────────────────
  //
  // Предлагается ПРОТИВ объявления: прилавок и есть «вот моя вещь, она
  // отдаётся», а мена говорит «отдам за своё, а не за пыль». Места, где видно
  // чужое собрание, в доме нет, и заводить его незачем.

  function beginTrade(listing: string) {
    trading = listing;
    picked = new Set();
  }

  function toggle(id: string) {
    const next = new Set(picked);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    picked = next;
  }

  async function sendTrade() {
    if (!token || !trading || picked.size === 0) return;
    busy = trading;
    try {
      await api.offerTrade(
        token,
        trading,
        [...picked].map((id) => ({ kind: 'copy' as const, subjectId: id })),
      );
      trading = null;
      picked = new Set();
      await reload();
      flash($t('studioTradeSent'));
    } catch (e) {
      flash(String(e));
    }
    busy = null;
  }

  async function answerTrade(trade: Trade, yes: boolean) {
    if (!token) return;
    busy = trade.id;
    try {
      if (yes) await api.acceptTrade(token, trade.id);
      else await api.refuseTrade(token, trade.id);
      await reload();
      flash(yes ? $t('studioTradeTaken') : $t('studioTradeRefused'));
    } catch (e) {
      flash(String(e));
    }
    busy = null;
  }

  /** Что можно положить в мену: своё и не запертое. */
  let tradable = $derived(myCopies.filter((c) => !c.locked));

  /** Карта по имени: выставленный экземпляр показывается настоящей картой с
   *  полки, а не строкой списка. */
  const cardOf = (id: string) => shelf.find((c) => c.id === id) ?? null;

  async function sellCopy(copy: MyCopy, asked: string) {
    if (!token) return;
    busy = copy.id;
    try {
      await api.listThing(token, 'copy', copy.id, Math.round(Number(asked)), 'dust');
      await reload();
      flash($t('studioSellDone'));
    } catch (e) {
      flash(String(e));
    }
    busy = null;
  }

  /** Своё на прилавке — по номеру экземпляра, а не по имени продавца: тёзка
   *  снял бы чужое. */
  let myCopyIds = $derived(new Set(myCopies.map((c) => c.id)));

  /** Свои выставленные — те же записи лавки, но с кнопкой «снять». Отдельной
   *  полки им не надо: где они стоят, там их и снимают. Узнаются по НОМЕРУ
   *  лицензии: имя продавца в доме не уникально. */
  let mineIds = $derived(new Set(mine.map((l) => l.id)));
</script>

<svelte:head>
  <title>{$t('studioMarket')}</title>
  <meta name="description" content={$t('studioMarketLead')} />
</svelte:head>

<div class="studio-room">
  <div class="page">
    <p class="eyebrow">
      <a href="/studio">{$t('studioBack')}</a>
      <span class="eyebrow-rule"></span>
      <span>{$t('studioEyebrow')}</span>
    </p>
    <h1 class="room-title">{$t('studioMarket')}</h1>
    <p class="room-lead">{$t('studioMarketLead')}</p>

    {#if token && dust !== null}
      <p class="mark">{$t('studioPurse').replace('{n}', String(dust))}</p>
    {/if}

    {#if said}<p class="said">{said}</p>{/if}

  {#if loading}
    <p class="empty">{$t('studioLoading')}</p>
  {:else}
    <!-- Прилавок -->
    {#if listings.length === 0}
      <p class="empty">{$t('studioMarketEmpty')}</p>
    {:else}
      <div class="shelf">
        {#each listings as listing (listing.id)}
          <div>
            <BattleCard card={sample} frames={[listing.body]} owned={true} />
            <p class="name">{listing.name}</p>
            <p class="by">
              {#if listing.authorSlug}
                <a href="/studio/authors/{listing.authorSlug}" class="hover:underline"
                  >{listing.author}</a
                >
              {:else}
                {listing.author}
              {/if}
              ·
              {#if listing.editionSize}
                {$t('studioSerialOf')
                  .replace('{n}', String(listing.serial))
                  .replace('{of}', String(listing.editionSize))}
              {:else}
                {$t('studioSerial').replace('{n}', String(listing.serial))}
              {/if}
            </p>
            {#if listing.seller !== listing.author}
              <p class="mark">
                {$t('studioSoldBy').replace('{name}', listing.seller)}
              </p>
            {/if}
            <div class="tag">
              <span class="tag-price">{listing.price}</span>
              <span class="tag-coin"
                >{$t('studioDust')}</span
              >
              {#if mineIds.has(listing.licenceId)}
                <button
                  onclick={() => withdraw(listing)}
                  disabled={busy === listing.id}
                  class="quiet quiet--danger"
                  >{$t('studioWithdraw')}</button
                >
              {:else if token}
                <button
                  onclick={() => buy(listing)}
                  disabled={busy === listing.id || (dust !== null && dust < listing.price)}
                  class="quiet quiet--buy"
                  >{dust !== null && dust < listing.price
                    ? $t('studioNotEnough')
                    : $t('studioBuy')}</button
                >
              {:else}
                <a
                  href="/login"
                  class="quiet" style="margin-left:auto"
                  >{$t('studioSignInToBuy')}</a
                >
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}

    <!-- Карты людей. Второй прилавок в той же комнате: право носить раму и
         сама карта — разные вещи, и мешать их в одну полку значило бы
         показывать их одинаково. -->
    <h2 class="shelf-title">{$t('studioMarketCards')}</h2>
    <p class="shelf-lead">{$t('studioMarketCardsLead')}</p>
    {#if copies.length === 0}
      <p class="empty">{$t('studioMarketEmpty')}</p>
    {:else}
      <div class="shelf">
        {#each copies as one (one.id)}
          <div>
            {#if cardOf(one.cardId)}
              <BattleCard
                card={cardOf(one.cardId)!}
                {frames}
                owned={true}
                level={one.level}
              />
            {/if}
            <p class="name">
              {$lang === 'en' ? one.titleEn : one.titleRu}
            </p>
            <p class="by">
              {#if one.serial}{$t('studioSerial').replace('{n}', String(one.serial))} · {/if}
              {$t('studioCopyLevel').replace('{n}', String(one.level))}
            </p>
            {#if one.creditName}
              <!-- Имя рядом с ценой — в тот самый миг, когда его запоминают. -->
              <p class="by">
                {$t('battlesCredit').replace('{name}', one.creditName)}
              </p>
            {/if}
            <div class="tag">
              <span class="tag-price">{one.price}</span>
              <span class="tag-coin"
                >{$t('studioDust')}</span
              >
              {#if myCopyIds.has(one.copyId)}
                <button
                  onclick={() => withdrawId(one.id)}
                  disabled={busy === one.id}
                  class="quiet quiet--danger"
                  >{$t('studioWithdraw')}</button
                >
              {:else if token}
                <button
                  onclick={() => beginTrade(one.id)}
                  disabled={busy === one.id}
                  class="ml-auto text-[10px] uppercase tracking-[0.14em] text-[#8a6a55] hover:text-[#c65f3c] disabled:opacity-40"
                  >{$t('studioTradeOffer')}</button
                >
                <button
                  onclick={() =>
                    (ask = {
                      kind: 'buy',
                      listing: one.id,
                      name: $lang === 'en' ? one.titleEn : one.titleRu,
                      price: one.price,
                    })}
                  disabled={busy === one.id || (dust !== null && dust < one.price)}
                  class="btn btn--lit"
                  >{dust !== null && dust < one.price
                    ? $t('studioNotEnough')
                    : $t('studioBuy')}</button
                >
              {:else}
                <a
                  href="/login"
                  class="quiet" style="margin-left:auto"
                  >{$t('studioSignInToBuy')}</a
                >
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}

    <!-- Молоток. Третья полка той же комнаты: вещь тоже отдаётся, только цену
         называет не хозяин, а тот, кто больше даст. -->
    <h2 class="shelf-title">{$t('studioHammer')}</h2>
    <p class="shelf-lead">{$t('studioHammerLead')}</p>
    {#if auctions.length === 0}
      <p class="empty">{$t('studioHammerEmpty')}</p>
    {:else}
      <div class="shelf">
        {#each auctions as lot (lot.id)}
          <div>
            {#if lot.cardId && cardOf(lot.cardId)}
              <BattleCard card={cardOf(lot.cardId)!} {frames} owned={true} />
            {/if}
            <p class="name">{lot.name}</p>
            <p class="by">
              {#if lot.estate}
                <!-- Вещь ушедшего названа словом: у неё нет продавца, и
                     вырученное за неё сгорает. -->
                {$t('studioLotEstate')}
              {:else if lot.seller}
                {$t('studioSoldBy').replace('{name}', lot.seller)}
              {/if}
              · {timeLeft(lot.endsAt)}
            </p>
            <div class="tag">
              <span class="tag-price">{lot.topBid ?? lot.startPrice}</span>
              <span class="tag-coin">
                {lot.topBid
                  ? $t('studioLotBids').replace('{n}', String(lot.bids))
                  : $t('studioLotStart')}
              </span>
              {#if token}
                <button
                  onclick={() => (ask = { kind: 'bid', lot })}
                  disabled={busy === lot.id}
                  class="quiet quiet--buy"
                  >{$t('studioBid')}</button
                >
              {:else}
                <a
                  href="/login"
                  class="quiet" style="margin-left:auto"
                  >{$t('studioSignInToBuy')}</a
                >
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}

    <!-- Свои лицензии: та же комната, второй прилавок. -->
    {#if token}
      <h2 class="shelf-title">{$t('studioMyLicences')}</h2>
      <p class="shelf-lead">{$t('studioMyLicencesLead')}</p>

      {#if mine.length === 0}
        <p class="empty">{$t('studioNoLicences')}</p>
      {:else}
        <div class="shelf">
          {#each mine as licence (licence.id)}
            <div>
              <BattleCard card={sample} frames={[licence.body]} owned={true} />
              <p class="name">{licence.name}</p>
              <p class="by">
                {licence.author} ·
                {#if licence.editionSize}
                  {$t('studioSerialOf')
                    .replace('{n}', String(licence.serial))
                    .replace('{of}', String(licence.editionSize))}
                {:else}
                  {$t('studioSerial').replace('{n}', String(licence.serial))}
                {/if}
              </p>
              {#if licence.locked}
                <!-- Запертая стоит на прилавке: снимают её там, а не здесь. -->
                <p class="mark mark--lit">
                  {$t('studioOnSale')}
                </p>
              {:else if pricing === licence.id}
                <div class="mt-2 border border-[#d8c6b1] bg-[#fdf9f3] p-3">
                  <label class="block text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
                    {$t('studioPrice')}
                    <input
                      type="number"
                      bind:value={price}
                      min={settings?.priceFloor ?? 1}
                      max={settings?.priceCeil ?? 1000000}
                      class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
                    />
                  </label>
                  <p class="mt-1 text-[10px] text-[#8a6a55]">
                    {$t('studioPriceRange')
                      .replace('{min}', String(settings?.priceFloor ?? 1))
                      .replace('{max}', String(settings?.priceCeil ?? 0))}
                  </p>
                  <!-- Доля дома названа числом до сделки, а не после неё. -->
                  <p class="mt-1 text-[10px] text-[#8a6a55]">
                    {$t('studioTakeHome')
                      .replace('{n}', String(takeHome))
                      .replace('{cut}', String(settings?.commissionPercent ?? 0))}
                  </p>
                  <div class="mt-2 flex gap-2">
                    <button
                      onclick={() => sell(licence)}
                      disabled={busy === licence.id}
                      class="btn btn--lit"
                      >{$t('studioSell')}</button
                    >
                    <button
                      onclick={() => (pricing = null)}
                      class="quiet"
                      >{$t('studioCancel')}</button
                    >
                  </div>
                </div>
              {:else}
                <button
                  onclick={() => openPrice(licence)}
                  class="mt-1 text-[10px] uppercase tracking-[0.14em] text-[#8a6a55] hover:text-[#c65f3c]"
                  >{$t('studioSell')}</button
                >
              {/if}
            </div>
          {/each}
        </div>
      {/if}
      <!-- Свои карты. Продают их отсюда же: вещь, пропавшая из списка ровно
           потому, что выставлена, — это вещь, которую нельзя снять. -->
      <h2 class="shelf-title">{$t('studioMyCards')}</h2>
      <p class="shelf-lead">{$t('studioMyCardsLead')}</p>
      {#if myCopies.length === 0}
        <p class="empty">{$t('studioNoCopies')}</p>
      {:else}
        <div class="shelf">
          {#each myCopies as own (own.id)}
            <div>
              {#if cardOf(own.cardId)}
                <BattleCard card={cardOf(own.cardId)!} {frames} owned={true} level={own.level} />
              {/if}
              <p class="name">
                {$lang === 'en' ? own.titleEn : own.titleRu}
              </p>
              <p class="by">
                {#if own.serial}{$t('studioSerial').replace('{n}', String(own.serial))} · {/if}
                {$t('studioCopyLevel').replace('{n}', String(own.level))}
              </p>
              {#if own.locked}
                <p class="mark mark--lit">
                  {$t('studioOnSale')}
                </p>
              {:else}
                <div class="deeds">
                  <button
                    onclick={() => (ask = { kind: 'sellCopy', copy: own })}
                    disabled={busy === own.id}
                    class="quiet"
                    >{$t('studioSell')}</button
                  >
                  <button
                    onclick={() => (ask = { kind: 'hammer', copy: own })}
                    disabled={busy === own.id}
                    class="quiet"
                    >{$t('studioToHammer')}</button
                  >
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
      <!-- Мены. И те, что предложили мне, и те, что предложил я: это одна
           комната, и разносить их по разным местам значило бы прятать половину
           разговора. -->
      {#if trades.length}
        <h2 class="shelf-title">{$t('studioTrades')}</h2>
        <div class="mt-4 space-y-3">
          {#each trades as trade (trade.id)}
            <div class="border border-[#d8c6b1] bg-[#fdf9f3] p-3">
              <p class="text-sm text-[#34251c]">
                {#if trade.mine}
                  {$t('studioTradeMine').replace('{want}', trade.want)}
                {:else}
                  {$t('studioTradeTheirs')
                    .replace('{who}', trade.from)
                    .replace('{want}', trade.want)}
                {/if}
              </p>
              <p class="mt-1 text-[11px] text-[#6f3b24]">
                {$t('studioTradeGives').replace('{what}', trade.gives.join(', '))}
              </p>
              <div class="mt-2 flex gap-3">
                {#if !trade.mine}
                  <button
                    onclick={() => answerTrade(trade, true)}
                    disabled={busy === trade.id}
                    class="text-[10px] uppercase tracking-[0.14em] text-[#c65f3c] hover:underline disabled:opacity-40"
                    >{$t('studioTradeAccept')}</button
                  >
                {/if}
                <button
                  onclick={() => answerTrade(trade, false)}
                  disabled={busy === trade.id}
                  class="text-[10px] uppercase tracking-[0.14em] text-[#b0a08e] hover:text-[#8f2f22] disabled:opacity-40"
                  >{trade.mine ? $t('studioTradeTakeBack') : $t('studioTradeRefuse')}</button
                >
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {:else}
      <p class="empty">{$t('studioMarketSignIn')}</p>
    {/if}
  {/if}
  </div>
</div>

<!-- Что кладём в мену. Выбирают из СВОЕГО и не запертого: вещь, уже занятая
       другой меной или прилавком, в мену не идёт. -->
  {#if trading}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-[#34251c]/40 p-4">
      <div class="max-h-[80vh] w-full max-w-2xl overflow-y-auto border border-[#d8c6b1] bg-[#f8f1e7] p-5">
        <h2 class="font-serif text-xl text-[#34251c]">{$t('studioTradeWhat')}</h2>
        <p class="mt-1 text-sm text-[#6f3b24]">{$t('studioTradeWhatLead')}</p>
        {#if tradable.length === 0}
          <p class="empty">{$t('studioNoCopies')}</p>
        {:else}
          <div class="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-3">
            {#each tradable as own (own.id)}
              <button
                onclick={() => toggle(own.id)}
                class="border p-2 text-left text-xs {picked.has(own.id)
                  ? 'border-[#c65f3c] bg-[#c65f3c]/8'
                  : 'border-[#d8c6b1] hover:bg-[#34251c]/5'}"
              >
                <span class="block truncate text-[#34251c]"
                  >{$lang === 'en' ? own.titleEn : own.titleRu}</span
                >
                <span class="text-[10px] text-[#8a6a55]"
                  >{$t('studioCopyLevel').replace('{n}', String(own.level))}</span
                >
              </button>
            {/each}
          </div>
        {/if}
        <div class="mt-5 flex flex-wrap gap-3">
          <button
            onclick={sendTrade}
            disabled={picked.size === 0}
            class="border border-[#c65f3c]/60 px-4 py-2 text-xs uppercase tracking-[0.16em] text-[#c65f3c] hover:bg-[#c65f3c]/8 disabled:opacity-40"
            >{$t('studioTradeSend')}</button
          >
          <button
            onclick={() => (trading = null)}
            class="border border-[#34251c]/20 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
            >{$t('studioCancel')}</button
          >
        </div>
      </div>
    </div>
  {/if}

{#if ask}
  <StudioAsk
    title={askTitle}
    lead={ask.kind === 'buy'
      ? $t('studioBuyAsk').replace('{name}', ask.name).replace('{n}', String(ask.price))
      : ask.kind === 'bid'
        ? $t('studioBidAsk').replace('{n}', String(leastBid(ask.lot)))
        : $t('studioSellCopyAsk')
            .replace('{min}', String(settings?.priceFloor ?? 1))
            .replace('{max}', String(settings?.priceCeil ?? 0))}
    field={ask.kind === 'buy' ? undefined : 'number'}
    value={ask.kind === 'bid'
      ? String(leastBid(ask.lot))
      : ask.kind === 'buy'
        ? ''
        : String(settings?.priceFloor ?? 50)}
    min={ask.kind === 'bid' ? leastBid(ask.lot) : (settings?.priceFloor ?? 1)}
    max={ask.kind === 'bid' ? undefined : settings?.priceCeil}
    yes={askTitle}
    onyes={answered}
    onclose={() => (ask = null)}
  />
{/if}

<style>
  /* Цена — ЯРЛЫК на вещи, а не ценник у кнопки: лавка дома выглядит прилавком,
     а не витриной. Число тушью, монета шёпотом, кнопка такая же тихая, как
     «показать хозяину». */
  .tag-price {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.05rem;
    color: #6f3b24;
  }

  :global(.studio-room .quiet--buy) {
    margin-left: auto;
    color: #c65f3c;
  }

  :global(.studio-room .quiet--buy:disabled) {
    color: #b0a08e;
  }
</style>
