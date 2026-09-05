<script lang="ts">
  // Задания — один компонент на страницу игры и на предпросмотр в админке.
  // Второй отрисовщик неизбежно разошёлся бы с первым, и стол хранителя начал
  // бы врать ровно про то, что на нём настраивают.
  //
  // Три раздела, и порядок у них не случайный:
  //   1. СЕГОДНЯ — повторяющиеся. Первыми, потому что это единственное, что
  //      меняется день ото дня, и именно за этим сюда возвращаются.
  //   2. ЗАДАНИЯ — разовые. Показаны не все: три ближайших, остальные по
  //      кнопке. Девять строк с описаниями отодвигали карты вниз, а список
  //      того, что МОЖНО сделать, не должен занимать больше места, чем то,
  //      ради чего сюда пришли.
  //   3. НАГРАДЫ ОТ АВТОРА — выдаются вручную. Ни прогресса, ни отметки, ни
  //      кнопки получения: они просто названы, чтобы игрок видел, за что
  //      бывает вторая валюта.
  import { fade } from 'svelte/transition';
  import { t, lang } from '$lib/i18n';
  import { errandHref } from '$lib/battles';
  import type { BattleErrand } from '$lib/types/api';

  let {
    errands,
    /** Предпросмотр на столе хранителя: ссылки увели бы его со стола. */
    preview = false,
    /** В двери комнат: без внешнего поля, список уже стоит в рамке строки. */
    nest = false,
  }: {
    errands: BattleErrand[];
    preview?: boolean;
    nest?: boolean;
  } = $props();

  /** Сколько разовых заданий видно до нажатия «показать все». */
  const SHOWN = 3;

  let showAll = $state(false);

  const rank = (e: BattleErrand) => e.have / Math.max(1, e.threshold);

  /** Повторяющиеся. Выполненные — вниз, но остаются: иначе не видно, что на
   *  сегодня всё сделано, и человек ищет пропавшую строку. */
  let daily = $derived(
    errands
      .filter((e) => !e.byHand && e.period !== 'once')
      .sort((a, b) => Number(a.done) - Number(b.done)),
  );

  /** Разовые: невыполненные сверху, среди них — начатые первыми. Ближе то, где
   *  человек уже на полпути. */
  let once = $derived(
    errands
      .filter((e) => !e.byHand && e.period === 'once')
      .sort((a, b) => (a.done === b.done ? rank(b) - rank(a) : Number(a.done) - Number(b.done))),
  );

  let deeds = $derived(errands.filter((e) => e.byHand));

  let onceShown = $derived(showAll || preview ? once : once.slice(0, SHOWN));
  let onceHidden = $derived(Math.max(0, once.length - onceShown.length));
  let onceDone = $derived(once.filter((e) => e.done).length);
  let dailyLeft = $derived(daily.filter((e) => !e.done).length);

  const titleOf = (e: BattleErrand) => ($lang === 'ru' ? e.titleRu : e.titleEn);
  const noteOf = (e: BattleErrand) => ($lang === 'ru' ? e.noteRu : e.noteEn);
  const coinOf = (e: BattleErrand) =>
    e.currency === 'dust' ? $t('battleGreetDust') : $t('battleGreetFeed');
  const linkOf = (e: BattleErrand) => (preview || e.done ? null : errandHref(e.rule));

  /**
   * Прогресс — заливкой самой строки, а не полоской под текстом.
   *
   * Полоска стояла сразу под описанием и читалась как подчёркивание: красная
   * черта под серой строчкой выглядит ошибкой правописания, а не «три из пяти».
   * Заливка не добавляет ни пикселя высоты, её ни с чем не спутать, и она
   * растёт слева направо ровно там, где взгляд и так идёт.
   */
  function progressStyle(e: BattleErrand): string {
    if (e.done || e.byHand || e.threshold <= 1 || e.have <= 0) return '';
    const part = Math.min(100, (e.have / e.threshold) * 100);
    // Край растушёван: жёсткая граница читается как приклеенный прямоугольник,
    // а не как «докуда дошло».
    const fade = Math.min(100, part + 4);
    return (
      'background: linear-gradient(to right,' +
      ` rgba(198,95,60,0.085) ${part}%, rgba(198,95,60,0) ${fade}%)`
    );
  }
</script>

{#snippet row(e: BattleErrand, withNote: boolean)}
  {@const href = linkOf(e)}
  <svelte:element
    this={href ? 'a' : 'div'}
    href={href ?? undefined}
    class="task"
    class:task--done={e.done}
    class:task--link={!!href}
    style={progressStyle(e)}
  >
    <span class="task-mark" aria-hidden="true">{e.done ? '✓' : ''}</span>

    <span class="task-main">
      <span class="task-name">{titleOf(e)}</span>
      {#if withNote && !e.done && noteOf(e)}
        <span class="task-note">{noteOf(e)}</span>
      {/if}
    </span>

    <span class="task-side">
      {#if e.done}
        <span class="task-state">{$t('battlesErrandDone')}</span>
      {:else}
        {#if !e.byHand && e.threshold > 1}
          <span class="task-count">{e.have}/{e.threshold}</span>
        {/if}
        <span class="task-prize">
          <i class="coin coin--{e.currency}" aria-hidden="true"></i>{e.amount}
          <span class="task-coin">{coinOf(e)}</span>
        </span>
      {/if}
    </span>
  </svelte:element>
{/snippet}

<section class="tasks" class:tasks--nest={nest} in:fade={{ duration: 500 }}>
  <header class="tasks-head">
    <span class="tasks-title">{$t('battlesErrands')}</span>
    <span class="tasks-tally">
      {$t('battlesTasksDoneOf')}
      {onceDone} {$t('battleGreetOf')} {once.length}
    </span>
  </header>

  <!-- Сегодня. Первым разделом и всегда развёрнуто: это единственное, что
       меняется день ото дня, и именно за этим возвращаются. -->
  {#if daily.length}
    <div class="group group--today">
      <p class="group-head">
        <span class="group-title">{$t('battlesTasksToday')}</span>
        <span class="group-note">
          {dailyLeft === 0 ? $t('battlesTasksTodayClear') : $t('battlesTasksTodayLeft')}
        </span>
      </p>
      {#each daily as e (e.id)}{@render row(e, true)}{/each}
    </div>
  {/if}

  <div class="group">
    <p class="group-head">
      <span class="group-title">{$t('battlesTasksOnce')}</span>
    </p>
    {#each onceShown as e, i (e.id)}{@render row(e, i < SHOWN && !e.done)}{/each}
    {#if onceHidden > 0}
      <button type="button" class="more" onclick={() => (showAll = true)}>
        {$t('battlesTasksMore')} ({onceHidden})
      </button>
    {:else if showAll && once.length > SHOWN}
      <button type="button" class="more" onclick={() => (showAll = false)}>
        {$t('battlesTasksLess')}
      </button>
    {/if}
  </div>

  {#if deeds.length}
    <div class="group">
      <p class="group-head">
        <span class="group-title">{$t('battlesDeeds')}</span>
      </p>
      <p class="group-rule">{$t('battlesDeedsRule')}</p>
      {#each deeds as e (e.id)}{@render row(e, true)}{/each}
    </div>
  {/if}
</section>

<style>
  .tasks {
    max-width: 44rem;
    margin: 2rem 0 0;
    border: 1px solid #d8c6b1;
    background: rgba(255, 253, 249, 0.55);
    color: #34251c;
  }

  .tasks--nest {
    margin: 0;
    max-width: none;
  }

  .tasks-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid rgba(216, 198, 177, 0.7);
  }

  .tasks-title {
    font-size: 0.7rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .tasks-tally {
    font-size: 0.78rem;
    color: #8a6a55;
    font-variant-numeric: tabular-nums;
  }

  .group {
    padding: 0.55rem 0 0.7rem;
    border-bottom: 1px solid rgba(216, 198, 177, 0.45);
  }

  /* «Сегодня» — единственный раздел, который меняется день ото дня, и глаз
     должен находить его без чтения заголовков. */
  .group--today {
    background: rgba(198, 95, 60, 0.035);
  }

  .group--today .group-title {
    color: #c65f3c;
  }

  .group:last-child {
    border-bottom: none;
  }

  .group-head {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    margin: 0 0 0.15rem;
    padding: 0 1rem;
  }

  .group-title {
    font-size: 0.64rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .group-note,
  .group-rule {
    font-size: 0.72rem;
    color: #8a6a55;
    opacity: 0.85;
  }

  .group-rule {
    margin: 0 1rem 0.35rem;
    line-height: 1.45;
  }

  /* ── Строка ───────────────────────────────────────────────────────────
     Вся строка — ссылка туда, где задание делается. Стрелка в углу была
     мишенью в двенадцать пикселей; строка целиком попадается и мышью, и
     пальцем. */
  .task {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    padding: 0.45rem 1rem;
    text-decoration: none;
    color: inherit;
  }

  .task--link {
    cursor: pointer;
  }

  .task--link:hover {
    background: rgba(198, 95, 60, 0.06);
  }

  .task--done {
    opacity: 0.5;
  }

  .task-mark {
    width: 0.85rem;
    flex-shrink: 0;
    font-size: 0.8rem;
    line-height: 1.5;
    color: #c65f3c;
  }

  .task-main {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.12rem;
  }

  .task-name {
    font-size: 0.88rem;
    line-height: 1.4;
  }

  .task-note {
    font-size: 0.75rem;
    line-height: 1.45;
    color: #8a6a55;
  }

  .task-side {
    flex-shrink: 0;
    display: flex;
    align-items: baseline;
    gap: 0.55rem;
    font-size: 0.78rem;
    color: #6f3b24;
    font-variant-numeric: tabular-nums;
  }

  .task-count {
    color: #c65f3c;
  }

  .task-prize {
    display: inline-flex;
    align-items: baseline;
    gap: 0.28rem;
    white-space: nowrap;
  }

  .coin {
    width: 0.42rem;
    height: 0.42rem;
    border-radius: 50%;
    align-self: center;
    background: #cbb79c;
  }

  .coin--feed {
    background: #6f3b24;
  }

  .task-coin {
    color: #8a6a55;
  }

  .task-state {
    font-size: 0.75rem;
    font-style: italic;
    color: #8a6a55;
  }

  .more {
    display: block;
    margin: 0.35rem 1rem 0;
    padding: 0;
    font-family: inherit;
    font-size: 0.75rem;
    color: #c65f3c;
    background: none;
    border: none;
    border-bottom: 1px solid rgba(198, 95, 60, 0.35);
    cursor: pointer;
  }

  /* На узком экране слово-название монеты уходит: число и цвет точки говорят
     то же самое, а строка перестаёт переноситься. */
  @media (max-width: 30rem) {
    .task-coin {
      display: none;
    }
  }
</style>
