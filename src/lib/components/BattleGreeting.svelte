<script lang="ts">
  // Встреча — окно, которое человек видит, когда ему что-то дали.
  //
  // ЕДИНСТВЕННОЕ место в доме, где написано обычными словами, без образов
  // (`BATTLE-ERRANDS.md` §9.1). Комната говорит метафорами: пыль, корм,
  // хранитель, полка. Здесь — нет. Метафора стоила бы ровно того, ради чего
  // окно и заводится: человек, впервые увидевший «книжную пыль» и «корм для
  // ворона», не понимает ни что это, ни откуда берётся, ни зачем ему это —
  // и уходит.
  //
  // Комната остаётся собой. Просто дверь говорит понятно.
  //
  // Открывается только когда дали что-то ИМЕННО СЕЙЧАС. Окно, которое
  // открывается на каждый заход и сообщает ноль, закрывают не читая — а через
  // неделю закрывают вместе с настоящей новостью.
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { t, lang } from '$lib/i18n';
  import { errandHref } from '$lib/battles';
  import WaxSeal from '$lib/components/WaxSeal.svelte';
  import type { BattleErrand, BattleWelcomeGift } from '$lib/types/api';

  let {
    gift = null,
    developedWorks = 0,
    developedDust = 0,
    paid = [],
    errands = [],
    /**
     * `about` — то же окно, открытое вручную по ссылке «Что это такое?».
     *
     * Объяснение про две валюты человек читает здесь один раз, при первом
     * входе, и с полки оно убрано: два абзаца, показываемые каждый визит
     * навсегда, — это не объяснение, а шум. Но убрать совсем нельзя, поэтому
     * то же самое окно открывается по требованию.
     */
    mode = 'greeting',
    onclose,
  }: {
    gift?: BattleWelcomeGift | null;
    developedWorks?: number;
    developedDust?: number;
    paid?: BattleErrand[];
    errands?: BattleErrand[];
    mode?: 'greeting' | 'about';
    onclose: () => void;
  } = $props();

  const titleOf = (e: BattleErrand) => ($lang === 'ru' ? e.titleRu : e.titleEn);
  const noteOf = (e: BattleErrand) => ($lang === 'ru' ? e.noteRu : e.noteEn);

  const coinWord = (currency: 'dust' | 'feed') =>
    currency === 'dust' ? $t('battleGreetDust') : $t('battleGreetFeed');

  /** Строки «что вам дали» — по одной на повод, в порядке появления. */
  type Line = { amount: number; coin: 'dust' | 'feed'; why: string };

  let lines = $derived<Line[]>([
    ...(gift?.dust ? [{ amount: gift.dust, coin: 'dust' as const, why: $t('battleGreetGift') }] : []),
    ...(gift?.feed ? [{ amount: gift.feed, coin: 'feed' as const, why: $t('battleGreetGift') }] : []),
    ...(developedDust > 0
      ? [{ amount: developedDust, coin: 'dust' as const, why: $t('battleGreetDeveloped') }]
      : []),
    ...paid.map((e) => ({ amount: e.amount, coin: e.currency, why: titleOf(e) })),
  ]);

  /**
   * Ближайшие незакрытые. Четыре, а не весь список: четыре читают, девять — нет.
   * Уже начатые идут первыми — человеку ближе то, где он на полпути.
   */
  let next = $derived(
    errands
      // Дела сюда не идут: окно говорит «что можно сделать ПРЯМО СЕЙЧАС», а
      // корм за поступок даёт автор рукой, и предлагать его как задачу — значит
      // обещать выплату, которой машина не сделает. Про вторую монету человек
      // читает выше, во второй части окна, и этого довольно.
      .filter((e) => !e.done && !e.byHand)
      .sort((a, b) => b.have / b.threshold - a.have / a.threshold)
      .slice(0, 4),
  );

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="veil" transition:fade={{ duration: 250 }}>
  <div
    class="sheet"
    role="dialog"
    aria-modal="true"
    aria-label={$t('battleGreetTitle')}
    in:fly={{ y: 18, duration: 500, easing: cubicOut }}
  >
    <div class="seal" aria-hidden="true"><WaxSeal size="3.2rem" /></div>

    <h2 class="title">{mode === 'about' ? $t('battlesWalletAbout') : $t('battleGreetTitle')}</h2>

    <!-- 1. Что вам дали. Сколько, какой монеты и за что — за что важнее числа:
         число без причины и есть тот молча выросший счётчик, от которого дом
         отказывается везде. -->
    {#if mode !== 'about'}
    <ul class="lines">
      {#each lines as line, i (i)}
        <li class="line">
          <span class="num">+{line.amount}</span>
          <span class="coin">{coinWord(line.coin)}</span>
          <span class="why">— {line.why}</span>
        </li>
      {/each}
      {#if developedWorks > 0}
        <li class="aside">{$t('battleGreetDevelopedNote')} {developedWorks}</li>
      {/if}
    </ul>
    {/if}

    <!-- 2. Что это такое. Две строки, без образов: одна про то, что копится
         само, вторая про то, что не копится никогда. -->
    <div class="what">
      <p>{$t('battleGreetWhatDust')}</p>
      <p>{$t('battleGreetWhatFeed')}</p>
      <!-- Правило про уровень переехало сюда с полки: оно относится к кнопке
           «Улучшить», а не к балансу, и в кошельке ему было нечего делать. -->
      {#if mode === 'about'}<p>{$t('battlesLevelIsNotStrength')}</p>{/if}
    </div>

    <!-- 3. Что можно сделать. С ссылкой прямо туда, где это делается: список
         дел, из которого не видно, куда идти, — это список претензий. -->
    {#if next.length}
      <h3 class="next-title">{$t('battleGreetNext')}</h3>
      <ul class="next">
        {#each next as e, i (e.id)}
          {@const href = errandHref(e.rule)}
          <li class="task">
            <div class="task-body">
              <span class="task-name">{titleOf(e)}</span>
              {#if i === 0 && noteOf(e)}<span class="task-note">{noteOf(e)}</span>{/if}
              {#if e.threshold > 1}
                <span class="task-progress">{e.have} {$t('battleGreetOf')} {e.threshold}</span>
              {/if}
            </div>
            <div class="task-side">
              <span class="task-prize">+{e.amount} {coinWord(e.currency)}</span>
              {#if href}
                <a class="task-go" {href}>{$t('battleGreetGo')} →</a>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}

    <button type="button" class="close" onclick={onclose}>{$t('battleGreetClose')}</button>
  </div>
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: center;
    padding: 1.25rem;
    overflow-y: auto;
    background: rgba(24, 16, 11, 0.72);
    backdrop-filter: blur(2px);
  }

  /* Лист бумаги, а не карточка приложения: рамка двойная и слегка повёрнута,
     как всё, что в этом доме кладут на стол. Голос простой — вид домашний. */
  .sheet {
    position: relative;
    width: min(34rem, 100%);
    margin: auto;
    padding: 2rem 1.75rem 1.5rem;
    background: #f8f1e7;
    border: 1px solid #d8c6b1;
    outline: 1px solid #d8c6b1;
    outline-offset: 5px;
    transform: rotate(-0.5deg);
    box-shadow: 0 18px 50px rgba(24, 16, 11, 0.45);
  }

  .seal {
    display: grid;
    place-items: center;
    margin-bottom: 0.75rem;
  }

  .title {
    margin: 0 0 1.1rem;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.5rem;
    font-weight: 500;
    text-align: center;
    color: #34251c;
  }

  .lines {
    margin: 0 0 1.4rem;
    padding: 0;
    list-style: none;
  }

  .line {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 0.4rem;
    padding: 0.4rem 0;
    border-bottom: 1px solid rgba(216, 198, 177, 0.55);
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.92rem;
    color: #34251c;
  }

  .num {
    font-weight: 600;
    color: #6f3b24;
  }

  .coin {
    color: #6f3b24;
  }

  .why {
    color: #7a6353;
  }

  .aside {
    padding-top: 0.5rem;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.8rem;
    color: #8a6a55;
  }

  .what {
    margin-bottom: 1.4rem;
    padding: 0.85rem 0.95rem;
    background: rgba(216, 198, 177, 0.22);
  }

  .what p {
    margin: 0 0 0.55rem;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.85rem;
    line-height: 1.55;
    color: #5c4838;
  }

  .what p:last-child {
    margin-bottom: 0;
  }

  .next-title {
    margin: 0 0 0.7rem;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .next {
    margin: 0 0 1.5rem;
    padding: 0;
    list-style: none;
  }

  .task {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.6rem 0;
    border-bottom: 1px solid rgba(216, 198, 177, 0.55);
  }

  .task-body {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    font-family: 'Inter', system-ui, sans-serif;
  }

  .task-name {
    font-size: 0.9rem;
    color: #34251c;
  }

  .task-note {
    font-size: 0.78rem;
    line-height: 1.45;
    color: #8a6a55;
  }

  .task-progress {
    font-size: 0.75rem;
    color: #c65f3c;
  }

  .task-side {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.2rem;
    white-space: nowrap;
  }

  .task-prize {
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.8rem;
    color: #6f3b24;
  }

  .task-go {
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.78rem;
    color: #c65f3c;
    text-decoration: none;
    border-bottom: 1px solid rgba(198, 95, 60, 0.4);
  }

  .task-go:hover {
    border-bottom-color: #c65f3c;
  }

  .close {
    display: block;
    width: 100%;
    padding: 0.6rem;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.78rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #34251c;
    background: transparent;
    border: 1px solid rgba(52, 37, 28, 0.25);
    cursor: pointer;
  }

  .close:hover {
    background: rgba(52, 37, 28, 0.05);
  }

  @media (prefers-reduced-motion: reduce) {
    .sheet {
      transform: none;
    }
  }
</style>
