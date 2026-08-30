<script lang="ts">
  // Ваш стол — собрание и расстановка одной комнатой.
  //
  // Главное решение: это НЕ два экрана. «Что у меня есть» и «как собрать
  // колоду» — один жест: разложить свои карты и выбрать шесть. Два экрана
  // заставили бы человека держать связь между ними в голове.
  //
  // Чего в комнате нет и не будет: фильтров, сортировок, поиска, счётчиков,
  // кнопки «собрать автоматически», подсказки «перетащите карту сюда». Шкаф на
  // восемь карт не фильтруют.
  //
  // Ни одного правила колоды здесь не вычисляется. Сколько мест, что законно и
  // чем дом закрывает пустое — решает сервер; страница показывает присланное и
  // отправляет обратно то, что выбрал человек. Вторая реализация одного правила
  // разошлась бы с той, по которой играют.
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t, lang, brandName } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { cardCopy } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import type {
    BattleCard as BattleCardDto,
    BattleDeck,
    BattleFrame,
    BattleMe,
  } from '$lib/types/api';

  /** Половина гостя, как её знает `battle-core::board`: три в ширину, ряды 3..5.
   *  Ряд 3 ближе к хранителю и потому рисуется сверху. */
  const ROWS = [3, 4, 5];
  const COLS = [0, 1, 2];

  let cards = $state<BattleCardDto[]>([]);
  let frames = $state<BattleFrame[]>([]);
  let me = $state<BattleMe | null>(null);
  let deck = $state<BattleDeck | null>(null);
  let loading = $state(true);
  let saving = $state(false);
  let complaint = $state<string | null>(null);
  let said = $state<string | null>(null);

  let signedIn = $derived(authStore.isLoggedIn);

  // ── Что человек выбрал сам ───────────────────────────────────────────────
  //
  // Только своё: заём сюда не попадает, потому что его не выбирают.
  let placed = $state<{ card: string; x: number; y: number }[]>([]);
  let held = $state<string[]>([]);
  /**
   * Тронут ли стол с последнего сохранения.
   *
   * Пока не тронут — комната показывает разбор сервера, вместе с заёмом. Как
   * только тронут, пустое место рисуется бумажной полоской «дом одолжит», а не
   * конкретной картой: какую именно карту дом положит, решает дом, и досчитать
   * это в браузере значило бы завести второе правило.
   */
  let dirty = $state(false);
  /** Карта, взятая в руку: касание по карте, потом касание по месту. */
  let picked = $state<string | null>(null);

  const byId = $derived(new Map(cards.map((c) => [c.id, c])));
  const cardOf = (id: string | null | undefined) => (id ? byId.get(id) ?? null : null);
  const levelOf = (id: string) => me?.owned.find((o) => o.cardId === id)?.level ?? null;

  /** Собрание за вычетом того, что уже на столе: карта лежит в одном месте. */
  let drawer = $derived(
    (me?.owned ?? [])
      .map((o) => byId.get(o.cardId))
      .filter((c): c is BattleCardDto => !!c)
      .filter((c) => !placed.some((p) => p.card === c.id) && !held.includes(c.id)),
  );

  async function readAll() {
    const token = authStore.token;
    const [shelf, dressing] = await Promise.all([api.getBattleCards(), api.getBattleFrames()]);
    cards = shelf;
    frames = dressing.frames;
    if (!token) {
      loading = false;
      return;
    }
    const [mine, table] = await Promise.all([
      api.getBattleMe(token),
      api.getBattleDeck(token),
    ]);
    me = mine;
    deck = table;
    lay(table);
    loading = false;
  }

  /** Разложить присланный сервером стол в то, что редактируется. Заём при этом
   *  отбрасывается: редактируется только своё. */
  function lay(table: BattleDeck) {
    placed = table.board
      .filter((s) => s.cardId && s.x !== undefined && s.y !== undefined)
      .map((s) => ({ card: s.cardId as string, x: s.x as number, y: s.y as number }));
    held = table.hand.map((s) => s.cardId).filter((id): id is string => !!id);
    dirty = false;
    picked = null;
  }

  onMount(readAll);

  // ── Что на каком месте ───────────────────────────────────────────────────

  const mineAt = (x: number, y: number) => placed.find((p) => p.x === x && p.y === y) ?? null;

  /** Заём на этой клетке — только пока стол не тронут: после правки разбор
   *  сервера относится к прежней расстановке и врал бы. */
  function lentAt(x: number, y: number): string | null {
    if (dirty || !deck) return null;
    const slot = deck.board.find((s) => s.x === x && s.y === y);
    return slot && !slot.cardId ? slot.lentCardId : null;
  }

  /** Снята ли с полки карта, которую человек сюда поставил. */
  function goneCard(id: string): boolean {
    if (!deck) return false;
    return [...deck.board, ...deck.hand].some((s) => s.cardId === id && s.gone);
  }

  function lentInHand(at: number): string | null {
    if (dirty || !deck) return null;
    // Свои карты руки идут первыми, заём — за ними: место `at` за пределами
    // своих есть заём под тем же номером.
    const own = deck.hand.filter((s) => s.cardId).length;
    if (at < own) return null;
    return deck.hand[at]?.lentCardId ?? null;
  }

  /**
   * Выйдет ли бой с пустым полем.
   *
   * Ровно то, обо что споткнулись: колода из одних карт руки законна, но первый
   * ход в ней пустой — карты с руки выкладываются за ману, а её на первом ходу
   * единица. Тот, кто уже стоит на поле, ходит и бьёт СРАЗУ и бесплатно
   * (`Move`, `Attack` и `Mend` ману не проверяют вовсе), поэтому одна карта на
   * поле — разница между «нечего делать» и обычным первым ходом.
   *
   * Временные карты дома встают на поле и эту дыру закрывают, поэтому
   * предупреждать надо только когда не встанет вообще никто.
   */
  let fieldWillBeEmpty = $derived(
    placed.length === 0 &&
      (dirty
        ? !!deck?.nothingToLend
        : (deck?.board.every((s) => !s.cardId && !s.lentCardId) ?? false)),
  );

  let boardFull = $derived(placed.length >= ROWS.length);
  let handFull = $derived(held.length >= 3);

  // ── Касание по карте, потом касание по месту ─────────────────────────────
  //
  // Работает мышью и пальцем одинаково и не требует объяснения. Перетаскивание
  // на телефоне мучительно, а комната читается с телефона.

  function pick(id: string) {
    picked = picked === id ? null : id;
  }

  function putOnCell(x: number, y: number) {
    const standing = mineAt(x, y);
    if (!picked) {
      // Пустое касание по своей карте — взять её обратно в собрание.
      if (standing) {
        placed = placed.filter((p) => p !== standing);
        dirty = true;
      }
      return;
    }
    if (standing) placed = placed.filter((p) => p !== standing);
    else if (boardFull) return;
    placed = [...placed, { card: picked, x, y }];
    picked = null;
    dirty = true;
    said = null;
  }

  function putInHand(at: number) {
    const standing = held[at] ?? null;
    if (!picked) {
      if (standing) {
        held = held.filter((_, i) => i !== at);
        dirty = true;
      }
      return;
    }
    if (standing) held = held.map((id, i) => (i === at ? (picked as string) : id));
    else if (handFull) return;
    else held = [...held, picked];
    picked = null;
    dirty = true;
    said = null;
  }

  /** Отказ приходит словом (`deck:tooManyOnBoard`), а не текстом: текст живёт
   *  здесь, на двух языках, а сервер, который его сочиняет, сочиняет его на
   *  одном. Незнакомое слово не молчит — комната говорит общее. */
  function faultLine(e: unknown): string {
    const word = String(e).match(/deck:(\w+)/)?.[1];
    const key = word ? `battlesDeckFault${word[0].toUpperCase()}${word.slice(1)}` : '';
    const said = key ? $t(key as Parameters<typeof $t>[0]) : '';
    return said && said !== key ? said : $t('battlesTableSaveFailed');
  }

  async function keep() {
    const token = authStore.token;
    if (!token || saving) return;
    saving = true;
    complaint = null;
    try {
      const table = await api.saveBattleDeck(token, {
        board: placed.map((p) => ({ card: p.card, x: p.x, y: p.y })),
        hand: held,
      });
      deck = table;
      lay(table);
      said = $t('battlesTableSaved');
    } catch (e) {
      complaint = faultLine(e);
    } finally {
      saving = false;
    }
  }
</script>

<svelte:head>
  <title>{$t('battlesTableTitle')} — {$brandName}</title>
  <!-- Стол принадлежит человеку: его незачем ни искать, ни индексировать. -->
  <meta name="robots" content="noindex" />
</svelte:head>

<div class="root">
  <div class="grain" aria-hidden="true"></div>
  <div class="page">
    <nav class="back-nav" in:fade={{ duration: 600 }}>
      <a href="/battles" class="back-link">{$t('battlesTableBack')}</a>
    </nav>

    <header class="masthead">
      <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('battlesTableKicker')}</p>
      <h1 class="page-title">{$t('battlesTableTitle')}</h1>
      <!-- Три вещи, которые страница обязана сказать до того, как её начнут
           трогать: ЧТО это, ЧТО нажимать и ОТКУДА берутся чужие карты.
           Раньше здесь стояли две строки, не отвечавшие ни на один из этих
           вопросов, и попасть отсюда в бой было нельзя вовсе. -->
      <p class="page-rule">{$t('battlesTableWhat')}</p>
      <p class="page-rule">{$t('battlesTableHow')}</p>
      <p class="page-rule page-rule--aside">{$t('battlesTableLoan')}</p>
    </header>

    {#if !signedIn}
      <p class="quiet">{$t('battlesTableSignIn')}</p>
    {:else if loading}
      <p class="quiet">…</p>
    {:else}
      {#if deck?.nothingToLend}
        <p class="quiet">{$t('battlesTableNothingToLend')}</p>
      {/if}
      {#if complaint}
        <p class="complaint" transition:fade={{ duration: 200 }}>{complaint}</p>
      {/if}
      {#if said}
        <p class="said" transition:fade={{ duration: 200 }}>{said}</p>
      {/if}

      <!-- ── Поле ──────────────────────────────────────────────────────── -->
      <section class="band">
        <h2 class="band-title">{$t('battlesTableBoard')}</h2>
        <!-- Колода без единой карты на поле законна, но начинает бой с пустого
             хода. Раньше страница позволяла собрать такую и молчала. -->
        {#if fieldWillBeEmpty}
          <p class="warn">{$t('battlesTableFieldEmpty')}</p>
        {/if}
        <div class="half">
          {#each ROWS as y (y)}
            {#each COLS as x (x)}
              {@const mine = mineAt(x, y)}
              {@const lent = lentAt(x, y)}
              {@const shown = cardOf(mine?.card ?? lent)}
              <button
                type="button"
                class="place"
                class:place--open={!!picked && !mine}
                onclick={() => putOnCell(x, y)}
                aria-label={mine ? $t('battlesTableTake') : $t('battlesTablePut')}
              >
                {#if shown}
                  <BattleCard
                    card={shown}
                    {frames}
                    owned={true}
                    level={mine ? levelOf(mine.card) : null}
                    transition={false}
                    interactive={false}
                  />
                  {#if !mine}
                    <span class="slip">{$t('battlesTableLent')}</span>
                  {:else if goneCard(mine.card) && !dirty}
                    <span class="slip slip--gone">{$t('battlesTableGone')}</span>
                  {/if}
                {:else}
                  <span class="empty-word">{$t('battlesTableEmptySlot')}</span>
                {/if}
              </button>
            {/each}
          {/each}
        </div>
      </section>

      <!-- ── Рука ──────────────────────────────────────────────────────── -->
      <section class="band">
        <h2 class="band-title">{$t('battlesTableHand')}</h2>
        <div class="row">
          {#each [0, 1, 2] as at (at)}
            {@const mine = held[at] ?? null}
            {@const lent = lentInHand(at)}
            {@const shown = cardOf(mine ?? lent)}
            <button
              type="button"
              class="place"
              class:place--open={!!picked && !mine}
              onclick={() => putInHand(at)}
              aria-label={mine ? $t('battlesTableTake') : $t('battlesTablePut')}
            >
              {#if shown}
                <BattleCard
                  card={shown}
                  {frames}
                  owned={true}
                  level={mine ? levelOf(mine) : null}
                  transition={false}
                  interactive={false}
                />
                {#if !mine}
                  <span class="slip">{$t('battlesTableLent')}</span>
                {:else if goneCard(mine) && !dirty}
                  <span class="slip slip--gone">{$t('battlesTableGone')}</span>
                {/if}
              {:else}
                <span class="empty-word">{$t('battlesTableEmptySlot')}</span>
              {/if}
            </button>
          {/each}
        </div>
      </section>

      <!-- Отсюда должен быть виден выход в бой. Без него страница — тупик:
           человек собирал колоду и не узнавал, где ею играют. Пока есть
           несохранённые правки, ссылка не ведёт никуда и говорит почему —
           иначе бой начался бы прежней колодой, а не той, что на экране. -->
      <div class="keep-row">
        <button type="button" class="keep" disabled={saving || !dirty} onclick={keep}>
          {saving ? $t('battlesTableSaving') : $t('battlesTableSave')}
        </button>
        {#if dirty}
          <span class="keep-hint">{$t('battlesTableSaveFirst')}</span>
        {:else}
          <a class="to-battle" href="/battles/etude">{$t('battlesTableToBattle')} →</a>
        {/if}
      </div>

      <!-- ── Ящик ──────────────────────────────────────────────────────── -->
      <section class="band">
        <h2 class="band-title">{$t('battlesTableDrawer')}</h2>
        {#if !drawer.length}
          <p class="quiet">{$t('battlesTableDrawerEmpty')}</p>
          <p class="quiet"><a class="shelf-link" href="/battles">{$t('battlesTableToShelf')} →</a></p>
        {:else}
          <div class="drawer">
            {#each drawer as card (card.id)}
              {@const copy = cardCopy(card, $lang)}
              <button
                type="button"
                class="pull"
                class:pull--picked={picked === card.id}
                onclick={() => pick(card.id)}
                aria-pressed={picked === card.id}
                aria-label={copy.title}
              >
                <BattleCard
                  {card}
                  {frames}
                  owned={true}
                  level={levelOf(card.id)}
                  transition={false}
                  interactive={false}
                />
              </button>
            {/each}
          </div>
        {/if}
      </section>
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
    max-width: 56ch;
    margin: 1rem 0 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1rem;
    line-height: 1.6;
    opacity: 0.74;
  }

  /* Про временные карты — сноска, а не третье равноправное правило. */
  .page-rule--aside {
    font-size: 0.9rem;
    opacity: 0.6;
  }

  .band {
    margin-top: 3rem;
    padding-top: 1.4rem;
    border-top: 1px solid #d8c6b1;
  }

  .band-title {
    margin: 0 0 1.2rem;
    font-size: 0.68rem;
    font-weight: 400;
    letter-spacing: 0.28em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  /* Половина гостя: три в ширину, три в глубину. Ряд 3 сверху — он ближе к
     хранителю, ровно как на доске. */
  .half {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1.4rem;
    max-width: 42rem;
  }

  .row {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1.4rem;
    max-width: 42rem;
  }

  /* Место — рамка на пергаменте, а не кнопка лавки. */
  .place {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 9rem;
    padding: 0.5rem;
    background: none;
    border: 1px dashed rgba(52, 37, 28, 0.18);
    cursor: pointer;
  }

  .place--open {
    border-style: solid;
    border-color: rgba(198, 95, 60, 0.55);
    background: rgba(198, 95, 60, 0.05);
  }

  .empty-word {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.78rem;
    font-style: italic;
    color: #8a6a55;
  }

  /* Бумажная полоска поперёк карты: заём — не другой вид карты, а обычная
     карта с припиской, что она не ваша. */
  .slip {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 1.2rem;
    padding: 0.15rem 0;
    font-size: 0.6rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    text-align: center;
    color: #4a3527;
    background: rgba(248, 241, 231, 0.92);
    border-top: 1px solid rgba(52, 37, 28, 0.16);
    border-bottom: 1px solid rgba(52, 37, 28, 0.16);
  }

  .slip--gone {
    color: #8f2f22;
    border-color: rgba(143, 47, 34, 0.3);
  }

  .drawer {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 1.6rem 1.2rem;
  }

  .pull {
    padding: 0.35rem;
    background: none;
    border: 1px solid transparent;
    cursor: pointer;
  }

  /* Взятая в руку карта чуть выступает из ящика — то же, что делает рука. */
  .pull--picked {
    border-color: rgba(198, 95, 60, 0.55);
    background: rgba(198, 95, 60, 0.06);
    transform: translateY(-6px);
  }

  .keep-row {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 1.5rem;
    margin-top: 2rem;
  }

  /* Выход в бой — главное действие страницы после сохранения, и выглядит он
     заметнее, чем «сохранить»: сохранение это шаг, бой это цель. */
  .to-battle {
    font-size: 0.72rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #f8f1e7;
    background: #34251c;
    padding: 0.5rem 1rem;
    text-decoration: none;
  }

  .to-battle:hover {
    background: #6f3b24;
  }

  .keep-hint {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.85rem;
    font-style: italic;
    color: #8a6a55;
  }

  /* «Оставить» — подчёркнутое слово, а не кнопка лавки: тем же голосом, каким
     полка говорит «Взять». */
  .keep {
    padding: 0;
    font: inherit;
    font-size: 0.72rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #6f3b24;
    background: none;
    border: none;
    border-bottom: 1px solid rgba(111, 59, 36, 0.35);
    cursor: pointer;
  }

  .keep:hover:not(:disabled) {
    color: #c65f3c;
    border-bottom-color: #c65f3c;
  }

  .keep:disabled {
    color: #8a6a55;
    border-bottom-color: transparent;
    cursor: default;
  }

  .quiet {
    margin: 1.5rem 0 0;
    max-width: 56ch;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.92rem;
    font-style: italic;
    line-height: 1.6;
    opacity: 0.66;
  }

  .shelf-link {
    color: #6f3b24;
    text-decoration: none;
    border-bottom: 1px solid rgba(111, 59, 36, 0.3);
  }

  .shelf-link:hover {
    color: #c65f3c;
  }

  .complaint {
    margin: 1.5rem 0 0;
    font-size: 0.85rem;
    color: #8f2f22;
  }

  /* Не отказ, а предупреждение: так собрать можно, но играть будет неудобно. */
  .warn {
    max-width: 56ch;
    margin: 0 0 1.2rem;
    padding-left: 0.8rem;
    border-left: 2px solid #c65f3c;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.88rem;
    line-height: 1.55;
    color: #6f3b24;
  }

  .said {
    margin: 1.5rem 0 0;
    font-size: 0.85rem;
    color: #4a6141;
  }

  /* Обязательство, не украшение. */
  @media (prefers-reduced-motion: reduce) {
    .pull--picked {
      transform: none;
    }
  }
</style>
