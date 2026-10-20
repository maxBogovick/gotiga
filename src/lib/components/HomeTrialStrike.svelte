<script lang="ts">
  // Пробный удар на главной: Капитан Салазар против Сшитого.
  //
  // Посетитель ходит за Салазара, ход Сшитого играется сам. Это не партия
  // движка, а записанная заранее короткая стычка, но числа в ней те, что дал
  // бы движок (`battle-core`, `Rules::default()`):
  //   - молния Салазара — магический урон, броня его не держит, защиты от чар
  //     у Сшитого нет: 4 урона;
  //   - булава Сшитого: 5 минус 1 брони Салазара — 4;
  //   - вплотную дальнобойный бьёт вполсилы (`point_blank_power` = 50): 2;
  //   - шаг не тратит ход (`walk_spends_turn` = false), поэтому «отступить и
  //     ударить» — один ход.
  // Обе карты — Нежить, урон по расам между ними не действует.
  //
  // Рисует тот же `BattleCard`, движения — тот же `stage()` и
  // `BattleMotionStage`, что и сцена боя: второй отрисовщик однажды соврал бы.
  // Стоит строкой в правой колонке первого экрана (`.hero-doors`), рядом с
  // мастерской, вестником и байками, и оформлена как они: светлая строка с
  // волосяной чертой сверху, заглавие Cormorant, кнопки как у призывов
  // страницы. Тёмной плашкой она выбивалась из колонки.
  //
  // Сама играет: после загрузки Салазар бьёт, Сшитый отвечает, на развилке
  // демо выбирает «отступить и ударить» и показывает победу, потом начинает
  // заново. Любое нажатие забирает управление: автоигра останавливается и
  // больше не возвращается, дальше ходит посетитель. Автоигра ждёт, пока плашка
  // на экране и вкладка видна, пока над кнопками указатель или в них фокус, и
  // не включается при `prefers-reduced-motion`. Её действия в замер не идут:
  // `trial_strike` считает только нажатия людей.
  // Монтирует её главная после загрузки (`deferHomeExtras`), карты, рамки и
  // движения читаются в браузере: главная пререндерится, а карты меняются
  // чаще, чем её собирают. Карт нет — плашки нет.
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import { interact } from '$lib/analytics';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import BattleMotionStage from '$lib/components/BattleMotionStage.svelte';
  import BattleIcon from '$lib/components/BattleIcon.svelte';
  import WaxSeal from '$lib/components/WaxSeal.svelte';
  import {
    DEFAULT_ASPECT,
    cellPrints,
    frameForCard,
    motionFor,
    motionSpan,
    motionWound,
    stage,
    statMark,
    struckOf,
    type HitWear,
    type Staged,
  } from '$lib/battles';
  import type { TranslationKey } from '$lib/i18n';
  import type { BattleCard as BattleCardDto, BattleFrame, Motion } from '$lib/types/api';

  type Who = 'sal' | 'sti';
  type Phase = 'start' | 'busy' | 'choose' | 'won';

  const SLUG: Record<Who, string> = { sal: 'captain-salazar', sti: 'the-creature' };
  const SEED: Record<Who, number> = { sal: 1, sti: 2 };
  const CELLS = 4;
  const START: Record<Who, number> = { sal: 1, sti: 3 };
  /** Сколько длится шаг на клетку — столько же, сколько переход `left`. */
  const WALK_MS = 520;
  /** Урон молнии Салазара издали — то же число, что в сводке выше. */
  const BOLT = 4;

  let cards = $state<Record<Who, BattleCardDto> | null>(null);
  let frames = $state<BattleFrame[]>([]);
  let motions = $state<Motion[]>([]);
  let calm = $state(false);
  let failed = $state(false);

  let pos = $state<Record<Who, number>>({ ...START });
  let hp = $state<Record<Who, number>>({ sal: 0, sti: 0 });
  let fallen = $state<Record<Who, boolean>>({ sal: false, sti: false });
  let phase = $state<Phase>('start');
  let line = $state<TranslationKey>('trialStrikeLineStart');

  let play = $state<Staged | null>(null);
  let playKey = $state(0);
  let acting = $state<{ striker: Who | null; target: Who } | null>(null);
  let wound = $state<{ who: Who; hit: HitWear } | null>(null);

  /** Поколение стычки: «ещё раз» посреди хода гасит всё, что ещё ждёт. */
  let gen = 0;
  /** Номер запуска автоигры: нажатие посетителя увеличивает его, и прежний
   *  запуск, дойдя до очередной паузы, уходит. */
  let autoRun = 0;
  /** Кнопка, которую демо «нажимает» прямо сейчас (подсвечивается). */
  let autoPick = $state(false);
  /** Идёт ли показ сам: пока да, скринридеру не читают смену фразы хода. */
  let auto = $state(false);
  let root: HTMLElement | undefined = $state();
  let inView = true;
  let holding = false;

  const maxOf = (who: Who) => cards?.[who].health ?? 1;
  const aspectOf = (who: Who) =>
    cards ? frameForCard(cards[who], frames).aspect || DEFAULT_ASPECT : DEFAULT_ASPECT;
  /** Карта, которая сама печатает здоровье, своего кружка под собой не
   *  получает: два отрисовщика одного числа — два числа в одном углу. */
  const printsHealth = (who: Who) =>
    !!cards && cellPrints(frameForCard(cards[who], frames), 'healthMark');
  const spot = (who: Who) => ({ x: pos[who], y: 0 });
  const wait = (ms: number) => new Promise<void>((r) => setTimeout(r, Math.max(0, ms)));

  function stirOf(who: Who): string | undefined {
    if (!acting || !play) return undefined;
    if (acting.striker === who && play.striker) return play.striker;
    if (acting.target === who && play.target) return play.target;
    return undefined;
  }

  /** Один удар: движение, след на карте, число. Возвращает false, если
   *  стычку за это время начали заново. */
  async function strike(from: Who, to: Who, dmg: number, my: number): Promise<boolean> {
    if (!cards) return false;
    const card = cards[from];
    const far = Math.abs(pos[from] - pos[to]) > 1;
    const magic = card.attackChannel === 'magic';
    const motion = motionFor('blow', card, motions, far ? { magic } : null);
    const max = maxOf(to);
    const after = Math.max(0, hp[to] - dmg);
    const contact = calm ? 0 : motionWound(motion);
    const hit: HitWear = {
      remain: after / max,
      blow: dmg / max,
      seed: SEED[to],
      channel: magic ? 'magic' : 'physical',
      at: contact,
    };
    play = stage(motion, spot(from), spot(to), {
      spanX: CELLS,
      spanY: 1,
      along: false,
      calm,
    });
    acting = { striker: from, target: to };
    wound = null;
    playKey += 1;
    await wait(contact);
    if (my !== gen) return false;
    hp[to] = after;
    wound = { who: to, hit };
    await wait(Math.max(calm ? 0 : motionSpan(motion) - contact, 0) + 420);
    if (my !== gen) return false;
    acting = null;
    play = null;
    if (after === 0) return fall(to, my);
    return true;
  }

  async function fall(who: Who, my: number): Promise<boolean> {
    if (!cards) return false;
    const motion = motionFor('fall', cards[who], motions);
    play = stage(motion, null, spot(who), { spanX: CELLS, spanY: 1, along: false, calm });
    acting = { striker: null, target: who };
    playKey += 1;
    await wait(calm ? 0 : motionSpan(motion));
    if (my !== gen) return false;
    fallen[who] = true;
    acting = null;
    play = null;
    return true;
  }

  async function walk(who: Who, to: number, my: number): Promise<boolean> {
    pos[who] = to;
    await wait(calm ? 0 : WALK_MS + 120);
    return my === gen;
  }

  /** Три хода возвращают true, если дошли до конца, и false, если стычку
   *  за это время забрали или начали заново. */
  async function firstBlow(): Promise<boolean> {
    if (phase !== 'start') return false;
    const my = ++gen;
    phase = 'busy';
    if (!(await strike('sal', 'sti', BOLT, my))) return false;
    line = 'trialStrikeLineHit';
    await wait(900);
    if (my !== gen) return false;
    // Ход Сшитого: шаг на клетку — и булава.
    line = 'trialStrikeLineWalk';
    if (!(await walk('sti', 2, my))) return false;
    if (!(await strike('sti', 'sal', 4, my))) return false;
    line = 'trialStrikeLineChoose';
    phase = 'choose';
    return true;
  }

  /** Второй ход единственной кнопки: отступить на клетку и ударить в полную
   *  силу. Вплотную молния бьёт вполсилы, поэтому здесь ходит выигрышный ход. */
  async function stepBack(): Promise<boolean> {
    if (phase !== 'choose') return false;
    const my = ++gen;
    phase = 'busy';
    if (!(await walk('sal', 0, my))) return false;
    if (!(await strike('sal', 'sti', BOLT, my))) return false;
    line = 'trialStrikeLineWon';
    phase = 'won';
    return true;
  }

  /** Нажатие посетителя: автоигра снимается насовсем, дальше ходит он. */
  function takeOver(kind: string) {
    autoRun += 1;
    autoPick = false;
    auto = false;
    interact('trial_strike', kind);
  }
  /** Кнопка «Ударить» одна на всю стычку: что она делает, решает ход. Пока
   *  идёт ход, нажатие игнорируется. */
  function clickStrike(): Promise<boolean> | undefined {
    if (!cards || phase === 'busy') return;
    if (phase === 'choose') return (takeOver('step_back'), stepBack());
    if (ended) {
      takeOver('again');
      reset();
      return firstBlow();
    }
    return (takeOver('strike'), firstBlow());
  }

  /** Пауза автоигры: стоит, пока плашки не видно, вкладка скрыта или над
   *  кнопками указатель. Возвращает false, если запуск сняли. */
  async function pause(run: number, ms: number): Promise<boolean> {
    let left = ms;
    while (left > 0) {
      if (run !== autoRun) return false;
      await wait(100);
      if (inView && !holding && !document.hidden) left -= 100;
    }
    return run === autoRun;
  }

  async function autopilot(run: number) {
    while (run === autoRun) {
      if (!(await pause(run, 1800))) return;
      if (!(await firstBlow()) || run !== autoRun) return;
      // На развилке демо думает, как человек, и берёт отступление.
      if (!(await pause(run, 2600))) return;
      autoPick = true;
      if (!(await pause(run, 700))) return;
      autoPick = false;
      if (!(await stepBack()) || run !== autoRun) return;
      if (!(await pause(run, 6000))) return;
      reset();
    }
  }

  function reset() {
    gen += 1;
    pos = { ...START };
    hp = { sal: cards?.sal.health ?? 0, sti: cards?.sti.health ?? 0 };
    fallen = { sal: false, sti: false };
    play = null;
    acting = null;
    wound = null;
    line = 'trialStrikeLineStart';
    phase = 'start';
  }

  async function load() {
    const [all, fr, mo] = await Promise.all([
      api.getBattleCards(),
      api.getBattleFrames(),
      api.getBattleMotions(),
    ]);
    const sal = all.find((c) => c.slug === SLUG.sal);
    const sti = all.find((c) => c.slug === SLUG.sti);
    if (!sal || !sti) {
      failed = true;
      return;
    }
    frames = fr.frames ?? [];
    motions = mo.motions ?? [];
    cards = { sal, sti };
    reset();
    if (!calm) {
      auto = true;
      autopilot(autoRun);
    }
  }

  onMount(() => {
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    calm = mq.matches;
    const onCalm = () => (calm = mq.matches);
    mq.addEventListener('change', onCalm);
    const io = new IntersectionObserver(
      (list) => (inView = list[list.length - 1].isIntersecting),
      { threshold: 0.4 },
    );
    if (root) io.observe(root);
    load().catch(() => (failed = true));
    return () => {
      gen += 1;
      autoRun += 1;
      io.disconnect();
      mq.removeEventListener('change', onCalm);
    };
  });

  let ended = $derived(phase === 'won');

  /** Место под счётчик здоровья нужно, только если карта сама его не печатает. */
  let needsTally = $derived(!!cards && (!printsHealth('sal') || !printsHealth('sti')));
</script>

{#if !failed}
<div class="mini-wrap">
<section
  class="mini"
  class:mini--ready={!!cards}
  aria-labelledby="trial-title"
  bind:this={root}
>
  <div class="cloth" class:cloth--tally={needsTally}>
    <div class="field" style="--cells:{CELLS}">
      <div class="grid" aria-hidden="true">
        {#each Array.from({ length: CELLS }, (_, i) => i) as i (i)}
          <span class="cell"></span>
        {/each}
      </div>

      {#if cards}
        {#each ['sal', 'sti'] as const as who (who)}
          <div class="unit" class:unit--fallen={fallen[who]} style="--at:{pos[who]}">
            {#key playKey}
              <span class="figure" style:--fit={aspectOf(who)} style={stirOf(who)}>
                <BattleCard
                  card={cards[who]}
                  {frames}
                  owned={true}
                  transition={false}
                  interactive={false}
                  hurt={hp[who] / maxOf(who)}
                  alive={hp[who]}
                  wearSeed={SEED[who]}
                  struck={wound?.who === who && acting ? struckOf(wound.hit) : null}
                  scrap={wound?.who === who && acting && struckOf(wound.hit) === 'bruise'
                    ? { blow: wound.hit.blow, remain: wound.hit.remain, seed: wound.hit.seed }
                    : null}
                />
              </span>
            {/key}
            {#if !fallen[who]}
              {#if ended}
                <span
                  class="crest"
                  role="img"
                  aria-label={$t('battleWonByPlayer')}
                >
                  <WaxSeal size="1.7rem" />
                </span>
              {/if}
              {#if !printsHealth(who)}
                <span class="tally" aria-label={`${$t('battlesHealthLabel')}: ${hp[who]}`}>
                  <BattleIcon name={statMark('health')} size={11} />
                  <b>{hp[who]}</b>
                </span>
              {/if}
            {/if}
          </div>
        {/each}

        {#key playKey}
          <BattleMotionStage motes={play?.motes ?? []} />
        {/key}
      {/if}
    </div>
  </div>

  <div class="side">
    <h2 id="trial-title" class="title">
      {$t('trialStrikeTitle')
        .replace('{a}', $t('trialStrikeSalazar'))
        .replace('{b}', $t('trialStrikeStitched'))}
    </h2>
    <div
      class="acts"
      role="group"
      aria-label={$t('trialStrikeTitle')
        .replace('{a}', $t('trialStrikeSalazar'))
        .replace('{b}', $t('trialStrikeStitched'))}
      onpointerenter={() => (holding = true)}
      onpointerleave={() => (holding = false)}
      onfocusin={() => (holding = true)}
      onfocusout={() => (holding = false)}
    >
      <!-- Две кнопки, и они не меняются: «Ударить» делает то, что сейчас нужно
           по ходу (молния, затем отступить и ударить, затем заново), «Сыграть»
           ведёт в этюды. Пока идёт ход, нажатие молча игнорируется — вид
           кнопки при этом не меняется. -->
      <button
        type="button"
        class="act act--main"
        class:act--chosen={autoPick}
        aria-disabled={phase === 'busy' || !cards}
        onclick={clickStrike}
      >
        {$t('trialStrikeStrike')}
      </button>
      <a
        class="act"
        href="/battles/etude"
        onclick={() => interact('world_link', 'home_to_etude')}
      >{$t('trialStrikePlay')}</a>
    </div>
  </div>

  <!-- На экране строки хода нет, но скринридеру она нужна: что произошло, он
       узнаёт только отсюда. Во время показа фраза меняется сама, и читать её
       вслух каждые несколько секунд нельзя — только в ответ на ход читателя. -->
  <p class="sr-only" aria-live={auto ? 'off' : 'polite'}>{$t(line)}</p>
</section>
</div>
{/if}

<style>
  /* Карточка в колонке первого экрана: мягкая светлая подложка со скруглением,
     как плитки соседних входов. Слева название и кнопки, справа поле с картами
     (карты стоят прямо на подложке, без внутренней рамки), под ними строка
     хода. Узкая колонка (телефон) кладёт всё в столбик — контейнерный запрос,
     потому что решает ширина колонки, а не окна. */
  .mini-wrap {
    container-type: inline-size;
  }
  .mini {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.45fr);
    grid-template-areas: 'side field';
    gap: 12px;
    margin: 10px 0;
    padding: 14px 12px 12px 16px;
    color: var(--ink);
    background: color-mix(in srgb, var(--color-canvas-sunken, #f0e9da) 72%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 6%, transparent);
    border-radius: 14px;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.55);
  }

  .cloth {
    grid-area: field;
    position: relative;
    align-self: center;
    /* Линия земли под картами: поле без рамки иначе не имеет низа. Карта шире
       своей клетки, и крайняя выступала за поле: отступ по бокам держит её над
       линией, а не за её концом. Место под счётчиком здоровья — снизу. */
    padding-inline: 2.5%;
    border-bottom: 1px solid var(--border2);
  }
  .cloth--tally {
    margin-bottom: 20px;
  }
  .field {
    position: relative;
    /* Клетка уже и выше карты: карта шире своей клетки на пятую часть и чуть
       заходит на соседние. Так карты крупнее при той же ширине поля; когда
       противники стоят вплотную, они перекрываются краями. */
    --cell: 0.55;
    aspect-ratio: calc(var(--cells) * var(--cell));
  }
  .grid {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-columns: repeat(var(--cells), minmax(0, 1fr));
  }
  .cell {
    border-right: 1px solid rgba(52, 37, 28, 0.07);
  }
  .cell:nth-child(2) {
    border-right-color: rgba(198, 95, 60, 0.32);
  }
  .cell:last-child {
    border-right: none;
  }
  /* Пока карты не пришли, поле дышит, а не пустует. */
  .mini:not(.mini--ready) .cell {
    animation: trial-wait 1.6s ease-in-out infinite alternate;
  }
  @keyframes trial-wait {
    from { background: rgba(52, 37, 28, 0.015); }
    to { background: rgba(52, 37, 28, 0.06); }
  }

  .unit {
    position: absolute;
    top: 0;
    bottom: 0;
    left: calc(var(--at) * 100% / var(--cells));
    width: calc(100% / var(--cells));
    transition:
      left 520ms cubic-bezier(0.45, 0, 0.25, 1),
      transform 700ms ease,
      filter 700ms ease;
    z-index: 1;
  }
  /* Выбитая карта не бледнеет до призрака, а оседает и темнеет: она по-прежнему
     на поле, и видно, что с ней случилось. */
  .unit--fallen {
    transform: translateY(7%) rotate(4deg);
    filter: grayscale(1) brightness(0.72);
    z-index: 0;
  }
  /* Печать победы лежит на углу карты победителя: слов и вуали на поле нет. */
  .crest {
    position: absolute;
    top: -6px;
    right: -9%;
    z-index: 4;
    line-height: 0;
    filter: drop-shadow(0 2px 3px rgba(52, 37, 28, 0.35));
    animation: trial-crest 520ms ease both;
  }
  @keyframes trial-crest {
    from { opacity: 0; transform: scale(0.6) rotate(-12deg); }
    to { opacity: 1; transform: none; }
  }
  .figure {
    display: grid;
    place-items: center;
    height: 100%;
    padding: 1.5%;
    box-sizing: border-box;
    overflow: visible;
    will-change: transform;
    filter: drop-shadow(0 5px 7px rgba(52, 37, 28, 0.38));
  }
  .figure > :global(.slot) {
    width: calc(var(--fit, 0.714) / var(--cell) * 100%);
  }
  .tally {
    position: absolute;
    left: 50%;
    top: 100%;
    margin-top: 5px;
    transform: translateX(-50%);
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 7px;
    font-family: Georgia, serif;
    font-size: 12px;
    color: #f6ead6;
    background: #6f3b24;
    border: 1px solid #c65f3c;
    border-radius: 999px;
    z-index: 3;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
  }

  .side {
    grid-area: side;
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 6px;
    min-width: 0;
  }
  .title {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(19px, 4.6cqi, 24px);
    font-weight: 500;
    line-height: 1.1;
    color: var(--ink);
  }
  .acts {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px;
    margin-top: 8px;
  }
  /* Кнопки — как призывы первого экрана («Смотреть работы» / «Написать автору»):
     главное действие — тёмная таблетка, вторая — контурная, шрифт страницы,
     заглавные. Две, в один ряд, и они не меняются по ходу стычки. */
  .act {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    min-width: 0;
    gap: 6px;
    min-height: 32px;
    padding: 4px 10px;
    font-family: Inter, system-ui, sans-serif;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    line-height: 1.2;
    text-transform: uppercase;
    text-align: center;
    white-space: nowrap;
    text-decoration: none;
    color: var(--ink);
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 32%, transparent);
    border-radius: 999px;
    cursor: pointer;
    transition:
      background 0.22s ease,
      border-color 0.22s ease,
      color 0.22s ease,
      transform 0.18s ease;
  }
  .act--main {
    color: var(--cream2);
    background: var(--ink);
    border-color: var(--ink);
  }
  .act:hover,
  .act--chosen {
    color: var(--copper);
    border-color: var(--copper);
  }
  .act--main:hover,
  .act--main.act--chosen {
    color: var(--cream2);
    background: var(--mid);
    border-color: var(--mid);
  }
  .act:active {
    transform: translateY(1px);
  }
  .act:focus-visible {
    outline: 2px solid rgba(198, 95, 60, 0.56);
    outline-offset: 3px;
  }

  @container (max-width: 360px) {
    .mini {
      grid-template-columns: 1fr;
      grid-template-areas: 'side' 'field';
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .crest,
    .mini:not(.mini--ready) .cell {
      animation: none;
    }
    .unit,
    .act {
      transition: none;
    }
  }
</style>
