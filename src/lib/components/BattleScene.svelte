<script lang="ts">
  // Стол с фигурами, а не панель показателей.
  //
  // Ни одного правила здесь нет и быть не должно: пометки выводятся из
  // `legalActions`, которые уже посчитал движок. Клиент не знает ни дальности,
  // ни маны, ни покрова — он знает, куда можно ткнуть. Всё, что сцена делает
  // сама, — это показывает и выдерживает паузы.
  //
  // Описание комнаты целиком — `BATTLE-SCENE.md`. Всё, что ниже похоже на
  // произвол, объяснено там.
  import { onMount, untrack } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { t, lang, type TranslationKey } from '$lib/i18n';
  import BattleGroundMark from './BattleGroundMark.svelte';
  import BattleGroundSigil from './BattleGroundSigil.svelte';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import BattleIcon from '$lib/components/BattleIcon.svelte';
  import BattleSheet from '$lib/components/BattleSheet.svelte';
  import BattleMotionStage from '$lib/components/BattleMotionStage.svelte';
  import WaxSeal from '$lib/components/WaxSeal.svelte';
  import {
    DEFAULT_ASPECT,
    firstToHand,
    HOLD_WORDS,
    holdKind,
    intentLive,
    intentsOf,
    riderWord,
    RIDER_STATS,
    statMark,
    statLabel,
    type MarkedStat,
    bodyPassport,
    cardCopy,
    channelLabelKey,
    cellPrints,
    frameForCard,
    kindLabelKey,
    MOTION_MS_MAX,
    motionFor,
    motionSpan,
    motionWound,
    occasionOf,
    rulesInForce,
    terrainLines,
    GROUND_KEY,
    GROUNDS,
    GROUND_NAME,
    GROUND_SHORT,
    stage,
    struckOf,
    WARD_MOTION,
    type HitWear,
    type Intent,
    type ScrapFly,
    type Staged,
    type StruckKind,
  } from '$lib/battles';
  import type {
    BattleAction,
    BattleCard as BattleCardDto,
    SheetSlot,
    BattleCell,
    BattleEvent,
    BattleFrame,
    BattleMatch,
    BattleMatchState,
    BattleUnit,
    Foresight,
    Motion,
  } from '$lib/types/api';
  import './battle-chamber.css';

  let {
    match,
    cards = [],
    frames = null,
    motions = null,
    busy = false,
    control = 'player',
    onact,
    onforesee,
    onreplay,
    onleave,
    onexit,
    fill = false,
  }: {
    match: BattleMatch;
    cards?: BattleCardDto[];
    /** Рамки чинов. Без них карта наденет рамку по умолчанию. */
    frames?: BattleFrame[] | null;
    /** Свод движений. Пустой — комната играет умолчания дома, то есть ровно
     *  то, что делала до движка (`BATTLE-MOTION.md` §4). */
    motions?: Motion[] | null;
    busy?: boolean;
    /** `both` — стол хранителя: ходить можно за обе стороны. */
    control?: 'player' | 'both';
    onact: (action: BattleAction) => void;
    /**
     * «Если сделать это и на этом закончить ход — чем ответит хранитель».
     *
     * Приходит колбэком, а не запросом отсюда: сцена не знает ни одного адреса
     * и ни одного правила, и это второе тоже. Не передан — предвестия в этой
     * комнате нет вовсе, и стол хранителя обходится без него.
     */
    onforesee?: (action: BattleAction) => Promise<Foresight | null>;
    /** Под печатью. Обе необязательны: у стола хранителя их нет. */
    onreplay?: () => void;
    onleave?: () => void;
    /** Шапка комнаты: спросить, уйти. Не `onleave` — тот уже «назад к списку» под печатью. */
    onexit?: () => void;
    /**
     * Этюд занимает окно: доска берёт оставшуюся высоту, шапка кабинета
     * при этом снята. Стол хранителя в 24rem это не передаёт — там сцена
     * складывается в колонку и меряет себя содержимым.
     */
    fill?: boolean;
  } = $props();

  /** Величина поля этой партии: клеток поперёк и рядов от края до края.
   *  Партия, начатая до ручки, приходит без поля — это 3 × 3 на половину. */
  let WIDTH = $derived(match.state.field?.width ?? 3);
  let HALF = $derived(match.state.field?.depth ?? 3);
  let DEPTH = $derived(HALF * 2);
  /** Комната, в которой стол ложится вдоль: шесть портретов в ряд на более
   *  узкой — штампы. Совпадает с `.scene--along`. Полоса хода больше не ест
   *  тринадцать колонн слева, поэтому порог чуть ниже прежних 1100. */
  const ALONG = 960;

  // ── Темп ──────────────────────────────────────────────────────────────────
  //
  // Числа из BATTLE-SCENE.md §6 переехали в свод движений: длительность такта
  // теперь говорит САМО движение (`BATTLE-MOTION.md` §3.4), а не константа
  // здесь. Умолчания дома дают ровно те же 300/400/300/500, так что комната от
  // переезда не изменилась — это проверено в `battles.rs`.
  //
  // Шаг остался числом: перемещение по клеткам — не движение из свода, а
  // перекладка, и словаря жестов у неё нет.
  const BEAT_MOVED = 350;
  const REST = 500;

  /** `prefers-reduced-motion` — не украшение, а обязательство: при нём весь ход
   *  применяется мгновенно, а разбор остаётся доступен. */
  let calm = $state(false);
  onMount(() => {
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    calm = mq.matches;
    const listen = () => (calm = mq.matches);
    mq.addEventListener('change', listen);
    return () => mq.removeEventListener('change', listen);
  });

  // ── Что показано ──────────────────────────────────────────────────────────
  //
  // Обычно — ровно то, что прислал сервер. Во время проигрывания журнала —
  // промежуточная запись: события переписываются на снимок предыдущей позиции
  // по одному. Это не счёт правил, а перепись готового ответа: событие уже
  // говорит, сколько снято и куда шагнули. По окончании доска снимается со
  // снимка сервера, так что любое расхождение живёт две секунды и умирает.
  let live = $state<BattleMatchState | null>(null);
  let position = $derived(live ?? match.state);
  let legal = $derived(match.legalActions);

  /** Снимок, с которого начнётся проигрывание следующей посылки. Не руна:
   *  ничего от него не перерисовывается. */
  let before: BattleMatchState | null = null;
  /** Для доски: пока журнал играет, она не принимает касаний. */
  let playing = $state(false);
  /** События, которые сцена уже показала. Не `match.events`: пакет приходит
   *  целиком, а журнал не должен выдавать ход хранителя до его такта. */
  let told = $state<BattleEvent[]>([]);
  /** То же самое, но не руна: эффект ниже обязан читать это без подписки. */
  let running = false;
  let run = 0;

  let me = $derived(control === 'both' ? position.active : 'player');
  let mine = $derived(position.active === me && !position.outcome && !busy && !playing);

  const dtoOf = (slug: string) => cards.find((c) => c.slug === slug) ?? null;
  const titleOf = (slug: string) => {
    const dto = dtoOf(slug);
    return dto ? cardCopy(dto, $lang).title || slug : slug;
  };

  /** Отношение сторон рамки этой карты. Клетке нужно знать его заранее: карта,
   *  которой позволено мерить себя по содержимому, из клетки вылезает. */
  const aspectOf = (dto: BattleCardDto) =>
    frameForCard(dto, frames).aspect || DEFAULT_ASPECT;

  /**
   * Печатает ли ЭТА карта число сама, стоя в клетке.
   *
   * Спрашивается у описи той рамы, которую карта носит, а не у дома: наряд
   * может снять значок, и тогда число обязана сказать доска. Нет карты (тело
   * без карточки) — не говорит никто, и оба кружка доски остаются.
   *
   * Функцией, а не `{@const}` в разметке: тот обязан стоять непосредственным
   * ребёнком блока, а спрашивают здесь изнутри обычного элемента.
   */
  const cardSays = (dto: BattleCardDto | null | undefined, slot: SheetSlot) =>
    !!dto && cellPrints(frameForCard(dto, frames), slot);

  let byCell = $derived(
    new Map(position.board.map((s) => [`${s.cell.x},${s.cell.y}`, s.unit])),
  );
  /** Местность поля, по клетке. */
  /** Какие земли лежат на этом поле — по строке на род, в порядке `GROUNDS`
   *  (сперва запрет, потом опасность, потом подмога). Легенда стоит рядом с
   *  полем всегда, а не за кнопкой листка: правило, которое надо искать,
   *  узнают, только споткнувшись о него. */
  let legend = $derived(
    GROUNDS.filter((g) => (position.terrain ?? []).some((t) => t.ground === g)),
  );
  let grounds = $derived(
    new Map((position.terrain ?? []).map((t) => [`${t.cell.x},${t.cell.y}`, t.ground] as const)),
  );
  /** Прорыв в этой партии: дальние ряды — цели, и они помечены. */
  let edges = $derived(!!position.rules?.breakthrough);
  /** Кто закончил ход своей стороны на чужом краю: если простоит ход
   *  противника — партия его. Помечен рамкой, а не словом: стоящий там виден. */
  let poised = $derived(new Set(edges ? (position.poised ?? []) : []));
  /** Кто решил партию прорывом. Читается из доски, а не из события: события
   *  есть только у хода, которым партия кончилась, а печать видна и после
   *  перезагрузки. */
  let breachedBy = $derived.by<number | null>(() => {
    const o = position.outcome;
    if (!edges || (o !== 'player' && o !== 'keeper')) return null;
    const goal = o === 'player' ? 0 : DEPTH - 1;
    for (const id of position.poised ?? []) {
      const u = position.units[id];
      const at = position.board.find((s) => s.unit === id)?.cell;
      if (u && u.owner === o && at?.y === goal) return id;
    }
    return null;
  });
  const unitAt = (x: number, y: number): BattleUnit | null => {
    const id = byCell.get(`${x},${y}`);
    return id === undefined ? null : (position.units[id] ?? null);
  };
  const cellOf = (unit: number): BattleCell | null => {
    const spot = position.board.find((s) => s.unit === unit);
    return spot ? spot.cell : null;
  };
  /**
   * Опасна ли эта клетка — и чья она.
   *
   * Читается из партии, а не вычисляется: зона живёт в состоянии, потому что
   * котёл остаётся стоять и когда ведьмы не стало.
   */
  const zoneAt = (x: number, y: number) =>
    (position.zones ?? []).find((z) => z.cells.some((c) => c.x === x && c.y === y)) ?? null;

  /** Королевский шаг — та же мерка, какой меряет движок (`Cell::distance`). */
  const stride = (a: BattleCell, b: BattleCell) =>
    Math.max(Math.abs(a.x - b.x), Math.abs(a.y - b.y));

  // ── Выбор ─────────────────────────────────────────────────────────────────
  type Picked = { kind: 'unit'; id: number } | { kind: 'hand'; index: number } | null;
  let picked = $state<Picked>(null);
  /** Тот же лист, что с полки и со стола. Не `state`: `$state` тогда подписка. */
  let sheet = $state<BattleCardDto | null>(null);

  /** Клетки, куда выбранное может встать или шагнуть. */
  let openCells = $derived.by(() => {
    const out = new Set<string>();
    if (!picked || !mine) return out;
    for (const a of legal) {
      if (typeof a === 'string') continue;
      if (picked.kind === 'hand' && 'play' in a && a.play.handIndex === picked.index) {
        out.add(`${a.play.cell.x},${a.play.cell.y}`);
      }
      if (picked.kind === 'unit' && 'move' in a && a.move.unit === picked.id) {
        out.add(`${a.move.to.x},${a.move.to.y}`);
      }
    }
    return out;
  });

  // ── Чем заняться ──────────────────────────────────────────────────────────
  //
  // Пока тело умело одно, выбора не было: подсветилось — ткнули. У карты,
  // которая бьёт, лечит и проклинает, в одну и ту же чужую клетку ведут ТРИ
  // разных дела, и подсветка перестала отвечать на вопрос «что случится».
  // Поэтому сперва выбирается СПОСОБ, а доска показывает цели только его.
  //
  // Правил здесь по-прежнему нет: способы собраны из `legalActions`, и цель
  // несёт готовое действие из того же списка, которое уходит назад неизменным.

  /** Способы, которыми выбранное тело может заняться делом. */
  let intents = $derived.by((): Intent[] => {
    if (picked?.kind !== 'unit') return [];
    const u = position.units[picked.id];
    if (!u || u.owner !== me) return [];
    return intentsOf(u, dtoOf(u.card.name), legal, position[me].mana, $lang);
  });

  /**
   * Что в руке. Ключ намерения, а не оно само: список пересобирается на каждую
   * посылку, и рука, державшая объект, держала бы вчерашний.
   */
  let toolHeld = $state<string | null>(null);
  /**
   * Чего коснулись, не взяв. Наведение на печать ПОКАЗЫВАЕТ её цели, не
   * выбирая: иначе узнать, кого достаёт проклятие, можно только сделав выбор,
   * а выбор тут тратит ход. Тот же приём, что у колодца движений (`tasting`).
   */
  let toolTasted = $state<string | null>(null);

  /** Рука сама берёт то, чем есть что сделать; выбор человека её перебивает. */
  let toolInHand = $derived.by((): Intent | null => {
    const byKey = toolHeld ? intents.find((i) => i.key === toolHeld) : null;
    return byKey ?? firstToHand(intents);
  });
  /** Чем доска светится прямо сейчас: тронутое пересиливает взятое. */
  let toolShown = $derived.by((): Intent | null => {
    const tasted = toolTasted ? intents.find((i) => i.key === toolTasted) : null;
    return tasted ?? toolInHand;
  });

  /**
   * Поднос выходит тогда, когда тело знает БОЛЬШЕ ОДНОГО способа, — и знает,
   * а не может: спящая чара, которой не показали, отличается от несуществующей
   * только памятью хранителя. Одним способом поднос не выходит вовсе: прежний
   * жест остаётся прежним у всех, кто умеет одно.
   */
  let trayShown = $derived(
    mine && !playing && picked?.kind === 'unit' && intents.length > 1,
  );

  /**
   * Где лежит поднос: колонка и ряд СЕТКИ, а не пиксели.
   *
   * Местом в сетке, потому что геометрия доски уже посчитана ею самой, а
   * второй расчёт — в долях или в пикселях — разошёлся бы с ней на первом же
   * повороте стола вдоль комнаты.
   *
   * Лежит он НА теле, чуть ниже середины карты: рука тянется к тому, что
   * лежит на фигуре, и глазу не надо уходить с неё, чтобы выбрать. Под картой
   * поднос стоял раньше и был честнее к рисунку — но выбор оказывался в другом
   * месте, чем то, о чём он.
   *
   * Две оговорки о краях, и обе читаются с номера клетки, а не с линейки:
   * у последнего ряда подпись уходит НАД печатями (под ними ей уже нет места);
   * крайней колонке некуда расширяться — поднос прижимается к её кромке вместо
   * того, чтобы встать посередине и уехать за сукно.
   */
  let trayAt = $derived.by(() => {
    if (!trayShown || picked?.kind !== 'unit') return null;
    const at = cellOf(picked.id);
    if (!at) return null;
    const col = along ? DEPTH - at.y : at.x + 1;
    const row = along ? at.x + 1 : at.y + 1;
    const cols = along ? DEPTH : WIDTH;
    const rows = along ? WIDTH : DEPTH;
    return {
      col,
      row,
      over: row === rows,
      edge: col === 1 ? 'start' : col === cols ? 'end' : 'mid',
    };
  });

  /** Почему этой печатью сейчас нельзя. Откат, мана, некого — в этом порядке:
   *  откат единственный из трёх не виден на доске ничем. */
  function toolWhy(i: Intent): string {
    if (i.spent) return $t('battleIntentSpent');
    if (i.asleep !== null) return $t('battleIntentAsleep').replace('{n}', String(i.asleep));
    if (i.dear) return $t('battleIntentMana');
    // У клеточной чары «некого» — это «некуда», и слово должно быть своё:
    // человек ищет глазами тело, а ткнуть надо в пустое место. Спрашивается
    // это у САМОЙ чары, а не у того, есть ли у неё сейчас клетки: пустой ящик
    // не говорит, какого рода цель он не нашёл.
    return $t(i.atSpot ? 'battleIntentNoRoom' : 'battleIntentNoAim');
  }

  /** Та же подпись словами — для чтения вслух и для наведения мышью. Одна
   *  строка на оба, иначе читающему с голоса однажды скажут не то, что нарисовано. */
  function toolSay(i: Intent): string {
    const out = [i.name || $t(i.word)];
    if (i.amount !== null) out.push(String(i.amount));
    if (i.stat) out.push($t(RIDER_STATS[i.stat].label));
    if (i.turns) out.push(`${$t('battleStatusTurns')} ${i.turns}`);
    if (i.mana) out.push(`${$t('battleManaShort')} ${i.mana}`);
    if (!intentLive(i)) out.push(toolWhy(i));
    return out.join(' · ');
  }

  /** Чем помечены цели того, что в руке. Разница не в цвете, а в форме. */
  const MARK_OF = { blow: 'attack', mend: 'mend', cast: 'charm' } as const;

  /**
   * Клетки, в которые можно ткнуть ТЕМ, ЧТО В РУКЕ: опасная клетка, призыв,
   * свой шаг. Ключ — `x,y`, значение — готовое действие из `legalActions`.
   *
   * Отдельно от `openCells` (куда шагнуть и куда выложить), и не из
   * аккуратности: одна и та же клетка бывает и тем, и другим, а значит на
   * нажатие должен отвечать кто-то ОДИН. Отвечает взятое в руку — оно и есть
   * выбор человека; шаг остаётся доступен, пока в руке ничего клеточного нет.
   */
  let openSpots = $derived.by(() => {
    const out = new Map<string, BattleAction>();
    if (picked?.kind !== 'unit' || !mine) return out;
    const tool = toolShown;
    if (!tool) return out;
    for (const [cell, action] of tool.spots) out.set(cell, action);
    return out;
  });

  /** Тела, которые можно взять ТЕМ, ЧТО В РУКЕ, — и готовое действие к каждому. */
  let openUnits = $derived.by(() => {
    const out = new Map<number, { mark: 'attack' | 'mend' | 'charm'; action: BattleAction }>();
    if (picked?.kind !== 'unit' || !mine) return out;
    const tool = toolShown;
    if (!tool) return out;
    for (const [unit, action] of tool.aims) {
      out.set(unit, { mark: MARK_OF[tool.kind], action });
    }
    return out;
  });

  /** Тела, которым вообще есть чем ходить, — чтобы не тыкать в пустое. */
  let ready = $derived.by(() => {
    const out = new Set<number>();
    for (const a of legal) {
      if (typeof a === 'string') continue;
      if ('move' in a) out.add(a.move.unit);
      if ('attack' in a) out.add(a.attack.attacker);
      if ('mend' in a) out.add(a.mend.healer);
      if ('cast' in a) out.add(a.cast.caster);
    }
    return out;
  });

  // ── Предвестие ────────────────────────────────────────────────────────────
  //
  // Ход хранителя ВЫЧИСЛИМ: случайности в движке нет, скрытых карт у него нет,
  // `reduce` чиста. Человек с карандашом получил бы то же самое, только за
  // полчаса. Прятать вычислимое — значит продавать не глубину, а неудобство;
  // «Into the Breach» показывает следующий удар врага целиком, и игра от этого
  // стала не проще, а глубже: перестаёшь угадывать, начинаешь считать.
  //
  // Вопрос поставлен ровно так: «если сделать это и НА ЭТОМ ЗАКОНЧИТЬ ХОД».
  // Иначе ответа не существует — хранитель отвечает не на удар, а на конец
  // хода, и предвестие после удара показывало бы ответ на несделанный ход.
  //
  // Считает сервер. Сцена по-прежнему не знает ни одного правила и ни одного
  // адреса: она задаёт вопрос колбэком и показывает, что ответили.

  const FORESIGHT_KEY = 'gotiga_battle_foresight';
  /** Придержано на удар: провести мышью по доске — это не вопрос. */
  const FORESIGHT_HOLD = 140;

  let foresight = $state(true);
  onMount(() => {
    try {
      foresight = localStorage.getItem(FORESIGHT_KEY) !== 'off';
    } catch {
      // Приватное окно — предвестие просто останется включённым.
    }
  });
  function toggleForesight() {
    foresight = !foresight;
    if (!foresight) forget();
    try {
      localStorage.setItem(FORESIGHT_KEY, foresight ? 'on' : 'off');
    } catch {
      // Не сохранилось — переживём: это удобство, а не состояние партии.
    }
  }

  let foretold = $state<Foresight | null>(null);
  /** По какому действию оно посчитано. Ключ, а не флаг: показывать вчерашнее
   *  предвестие под сегодняшним наведением — худшее из возможных вранья. */
  let foretoldKey = $state<string | null>(null);
  /** Спрошенное однажды не спрашивается снова: тот же ход при том же номере
   *  даёт тот же ответ — это и есть чистота `reduce`, только с той стороны. */
  let foreseen = new Map<string, Foresight>();
  let asking: string | null = null;
  let holdOff: ReturnType<typeof setTimeout> | null = null;

  const foresightKey = (action: BattleAction) => `${match.seq}:${JSON.stringify(action)}`;

  function forget() {
    if (holdOff) clearTimeout(holdOff);
    holdOff = null;
    asking = null;
    foretold = null;
    foretoldKey = null;
  }

  /** Доска ушла вперёд — прежние ответы больше не про неё. */
  $effect(() => {
    match.seq;
    untrack(() => {
      foreseen.clear();
      forget();
    });
  });

  function ponder(action: BattleAction | null) {
    if (!onforesee || !foresight || !mine || !action) {
      forget();
      return;
    }
    const key = foresightKey(action);
    if (key === foretoldKey) return;
    const known = foreseen.get(key);
    if (known) {
      foretold = known;
      foretoldKey = key;
      return;
    }
    if (holdOff) clearTimeout(holdOff);
    asking = key;
    holdOff = setTimeout(async () => {
      const got = await onforesee!(action);
      // Пока считали, увели мышь или сходили — ответ уже не про то, на что
      // человек смотрит.
      if (!got || asking !== key) return;
      foreseen.set(key, got);
      foretold = got;
      foretoldKey = key;
    }, FORESIGHT_HOLD);
  }

  /** Сколько снимут с каждого тела и кто при этом падёт. */
  function toll(events: BattleEvent[]) {
    const out = new Map<number, { off: number; falls: boolean }>();
    const at = (id: number) => {
      const had = out.get(id) ?? { off: 0, falls: false };
      out.set(id, had);
      return had;
    };
    for (const ev of events) {
      if (typeof ev === 'string') continue;
      if ('damaged' in ev) {
        const row = at(ev.damaged.target);
        row.off += ev.damaged.toHealth + ev.damaged.toShield;
      }
      if ('healed' in ev) at(ev.healed.target).off -= ev.healed.amount;
      if ('died' in ev) at(ev.died.target).falls = true;
    }
    return out;
  }

  /** Что сделает само действие, и чем на это ответят. Порознь: своё и чужое на
   *  доске должны читаться по-разному, иначе предвестие — просто россыпь чисел. */
  let toldMine = $derived(foretold ? toll(foretold.yours) : new Map());
  let toldTheirs = $derived(foretold ? toll(foretold.theirs) : new Map());

  /** Ответ хранителя словами. Метки на доске говорят «сколько и с кого», а
   *  строка — «кто и по кому», и без неё при дальности четыре число меняется
   *  на другом конце поля без всякого видимого автора. */
  let foretoldWords = $derived.by(() => {
    const out: { by: string; target: string; off: number; falls: boolean }[] = [];
    if (!foretold) return out;
    const falls = new Set<number>();
    for (const ev of foretold.theirs) {
      if (typeof ev !== 'string' && 'died' in ev) falls.add(ev.died.target);
    }
    for (const ev of foretold.theirs) {
      if (typeof ev === 'string' || !('damaged' in ev)) continue;
      const d = ev.damaged;
      const who = d.by === null ? null : (position.units[d.by] ?? null);
      const whom = position.units[d.target] ?? null;
      // Только по своим. Плату за простой хранитель платит и со своих тел —
      // на доске она видна меткой, но в строке «чем ответит противник» чужая
      // потеря читалась бы как угроза.
      if (!whom || whom.owner !== me) continue;
      out.push({
        by: who ? titleOf(who.card.name) : '',
        target: titleOf(whom.card.name),
        off: d.toHealth + d.toShield,
        falls: falls.has(d.target),
      });
    }
    return out;
  });

  let hand = $derived(me === 'player' ? position.player.hand : position.keeper.hand);
  let theirHand = $derived(me === 'player' ? position.keeper.hand : position.player.hand);

  let playableHand = $derived.by(() => {
    const out = new Set<number>();
    for (const a of legal) {
      if (typeof a !== 'string' && 'play' in a) out.add(a.play.handIndex);
    }
    return out;
  });

  /**
   * Почему из руки нельзя выложить ни одной карты.
   *
   * Движок про это молчит: он присылает список законных действий, и если
   * выложить нечего, «выложить» в нём просто нет. Отличить «не хватает маны» от
   * «некуда ставить» по пустому списку невозможно, и человек видел только
   * тускловатые карты, которые не нажимаются, — без единого слова о причине.
   *
   * Сравнение цены с маной здесь — не второе правило игры, а объяснение уже
   * принятого сервером решения: играть по нему нельзя, оно только называет то,
   * что и так видно на карте.
   */
  let handTrouble = $derived.by((): 'mana' | 'room' | null => {
    if (!mine || !hand.length || playableHand.size > 0) return null;
    const cheapest = Math.min(...hand.map((h) => h.cost));
    return cheapest > position.player.mana ? 'mana' : 'room';
  });

  /** Удар по подсказке, когда ткнули в цель вне досягаемости. */
  let tipBeat = $state(0);
  /**
   * Спросили ли слово. Отказ — ответ на ЖЕСТ, а не подпись к выбору: тело,
   * которому есть куда идти, не застряло, и висящая над ним причина, почему
   * оно вдобавок не бьёт, повторяется на каждом теле стороны и читается как
   * брань. Молча выбранное тело слова не получает — получает ткнувший.
   */
  let tipAsked = $state(false);
  $effect(() => {
    picked;
    position;
    tipAsked = false;
    // Рука пустеет вместе с выбором: печать, взятая у прошлого тела, на этом
    // значит другое — а ключ у неё тот же (`blow` есть у всякого, кто бьёт).
    toolHeld = null;
    toolTasted = null;
  });

  /**
   * Сколько ходов ждать, пока лечение этого тела вернётся, — и только тогда,
   * когда откат ЕДИНСТВЕННОЕ, что стоит на пути.
   *
   * «Единственное» — это условие, а не осторожность. Откат на теле, которому
   * всё равно некого лечить, назван не был бы причиной, а был бы вторым
   * ответом на чужой вопрос; поэтому дар считается спящим, только когда
   * раненый свой стоит в пределах дара и маны на дар хватает.
   *
   * Возвращается число, а не слово: «вернётся» без «когда» — та же тишина,
   * только вежливее.
   */
  function mendAsleep(u: BattleUnit): number | null {
    // Тело, которое лечит само, лечить не перестало: у его лечения отката нет.
    if (u.mend > 0) return null;

    const from = cellOf(u.id);
    if (!from) return null;

    const left = new Map((u.abilityCds ?? []).map((c) => [c.id, c.left]));
    const mana = position[me].mana;
    let soonest: number | null = null;

    const abilities = u.card.abilities ?? [];
    for (let i = 0; i < abilities.length; i++) {
      const a = abilities[i];
      if (a.verb !== 'heal' || a.trigger !== 'active' || a.amount <= 0) continue;
      // Те же две пригоршни, что берёт `ready_heal`: остальные движок не играет.
      if (a.shape !== 'one' && a.shape !== 'self') continue;
      // Пустой `id` всё равно ключ — так его завёл движок (`ability_key`).
      const cd = left.get(a.id || `#${i}`) ?? 0;
      // Хоть один готовый дар — и отказ уже не в откате, что бы ни спало рядом.
      if (cd === 0) return null;
      if (a.manaCost > mana) continue; // Держит мана, а не откат.
      const someone = position.units.some((x) => {
        if (x.owner !== u.owner || x.health.current <= 0) return false;
        if (x.health.current >= x.health.max) return false;
        if (a.shape === 'self') return x.id === u.id;
        if (x.id === u.id) return false;
        const to = cellOf(x.id);
        return !!to && stride(from, to) <= a.range;
      });
      if (!someone) continue;
      soonest = soonest === null ? cd : Math.min(soonest, cd);
    }

    return soonest;
  }

  /**
   * Почему выбранное сейчас не бьёт / не лечит / не ходит / не ложится на стол.
   *
   * Не второе правило: список `legalActions` уже решил, что можно. Здесь только
   * имя отказа — по тем же признакам, которые движок уже положил в тело и в
   * правила партии (`acted`, opening, mana, откат), плюс по тому, чего в списке
   * нет (есть шаг — нет удара → вне досягаемости). Иначе человек видит голубое
   * свечение и молчание.
   *
   * Слово несёт своё число: отказ «через сколько-то ходов» без числа человек
   * читает как «никогда».
   */
  let stuck = $derived.by((): { key: TranslationKey; turns?: number } | null => {
    if (!mine || playing || position.outcome) return null;

    if (picked?.kind === 'hand') {
      if (playableHand.has(picked.index)) return null;
      const card = hand[picked.index];
      if (!card) return null;
      return { key: card.cost > position.player.mana ? 'battleWhyMana' : 'battleWhyRoom' };
    }

    if (picked?.kind !== 'unit') return null;
    const u = position.units[picked.id];
    if (!u || u.owner !== me) return null;

    if (openUnits.size > 0 || openSpots.size > 0) return null;

    // Откат назван ПЕРВЫМ, прежде досягаемости, и это не прихоть порядка: он
    // единственный из отказов не виден на доске ничем — ни расстоянием, ни
    // камнями маны, — а «шагните ближе» на спящем даре обещает то, чего шаг
    // не даст. Досягаемость человек и так измеряет глазами.
    const asleep = mendAsleep(u);
    if (asleep !== null) return { key: 'battleWhyCooldown', turns: asleep };

    const foesAlive = position.units.some((x) => x.owner !== me && x.health.current > 0);
    const canStrike = u.power > 0;
    const canMend =
      u.mend > 0 ||
      (u.card.abilities ?? []).some(
        (a) => a.verb === 'heal' && a.trigger === 'active' && a.amount > 0,
      );
    // Чары спрашиваются у уже собранного подноса, а не у карты второй раз:
    // тело, у которого всё спит, «не наносит ударов» — неправда, и ровно про
    // такое тело эта строка и печаталась бы чаще всего.
    const canCast = intents.some((i) => i.kind === 'cast');

    const { rules } = position;

    // Запрет первого круга держит УДАР, а не шаг, — и потому назван ПРЕЖДЕ
    // досягаемости, как откат. Стоя под шагом, он молчал у всякого тела,
    // которому есть куда идти, а «шагните ближе» обещало то, чего шаг не
    // даст: подойти вплотную к телу, бить которое в этом круге уже нельзя.
    // Сколько ударов израсходовано, на доске не написано нигде.
    if (
      position.round === 1 &&
      position.active === 'player' &&
      me === 'player' &&
      canStrike &&
      foesAlive &&
      position.openingAttacksUsed >= rules.openingAttacks
    ) {
      return { key: 'battleWhyOpening' };
    }

    // Шаг есть, удара нет — ровно случай «выбрана, а бить нельзя».
    if (openCells.size > 0) {
      if (canStrike && foesAlive) return { key: 'battleWhyReach' };
      return null;
    }

    if (u.acted) return { key: 'battleWhyActed' };

    if (position.actsThisTurn >= rules.actsPerTurn) return { key: 'battleWhyActs' };

    if (u.moved && rules.walkSpendsTurn) return { key: 'battleWhyActed' };

    if (!canStrike && !canMend && !canCast) return { key: 'battleWhyPeace' };

    if (canStrike && foesAlive) {
      return { key: u.step === 0 ? 'battleWhyStuck' : 'battleWhyReach' };
    }

    return { key: 'battleWhyIdle' };
  });

  let stuckWord = $derived(stuck?.key ?? null);
  /**
   * Тело, которому нечем ходить вовсе: ни шага, ни удара, ни лечения. Только
   * оно получает слово без спроса — голубое свечение и молчание и есть та
   * поломка, ради которой отказ заведён.
   */
  let bodyIdle = $derived(
    picked?.kind === 'unit' &&
      openCells.size === 0 &&
      openUnits.size === 0 &&
      openSpots.size === 0,
  );
  /** Видно ли слово сейчас. Своё тело молчит, пока его не спросили. */
  let stuckShown = $derived(
    !!stuckWord && (picked?.kind === 'hand' || bodyIdle || tipAsked),
  );
  /**
   * Отказ словами, с уже подставленным числом. Подстановка ОДНА на все места,
   * где отказ печатается, — иначе строка с `{n}` однажды выйдет на карту как
   * есть, и ровно в том месте, куда реже всего смотрят.
   */
  let stuckSay = $derived(stuck ? $t(stuck.key).replace('{n}', String(stuck.turns ?? '')) : '');


  let chosen = $derived(picked?.kind === 'unit' ? (position.units[picked.id] ?? null) : null);
  let chosenDto = $derived(chosen ? dtoOf(chosen.card.name) : null);
  let chosenFace = $derived(chosenDto ? cardCopy(chosenDto, $lang) : null);
  let chosenPass = $derived(chosenDto ? bodyPassport(chosenDto) : []);
  let chosenKind = $derived(chosenDto ? $t(kindLabelKey(chosenDto.kind)) : '');
  let chosenChannel = $derived(
    chosenDto
      ? (() => {
          const key = channelLabelKey(chosenDto.attackChannel);
          return key ? $t(key) : '';
        })()
      : '',
  );

  /**
   * Что случится, если ткнуть в эту клетку сейчас. `null` — ткнуть означает
   * выбрать или прочитать, а не сходить.
   *
   * Одна функция на нажатие и на предвестие. Две разошлись бы — и предвестие
   * стало бы обещать не тот ход, чего не видно ни в одном тесте.
   */
  function actionAt(x: number, y: number): BattleAction | null {
    if (!mine) return null;
    const here = unitAt(x, y);
    // Действие НЕ собирается здесь: оно лежит готовым в намерении, откуда
    // пришло из `legalActions`, и уходит назад неизменным. Собранное вручную
    // пришлось бы собирать и для чары — по ключу умения, которого у сцены нет.
    if (here && picked?.kind === 'unit') {
      const aim = openUnits.get(here.id);
      if (aim) return aim.action;
    }
    // Клеточная чара отвечает ПЕРВОЙ: она в руке, и значит её и выбрали.
    const spot = openSpots.get(`${x},${y}`);
    if (spot) return spot;
    if (!here && picked && openCells.has(`${x},${y}`)) {
      const cell: BattleCell = { x, y };
      return picked.kind === 'hand'
        ? { play: { handIndex: picked.index, cell } }
        : { move: { unit: picked.id, to: cell } };
    }
    return null;
  }

  function tapCell(x: number, y: number) {
    if (playing) return;
    const here = unitAt(x, y);

    if (mine) {
      const move = actionAt(x, y);
      if (move) {
        onact(move);
        return;
      }

      // Своё тело в руке, а ткнули куда ход не ведёт — не прыгаем на чужой
      // лист и не снимаем выбор: бьём в колокол причины.
      if (picked?.kind === 'unit') {
        const held = position.units[picked.id];
        if (held?.owner === me) {
          if (here && here.owner !== me) {
            tipAsked = true;
            tipBeat += 1;
            return;
          }
          if (!here && !openCells.has(`${x},${y}`) && stuckWord) {
            tipAsked = true;
            tipBeat += 1;
            return;
          }
        }
      }

      // Готовое своё тело — выбор для хода. Подсветка клеток только у него.
      if (here && here.owner === me && ready.has(here.id)) {
        picked =
          picked?.kind === 'unit' && picked.id === here.id ? null : { kind: 'unit', id: here.id };
        return;
      }

      // Своё, но ходить нечем — всё равно берём в руку, чтобы сказать почему.
      if (here && here.owner === me) {
        picked =
          picked?.kind === 'unit' && picked.id === here.id ? null : { kind: 'unit', id: here.id };
        return;
      }
    }

    // Любое тело можно открыть, чтобы прочитать. Ходов это не предлагает.
    if (here) {
      picked =
        picked?.kind === 'unit' && picked.id === here.id ? null : { kind: 'unit', id: here.id };
    } else {
      picked = null;
    }
  }

  /**
   * Взять печать с подноса.
   *
   * Мёртвая печать не отказывает и не молчит: нажатие на неё ПРИКАЛЫВАЕТ её к
   * подписи, и та говорит, почему нельзя. На мыши это же делает наведение, но
   * на телефоне наведения нет, а спящая чара без слова — ровно та поломка, из-за
   * которой отказ в этой комнате вообще завёлся.
   */
  function takeTool(intent: Intent) {
    if (!intentLive(intent)) {
      toolTasted = intent.key;
      return;
    }
    toolHeld = intent.key;
    toolTasted = null;
  }

  function tapHand(index: number) {
    if (!mine) return;
    picked = picked?.kind === 'hand' && picked.index === index ? null : { kind: 'hand', index };
  }

  /** Нажатие мимо печати закрывает листок: он лежит поверх доски, и ходить
   *  сквозь него нельзя. Ловится на `pointerdown`, до нажатия клетки. */
  function offLeaf(e: PointerEvent) {
    if (!leafOpen) return;
    const el = e.target as HTMLElement | null;
    if (el?.closest?.('.seal-wrap')) return;
    leafOpen = false;
  }

  function onkey(e: KeyboardEvent) {
    // Цифра берёт печать с подноса — по порядку, как она на нём лежит. Нигде не
    // написано и написано не будет: подсказок «нажмите сюда» в этой комнате нет,
    // а рука, которая однажды попробовала, помнит сама.
    if (trayShown && !leafOpen && !sheet && /^[1-9]$/.test(e.key)) {
      const intent = intents[Number(e.key) - 1];
      if (intent) {
        takeTool(intent);
        e.stopImmediatePropagation();
        return;
      }
    }
    if (e.key !== 'Escape') return;
    // Листок закрывается первым: он лежит поверх всего, и Escape при открытом
    // листке — про листок, а не про выбранное тело под ним.
    if (leafOpen) {
      leafOpen = false;
      e.stopImmediatePropagation();
      return;
    }
    if (logOpen) {
      logOpen = false;
      e.stopImmediatePropagation();
      return;
    }
    if (sheet) {
      sheet = null;
      e.stopImmediatePropagation();
      return;
    }
    if (picked) {
      picked = null;
      e.stopImmediatePropagation();
    }
  }

  // ── Движение ──────────────────────────────────────────────────────────────
  //
  // Что играется прямо сейчас: какие два тела заняты и что с ними делается.
  // Стиль приходит готовой строкой из `stage()` — сцена его не собирает, иначе
  // стол хранителя показывал бы одно, а комната другое.
  let acting = $state<{
    striker: number | null;
    target: number | null;
    play: Staged;
    struck: StruckKind | null;
    hit: HitWear | null;
    contact: number;
  } | null>(null);
  /** Обломок живёт на карте дольше такта, чтобы его было видно. */
  let flying = $state<(ScrapFly & { unit: number; key: number }) | null>(null);
  let flyKey = 0;

  /** Стиль для тела на клетке: пусто, если оно сейчас не занято. */
  const stirOf = (id: number): string | undefined => {
    if (!acting) return undefined;
    if (acting.striker === id && acting.play.striker) return acting.play.striker;
    if (acting.target === id && acting.play.target) return acting.play.target;
    return undefined;
  };

  function dropActing() {
    acting = null;
  }

  function openWound(target: number, hit: HitWear | null) {
    const kind = struckOf(hit);
    if (acting) acting = { ...acting, struck: kind };
    if (kind === 'bruise' && hit) {
      const key = ++flyKey;
      flying = { unit: target, blow: hit.blow, remain: hit.remain, seed: hit.seed, key };
      setTimeout(() => {
        if (flying?.key === key) flying = null;
      }, 580);
    }
  }

  function hitOf(pos: BattleMatchState, e: BattleEvent): HitWear | null {
    if (!('damaged' in e)) return null;
    const u = pos.units[e.damaged.target];
    if (!u) return null;
    const max = Math.max(1, u.health.max);
    return {
      remain: Math.max(0, u.health.current - e.damaged.toHealth) / max,
      blow: e.damaged.toHealth / max,
      seed: e.damaged.target,
      channel: e.damaged.channel,
      source: e.damaged.source,
      at: 0,
    };
  }

  /** Разложить событие на сцене. Клетки берутся из показанной позиции: тело,
   *  которое ещё не сдвинулось, стоит там, где его видно. */
  function put(
    pos: BattleMatchState,
    motion: Motion | null,
    striker: number | null,
    target: number | null,
    hit: HitWear | null = null,
  ): Staged {
    const spotOf = (id: number | null) =>
      id == null ? null : (pos.board.find((s) => s.unit === id)?.cell ?? null);
    const wound = calm ? 0 : motionWound(motion);
    const wear = hit ? { ...hit, at: wound } : null;
    const play = stage(motion, spotOf(striker), spotOf(target), {
      spanX: along ? DEPTH : WIDTH,
      spanY: along ? WIDTH : DEPTH,
      along,
      calm,
    });
    acting = {
      striker,
      target,
      play,
      struck: null,
      hit: wear,
      contact: wound,
    };
    return play;
  }

  /** Какое движение играет это тело на этом поводе. Цепочка — в `battles.ts`. */
  const motionOf = (pos: BattleMatchState, unit: number | null, event: BattleEvent) => {
    const occasion = occasionOf(event);
    if (!occasion) return null;
    const dto = unit == null ? null : dtoOf(pos.units[unit]?.card.name ?? '');
    return motionFor(occasion, dto, motions);
  };

  const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

  /**
   * Снимок позиции, отвязанный от рун.
   *
   * Не `structuredClone`: то, что приходит сюда, — прокси Svelte, и он на них
   * падает. Позиция целиком состоит из чисел, строк и списков, так что круг
   * через JSON — честная копия, а не приближение.
   */
  const copy = <T,>(x: T): T => JSON.parse(JSON.stringify(x)) as T;

  /** Переписать одно событие на показанную позицию. Не решение — перепись. */
  function transcribe(pos: BattleMatchState, e: BattleEvent) {
    if ('played' in e) {
      // Тело, которого в снимке ещё нет: берётся из окончательной позиции,
      // но целым — карта выходит на поле целой, а раны придут своими событиями.
      const born = copy(match.state.units[e.played.unit]);
      if (born) {
        born.health = { ...born.health, current: born.health.max };
        pos.units[e.played.unit] = born;
        pos.board = [...pos.board, { cell: e.played.cell, unit: e.played.unit }];
      }
    } else if ('moved' in e) {
      pos.board = pos.board.map((s) =>
        s.unit === e.moved.unit ? { ...s, cell: e.moved.to } : s,
      );
    } else if ('fell' in e) {
      // Тело переезжает на клетку оврага и там оседает: `died` идёт следом.
      pos.board = pos.board.map((s) => (s.unit === e.fell.unit ? { ...s, cell: e.fell.to } : s));
    } else if ('damaged' in e) {
      const u = pos.units[e.damaged.target];
      if (u) {
        u.shield = Math.max(0, u.shield - e.damaged.toShield);
        u.health = { ...u.health, current: Math.max(0, u.health.current - e.damaged.toHealth) };
      }
    } else if ('healed' in e) {
      const u = pos.units[e.healed.target];
      if (u) {
        u.health = {
          ...u.health,
          current: Math.min(u.health.max, u.health.current + e.healed.amount),
        };
      }
    } else if ('rider' in e) {
      const u = pos.units[e.rider.target];
      if (u) {
        // Одноимённый освежает срок, а не встаёт вторым: то же правило, что у
        // движка (`apply_status`). Здесь это не решение, а перепись готового —
        // но перепись, считающая иначе, нарисовала бы лишнюю зарубку.
        const had = u.statuses.findIndex((st) => st.name === e.rider.status.name);
        u.statuses =
          had >= 0
            ? u.statuses.map((st, i) => (i === had ? e.rider.status : st))
            : [...u.statuses, e.rider.status];
      }
    } else if ('shielded' in e) {
      const u = pos.units[e.shielded.target];
      if (u) u.shield += e.shielded.amount;
    } else if ('held' in e) {
      const u = pos.units[e.held.target];
      if (u) {
        const had = (u.holds ?? []).findIndex((h) => h.name === e.held.name);
        const laid = {
          name: e.held.name,
          kind: e.held.kind,
          amount: e.held.amount,
          turns: e.held.turns,
        };
        u.holds = had >= 0 ? (u.holds ?? []).map((h, i) => (i === had ? laid : h)) : [...(u.holds ?? []), laid];
      }
    } else if ('lifted' in e) {
      const u = pos.units[e.lifted.target];
      if (u) {
        // Сняли столько-то и в одну сторону. Какие именно — событие не
        // называет, и называть не должно: снимает их движок, а перепись живёт
        // две секунды и умирает на снимке сервера.
        let left = e.lifted.count;
        u.statuses = u.statuses.filter((st) => {
          if (left <= 0) return true;
          const against = (st.stat === 'vulnerable') !== st.amount < 0;
          if (against === e.lifted.ill) {
            left -= 1;
            return false;
          }
          return true;
        });
      }
    } else if ('mana' in e) {
      const s = e.mana.side === 'player' ? pos.player : pos.keeper;
      s.mana = Math.min(s.manaMax, s.mana + e.mana.amount);
    } else if ('zoned' in e) {
      pos.zones = [
        ...(pos.zones ?? []).filter((z) => z.name !== ''),
        {
          name: '',
          side: e.zoned.side,
          by: e.zoned.by,
          cells: e.zoned.cells,
          amount: e.zoned.amount,
          channel: 'physical',
          turns: e.zoned.turns,
        },
      ];
    } else if ('died' in e) {
      pos.board = pos.board.filter((s) => s.unit !== e.died.target);
    }
  }

  /**
   * Показать одно событие: разложить движение, переписать позицию в мгновение
   * касания, доиграть остаток.
   *
   * Порядок один на все поводы, и это существенно: пока подача, вздрагивание и
   * оседание были тремя ветками с тремя константами, стрела не помещалась
   * никуда — её пришлось бы делать четвёртой. Теперь ветвление осталось ровно
   * там, где события ПРАВДА разные: у выставления тело сначала появляется, у
   * падения — исчезает последним.
   */
  async function beat(pos: BattleMatchState, e: BattleEvent, token: number) {
    const alive = () => run === token;

    // Шаг — не движение из свода, а перекладка: тело переезжает на другую
    // клетку, и жеста для этого в словаре нет.
    if ('moved' in e || 'fell' in e) {
      transcribe(pos, e);
      live = copy(pos);
      if (!calm) await sleep(BEAT_MOVED);
      return alive();
    }

    if ('damaged' in e || 'immune' in e) {
      const target = 'damaged' in e ? e.damaged.target : e.immune.target;
      const by = 'damaged' in e ? e.damaged.by : e.immune.by;
      // Промах — не удар: оберег, который взял на себя, не дрожит как раненый.
      const motion = 'immune' in e ? WARD_MOTION : motionOf(pos, by ?? target, e);
      const span = calm ? 0 : (motion ? Math.min(MOTION_MS_MAX, motionSpan(motion)) : 0);

      put(pos, motion, by, target, 'damaged' in e ? hitOf(pos, e) : null);
      const wound = calm ? 0 : Math.min(acting?.contact ?? 0, span);
      if (wound > 0) {
        await sleep(wound);
        if (!alive()) return false;
      }

      transcribe(pos, e);
      live = copy(pos);
      if ('damaged' in e) openWound(e.damaged.target, acting?.hit ?? null);

      if (span > wound) await sleep(span - wound);
      dropActing();
      return alive();
    }

    if ('died' in e) {
      // Единственное событие, у которого перепись стоит ПОСЛЕ: тело обязано
      // быть видно, пока оседает, а перепись убирает его с доски.
      const motion = motionOf(pos, e.died.target, e);
      put(pos, motion, null, e.died.target);
      if (!calm) {
        await sleep(motionSpan(motion));
        dropActing();
        if (!alive()) return false;
      }
      dropActing();
      transcribe(pos, e);
      live = copy(pos);
      return alive();
    }

    // Выставление, лечение, всадник, щит: тело должно существовать (или уже
    // быть залечено), прежде чем ему что-то показывать.
    transcribe(pos, e);
    live = copy(pos);
    // Кто и по кому — одной таблицей, а не лестницей тернарников: поводов
    // теперь девять, и лестница на девять ступеней читается только сверху вниз.
    const hands = (): { striker: number | null; target: number | null } => {
      if ('played' in e) return { striker: e.played.unit, target: null };
      if ('healed' in e) return { striker: e.healed.by, target: e.healed.target };
      if ('rider' in e) return { striker: e.rider.by, target: e.rider.target };
      if ('shielded' in e) return { striker: e.shielded.by, target: e.shielded.target };
      if ('held' in e) return { striker: e.held.by, target: e.held.target };
      if ('lifted' in e) return { striker: e.lifted.by, target: e.lifted.target };
      // У опасной клетки цели-тела нет вовсе: движение играет ведьма одна.
      if ('zoned' in e) return { striker: e.zoned.by, target: null };
      return { striker: null, target: null };
    };
    if (
      'played' in e ||
      'healed' in e ||
      'rider' in e ||
      'shielded' in e ||
      'held' in e ||
      'lifted' in e ||
      'zoned' in e
    ) {
      const { striker, target } = hands();
      const motion = motionOf(pos, striker ?? target, e);
      // Выставление показывается на самом вышедшем теле, а не на бьющем:
      // у него нет ни автора, ни цели, есть только оно само.
      put(pos, motion, striker, target);
      if (!calm) await sleep(motionSpan(motion));
      dropActing();
    }
    return alive();
  }

  /**
   * Ход пришёл целиком: своё действие и ответ хранителя. Граница — `turnEnded`
   * своей стороны.
   *
   * Своё тоже играют, иначе стрела существует только у противника. Рука
   * остаётся своей тем, что между своими событиями нет REST: такт касания
   * есть, театральной паузы перед ним — нет. Хвост хранителя — по одному,
   * с паузами, как и было.
   */
  async function playThrough(from: BattleMatchState, events: BattleEvent[]) {
    const token = ++run;
    const pos = copy(from);

    let cut = events.findIndex((e) => 'turnEnded' in e && e.turnEnded.side === me);
    cut = cut < 0 ? events.length : cut + 1;
    const own = events.slice(0, cut);
    const tail = events.slice(cut);

    if (calm) {
      for (const e of events) {
        transcribe(pos, e);
      }
      told = events;
      live = null;
      before = match.state;
      return;
    }

    live = pos;
    running = true;
    playing = true;
    picked = null;
    sheet = null;
    told = [];

    for (const e of own) {
      if ('turnEnded' in e || 'breached' in e || 'finished' in e) {
        transcribe(pos, e);
        live = copy(pos);
        told = [...told, e];
        continue;
      }
      if (!(await beat(pos, e, token))) return;
      told = [...told, e];
    }

    for (const e of tail) {
      // Пауза — между тем, что видно. Конец хода и итог ничего не показывают,
      // и полсекунды тишины перед ними человек читает как задержку, а не как ритм.
      if (!('turnEnded' in e) && !('breached' in e) && !('finished' in e)) {
        await sleep(REST);
        if (run !== token) return;
      }
      if (!(await beat(pos, e, token))) return;
      told = [...told, e];
    }
    if (run !== token) return;
    running = false;
    playing = false;
    live = null;
    before = match.state;
  }

  /**
   * Новая посылка пришла.
   *
   * Зависимость одна — журнал: у стола хранителя `seq` всегда ноль, а журнал
   * приходит новым массивом на каждый ответ. Всё остальное читается под
   * `untrack`, иначе эффект подпишется на то, что сам же и пишет, — и первый же
   * `playing = true` перезапустил бы его, оборвав собственное проигрывание.
   */
  $effect(() => {
    const events = match.events;
    untrack(() => arrive(events));
  });

  function arrive(events: BattleEvent[]) {
    picked = null;
    sheet = null;
    acting = null;
    flying = null;
    told = [];

    if (before === null || running) {
      // Первый показ — или посылка догнала предыдущую, пока играла прошлая:
      // снимок сервера и есть истина, играть нечего. Журнал всё равно носит
      // эту посылку: иначе комната открывается пустым «пока ничего».
      run++;
      running = false;
      playing = false;
      live = null;
      before = match.state;
      told = events;
      return;
    }

    void playThrough(before, events);
  }

  // ── Журнал ────────────────────────────────────────────────────────────────
  const STEP_KEY: Record<string, TranslationKey> = {
    immunity: 'battleStepImmunity',
    pointBlank: 'battleStepPointBlank',
    cover: 'battleStepCover',
    attackerBless: 'battleStepAttackerBless',
    attackerCurse: 'battleStepAttackerCurse',
    targetVulnerable: 'battleStepTargetVulnerable',
    channelDefence: 'battleStepChannelDefence',
    floor: 'battleStepFloor',
    shield: 'battleStepShield',
  };

  /** Признак шага — в слово. Незнакомый шаг показывается как есть: движок может
   *  завести новый раньше, чем словарь, и честнее показать признак, чем чужое
   *  слово вместо него. */
  function stepWord(step: string, tr: (key: TranslationKey) => string): string {
    const key = STEP_KEY[step] as TranslationKey | undefined;
    return key ? tr(key) : step;
  }

  const nameOf = (id: number) => titleOf(match.state.units[id]?.card.name ?? String(id));

  type LineKind =
    | 'play'
    | 'move'
    | 'hit'
    | 'mend'
    | 'fall'
    | 'ward'
    | 'turn'
    | 'charm'
    | 'other';
  type Line = {
    text: string;
    kind: LineKind;
    trail: { step: string; from: number; to: number }[];
    total: number;
    head: string;
  };
  let journal = $derived.by<Line[]>(() =>
    told.map((e) => {
      const bare = (text: string, kind: LineKind = 'other'): Line => ({
        text,
        kind,
        trail: [],
        total: 0,
        head: '',
      });
      if ('played' in e) return bare(`${nameOf(e.played.unit)} — ${$t('battleLogPlayed')}`, 'play');
      if ('moved' in e) return bare(`${nameOf(e.moved.unit)} — ${$t('battleLogMoved')}`, 'move');
      if ('healed' in e)
        return bare(`${nameOf(e.healed.target)} — ${$t('battleLogHealed')} ${e.healed.amount}`, 'mend');
      if ('rider' in e) {
        // Словом всадника называет та чара, что его навела, а не ключ в записи:
        // в журнале лежит ключ, и имя ему подставляет комната — ровно так же,
        // как она подставляет название карты по её слагу.
        const on = match.state.units[e.rider.target];
        const word =
          riderWord(e.rider.status, on ? dtoOf(on.card.name) : null, $lang) ||
          $t(RIDER_STATS[e.rider.status.stat as keyof typeof RIDER_STATS]?.label ?? 'battlesPowerLabel');
        const sign = e.rider.status.amount > 0 ? '+' : '−';
        return bare(
          `${nameOf(e.rider.target)} — ${$t(
            e.rider.ill ? 'battleLogCursed' : 'battleLogBlessed',
          )}: ${word} ${sign}${Math.abs(e.rider.status.amount)}`,
          'charm',
        );
      }
      if ('shielded' in e)
        return bare(
          `${nameOf(e.shielded.target)} — ${$t('battleLogShielded')} ${e.shielded.amount}`,
          'charm',
        );
      if ('held' in e) {
        // Слово наложенного: своё имя чары, а нет его — род удержания. Тот же
        // порядок, что у всадника, и по той же причине.
        const on = match.state.units[e.held.target];
        const of = HOLD_WORDS[holdKind(e.held.kind)];
        const own = riderWord({ name: e.held.name }, on ? dtoOf(on.card.name) : null, $lang);
        const word = own || (of ? $t(of.word) : '');
        return bare(
          `${nameOf(e.held.target)} — ${$t('battleLogHeld')}: ${word}`,
          'charm',
        );
      }
      if ('lifted' in e)
        return bare(
          `${nameOf(e.lifted.target)} — ${$t('battleLogLifted')} ${e.lifted.count} ${$t(
            e.lifted.ill ? 'battleLogLiftedIll' : 'battleLogLiftedGood',
          )}`,
          'charm',
        );
      if ('mana' in e)
        return bare(
          `${e.mana.side === me ? $t('battleManaYours') : $t('battleManaKeeper')} — ${$t(
            'battleLogMana',
          )} ${e.mana.amount}`,
          'charm',
        );
      if ('zoned' in e) return bare($t('battleLogZoned'), 'charm');
      if ('died' in e) return bare(`${nameOf(e.died.target)} — ${$t('battleLogDied')}`, 'fall');
      if ('fell' in e) return bare(`${nameOf(e.fell.unit)} — ${$t('battleLogFell')}`, 'fall');
      if ('breached' in e) return bare(`${nameOf(e.breached.unit)} — ${$t('battleLogBreached')}`, 'move');
      if ('immune' in e) return bare(`${nameOf(e.immune.target)} — ${$t('battleLogImmune')}`, 'ward');
      if ('turnEnded' in e)
        return bare(
          e.turnEnded.side === me ? $t('battleLogTurnYours') : $t('battleLogTurnKeeper'),
          'turn',
        );
      if ('damaged' in e) {
        const d = e.damaged;
        const total = d.toHealth + d.toShield;
        return {
          text: `${nameOf(d.target)} — ${$t('battleLogDamaged')} ${total}`,
          kind: 'hit' as const,
          trail: d.trail,
          total,
          head: d.by == null ? '' : `${nameOf(d.by)} → ${nameOf(d.target)}`,
        };
      }
      return bare('');
    }).filter((l) => l.text),
  );

  let open = $state<number | null>(null);
  /** Правая колонка появляется, когда есть что сказать: выбранное тело или
   *  строки журнала. Пустое «пока ничего» не стоит трети окна. */
  let hasRail = $derived(fill || !!chosen || journal.length > 0);

  /**
   * Сколько ударов первого круга у вашей стороны ещё есть. `null` — правило
   * сейчас ничего не держит: круг не первый, потолок снят, или сторона не та.
   *
   * Считается из того же, из чего его считает движок (`holds_at_the_opening`),
   * и потому не второе правило, а его показ. Стоит у слова «Круг», а не у
   * тела: держит оно СТОРОНУ, и тело, которому это сказали, ни при чём.
   */
  let openingLeft = $derived.by((): number | null => {
    if (position.round !== 1) return null;
    if (position.rules.openingAttacks >= 255) return null;
    // Держат только ту сторону, что ходит первой; вторую — никогда.
    if (me !== 'player') return null;
    return Math.max(0, position.rules.openingAttacks - position.openingAttacksUsed);
  });
  let openingSay = $derived(
    openingLeft === null
      ? ''
      : openingLeft === 0
        ? $t('battleOpeningSpent')
        : $t('battleOpeningLeft').replace('{n}', String(openingLeft)),
  );

  /** Листок правил: открыт ли, и спрашивали ли его в этой партии. */
  let leafOpen = $state(false);
  let leafRead = $state(false);
  /**
   * Печать зовёт ровно один раз — в тот миг, когда правило впервые укусило:
   * удары кончились, а листок ещё не открывали. Открыли — зов гаснет
   * навсегда, потому что мигающее «нажмите сюда» тут и запрещено, и бесполезно
   * со второго раза.
   */
  let leafCalls = $derived(openingLeft === 0 && !leafRead);
  let leafLines = $derived([...rulesInForce(position.rules), ...terrainLines(position.terrain)]);
  function openLeaf() {
    leafOpen = !leafOpen;
    if (leafOpen) leafRead = true;
  }

  /**
   * Журнал в комнате этюда — шторка, а не колонна: колонна держала четверть
   * ширины, а поле упирается в высоту окна, и каждая точка сбоку, отнятая у
   * пустоты, — это точка, которой поле не получит. Шторка выезжает по значку
   * в шапке; точка на значке говорит, что с прошлого взгляда что-то
   * случилось, и гаснет, когда шторку открыли.
   */
  let logOpen = $state(false);
  let logSeen = $state(0);
  $effect(() => {
    if (logOpen) logSeen = journal.length;
  });
  let logFresh = $derived(!logOpen && journal.length > logSeen);

  const MANA_GEMS_CAP = 10;
  let manaCap = $derived(Math.max(0, Math.min(MANA_GEMS_CAP, position.player.manaMax)));
  let manaLit = $derived(
    position.active === 'player' ? Math.max(0, Math.min(manaCap, position.player.mana)) : 0,
  );
  let manaGems = $derived(Array.from({ length: manaCap }, (_, i) => i < manaLit));
  /** Кого спрашивает то, что в руке. Слово про ЭТО дело, а не про ход вообще:
   *  «выберите» на взведённом проклятии не говорит ничего. */
  const AIM_WORD: Record<string, TranslationKey> = {
    blow: 'battleAimBlow',
    mend: 'battleAimMend',
    harm: 'battleAimHarm',
    curse: 'battleAimCurse',
    bless: 'battleAimBless',
    shield: 'battleAimShield',
    fester: 'battleAimFester',
    knit: 'battleAimKnit',
    bind: 'battleAimBind',
    hush: 'battleAimHush',
    disarm: 'battleAimDisarm',
    sway: 'battleAimSway',
    veil: 'battleAimVeil',
    guard: 'battleAimGuard',
    numb: 'battleAimNumb',
    thorns: 'battleAimThorns',
    shove: 'battleAimShove',
    cleanse: 'battleAimCleanse',
    dispel: 'battleAimDispel',
    coin: 'battleAimCoin',
    offer: 'battleAimOffer',
    zone: 'battleAimZone',
    summon: 'battleAimSummon',
  };
  let promptWord = $derived.by((): TranslationKey => {
    if (playing || !mine) return 'battlePromptWait';
    if (stuckShown && stuckWord) return stuckWord;
    if (handTrouble === 'mana') return 'battlePromptMana';
    if (handTrouble === 'room') return 'battlePromptRoom';
    const tool = toolShown;
    if (trayShown && tool && intentLive(tool)) {
      return AIM_WORD[tool.casting ?? tool.kind] ?? 'battlePromptPick';
    }
    return 'battlePromptPick';
  });
  /** Та же строка, что в колокольчике отказа: число подставляется один раз. */
  let promptSay = $derived(promptWord === stuck?.key ? stuckSay : $t(promptWord));

  // ── Доска по высоте окна ──────────────────────────────────────────────────
  //
  // Пока стол стоит вертикально, ширина считается из высоты: шесть рядов
  // клеток 3:4 дают `H = 8·(W − зазоры)/3`, откуда `W = 0.375·H`. Значит, надо
  // знать, сколько высоты занято НЕ доской, — и это единственное, что нельзя
  // написать в CSS: над доской стоит шапка дома и поля страницы, а сцена о них
  // не знает. Когда стол вдоль комнаты, ширина берётся из колонки, и замер
  // не нужен.
  //
  // Поэтому замер, а не константа. Обе величины не зависят от ширины доски
  // (руки на широком экране стоят в боковой колонке), так что обратной связи
  // нет и мерить приходится только при изменении окна.
  let fieldEl = $state<HTMLElement | null>(null);
  let tableEl = $state<HTMLElement | null>(null);
  let roomEl = $state<HTMLElement | null>(null);
  let room = $state(310);
  let along = $state(fill);

  function measureRoom() {
    if (!fieldEl || !tableEl) return;
    const f = fieldEl.getBoundingClientRect();
    const t = tableEl.getBoundingClientRect();
    // `+ scrollY` — отступ от верха документа: сколько окажется над доской,
    // когда страница не прокручена. Ниже доски — строка хода и воздух.
    const above = f.top + window.scrollY;
    const below = t.bottom - f.bottom;
    const next = Math.max(0, Math.round(above + below + 24));
    // Пиксель туда-сюда не стоит перерисовки доски, а стоить может кругом.
    if (Math.abs(next - room) > 2) room = next;
  }

  /** Размер окна на прошлом замере. Полоса прокрутки, появившись, шлёт `resize`
   *  и меняет `innerWidth` — а от этого меняется ширина доски, от неё высота
   *  страницы, от неё снова полоса. Круг разрывается здесь: замер идёт, только
   *  если окно правда стало другим, и только если ответ правда изменился. */
  let seen = { w: 0, h: 0 };

  onMount(() => {
    seen = { w: window.innerWidth, h: window.innerHeight };
    measureRoom();
    const again = () => {
      if (window.innerWidth === seen.w && window.innerHeight === seen.h) return;
      seen = { w: window.innerWidth, h: window.innerHeight };
      measureRoom();
    };
    window.addEventListener('resize', again);
    return () => window.removeEventListener('resize', again);
  });

  $effect(() => {
    const el = roomEl;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const apply = (w: number) => {
      // Комната этюда всегда вдоль: портрет 3×6 даёт поле шириной в четверть
      // окна, а печать исхода тогда — узкая карточка внутри клетки. Те же
      // восемнадцать клеток, половины слева и справа. Стол хранителя по-
      // прежнему меряет ширину комнаты.
      along = fill || w >= ALONG;
    };
    apply(el.getBoundingClientRect().width);
    const ro = new ResizeObserver((entries) => {
      apply(entries[0]?.contentRect.width ?? 0);
    });
    ro.observe(el);
    return () => ro.disconnect();
  });

  let rows = $derived(Array.from({ length: DEPTH }, (_, y) => y));
  let cols = $derived(Array.from({ length: WIDTH }, (_, x) => x));
  /** Порядок клеток на экране. Движок не знает про это: x и y те же.
   *  Вдоль комнаты своя половина стоит СЛЕВА (глубина идёт справа налево):
   *  рука лежит в левой колонке, и между картой в руке и клеткой, куда её
   *  кладут, не должно быть чужого поля. Тот же разворот повторяют поднос
   *  (`trayAt`) и `stage()` в `battles.ts`: разойдутся — стрела полетит мимо. */
  let spots = $derived(
    along
      ? cols.flatMap((x) => [...rows].reverse().map((y) => ({ x, y })))
      : rows.flatMap((y) => cols.map((x) => ({ x, y }))),
  );
</script>

<svelte:window onkeydown={onkey} onpointerdown={offLeaf} />

<!-- Руки пишутся один раз и ставятся дважды: на широком экране — в боковую
     колонку, на узком — по краям доски. Не два куска разметки, а один снипет
     в двух местах: доска, которая рисует свою руку иначе, чем полка, начнёт
     врать ровно про то, что на ней проверяют. -->
<!-- Правило первого круга стоит у слова «Круг», а не у тела: держит оно
     сторону целиком. Печать рядом — вход для того, кто пришёл прочесть; она
     зовёт один раз, в тот миг, когда правило впервые укусило. -->
{#snippet roundMark()}
  {#if openingLeft !== null}
    <span class="opening" class:opening--spent={openingLeft === 0}>{openingSay}</span>
  {/if}
  <span class="seal-wrap">
    <button
      type="button"
      class="rules-seal"
      class:rules-seal--calls={leafCalls}
      aria-expanded={leafOpen}
      aria-label={$t('battleRulesSeal')}
      title={$t('battleRulesSeal')}
      onclick={openLeaf}
    >
      <BattleIcon name="scroll" size="1em" weight={1.4} />
    </button>
    {#if leafOpen}
      <div class="rules-leaf" role="dialog" aria-label={$t('battleRulesSeal')}
           transition:fly={{ y: -6, duration: 200 }}>
        <p class="rules-leaf-head">{$t('battleRulesInForce')}</p>
        <ul class="rules-leaf-list">
          {#each leafLines as line (line.key)}
            <li>
              <span class="rules-leaf-word">
                {$t(line.key)}{line.amount === null ? '' : ` — ${line.amount}`}
              </span>
              <!-- Пояснение есть только у первого круга: остальные правила
                   читаются с одного взгляда, а это одно читается как поломка. -->
              {#if line.key === 'battleRuleOpening'}
                <span class="rules-leaf-note">{$t('battleRuleOpeningWhole')}</span>
              {/if}
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </span>
{/snippet}

{#snippet keeperHand()}
  {#if theirHand.length}
    <p class="hand-label">{$t('battleHandKeeper')}</p>
    <div class="hand">
      {#each theirHand as held, i (i)}
        {@const dto = dtoOf(held.name)}
        <div class="held" style="--i:{i}; --n:{theirHand.length}">
          {#if dto}
            <BattleCard card={dto} {frames} owned={true} transition={false} interactive={false} />
          {:else}
            <span class="held-name">{titleOf(held.name)}</span>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
{/snippet}

<!-- Мана стоит в шапке, справа, рядом со словами хода: ход и чем за него
     платят — одна строка сведений, и внизу, у руки, ей не место. -->
{#snippet manaTrack()}
  <!-- `--lit`/`--max` уходят В СТИЛЬ, потому что свет камней ложится на саму
       бляху: чем больше маны, тем сильнее отсвет на металле. Считать его
       вторым списком классов («mana--3») значило бы завести одиннадцать имён
       под одно число. -->
  <div
    class="mana-track"
    style="--lit:{manaLit}; --max:{Math.max(1, manaCap)}"
    aria-label={`${$t('battleManaYours')} ${
      position.active === 'player'
        ? `${position.player.mana}/${position.player.manaMax}`
        : String(position.player.manaMax)
    }`}
  >
    <span class="mana-word">
      <BattleIcon name={statMark('mana')} size="1.15em" weight={1.4} />
      {$t('battleManaShort')}
    </span>
    <!-- Число и предел РАЗНЫМ голосом: сколько есть сейчас — это то, что
         меняется каждый ход, а предел напечатан и молчит. Одним кеглем и
         одной краской «3/3» читалось одним числом, которого нет. -->
    <span class="mana-count" aria-hidden="true">
      {#if position.active === 'player'}
        <b class="mana-now">{position.player.mana}</b>
        <i class="mana-of">/{position.player.manaMax}</i>
      {:else}
        <i class="mana-of mana-of--alone">{position.player.manaMax}</i>
      {/if}
    </span>
    <span class="mana-gems" aria-hidden="true">
      {#each manaGems as lit, i (i)}
        <i class="gem" class:gem--lit={lit} style="--i:{i}"></i>
      {/each}
    </span>
  </div>
{/snippet}

{#snippet ownHand()}
  {#if hand.length}
    <p class="hand-label">{$t('battleHandYours')}</p>
    <div class="hand" style="--n:{hand.length}">
      {#each hand as held, i (i)}
        {@const dto = dtoOf(held.name)}
        <button
          type="button"
          disabled={!mine}
          onclick={() => tapHand(i)}
          class="held held--mine"
          class:held--picked={picked?.kind === 'hand' && picked.index === i}
          class:held--dim={!playableHand.has(i)}
          class:held--low={i > 0 && i >= hand.length / 2}
          style="--i:{i}; --n:{hand.length}; --arc:{(hand.length - 1) / 2
            - Math.abs(i - (hand.length - 1) / 2)}"
        >
          {#if dto}
            <BattleCard card={dto} {frames} owned={true} transition={false} interactive={false} />
            <!-- Тот же закон, что у кружков доски (см. `cardSays` выше): карта
                 в руке носит свой значок стоимости, и бумажка руки, вставшая
                 рядом, была вторым отрисовщиком одного числа. Печатаем её
                 только там, где опись рамы стоимость сняла.
                 Со знаком, потому что цифра на бумажке одна и ничем себя не
                 называет — ровно та беда, ради которой знак и заводили. -->
            {#if !cardSays(dto, 'cost')}
              <span class="held-cost" aria-label={`${$t('battlesCostLabel')} ${held.cost}`}>
                <BattleIcon name={statMark('cost')} size="1em" weight={1.4} />{held.cost}
              </span>
            {/if}
          {:else}
            <span class="held-name">{titleOf(held.name)}</span>
          {/if}
          {#if stuckShown && picked?.kind === 'hand' && picked.index === i}
            <span class="stuck-anchor">
              {#key `h${picked.index}:${tipBeat}`}
                <span class="stuck-tip" role="status" transition:fly={{ y: 8, duration: 280 }}>
                  <i class="stuck-tip-flare" aria-hidden="true"></i>
                  <em class="stuck-tip-word">{stuckSay}</em>
                </span>
              {/key}
            </span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
{/snippet}

<!-- Комната и стол — разные элементы: элемент не может спрашивать свой
     собственный контейнер, и запрос ниже молча мерил бы страницу. -->
<div class="room" class:room--fill={fill} bind:this={roomEl}>
<div
  class="scene"
  class:scene--held={playing}
  class:scene--along={along}
  class:scene--fill={fill}
  class:scene--chamber={fill}
  class:scene--rail={hasRail}
  style="--fit:{DEFAULT_ASPECT}"
>
  {#if fill}
    <div class="crest">
      <!-- Выход — значком: им пользуются один раз и не на ходу, а словами в
           одиннадцать знаков он занимал угол шапки, который нужен полю. -->
      {#if onexit}
        <button
          type="button"
          class="exit"
          onclick={onexit}
          aria-label={$t('battleLeave')}
          title={$t('battleLeave')}
        >
          <BattleIcon name="door" size="1.25rem" weight={1.3} />
        </button>
      {:else}
        <span></span>
      {/if}
      <!-- Ход, раунд и мана — одна плашка в одну строку: это одна строка
           сведений («чей ход и чем за него платят»), и три бляхи на трёх
           этажах съедали высоту, которой полю не хватает. -->
      <div class="crest-mark">
        <p class="crest-turn">
          {position.active === me ? $t('battleWhoseTurnYours') : $t('battleWhoseTurnKeeper')}
        </p>
        <p class="crest-round">
          {$t('battleRound')} {position.round}
          {@render roundMark()}
        </p>
        {@render manaTrack()}
      </div>
      <button
        type="button"
        class="log-toggle"
        class:log-toggle--open={logOpen}
        aria-expanded={logOpen}
        aria-controls="battle-log"
        aria-label={$t('battleJournal')}
        title={$t('battleJournal')}
        onclick={() => (logOpen = !logOpen)}
      >
        <BattleIcon name="book" size="1.25rem" weight={1.3} />
        {#if logFresh}<i class="log-toggle-dot" aria-hidden="true"></i>{/if}
      </button>
    </div>
  {:else}
  <div class="strip">
    <p class="ledger-turn">
      {position.active === me ? $t('battleWhoseTurnYours') : $t('battleWhoseTurnKeeper')}
    </p>
    <p class="strip-meta">
      <!--
        «Сколько есть сейчас» имеет смысл только у той стороны, чей ход идёт. У
        другой `mana` — это остаток с её прошлого хода, а в первом раунде, пока
        она ещё не ходила ни разу, это просто ноль при непустом потолке: сторона,
        ходящая второй, показывала «0/2», а на своём первом ходу получала 3.
        Число, которое ни разу не было правдой.

        Поэтому у активной стороны показывается «есть из потолка», а у неактивной
        — только сам потолок. Дорисовывать ей «будет столько-то» здесь нельзя:
        прибавку на ход считает движок, и вторая её реализация разошлась бы с ним.
      -->
      <span>{$t('battleRound')} {position.round} {@render roundMark()}</span>
      <span>
        {$t('battleManaYours')}
        <span class="num">
          {#if position.active === 'player'}{position.player.mana}/{position.player.manaMax}
          {:else}{position.player.manaMax}{/if}
        </span>
      </span>
      <span>
        {$t('battleManaKeeper')}
        <span class="num">
          {#if position.active === 'keeper'}{position.keeper.mana}/{position.keeper.manaMax}
          {:else}{position.keeper.manaMax}{/if}
        </span>
      </span>
    </p>
  </div>
  {/if}

  <div class="play">
  <!-- Рука хранителя в этой колонке на широком экране: тогда под доской
       остаются только свои карты и ход, и они помещаются в окно. -->
  <aside class="ledger" class:ledger--empty={!theirHand.length}>
    <div class="ledger-hands">
      {@render keeperHand()}
    </div>
  </aside>

  <div class="table" bind:this={tableEl} style="--room:{room}px">
    <div class="table-hand table-hand--theirs">{@render keeperHand()}</div>

    <!-- Домашняя форма приходит СВЕРХУ, со сцены: гнездо руки и клетка доски
         обязаны быть одной формы, а два объявления однажды разойдутся. -->
    <div class="well">
    <div
      class="field"
      bind:this={fieldEl}
      style="--cols:{along ? DEPTH : WIDTH}; --rows:{along ? WIDTH : DEPTH}"
    >
      <div class="cloth">
      <div class="face">
      <div class="grid">
        {#each spots as { x, y } (`${x},${y}`)}
          {@const here = unitAt(x, y)}
          {@const open2 = openCells.has(`${x},${y}`)}
          {@const spot = openSpots.get(`${x},${y}`)}
          {@const burn = zoneAt(x, y)}
          {@const ground = grounds.get(`${x},${y}`)}
          {@const target = here ? openUnits.get(here.id)?.mark : undefined}
          {@const dto = here ? dtoOf(here.card.name) : null}
          {@const willTake = here ? toldMine.get(here.id) : undefined}
          {@const willGet = here ? toldTheirs.get(here.id) : undefined}
          <button
            type="button"
            disabled={playing}
            onclick={() => tapCell(x, y)}
            onpointerenter={() => ponder(actionAt(x, y))}
            onpointerleave={forget}
            onfocus={() => ponder(actionAt(x, y))}
            onblur={forget}
            aria-label={`${here ? titleOf(here.card.name) : `${x},${y}`}${ground ? ` — ${$t(GROUND_KEY[ground])}` : ''}`}
            title={ground ? $t(GROUND_KEY[ground]) : undefined}
            class="cell"
            class:cell--omen={!!willGet}
            class:cell--open={open2}
            class:cell--picked={picked?.kind === 'unit' && here?.id === picked.id}
            class:cell--attack={target === 'attack'}
            class:cell--mend={target === 'mend'}
            class:cell--charm={target === 'charm'}
            class:cell--spot={!!spot}
            class:cell--burn={!!burn}
            class:cell--live={mine && here?.owner === me && ready.has(here.id)}
            class:cell--theirs={y < HALF}
            class:cell--mine={y >= HALF}
            class:cell--poised={!!here && poised.has(here.id)}
          >
              {#if ground}
                <!-- Местность. Под телом: на укрытии и в топи стоят, а стена
                     пуста по правилу, и её можно закрасить целиком. -->
                <BattleGroundMark {ground} occupied={!!here} dark={fill} />
              {/if}
              {#if edges && (y === 0 || y === DEPTH - 1)}
                <!-- Ряд прорыва. Отдельным слоем, а не тенью клетки: тень
                     клетки в тёмной комнате занята её собственным светом. -->
                <i class="edge" aria-hidden="true"></i>
              {/if}
              {#if burn}
                <!-- Опасная клетка: та же штриховка, что у сукна, только
                     гуще, и число — сколько снимет с того, кто здесь
                     простоит. Рисуется ПОД телом: на клетке можно стоять. -->
                <span class="burn" aria-label={`${$t('battleZoneHere')} ${burn.amount}`}>
                  <i class="burn-hatch" aria-hidden="true"></i>
                  <i class="burn-num">{burn.amount}</i>
                </span>
              {/if}
              {#if here}
                <!-- Погасшим показывается и тело, которое уже сходило, и тело,
                     которому нечем ходить: с тех пор как шаг не тратит ход
                     целиком, второе случается часто, и без этого оно выглядело
                     бы свежим, не отзываясь на нажатие. -->
                <!-- Шевеление приходит готовой строкой из `stage()`: и
                     подача, и вздрагивание, и оседание — один и тот же путь,
                     потому что для сцены они одно и то же, движение из свода. -->
                <span
                  class="figure"
                  style:--fit={dto ? aspectOf(dto) : DEFAULT_ASPECT}
                  class:figure--spent={here.owner === me
                    && (here.acted || (mine && !ready.has(here.id)))}
                  class:figure--wound={acting?.target === here.id || flying?.unit === here.id}
                  style={stirOf(here.id)}
                >
                  <span class="figure-body">
                  {#if dto}
                    <!-- transition off: two copies of one work share a card id,
                         and two `view-transition-name`s abort the morph. -->
                    <BattleCard
                      card={dto}
                      {frames}
                      owned={true}
                      transition={false}
                      interactive={false}
                      hurt={here.health.max > 0 ? here.health.current / here.health.max : 1}
                      alive={here.health.current}
                      wearSeed={here.id}
                      struck={acting?.target === here.id ? acting.struck : null}
                      scrap={flying?.unit === here.id ? flying : null}
                    />
                  {:else}
                    <span class="figure-name">{titleOf(here.card.name)}</span>
                  {/if}
                  <!-- Числа, которых карта НЕ печатает сама.
                       Карта в клетке носит свои значки — со своей формой, своей
                       заливкой, своим местом на кромке окна и сургучом у
                       здоровья, — и кружок доски, вставший в тот же угол,
                       просто накрывал их собой: два отрисовщика одного числа
                       это не лишний код, это два кружка в одном месте, и
                       побеждал тот, что нарисован позже. Спрашиваем опись ТОЙ
                       рамы, которую носит это тело; нет карты — печатаем оба,
                       иначе на доске не осталось бы чисел вовсе. -->
                  {#if !cardSays(dto, 'healthMark') || !cardSays(dto, 'power')}
                    <span class="tally">
                      <!-- Знак ВОЗЛЕ цифры — тот же, что карта ставит на своей
                           плашке (`.corner-glyph`): кружок доски встаёт вместо
                           её значка и обязан говорить то же самое. -->
                      {#if !cardSays(dto, 'healthMark')}
                        <i class="tally-pip tally-pip--health">
                          <span class="tally-glyph" aria-hidden="true"
                            ><BattleIcon name={statMark('health')} size="100%" weight={1.35} /></span
                          >
                          <span class="tally-num">{here.health.current}</span>
                        </i>
                      {/if}
                      {#if !cardSays(dto, 'power')}
                        <i class="tally-pip tally-pip--power">
                          <span class="tally-glyph" aria-hidden="true"
                            ><BattleIcon name={statMark('power')} size="100%" weight={1.35} /></span
                          >
                          <span class="tally-num">{here.power}</span>
                        </i>
                      {/if}
                    </span>
                  {/if}
                  </span>
                </span>

                <!-- Предвестие на теле: сколько снимет этот ход и сколько
                     снимут в ответ. Два числа, а не одно: своё и чужое — не
                     одно и то же, и сложенные они не значат ничего. -->
                {#if willTake?.off || willGet?.off || willGet?.falls}
                  <span class="omen" class:omen--falls={willGet?.falls}>
                    {#if willTake?.off}
                      <i class="omen-mine"
                        >{willTake.off > 0 ? '−' : '+'}{Math.abs(willTake.off)}</i
                      >
                    {/if}
                    {#if willGet?.off}
                      <i class="omen-theirs">−{willGet.off}</i>
                    {/if}
                  </span>
                {/if}

                <!-- Зарубки: сколько всего на теле висит. Удержания считаются
                     наравне со всадниками — для доски это одно и то же
                     «на нём что-то есть», а что именно, скажет лист. -->
                {#if here.statuses.length || (here.holds ?? []).length}
                  <span class="nicks" aria-hidden="true">
                    {#each here.statuses as st, i (i)}<i class="nick"></i>{/each}
                    {#each here.holds ?? [] as h, i (i)}<i class="nick nick--hold"></i>{/each}
                  </span>
                {/if}

                <!-- Отказ словами: у тела, которому поднесён поднос, его
                     печатает сам поднос — две плашки об одном встали бы одна
                     на другую, и обе под телом. -->
                {#if stuckShown && !trayShown && picked?.kind === 'unit' && here.id === picked.id}
                  <span class="stuck-anchor">
                    {#key `${picked.id}:${tipBeat}`}
                      <span class="stuck-tip" role="status" transition:fly={{ y: 10, duration: 300 }}>
                        <i class="stuck-tip-flare" aria-hidden="true"></i>
                        <em class="stuck-tip-word">{stuckSay}</em>
                      </span>
                    {/key}
                  </span>
                {/if}
              {/if}
            </button>
          {/each}
        <!-- ── Поднос ──────────────────────────────────────────────────────
             Чем заняться телу, взятому в руку: по печати на способ, и подпись
             у той, что в руке. Слово одно, а не по слову под каждой печатью:
             трёх подписей в клетку не входит, а «подписана та, что в руке» —
             это и есть поднос, с которого берут.

             Стоит поднос ЧЛЕНОМ СЕТКИ, на месте тела, а не внутри его клетки:
             клетка — это `<button>`, и кнопка внутри кнопки не нажимается ни в
             одном браузере. Геометрию при этом считает сама сетка. -->
        {#if trayAt}
          <!-- Обе кромки названы числами, а не `span`: у абсолютного ребёнка
               сетки `auto` на дальней кромке значит НЕ «одна клетка», а край
               самой сетки, и поднос молча оказывался на клетку ниже. -->
          <span
            class="tray-slot"
            style="grid-column:{trayAt.col}/{trayAt.col + 1}; grid-row:{trayAt.row}/{trayAt.row + 1}"
          >
            <span
              class="tray-anchor"
              class:tray-anchor--over={trayAt.over}
              class:tray-anchor--start={trayAt.edge === 'start'}
              class:tray-anchor--end={trayAt.edge === 'end'}
            >
              <span class="tray" role="group" aria-label={$t('battleIntents')}>
                <span class="tray-row">
                  {#each intents as intent, i (intent.key)}
                    {@const live = intentLive(intent)}
                    {@const inHand = toolInHand?.key === intent.key}
                    {@const lit = toolShown?.key === intent.key}
                    <button
                      type="button"
                      class="tool"
                      class:tool--hand={inHand && live}
                      class:tool--lit={lit && live}
                      class:tool--dim={!live}
                      style="--i:{i}"
                      aria-pressed={inHand && live}
                      aria-label={toolSay(intent)}
                      title={toolSay(intent)}
                      onclick={() => takeTool(intent)}
                      onpointerenter={() => (toolTasted = intent.key)}
                      onpointerleave={() => (toolTasted = null)}
                      onfocus={() => (toolTasted = intent.key)}
                      onblur={() => (toolTasted = null)}
                    >
                      <i class="tool-seal">
                        <BattleIcon name={intent.mark} size="58%" weight={1.5} />
                      </i>
                      {#if intent.amount !== null}
                        <b class="tool-num">{intent.amount}</b>
                      {/if}
                      {#if intent.mana > 0}
                        <i class="tool-drops" aria-hidden="true">
                          {#each Array.from({ length: Math.min(3, intent.mana) }) as _, g (g)}<i
                              class="tool-drop"
                            ></i>{/each}
                        </i>
                      {/if}
                      {#if intent.spent}
                        <i class="tool-sleep" aria-hidden="true">×</i>
                      {:else if intent.asleep !== null}
                        <i class="tool-sleep" aria-hidden="true">{intent.asleep}</i>
                      {/if}
                    </button>
                  {/each}
                </span>
                <!-- Подпись: имя того, что в руке, его число — и, у всадника,
                     чем он правит и сколько держится. Мёртвой печати та же
                     подпись говорит, почему нельзя. -->
                <!-- Подпись меняется ВХОДЯ, а не сменяясь: у `transition`
                     уходящая плашка ещё занимает строку, пока приходящая уже
                     встала, и подпись на полсекунды двоилась. -->
                {#key toolShown?.key ?? (stuckShown ? `why${tipBeat}` : 'none')}
                  <span class="tray-tag" in:fade={{ duration: calm ? 0 : 130 }}>
                    {#if toolShown}
                      {@const tool = toolShown}
                      <em class="tray-word">{tool.name || $t(tool.word)}</em>
                      <span class="tray-nums">
                        {#if tool.amount !== null}
                          <span class="num">{tool.amount}</span>
                        {/if}
                        {#if tool.stat}
                          <span class="tray-of"
                            ><BattleIcon
                              name={RIDER_STATS[tool.stat].mark}
                              size="0.85em"
                              weight={1.4}
                            />{$t(RIDER_STATS[tool.stat].label)}</span
                          >
                        {/if}
                        {#if tool.turns}
                          <span class="tray-turns"
                            >{$t('battleStatusTurns')} <span class="num">{tool.turns}</span></span
                          >
                        {/if}
                        {#if tool.range !== null && tool.range > 1}
                          <span class="tray-reach"
                            ><BattleIcon name={statMark('reach')} size="0.85em" weight={1.4} />{tool
                              .range}</span
                          >
                        {/if}
                        <!-- Пригоршня: сколько тел чара берёт разом. Названа
                             СЛОВОМ, а не показана на доске: доска светит только
                             то, куда можно ткнуть, а круг считает движок — и
                             посчитает он его в ответе предвестия, когда человек
                             наведётся на цель. Второй расчёт здесь был бы
                             зеркалом правила, которое нечем проверить. -->
                        {#if tool.reach}
                          <span class="tray-spread"
                            ><BattleIcon name={tool.shape} size="0.85em" weight={1.4} />{$t(
                              tool.reach,
                            )}{#if tool.radius > 0 && (tool.shape === 'radius' || tool.shape === 'chain')}&nbsp;{tool.radius}{/if}</span
                          >
                        {/if}
                      </span>
                      {#if !intentLive(tool)}
                        <span class="tray-why">{toolWhy(tool)}</span>
                      {/if}
                    {:else if stuckShown}
                      <span class="tray-why">{stuckSay}</span>
                    {:else}
                      <em class="tray-word">{$t('battleIntents')}</em>
                    {/if}
                  </span>
                {/key}
              </span>
            </span>
          </span>
        {/if}

        <span class="midline" aria-hidden="true">{#if fill}<i class="vs"></i>{/if}</span>
      </div>

      {#if fill}
        <span class="zone zone--theirs">{$t('battleZoneTheirs')}</span>
        <span class="zone zone--mine">{$t('battleZoneYours')}</span>
      {/if}

      <!-- Нарисованное: стрела в полёте, вспышка на цели, полоса кадров.
           Лежит в `inset: 0` от поля — поле шире доски увело бы всё вбок. -->
      <BattleMotionStage motes={acting?.play.motes ?? []} />
      </div>
      </div>
    </div>
    </div>

    <!-- Своя рука и ход — ближний край стола: веер карт и фраза хода
         в одной полосе, чтобы оба оставались в окне. -->
    <div class="foot">
      {#if legend.length}
        <!-- Легенда местности: знак, имя и правило коротко. Знак тот же, что в
             углу клетки, — по нему клетку и легенду связывают взглядом. -->
        <section class="ground-legend" aria-label={$t('battleGroundLegend')}>
          <p class="ground-legend-head">{$t('battleGroundLegend')}</p>
          <ul class="ground-legend-list">
            {#each legend as g (g)}
              <li>
                <BattleGroundSigil ground={g} size="1.15em" />
                <span><b>{$t(GROUND_NAME[g])}</b> — {$t(GROUND_SHORT[g])}</span>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
      {#if fill}
        <div class="prompt">
          <div class="glass" class:glass--run={playing} aria-hidden="true"></div>
          <p class="prompt-word" class:prompt-word--stuck={stuckShown}>{promptSay}</p>
        </div>
      {/if}
      <div class="table-hand table-hand--mine">{@render ownHand()}</div>
      <div class="turn">
        {#if handTrouble}
          <p class="hand-why">
            {handTrouble === 'mana' ? $t('battleNoManaYet') : $t('battleNoRoomYet')}
          </p>
        {/if}
        <div class="turn-act">
          <button
            type="button"
            disabled={!mine}
            onclick={() => onact('endTurn')}
            onpointerenter={() => ponder('endTurn')}
            onpointerleave={forget}
            onfocus={() => ponder('endTurn')}
            onblur={forget}
            class="end"
          >
            {$t('battleEndTurn')}
          </button>
          {#if playing}
            <span class="waiting">{$t('battleKeeperThinks')}</span>
          {/if}
        </div>

        <!-- Ответ хранителя словами. Метки на доске говорят «сколько и с
             кого», строка — «кто и по кому»: при дальности четыре число иначе
             меняется на другом конце поля без всякого видимого автора.

             Строка занята ВСЕГДА, пока предвестие включено: пока она то
             появлялась, то исчезала, она толкала кнопку конца хода — а кнопка,
             уезжающая из-под пальца, хуже любой подсказки. Поэтому при
             включённом и ненаведённом предвестии здесь стоит то, что оно
             обещает, а не пустота. -->
        {#if onforesee}
          <div class="omens">
            <!-- Строка ПЕРЕД переключателем, а он прижат к тому же краю, что и
                 «Закончить ход». Наоборот было хуже: пустеющая строка тянула
                 переключатель вправо, и он уезжал из-под пальца ровно в тот
                 миг, когда по нему нажали. -->
            <p class="omen-word" class:omen-word--idle={!foretold}>
              {#if !foresight}
                &nbsp;
              {:else if !foretold}
                {$t('battleForesightHint')}
              {:else if foretoldWords.length}
                {#each foretoldWords as w, i (i)}<span class="omen-line"
                    >{w.by}{w.by ? ' → ' : ''}{w.target} −{w.off}{w.falls
                      ? ' †'
                      : ''}</span
                  >{/each}
              {:else if foretold.outcome}
                {$t('battleForesightEnds')}
              {:else}
                {$t('battleForesightQuiet')}
              {/if}
            </p>
            <button
              type="button"
              class="omen-switch"
              class:omen-switch--on={foresight}
              aria-pressed={foresight}
              onclick={toggleForesight}
            >
              {$t('battleForesight')}<span class="omen-state"
                >{foresight ? $t('battleForesightOn') : $t('battleForesightOff')}</span
              >
            </button>
          </div>
        {/if}
      </div>
    </div>
  </div>

  {#if hasRail}
  <aside
    class="aside"
    class:aside--shut={fill && !logOpen}
    id={fill ? 'battle-log' : undefined}
    inert={fill && !logOpen}
  >
    <!-- Карточка выбранного: спокойная и неподвижная, не всплывающая подсказка. -->
    {#if chosen}
      <div class="chosen">
        <div class="chosen-head">
          {#if chosenDto}
            <button type="button" class="chosen-name" onclick={() => (sheet = chosenDto)}>
              {titleOf(chosen.card.name)}
            </button>
            <button type="button" class="chosen-leaf" onclick={() => (sheet = chosenDto)}>
              {$t('battlesTableReadCard')}
            </button>
          {:else}
            <p class="chosen-name">{titleOf(chosen.card.name)}</p>
          {/if}
        </div>
        {#if chosenKind}
          <p class="chosen-kind">
            {chosenKind}{#if chosenChannel}<span class="sep">·</span>{chosenChannel}{/if}
          </p>
        {/if}
        <!-- Одна строка разбора — один снипет, а не пятнадцать раз выписанное
             `<dt>/<dd>`: знак числа приходит из `STAT_MARKS`, и место, которое
             называет своё сердце руками, однажды назовёт не то. -->
        {#snippet stat(slot: MarkedStat, shown: string)}
          <div>
            <dt>
              <BattleIcon name={statMark(slot)} size="1em" weight={1.35} />
              {$t(statLabel(slot))}
            </dt>
            <dd class="num">{shown}</dd>
          </div>
        {/snippet}
        <dl class="chosen-stats">
          {@render stat('health', `${chosen.health.current}/${chosen.health.max}`)}
          {#if chosenDto}
            {@render stat('cost', String(chosenDto.cost))}
            {@render stat('power', String(chosenDto.power))}
            {#each chosenPass.filter((row) => row.field !== 'health') as row (row.field)}
              {@render stat(row.field, String(row.value))}
            {/each}
          {:else}
            {@render stat('power', String(chosen.power))}
            {#if chosen.armor > 0}{@render stat('armor', String(chosen.armor))}{/if}
            {#if chosen.ward > 0}{@render stat('ward', String(chosen.ward))}{/if}
            {#if chosen.reach > 0}{@render stat('reach', String(chosen.reach))}{/if}
            {#if chosen.step > 0}{@render stat('step', String(chosen.step))}{/if}
            {#if chosen.mend > 0}{@render stat('mend', String(chosen.mend))}{/if}
          {/if}
        </dl>
        {#if chosenFace?.effect}
          <p class="chosen-effect">{chosenFace.effect}</p>
        {/if}
        <!-- Всадники: чем эта чара правит, на сколько и ещё сколько ходов.
             В записи у всадника лежит КЛЮЧ умения — показывать его человеку
             значило бы показать `#0`, — поэтому слово подставляется по ключу с
             карты, а не нашлось имя, так называет его сам показатель. -->
        <!-- Удержания: то, что правит не числа, а возможности. Списком рядом
             со всадниками, а не вперемешку: «−2 к силе» и «не бьёт» — разного
             рода сведения, и сложенные в один столбец они читаются как одно. -->
        {#if (chosen.holds ?? []).length}
          <ul class="chosen-riders">
            {#each chosen.holds ?? [] as hold, i (i)}
              {@const of = HOLD_WORDS[holdKind(hold.kind)]}
              {@const word = riderWord(hold, chosenDto, $lang)}
              <li class="rider rider--hold">
                {#if of}<BattleIcon name={of.mark} size="1em" weight={1.35} />{/if}
                <span class="rider-word">{word || (of ? $t(of.word) : hold.name)}</span>
                {#if word && of}<span class="rider-of">{$t(of.word)}</span>{/if}
                {#if hold.amount > 0}<span class="num rider-num">{hold.amount}</span>{/if}
                <span class="rider-turns">{$t('battleStatusTurns')} {hold.turns}</span>
              </li>
            {/each}
          </ul>
        {/if}
        <!-- Чем на него дышит поле. Отдельным списком, и срок у него не
             назван: аура держится не ходами, а тем, кто ею дышит, — пал, отошёл
             или его увели смутой, и её нет в тот же миг. -->
        {#if (chosen.aura ?? []).length}
          <ul class="chosen-riders">
            {#each chosen.aura ?? [] as st, i (i)}
              {@const of = RIDER_STATS[st.stat as keyof typeof RIDER_STATS]}
              {@const word = riderWord(st, chosenDto, $lang)}
              <li class="rider rider--aura" class:rider--ill={st.amount < 0}>
                {#if of}<BattleIcon name={of.mark} size="1em" weight={1.35} />{/if}
                <span class="rider-word">{word || (of ? $t(of.label) : st.name)}</span>
                <span class="num rider-num"
                  >{st.amount > 0 ? '+' : '−'}{Math.abs(st.amount)}</span
                >
                {#if word && of}<span class="rider-of">{$t(of.label)}</span>{/if}
                <span class="rider-turns">{$t('battleAuraWhile')}</span>
              </li>
            {/each}
          </ul>
        {/if}
        {#if chosen.statuses.length}
          <ul class="chosen-riders">
            {#each chosen.statuses as st, i (i)}
              {@const of = RIDER_STATS[st.stat as keyof typeof RIDER_STATS]}
              {@const word = riderWord(st, chosenDto, $lang)}
              <li class="rider" class:rider--ill={st.amount < 0}>
                {#if of}<BattleIcon name={of.mark} size="1em" weight={1.35} />{/if}
                <span class="rider-word">{word || (of ? $t(of.label) : st.name)}</span>
                <span class="num rider-num"
                  >{st.amount > 0 ? '+' : '−'}{Math.abs(st.amount)}</span
                >
                {#if word && of}<span class="rider-of">{$t(of.label)}</span>{/if}
                <span class="rider-turns">{$t('battleStatusTurns')} {st.turns}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}

    {#if journal.length || fill}
    <div class="journal">
      <p class="journal-label">{$t('battleJournal')}</p>
      {#if journal.length}
      <ul class="journal-lines">
        {#each journal as line, i (i)}
          <li class="log log--{line.kind}">
            {#if fill}<i class="log-ico log-ico--{line.kind}" aria-hidden="true"></i>{/if}
            <div class="log-body">
            {#if line.trail.length}
              <button type="button" class="journal-open" onclick={() => (open = open === i ? null : i)}>
                {line.text}
              </button>
              {#if open === i}
                <div class="breakdown">
                  {#if line.head}<p class="breakdown-head">{line.head}</p>{/if}
                  {#each line.trail as b, j (j)}
                    <p class="breakdown-row">
                      <span class="breakdown-why">{stepWord(b.step, $t)}</span>
                      <span class="num">{b.from} → {b.to}</span>
                    </p>
                  {/each}
                  <p class="breakdown-row breakdown-total">
                    <span class="breakdown-why">{$t('battleTrailTotal')}</span>
                    <span class="num">{line.total}</span>
                  </p>
                </div>
              {/if}
            {:else}
              <span class="journal-plain">{line.text}</span>
            {/if}
            </div>
          </li>
        {/each}
      </ul>
      {:else}
        <p class="journal-empty">{$t('battleJournalEmpty')}</p>
      {/if}
    </div>
    {/if}
  </aside>
  {/if}
  </div>

  <!-- Исход накрывает комнату, а не клетку поля: внутри 3×6 печать
       становилась узкой карточкой. Сургуч тот же, что при получении карты. -->
  {#if position.outcome && !playing}
    <div class="seal-veil" transition:fade={{ duration: 200 }}>
      <div class="verdict" class:verdict--dim={position.outcome !== 'player'}>
        <WaxSeal size={fill ? '9.5rem' : '6.5rem'} dim={position.outcome !== 'player'} />
        <div class="verdict-copy">
          <p class="seal-word">
            {position.outcome === 'player'
              ? $t('battleWonByPlayer')
              : position.outcome === 'keeper'
                ? $t('battleWonByKeeper')
                : $t('battleDrawn')}
          </p>
          {#if breachedBy !== null}
            <p class="seal-line">{$t('battleSealBreached')} — {nameOf(breachedBy)}</p>
          {/if}
          {#if match.rewardDust > 0}
            <p class="seal-dust">{match.rewardDust} {$t('battleDustGranted')}</p>
          {/if}
          <!-- Чем пройдено. Пыль платится однажды, и без этой строки
               пройденный этюд не даёт ни одной причины к нему вернуться:
               победа была двоичной. «За пять дел, а лучшее известное —
               шесть» — уже причина. -->
          {#if match.marks}
            <p class="seal-line">
              {$t('battleMarkYourLine')} — {match.marks.acts}<span class="sep">·</span>{match
                .marks.bodiesLost === 0
                ? $t('battleLineNoneLost')
                : `${$t('battleLineLost')} — ${match.marks.bodiesLost}`}
            </p>
            {#if match.marks.record}
              <p class="seal-record">{$t('battleLineRecord')}</p>
            {:else if match.marks.bestKnown != null}
              <p class="seal-line seal-line--bar">
                {$t('battleMarkBestLine')} — {match.marks.bestKnown}
              </p>
            {/if}
          {/if}
          {#if onleave || onreplay}
            <p class="seal-doors">
              {#if onleave}<button type="button" class="door" onclick={onleave}>{$t('battleBackToStudies')}</button>{/if}
              {#if onreplay}<button type="button" class="door" onclick={onreplay}>{$t('battleReplay')}</button>{/if}
            </p>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>
</div>

{#if sheet}
  <BattleSheet card={sheet} {frames} taking={false} onclose={() => (sheet = null)} />
{/if}

<style>
    .room {
    container-type: inline-size;
  }

  .room--fill {
    height: 100%;
    min-height: 0;
  }

  /* Стол хранителя не знает этой обёртки: она прозрачна, пока сцена не
     на весь экран. В этюде колодец — размерная коробка среднего ряда,
     а поле внутри неё держит отношение сторон карт, иначе сукно
     растягивается в широкий пустой прямоугольник. */
  .well {
    display: contents;
  }

  .scene {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    color: #34251c;
  }

  .scene--fill {
    height: 100%;
    min-height: 0;
    overflow: hidden;
    gap: 0.28rem;
  }

  .scene.scene--fill .strip {
    gap: 0.15rem 1rem;
  }

  .scene.scene--fill .strip-meta {
    font-size: 0.74rem;
  }

  .play {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
    min-width: 0;
    min-height: 0;
  }

  .scene--fill .play {
    flex: 1 1 auto;
    min-height: 0;
  }

  .strip {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.35rem 1.35rem;
    flex: 0 0 auto;
  }

  /* Листок свисает с шапки на доску, а доска в разметке ПОЗЖЕ и потому
     рисуется поверх. Поднимается вся шапка, а не листок: `.crest-mark` носит
     `filter`, фильтр открывает свой контекст наложения, и `z-index` листка
     внутри него бессилен против соседа шапки — ровно тем же кончился бы
     захват, положенный внутрь детали рамки. */
  .crest,
  .strip {
    position: relative;
    z-index: 40;
  }

  .strip-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.25rem 1.1rem;
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.82rem;
    letter-spacing: 0.04em;
    color: #8a6a55;
  }

  /* Три колонки только там, где они помещаются. Стол хранителя рисует ту же
     сцену в 24rem, и там она складывается в одну колонку сама. */
  @container (min-width: 900px) {
    .play {
      display: grid;
      grid-template-areas: 'hands board rail';
      grid-template-columns: auto minmax(0, 1fr) auto;
      align-items: start;
      gap: 0.7rem 1.25rem;
    }

    .ledger {
      grid-area: hands;
    }

    .table {
      grid-area: board;
    }

    .aside {
      grid-area: rail;
    }

    /* Руки уходят в боковую колонку — и только ради этого доска помещается в
       окно целиком. Две руки картами стоят двести с лишним пикселей высоты,
       а поле 3×6 и без них выше экрана: держать и то и другое в одной колонке
       значит либо прокручивать доску, либо ужать карту до пятна. */
    /* Через `.scene`, а не просто `.ledger-hands`: контейнерный запрос не
       добавляет веса, а стоит он в этом файле выше общего правила — значит,
       перебить его может только вес. */
    .scene .ledger-hands {
      display: block;
    }

    .table-hand--theirs {
      display: none;
    }

    /* Доска — по высоте окна: ширина из высоты (`W = 0.375·H`), запас замерен
       выше. Пол в 13rem не даёт доске ужаться в невидимое: на низком окне
       честнее прокрутить, чем разглядывать фотографию величиной с ноготь.
       Ширина считается один раз и достаётся полю целиком: следы ударов и
       печать лежат в `inset: 0` от поля, и поле шире доски увело бы их вбок. */
    .table {
      --board: clamp(13rem, calc((100dvh - var(--room, 19rem) - 20px) * 0.375 + 8px), 26rem);
    }

    .field,
    .foot {
      width: var(--board);
      margin-inline: auto;
    }
  }

  @container (max-width: 720px) {
    .foot .held {
      width: 5.4rem;
    }

    .end {
      font-size: 1.08rem;
    }

    .cloth {
      padding: 0.28rem;
      outline-offset: 2px;
    }

    .hand-why {
      max-width: 11rem;
    }
  }

  /* Стол вдоль комнаты: те же 18 клеток, половины слева и справа. Класс, а не
     второй контейнерный порог: порядок клеток в разметке должен совпасть с
     колонками, и это знает только сцена. */
  .scene.scene--along .play {
    display: grid;
    grid-template-areas: 'hands board rail';
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: stretch;
    gap: 0.7rem 1.1rem;
    flex: 1 1 auto;
    min-height: 0;
  }

  .scene--along .ledger {
    grid-area: hands;
  }

  .scene--along .table {
    grid-area: board;
  }

  .scene--along .aside {
    grid-area: rail;
  }

  .scene--along .ledger-hands {
    display: block;
  }

  .scene--along .table-hand--theirs {
    display: none;
  }

  .scene--along .table {
    max-width: none;
    width: 100%;
    min-width: 0;
    min-height: 0;
  }

  /* Шесть колонок 3:4 дают H ≈ ⅔·W. Ширину берём из остатка высоты, иначе
     поле занимает окно целиком, а рука и ход уезжают под складку. Потолка
     в 42rem больше нет: он держал клетку около ста пикселей посреди окна. */
  .scene--along .field,
  .scene--along .foot {
    width: min(100%, calc((100dvh - var(--room, 16rem)) * 1.5));
    margin-inline: auto;
  }

  .scene--along .grid {
    grid-template-columns: repeat(var(--cols, 6), 1fr);
  }

  .scene--along .midline {
    left: 50%;
    right: auto;
    top: 0;
    bottom: 0;
    border-top: none;
    border-left: 1px dashed rgba(52, 37, 28, 0.2);
  }

  /* Стол этюда — комната: шапка, поле с журналом справа, низ на всю ширину.
     `display: contents` у стола отдаёт его детей сетке `.play`, чтобы журнал
     стоял на высоту поля, а не на высоту футера. */
  .scene.scene--fill .play,
  .scene.scene--fill.scene--along .play {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(16rem, 19rem);
    grid-template-rows: 1.7rem minmax(0, 1fr) auto;
    grid-template-areas:
      'theirs rail'
      'board rail'
      'foot foot';
    position: relative;
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
    gap: 0.1rem 0.7rem;
    align-items: stretch;
  }

  .scene.scene--fill .well {
    display: grid;
    place-items: center;
    min-width: 0;
    min-height: 0;
    width: 100%;
    height: 100%;
    container-type: size;
    overflow: hidden;
  }

  .scene.scene--fill .table,
  .scene.scene--fill.scene--along .table {
    flex: unset;
    min-height: 0;
    gap: 0;
    overflow: visible;
  }

  .scene.scene--fill .foot,
  .scene.scene--fill.scene--along .foot {
    width: 100%;
    margin: 0;
    max-width: none;
  }

  .scene.scene--fill .field,
  .scene.scene--fill.scene--along .field {
    --cols: 3;
    --rows: 6;
    flex: unset;
    min-width: 0;
    min-height: 0;
    margin: 0;
    max-width: 100%;
    max-height: 100%;
    height: auto;
    aspect-ratio: calc(var(--cols) * var(--fit, 0.714) / var(--rows));
    container-type: size;
    display: block;
    position: relative;
  }

  .scene.scene--fill.scene--along .field {
    --cols: 6;
    --rows: 3;
  }

  .scene.scene--fill .cloth,
  .scene.scene--fill.scene--along .cloth {
    aspect-ratio: unset;
    width: 100%;
    height: 100%;
    max-height: none;
    min-width: 0;
    box-sizing: border-box;
    border-color: #5a4630;
    outline-color: rgba(90, 70, 40, 0.55);
  }

  .scene.scene--fill .face,
  .scene.scene--fill .grid {
    height: 100%;
    width: 100%;
  }

  .scene.scene--fill .grid {
    grid-template-rows: repeat(var(--rows, 6), minmax(0, 1fr));
  }

  .scene.scene--fill.scene--along .grid {
    grid-template-columns: repeat(var(--cols, 6), minmax(0, 1fr));
    grid-template-rows: repeat(var(--rows, 3), minmax(0, 1fr));
  }

  .scene.scene--fill .cell {
    aspect-ratio: auto;
    min-height: 0;
    min-width: 0;
    height: 100%;
    container-type: size;
  }

  .scene.scene--fill .cell:not(:has(.figure)):not(.cell--open):not(.cell--attack):not(.cell--mend):not(.cell--charm):not(.cell--spot):not(.cell--picked) {
    background: linear-gradient(160deg, rgba(48, 36, 28, 0.55), rgba(12, 10, 8, 0.8));
    border: 1px solid #4a3828;
    box-shadow: inset 0 0 10px rgba(0, 0, 0, 0.55);
  }

  .scene.scene--fill .cell--theirs:not(:has(.figure)):not(.cell--open):not(.cell--attack):not(.cell--mend):not(.cell--charm):not(.cell--spot):not(.cell--picked) {
    border-color: #6a3028;
    box-shadow:
      inset 0 0 14px rgba(160, 40, 30, 0.32),
      inset 0 0 10px rgba(0, 0, 0, 0.5);
  }

  .scene.scene--fill .cell--mine:not(:has(.figure)):not(.cell--open):not(.cell--attack):not(.cell--mend):not(.cell--charm):not(.cell--spot):not(.cell--picked) {
    border-color: #2a4068;
    box-shadow:
      inset 0 0 14px rgba(40, 80, 170, 0.3),
      inset 0 0 10px rgba(0, 0, 0, 0.5);
  }

  .scene.scene--fill .figure {
    height: 100%;
    width: 100%;
    padding: 1px;
  }

  .scene.scene--fill .figure-body {
    width: min(100cqw, calc(100cqh * var(--fit, 0.714)));
    height: auto;
    max-height: 100%;
    aspect-ratio: var(--fit, 0.714);
  }

  .scene.scene--fill .figure-body > :global(.slot) {
    width: 100%;
    height: 100%;
  }

  .scene.scene--fill .foot {
    flex-wrap: nowrap;
  }

  .scene.scene--fill .foot .hand {
    align-items: flex-end;
  }

  .scene.scene--fill .aside {
    border: 1px solid #d4b06a;
    outline-offset: -4px;
  }

  .scene.scene--fill .journal-plain,
  .scene.scene--fill .journal-open,
  .scene.scene--fill .log-body {
    color: #fff8ea;
  }

  .scene.scene--fill .breakdown-head,
  .scene.scene--fill .breakdown-row,
  .scene.scene--fill .breakdown-total {
    color: #fff8ea;
  }

  /* ГНЕЗДО руки — коробка домашней формы, ровно как клетка доски: гнездо не
     смеет менять форму от того, какую раму надела карта, иначе веер выходит
     ступеньками. Форма берётся из `--fit` со сцены, а не пишется литералом
     `5 / 7`: литерал был третьим местом, где названа форма карты, и он молча
     разошёлся с рамой — гнездо 107×150, карта внутри 105×157. */
  .scene.scene--fill .foot .held {
    flex: 0 0 auto;
    width: auto;
    aspect-ratio: var(--fit, 0.714);
    /* Гнёзда НАЛЕЗАЮТ друг на друга только тогда, когда иначе не влезают.
       Постоянные −0.28rem были заведены под полную руку, а при трёх картах
       давали ряд, в котором карты трогаются краями посреди пустого стола, —
       и он читался случайной стопкой, а не разложенной рукой. */
    margin-inline: calc(-0.28rem * max(0, var(--n, 3) - 5));
    transform: none;
    display: grid;
    /* По НИЗУ: гнездо домашней формы, карта внутри — своей, и центрованная
       низкая рама всплывала над соседками. Низ у ряда один. */
    place-items: end center;
  }

  .scene.scene--fill .table-hand--theirs .held {
    width: auto;
    height: 2.55rem;
    aspect-ratio: var(--fit, 0.714);
    margin-inline: -0.22rem;
    display: grid;
    place-items: center;
  }

  /* А КАРТА внутри — своей формы, и удерживается тем же способом, каким доска
     удерживает её в клетке (`.figure-body`): ширина считается так, чтобы при
     любой раме карта уместилась в гнездо целиком. До этого `.slot` брал всю
     ширину гнезда и выводил высоту из своего `--aspect` — рама 0.67 давала
     карту на 7px выше гнезда, и карты в руке налезали друг на друга.
     Делитель и форма гнезда — ОДНА переменная: два числа здесь и были
     болезнью. */
  .scene.scene--fill .held > :global(.slot) {
    width: min(100%, calc(100% * var(--aspect, 0.714) / var(--fit, 0.714)));
    height: auto;
  }

  .scene.scene--fill .foot .held--picked,
  .scene.scene--fill .foot .held--mine:hover:not(:disabled) {
    transform: translateY(-0.45rem);
  }

  .scene.scene--fill .omens {
    flex: 0 1 auto;
    flex-basis: auto;
    min-width: 0;
    margin-top: 0;
  }

  .scene.scene--fill .omen-word {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scene.scene--fill .tally-pip {
    width: 1.75em;
    height: 1.75em;
    font-size: clamp(0.9rem, 16cqi, 1.35rem);
  }

  @media (max-width: 820px) {
    .scene.scene--fill .play,
    .scene.scene--fill.scene--along .play {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: 2.1rem minmax(0, 1fr) auto auto;
      grid-template-areas:
        'theirs'
        'board'
        'rail'
        'foot';
    }

    .scene.scene--fill .aside {
      max-height: 7.5rem;
    }

    .scene.scene--fill .verdict {
      grid-template-columns: minmax(0, 1fr);
      justify-items: center;
      text-align: center;
      width: min(28rem, 96%);
      padding: 1.3rem 1.2rem;
    }

    .scene.scene--fill .verdict-copy {
      align-items: center;
    }

    .scene.scene--fill .seal-doors {
      justify-content: center;
    }
  }

  .ledger {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 13px;
    letter-spacing: 0.04em;
    color: #8a6a55;
    min-width: 0;
  }

  .ledger--empty {
    display: none;
  }

  /* Легенда местности. На бумаге — строка под полем; в тёмной комнате её место
     и краски назначает `battle-chamber.css`. */
  .ground-legend {
    margin: 0.4rem 0;
    font-family: Georgia, 'Fraunces', serif;
    color: #5f4636;
  }

  .ground-legend-head {
    margin: 0 0 0.25rem;
    font-size: 0.66rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .ground-legend-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 1rem;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 0.78rem;
    line-height: 1.3;
  }

  .ground-legend-list li {
    display: flex;
    gap: 0.4rem;
    align-items: flex-start;
  }

  .ground-legend-list b {
    font-weight: 600;
    text-transform: capitalize;
  }

  .ledger-turn {
    margin: 0;
    font-size: 1.15rem;
    font-style: italic;
    line-height: 1.3;
    color: #34251c;
  }

  /* На узком экране руки остаются у краёв доски, как задумано: своя половина
     ближе к рукам, и на телефоне это буквально так. */
  .ledger-hands {
    display: none;
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  .table {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    min-width: 0;
    overflow: visible;
    /* Пока стол стоит вертикально — колонка около 420 px: поле 3×6 само по
       себе высокое, растягивать его некуда. Вдоль комнаты потолок снимается. */
    max-width: 26rem;
  }

  .field {
    position: relative;
  }

  .grid {
    position: relative;
    display: grid;
    grid-template-columns: repeat(var(--cols, 3), 1fr);
    gap: 5px;
  }

  /* Черта между половинами. Абсолютная, а не отступ ряда: сетка должна остаться
     ровной, иначе следы ударов поедут вместе с ней. */
  .midline {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    border-top: 1px dashed rgba(52, 37, 28, 0.2);
    pointer-events: none;
  }

  .cell {
    position: relative;
    aspect-ratio: 3 / 4;
    border: 1px solid transparent;
    background: transparent;
    text-align: left;
    overflow: visible;
    transition: background-color 200ms ease, border-color 200ms ease;
  }

  .cell:not(:has(.figure)):not(.cell--open):not(.cell--attack):not(.cell--mend):not(.cell--charm):not(.cell--spot):not(.cell--picked) {
    background: radial-gradient(circle at 50% 50%, rgba(52, 37, 28, 0.1) 1.2px, transparent 1.7px);
  }

  /* Куда можно шагнуть: клетка светлеет на тон, кромка становится сплошной. */
  .cell--open {
    background: rgba(111, 59, 36, 0.06);
    border-color: rgba(52, 37, 28, 0.4);
  }

  .cell--live {
    cursor: pointer;
  }

  /* Выбранное: обводка по кромке КЛЕТКИ — кромка карты занята рамкой чина. */
  .cell--picked {
    border-color: #c65f3c;
  }

  .cell--live:hover .figure,
  .cell--picked .figure {
    transform: translateY(-2px);
    filter: drop-shadow(0 4px 10px rgba(52, 37, 28, 0.28));
  }

  /* Цель: подложка под картой, выступающая на два пиксела. Сплошная — удар,
     прерывистая — лечение. Ни зелёного, ни красного, и разница не в цвете. */
  .cell--attack::before,
  .cell--mend::before,
  .cell--charm::before {
    content: '';
    position: absolute;
    inset: 1px;
    pointer-events: none;
  }

  .cell--attack::before {
    background: #6f3b24;
  }

  .cell--mend::before {
    border: 2px dashed #6f3b24;
  }

  /* Цель чары — ДВОЙНАЯ кромка, приём обводки дома. Третьего цвета тут нет и
     быть не должно: удар от лечения отличается формой, и чара отличается ею
     же, иначе доска начинает говорить цветом то, что сказано формой. */
  .cell--charm::before {
    border: 3px double #6f3b24;
  }

  .figure {
    position: relative;
    z-index: 1;
    display: grid;
    place-items: center;
    height: 100%;
    padding: 3px;
    overflow: visible;
    transition:
      transform 180ms cubic-bezier(0.2, 0.8, 0.25, 1),
      opacity 500ms ease,
      filter 180ms ease;
  }

  /* Карта и её числа — одна коробка. Плашка на клетке уезжала в угол поля,
     пока рама стояла в центре. */
  .figure-body {
    position: relative;
    display: grid;
    place-items: center;
    width: min(100%, calc(133.3333% * var(--fit, 0.714)));
    height: auto;
    aspect-ratio: var(--fit, 0.714);
    container-type: inline-size;
  }

  .figure-body > :global(.slot) {
    width: 100%;
    height: 100%;
  }

  .figure--spent {
    opacity: 0.55;
  }

  .figure--wound {
    z-index: 4;
  }

  /* Подача, вздрагивание и оседание жили здесь тремя правилами и одним
     `@keyframes`. Теперь они — три записи словаря жестов в
     `BattleMotionStage.svelte`, а сюда приходят готовой строкой `animation`.
     Причина не в красоте: три правила не могли назвать движение, которое
     хранитель завёл пять секунд назад. */

  .figure-name,
  .held-name {
    display: block;
    padding: 0.3rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 12px;
    line-height: 1.2;
  }

  .tally {
    position: absolute;
    inset: 0;
    z-index: 3;
    pointer-events: none;
  }

  .tally-pip {
    position: absolute;
    bottom: 3.5%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.18em;
    width: auto;
    min-width: 1.7em;
    height: 1.7em;
    padding: 0 0.3em;
    border-radius: 999px;
    background: #f8f1e7;
    border: 1px solid #6f3b24;
    font-family: Georgia, 'Fraunces', serif;
    font-size: clamp(0.8rem, 14cqi, 1.2rem);
    font-style: normal;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    color: #34251c;
  }

  /* Знак ВОЗЛЕ цифры, а не под ней: кружок доски со знаком становится плашкой
     ровно на знак шире — тот же закон, что у `.corner--marked` на карте.

     И те же 0.72em роста с тем же опусканием на 0.078em: кружок доски набран
     той же Georgia со старостильными цифрами, и центровка коробок поставила бы
     знак выше тела числа (см. `.corner-glyph` в `BattleCard.svelte`, где это
     замерено). Два кружка одного числа обязаны быть сверстаны одинаково. */
  .tally-glyph {
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    width: 0.72em;
    height: 0.72em;
    transform: translateY(0.078em);
    color: inherit;
    opacity: 0.9;
  }

  .tally-glyph :global(svg) {
    width: 100%;
    height: 100%;
  }

  .tally-pip--health {
    left: 3.5%;
  }

  .tally-pip--power {
    right: 3.5%;
    margin-left: 0;
  }

  /* ── Предвестие ────────────────────────────────────────────────────────
     Метка, а не всплывающая подсказка: она стоит на теле, к которому
     относится, и не закрывает собой доску. Своё число тёмное, чужое —
     цветом дома, потому что складывать их нельзя. */
  .omen {
    position: absolute;
    right: 3px;
    bottom: 1.85rem;
    z-index: 3;
    display: flex;
    gap: 2px;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    font-style: normal;
  }

  .omen i {
    padding: 0 3px;
    border: 1px dashed rgba(52, 37, 28, 0.35);
    background: rgba(248, 241, 231, 0.92);
  }

  .omen-mine {
    color: #34251c;
  }

  .omen-theirs {
    color: #c65f3c;
    border-color: rgba(198, 95, 60, 0.5);
  }

  /* Тело, которое до конца круга не доживёт. Пунктир гуще, и больше ничего:
     череп над фигуркой — не эта комната. */
  .omen--falls i {
    border-style: solid;
  }

  .cell--omen {
    outline: 1px dashed rgba(198, 95, 60, 0.45);
    outline-offset: -3px;
  }

  /* Ряд прорыва: тонкая вторая кромка внутри клетки. Не цвет и не пунктир —
     пунктир занят предвестием, цвет — целями хода. */
  .edge {
    position: absolute;
    inset: 4px;
    z-index: 0;
    border: 1px solid rgba(111, 59, 36, 0.24);
    pointer-events: none;
  }

  /* В тёмной комнате у клетки уже есть своя одинарная рамка, и вторая
     одинарная с ней сливалась. Двойной линии на поле нет больше нигде. */
  .scene.scene--fill .edge {
    z-index: 1;
    inset: 7px;
    border: 3px double rgba(212, 176, 106, 0.55);
  }

  /* Тело на краю: та же кромка, сплошнее и в цвет акцента, и поверх карты —
     иначе её закрыло бы само тело, ради которого она нарисована. */
  .cell--poised .edge {
    z-index: 3;
    inset: -3px;
    border: 1.5px solid rgba(198, 95, 60, 0.75);
  }

  /* Строка под печатью. Тише самой печати: она не хвалит, она сообщает. */
  .seal-line {
    margin: 0.35rem 0 0;
    font-size: 0.76rem;
    font-variant-numeric: tabular-nums;
    color: #8a6a55;
  }

  .seal-line--bar {
    font-style: italic;
  }

  .seal-record {
    margin: 0.25rem 0 0;
    font-family: Georgia, 'Fraunces', serif;
    font-style: italic;
    font-size: 0.86rem;
    color: #c65f3c;
  }

  /* Своя строка целиком, а не сосед кнопки в общем ряду. Пока это был сосед,
     выросший текст переносил ряд, и кнопка конца хода уезжала из-под пальца. */
  .omens {
    flex: 1 0 100%;
    display: flex;
    flex-wrap: nowrap;
    align-items: baseline;
    justify-content: flex-end;
    gap: 0.55rem;
    margin-top: 0.25rem;
    /* Высота держится и при пустой строке — по той же причине. */
    min-height: 1.35rem;
  }

  /* Ощутимая мишень: строка в девятнадцать пикселей высотой — это попадание
     с третьего раза, а переключатель, в который не попали, выглядит
     сломанным. */
  .omen-switch {
    flex: 0 0 auto;
    padding: 0.25rem 0.15rem;
    margin: -0.25rem -0.15rem;
    border: none;
    background: none;
    font: inherit;
    font-size: 0.64rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #b9a48c;
    cursor: pointer;
  }

  .omen-switch--on {
    color: #8a6a55;
  }

  /* Состояние названо словом, а не оттенком: разница между #8a6a55 и #b9a48c
     на этой бумаге не читается, и переключатель выглядел неработающим. */
  .omen-state {
    display: inline-block;
    /* Оба слова занимают одно место: «вкл» уже «выкл», и без этой ширины
       переключатель дёргался на несколько пикселей от собственного нажатия. */
    min-width: 2.4rem;
    margin-left: 0.4rem;
    padding: 0 0.25rem;
    border: 1px solid currentColor;
    font-size: 0.9em;
    text-align: center;
  }

  .omen-switch--on .omen-state {
    color: #f8f1e7;
    background: #8a6a55;
  }

  /* Одна строка. Ударов за ход бывает несколько, но полная правда стоит
     метками на самих телах, а строка называет главное и не растёт в высоту —
     иначе она снова начнёт двигать то, что под ней. */
  .omen-word {
    flex: 1 1 auto;
    min-width: 0;
    text-align: right;
    margin: 0;
    font-size: 0.72rem;
    line-height: 1.5;
    color: #8a6a55;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .omen-word--idle {
    font-style: italic;
    color: #b9a48c;
  }

  .omen-line + .omen-line::before {
    content: ' · ';
  }

  .nicks {
    position: absolute;
    right: 2px;
    top: 5px;
    z-index: 2;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nick {
    display: block;
    width: 5px;
    height: 2px;
    background: #6f3b24;
  }

  /* Отказ у выбранного: якорь держит место, плашка внутри — иначе `fly`
     ломает центрирование своим `transform`. */
  .stuck-anchor {
    position: absolute;
    left: 50%;
    bottom: 0.35rem;
    z-index: 40;
    width: max(100%, 7.4rem);
    max-width: 11rem;
    transform: translateX(-50%);
    pointer-events: none;
  }

  .stuck-tip {
    position: relative;
    display: grid;
    place-items: center;
    width: 100%;
    padding: 0.42rem 0.55rem 0.48rem;
    border: 1px solid rgba(198, 95, 60, 0.55);
    background:
      linear-gradient(180deg, rgba(248, 241, 231, 0.97), rgba(232, 214, 190, 0.96));
    box-shadow:
      0 0 0 1px rgba(52, 37, 28, 0.18),
      0 8px 18px rgba(52, 37, 28, 0.28);
  }

  .stuck-tip-flare {
    display: none;
  }

  .stuck-tip-word {
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.68rem;
    font-style: italic;
    font-weight: 500;
    line-height: 1.25;
    text-align: center;
    color: #34251c;
  }

  .held .stuck-anchor {
    bottom: auto;
    top: 0.35rem;
    width: 8rem;
  }



  /* Клетка как ЦЕЛЬ чары: опасная клетка, призыв, свой шаг. Пунктирная
     кромка с уголками — не подложка (та занята ударом) и не двойная кромка
     (та занята чарой по телу): в клетку ткнут, а не по кому-то. */
  .cell--spot::after {
    content: '';
    position: absolute;
    inset: 2px;
    pointer-events: none;
    border: 1px dashed rgba(198, 95, 60, 0.85);
    background: rgba(198, 95, 60, 0.07);
  }

  /* Опасная клетка. Штриховка та же, что у сукна, только гуще и косее: это
     то же поле, только на нём теперь нельзя стоять. */
  .burn {
    position: absolute;
    inset: 1px;
    z-index: 0;
    pointer-events: none;
  }

  .burn-hatch {
    position: absolute;
    inset: 0;
    background: repeating-linear-gradient(
      -45deg,
      rgba(143, 47, 34, 0.22) 0 2px,
      transparent 2px 6px
    );
    border: 1px solid rgba(143, 47, 34, 0.4);
  }

  .burn-num {
    position: absolute;
    top: 2px;
    left: 3px;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.58rem;
    font-style: normal;
    font-variant-numeric: tabular-nums;
    color: #8f2f22;
  }

  .nick--hold {
    width: 3px;
    height: 3px;
    border-radius: 999px;
  }

  .rider--hold .rider-word,
  .rider--aura .rider-word {
    font-style: normal;
  }

  /* Аура — не наложенное: она держится тем, кто ею дышит, и потому подписана
     не сроком, а тем, что он стоит. Отличается тем же, чем всё в этом доме, —
     формой, а не цветом: пунктирная черта слева, как у прерывистой кромки
     лечения на доске. */
  .rider--aura {
    border-inline-start: 1px dashed rgba(111, 59, 36, 0.5);
    padding-inline-start: 0.3rem;
  }

  /* ── Поднос ──────────────────────────────────────────────────────────────
     Печать на способ, и подписана та, что в руке. Разница между взятым и
     просто тронутым видна: взятое — сургуч (дом говорит им «сделано»), всё
     остальное — оттиск на бумаге.

     Мёртвая печать отличается ФОРМОЙ, а не тусклостью: кромка пунктиром и
     никакого сургуча под ней. Это то же правило, по которому удар от лечения
     на доске отличается сплошной подложкой от прерывистой кромки, а не
     красным от зелёного. */
  .tray-slot {
    /* ВНЕ ПОТОКА, и это обязательно: место в сетке назначено прямо
       (`grid-area`), а раскладка ставит назначенное ПРЕЖДЕ остального — так
       что обычным членом сетки поднос занимал клетку доски, и восемнадцатая
       уезжала в новый ряд под сукно. Абсолютный ребёнок сетки место занимает,
       не отнимая: сетка ему — содержащий блок, `inset: 0` — та самая клетка. */
    position: absolute;
    inset: 0;
    z-index: 30;
    /* Поднос накрывает соседние клетки, и брать сквозь него нельзя ничем,
       кроме самих печатей. */
    pointer-events: none;
    /* Печать меряется КЛЕТКОЙ, а не точками: доска растягивается от телефона
       до окна на всю высоту, и поднос, назначивший себе рост в rem, на одном
       краю этого пути — заплатка, на другом — пуговица. Потолок и пол всё же
       есть: ниже полутора рем печать перестаёт быть нажимаемой пальцем. */
    container-type: inline-size;
    --tool: clamp(1.55rem, 26cqw, 2.3rem);
  }

  .tray-anchor {
    position: absolute;
    left: 50%;
    /* Чуть ниже середины КАРТЫ. Не под ней: выбор должен лежать там же, где
       то, о чём он, — на теле, которое взяли в руку. */
    top: 58%;
    display: flex;
    flex-direction: column;
    align-items: center;
    width: max-content;
    transform: translate(-50%, -50%);
  }

  /* Крайней колонке некуда расширяться: поднос прижимается к её кромке вместо
     того, чтобы встать посередине и уехать за сукно. */
  .tray-anchor--start {
    left: 8%;
    align-items: flex-start;
    transform: translate(0, -50%);
  }

  .tray-anchor--end {
    left: auto;
    right: 8%;
    align-items: flex-end;
    transform: translate(0, -50%);
  }

  .tray {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: inherit;
  }

  .tray-row {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.26rem;
    /* Шесть печатей в ряд, дальше — вторым рядом. Карта дома несёт до шести
       умений, то есть до восьми печатей вместе с ударом и лечением: в один ряд
       они не встают, а поднос шире доски — это поднос, который закрывает доску. */
    max-width: 15rem;
    pointer-events: auto;
  }

  .tray-anchor--start .tray-row {
    justify-content: flex-start;
  }

  .tray-anchor--end .tray-row {
    justify-content: flex-end;
  }

  /* Печать — ЧЕРНИЛЬНЫЙ кружок в светлом ореоле, а не бумажный на бумаге.
     Лежит она теперь на самой карте, а карта бывает любой: и почти белой
     бумагой, и тёмной фотографией. Бумажная печать пропадала на первой же
     светлой; чернильная в ореоле читается на обеих, и это не вкус, а условие —
     выбор, которого не видно, не выбор.

     Три кольца, и каждое за своё: светлое отделяет от карты, тёмное снаружи
     держит силуэт на светлом, тень поднимает над бумагой. */
  .tool {
    position: relative;
    display: grid;
    place-items: center;
    width: var(--tool, 2rem);
    height: var(--tool, 2rem);
    padding: 0;
    border: 1.5px solid rgba(248, 241, 231, 0.92);
    border-radius: 999px;
    background: radial-gradient(circle at 34% 28%, #55402f, #2a1f18 76%);
    color: #f6ecda;
    cursor: pointer;
    box-shadow:
      0 0 0 1px rgba(52, 37, 28, 0.85),
      0 3px 8px rgba(52, 37, 28, 0.45);
    transition:
      transform 160ms cubic-bezier(0.2, 0.8, 0.25, 1),
      box-shadow 160ms ease,
      opacity 200ms ease;
    /* Печати ПРИКЛАДЫВАЮТ по одной, а не выкладывают кучей: каждая падает,
       чуть переваливает через себя и садится. Задержка — по месту на подносе,
       и это единственное, для чего `--i` нужен. */
    animation: tool-press 320ms cubic-bezier(0.22, 0.9, 0.3, 1) backwards;
    animation-delay: calc(var(--i, 0) * 42ms);
  }

  /* След от нажатия: кольцо расходится в тот же миг, когда печать села. */
  .tool::before {
    content: '';
    position: absolute;
    inset: -3px;
    border: 1px solid rgba(111, 59, 36, 0.6);
    border-radius: 999px;
    pointer-events: none;
    opacity: 0;
    animation: tool-ripple 460ms ease-out backwards;
    animation-delay: calc(var(--i, 0) * 42ms + 150ms);
  }

  @keyframes tool-press {
    0% {
      opacity: 0;
      transform: translateY(-0.5rem) scale(0.5) rotate(-14deg);
    }
    60% {
      opacity: 1;
      transform: translateY(0) scale(1.12) rotate(3deg);
    }
    100% {
      opacity: 1;
      transform: translateY(0) scale(1) rotate(0deg);
    }
  }

  @keyframes tool-ripple {
    0% {
      opacity: 0.75;
      transform: scale(0.62);
    }
    100% {
      opacity: 0;
      transform: scale(1.6);
    }
  }

  .tool:hover,
  .tool--lit {
    transform: translateY(-2px);
    box-shadow:
      0 0 0 1px rgba(52, 37, 28, 0.85),
      0 5px 12px rgba(52, 37, 28, 0.5);
  }

  /* Тронутое, но не взятое: доска СВЕТИТ его цели, и сказать об этом надо —
     иначе человек читает чужие цели как цели того, что у него в руке.
     Обводка та же, какой обведено выбранное тело: это одно и то же «сейчас
     смотрим сюда». */
  .tool--lit:not(.tool--hand) {
    box-shadow:
      0 0 0 2px #c65f3c,
      0 5px 12px rgba(52, 37, 28, 0.5);
  }

  /* Взятое — сургуч. */
  .tool--hand {
    border-color: rgba(255, 238, 220, 0.95);
    background: radial-gradient(circle at 36% 28%, #e07a4e, #c65f3c 46%, #8d3a22);
    color: #fff6ea;
    transform: translateY(-3px) rotate(-3deg);
    box-shadow:
      0 0 0 1px rgba(90, 30, 16, 0.85),
      inset 0 -2px 6px rgba(90, 30, 16, 0.55),
      0 5px 13px rgba(52, 37, 28, 0.5);
  }

  /* Оттиск на сургуче: кольцо внутри, как у печати дома. */
  .tool--hand::after {
    content: '';
    position: absolute;
    inset: 3px;
    border: 1px dashed rgba(255, 236, 218, 0.5);
    border-radius: 999px;
    pointer-events: none;
  }

  /* Мёртвая печать отличается ФОРМОЙ, а не тусклостью: кромка пунктиром и
     бумага вместо чернил. То же правило, по которому удар от лечения на доске
     отличается сплошной подложкой от прерывистой кромки, а не цветом. */
  .tool--dim {
    border-style: dashed;
    border-color: rgba(52, 37, 28, 0.55);
    background: rgba(248, 241, 231, 0.9);
    color: rgba(52, 37, 28, 0.6);
    cursor: default;
    transform: none;
    box-shadow: 0 0 0 1px rgba(248, 241, 231, 0.7);
  }

  .tool--dim::before {
    display: none;
  }

  .tool--dim:hover {
    transform: none;
    box-shadow: 0 0 0 1px rgba(248, 241, 231, 0.7);
  }

  .tool-seal {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
  }

  /* Число на печати — тем же кружком, каким карта печатает свои. */
  .tool-num {
    position: absolute;
    right: -0.14rem;
    bottom: -0.14rem;
    min-width: 1.05em;
    padding: 0 0.16em;
    border: 1px solid #34251c;
    border-radius: 999px;
    background: #f8f1e7;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.56rem;
    font-weight: 400;
    font-variant-numeric: tabular-nums;
    line-height: 1.45;
    color: #34251c;
  }

  /* Цена — камнями маны, теми же, что в полосе хода. */
  .tool-drops {
    position: absolute;
    top: -0.1rem;
    left: -0.1rem;
    display: flex;
    gap: 1px;
  }

  .tool-drop {
    width: 3px;
    height: 3px;
    background: #f8f1e7;
    outline: 1px solid rgba(52, 37, 28, 0.5);
    transform: rotate(45deg);
  }

  .tool--dim .tool-drop {
    background: #6f3b24;
    outline: none;
  }

  /* Сколько ходов до возвращения. Стоит на самой печати: отката на доске не
     видно НИЧЕМ, и молчание о нём читается как поломка. */
  .tool-sleep {
    position: absolute;
    top: -0.2rem;
    right: -0.2rem;
    display: grid;
    place-items: center;
    width: 0.92rem;
    height: 0.92rem;
    border: 1px solid rgba(52, 37, 28, 0.75);
    border-radius: 999px;
    background: #f8f1e7;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.52rem;
    font-style: normal;
    font-variant-numeric: tabular-nums;
    color: #8f2f22;
  }

  /* Подпись — ВНЕ ПОТОКА, и это не мелочь: пока она стояла в столбце, середина
     подноса приходилась на печати ВМЕСТЕ с ней, и ряд уезжал выше середины
     карты ровно на половину подписи. Теперь по месту равняются печати, а
     подпись висит под ними. */
  .tray-tag {
    position: absolute;
    top: 100%;
    left: 50%;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: center;
    gap: 0 0.34rem;
    width: max-content;
    max-width: 12.5rem;
    margin-top: 0.26rem;
    transform: translateX(-50%) rotate(-0.6deg);
    padding: 0.2rem 0.44rem 0.24rem;
    border: 1px solid rgba(198, 95, 60, 0.5);
    background: linear-gradient(
      180deg,
      rgba(248, 241, 231, 0.97),
      rgba(232, 214, 190, 0.96)
    );
    box-shadow:
      0 0 0 1px rgba(52, 37, 28, 0.14),
      0 6px 14px rgba(52, 37, 28, 0.24);
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.64rem;
    line-height: 1.25;
    color: #34251c;
  }

  /* Нижнему ряду подпись уходит НАД печатями: под ними у него уже сукно. */
  .tray-anchor--over .tray-tag {
    top: auto;
    bottom: 100%;
    margin-top: 0;
    margin-bottom: 0.26rem;
  }

  /* У прижатого к кромке подноса подпись равняется по той же кромке: середина
     чужой ширины увела бы её за сукно. */
  .tray-anchor--start .tray-tag {
    left: 0;
    transform: translateX(0) rotate(-0.6deg);
  }

  .tray-anchor--end .tray-tag {
    left: auto;
    right: 0;
    transform: translateX(0) rotate(-0.6deg);
  }

  /* Светлый ореол — тот же приём, что у печатей: подпись лежит на карте, и
     карта под ней бывает и белой бумагой, и тёмной фотографией. */
  .tray-tag {
    border-color: #6f3b24;
    box-shadow:
      0 0 0 1.5px rgba(248, 241, 231, 0.9),
      0 0 0 2.5px rgba(52, 37, 28, 0.5),
      0 6px 14px rgba(52, 37, 28, 0.3);
  }

  .tray-word {
    font-style: italic;
    font-weight: 600;
    color: #6f3b24;
  }

  .tray-nums {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 0.3rem;
    opacity: 0.85;
  }

  .tray-of,
  .tray-turns,
  .tray-reach,
  .tray-spread {
    display: inline-flex;
    align-items: center;
    gap: 0.14rem;
  }

  .tray-why {
    font-style: italic;
    color: #8f2f22;
  }

  /* `prefers-reduced-motion` — обязательство: поднос выкладывается сразу. */
  @media (prefers-reduced-motion: reduce) {
    .tool,
    .tool::before {
      animation: none;
    }

    .tool::before {
      display: none;
    }

    .tool,
    .tool:hover,
    .tool--hand,
    .tool--lit {
      transform: none;
    }
  }

  /* ── Правило круга и листок ─────────────────────────────────────────────
     Приписка стоит у слова «Круг», потому что правило держит сторону, а не
     тело. Печать рядом — вход для того, кто пришёл прочесть; зовёт она один
     раз, в тот миг, когда правило впервые укусило, и гаснет навсегда, как
     только листок открыли: мигание, которое не кончается, перестаёт значить
     что-либо на третьем круге. */
  .opening {
    margin-inline-start: 0.5rem;
    font-size: 0.9em;
    opacity: 0.8;
  }

  .opening--spent {
    opacity: 1;
    color: #c65f3c;
  }

  .scene.scene--chamber .opening--spent {
    color: #ffc978;
  }

  .seal-wrap {
    position: relative;
    display: inline-flex;
    margin-inline-start: 0.45rem;
    vertical-align: -0.2em;
  }

  .rules-seal {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.45rem;
    height: 1.45rem;
    padding: 0;
    border: 1px solid currentColor;
    border-radius: 999px;
    background: transparent;
    color: inherit;
    opacity: 0.65;
    cursor: pointer;
    transition: opacity 200ms ease, color 200ms ease;
  }

  .rules-seal:hover,
  .rules-seal[aria-expanded='true'] {
    opacity: 1;
  }

  /* Один зов, а не постоянное мигание: класс снимается открытием листка. */
  .rules-seal--calls {
    opacity: 1;
    color: #c65f3c;
    animation: seal-call 2.2s ease-in-out infinite;
  }

  .scene.scene--chamber .rules-seal--calls {
    color: #ffc978;
  }

  @keyframes seal-call {
    0%,
    100% {
      box-shadow: 0 0 0 0 rgba(198, 95, 60, 0);
    }
    50% {
      box-shadow: 0 0 0 5px rgba(198, 95, 60, 0.22);
    }
  }

  /* `prefers-reduced-motion` — обязательство: зов остаётся, качание уходит. */
  @media (prefers-reduced-motion: reduce) {
    .rules-seal--calls {
      animation: none;
      box-shadow: 0 0 0 3px rgba(198, 95, 60, 0.22);
    }
  }

  .rules-leaf {
    position: absolute;
    top: calc(100% + 0.5rem);
    inset-inline-end: 0;
    z-index: 30;
    width: 20rem;
    max-width: 72vw;
    padding: 0.7rem 0.85rem 0.8rem;
    border: 1px solid #d8c6b1;
    background: #f8f1e7;
    box-shadow: 0 10px 26px rgba(0, 0, 0, 0.35);
    color: #34251c;
    /* Герб набран разрядкой и прописными; листок — обычный текст, и наследовать
       ту разрядку он не должен. */
    font-family: Georgia, 'Fraunces', serif;
    letter-spacing: normal;
    text-transform: none;
    text-align: start;
  }

  .rules-leaf-head {
    margin: 0 0 0.5rem;
    font-size: 0.72rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .rules-leaf-list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .rules-leaf-list li {
    font-size: 0.84rem;
    line-height: 1.35;
  }

  .rules-leaf-note {
    display: block;
    margin-top: 0.25rem;
    font-size: 0.78rem;
    line-height: 1.45;
    font-style: italic;
    color: #6f3b24;
  }

  /* Комната: стили ЗДЕСЬ, не в chamber.css — scoped `.stuck-tip` иначе
     перебивает внешний файл равным весом и оставляет пергамент поверх зала. */
  .scene.scene--chamber .stuck-tip {
    padding: 0.55rem 0.65rem 0.58rem;
    border: 0;
    background:
      url('/battles/chamber/chamber-plaque.png?v=2') center / 100% 100% no-repeat,
      radial-gradient(120% 90% at 50% 0%, rgba(90, 212, 240, 0.2), transparent 55%),
      linear-gradient(165deg, #2a1c12 0%, #120c09 48%, #1a100c 100%);
    box-shadow:
      0 0 0 1px rgba(232, 196, 120, 0.55),
      0 0 0 2px rgba(20, 12, 8, 0.9),
      0 0 18px 2px rgba(62, 200, 232, 0.45),
      0 10px 22px rgba(0, 0, 0, 0.55);
    animation: chamber-stuck-glow 2.4s ease-in-out infinite;
  }

  .scene.scene--chamber .stuck-tip-flare {
    display: block;
    position: absolute;
    inset: -22% -10% auto;
    height: 75%;
    pointer-events: none;
    background:
      radial-gradient(ellipse at 50% 100%, rgba(255, 196, 96, 0.35), transparent 68%),
      radial-gradient(ellipse at 50% 0%, rgba(90, 212, 240, 0.45), transparent 60%);
    mix-blend-mode: screen;
    opacity: 0.85;
    animation: chamber-stuck-flare 1.8s ease-in-out infinite;
  }

  .scene.scene--chamber .stuck-tip-word {
    position: relative;
    z-index: 1;
    font-size: clamp(0.58rem, 1.8cqi + 0.32rem, 0.78rem);
    font-style: normal;
    font-weight: 600;
    letter-spacing: 0.04em;
    line-height: 1.3;
    color: #fff4d8;
    text-shadow:
      0 1px 0 #1a1008,
      0 0 12px rgba(255, 180, 60, 0.35);
  }

  .scene.scene--chamber .held .stuck-anchor {
    animation: chamber-stuck-float 2.8s ease-in-out infinite;
  }

  @keyframes chamber-stuck-glow {
    0%,
    100% {
      box-shadow:
        0 0 0 1px rgba(232, 196, 120, 0.5),
        0 0 0 2px rgba(20, 12, 8, 0.9),
        0 0 14px 1px rgba(62, 200, 232, 0.35),
        0 10px 22px rgba(0, 0, 0, 0.55);
    }
    50% {
      box-shadow:
        0 0 0 1px rgba(255, 220, 140, 0.85),
        0 0 0 2px rgba(20, 12, 8, 0.95),
        0 0 26px 4px rgba(90, 212, 240, 0.65),
        0 12px 26px rgba(0, 0, 0, 0.6);
    }
  }

  @keyframes chamber-stuck-flare {
    0%,
    100% {
      opacity: 0.55;
      transform: scaleY(1);
    }
    50% {
      opacity: 1;
      transform: scaleY(1.08);
    }
  }

  @keyframes chamber-stuck-float {
    0%,
    100% {
      transform: translateX(-50%);
    }
    50% {
      transform: translateX(-50%) translateY(-3px);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .scene.scene--chamber .stuck-tip,
    .scene.scene--chamber .stuck-tip-flare,
    .scene.scene--chamber .held .stuck-anchor {
      animation: none;
    }
  }

  .breakdown {
    min-width: 0;
    margin: 0.3rem 0 0.5rem;
    padding: 0.6rem 0.75rem;
    background: #f8f1e7;
    border: 1px solid #d8c6b1;
    font-size: 12px;
    line-height: 1.5;
  }

  .breakdown-head {
    margin-bottom: 0.3rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 13px;
  }

  .breakdown-row {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    color: #5f4636;
  }

  .breakdown-total {
    margin-top: 0.25rem;
    padding-top: 0.25rem;
    border-top: 1px solid rgba(52, 37, 28, 0.15);
    color: #34251c;
  }

  .hand {
    display: flex;
    flex-wrap: nowrap;
    justify-content: flex-start;
    gap: 0;
  }

  .hand-label {
    margin: 0 0 0.2rem;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.78rem;
    font-style: italic;
    letter-spacing: 0.02em;
    text-transform: none;
    color: #8a6a55;
  }

  .held {
    position: relative;
    display: block;
    width: 4.6rem;
    margin-inline: 0;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
  }

  .held-cost {
    position: absolute;
    left: 3px;
    bottom: 3px;
    z-index: 2;
    display: inline-flex;
    align-items: center;
    gap: 0.3em;
    min-width: 1.15rem;
    padding: 0.04rem 0.28rem;
    background: #f8f1e7;
    border: 1px solid rgba(52, 37, 28, 0.4);
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
    line-height: 1.15;
    color: #34251c;
    pointer-events: none;
  }

  .held--mine {
    cursor: pointer;
  }

  .held--picked {
    outline: 1px solid #c65f3c;
    outline-offset: 1px;
  }

  /* Выложить нельзя — и это должно читаться сразу, а не угадываться. Было
     0.4 без каких-либо других отличий, и карта выглядела просто бледной. */
  .held--dim {
    opacity: 0.32;
    filter: grayscale(0.7);
    cursor: default;
  }

  .ledger-hands .hand {
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  .ledger-hands .held {
    width: 5.6rem;
  }

  .table-hand--theirs .hand {
    justify-content: center;
  }

  .table-hand--theirs .held {
    width: 4.15rem;
    margin-inline: -0.4rem;
  }

  .ledger-note {
    display: none;
  }

  .hand-why {
    margin: 0;
    max-width: 13rem;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.68rem;
    font-style: italic;
    line-height: 1.35;
    text-align: right;
    color: #8a6a55;
  }

  /* Ближний край стола: карты лежат веером, ход — фраза справа, не столбик. */
  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 0.55rem 0.9rem;
    min-width: 0;
    padding: 0.15rem 0.1rem 0.1rem;
    overflow: visible;
  }

  .foot .table-hand--mine {
    flex: 1 1 auto;
    min-width: 0;
    overflow: visible;
  }

  .foot .hand-label {
    display: none;
  }

  .foot .hand {
    justify-content: flex-start;
    padding: 0.7rem 0.95rem 0.12rem 0.8rem;
    overflow: visible;
  }

  .foot .held {
    position: relative;
    z-index: calc(1 + var(--i, 0));
    width: 10.2rem;
    flex: 0 0 auto;
    margin-inline: calc(-0.85rem - 0.16rem * var(--n, 3));
    transform-origin: 50% 100%;
    transform: rotate(calc((var(--i, 0) - (var(--n, 1) - 1) / 2) * 4.6deg))
      translateY(0.28rem);
    filter: drop-shadow(0 3px 7px rgba(52, 37, 28, 0.18));
    transition:
      transform 180ms cubic-bezier(0.2, 0.8, 0.25, 1),
      filter 180ms ease;
  }

  .foot .held--picked,
  .foot .held--mine:hover:not(:disabled) {
    z-index: 12;
    transform: rotate(0deg) translateY(-0.55rem);
    filter: drop-shadow(0 8px 14px rgba(52, 37, 28, 0.28));
  }

  /* Ширина здесь ЗАДАНА, а не набрана содержимым. Пока она набиралась, любая
     строка предвестия — появившаяся, сменившаяся, опустевшая — меняла ширину
     всего столбца, и «Закончить ход» уезжал на две сотни пикселей в сторону.
     Кнопка, уходящая из-под пальца, хуже любой подсказки. */
  .turn {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 0.7rem 0.85rem;
    flex: 0 1 22rem;
    width: 22rem;
    max-width: 100%;
  }

  .turn-act {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.15rem;
    flex: 0 0 auto;
  }

  .end {
    position: relative;
    font-family: Georgia, 'Fraunces', serif;
    font-style: italic;
    font-size: 1.38rem;
    line-height: 1.15;
    letter-spacing: 0.01em;
    padding: 0.45rem 0.1rem 0.38rem;
    border: none;
    background: none;
    color: #6f3b24;
    white-space: nowrap;
    cursor: pointer;
  }

  .end:not(:disabled)::before {
    content: '';
    display: inline-block;
    width: 0.38rem;
    height: 0.38rem;
    margin-right: 0.5rem;
    vertical-align: 0.12em;
    border-radius: 42% 58% 47% 53% / 52% 44% 56% 48%;
    background: #c65f3c;
  }

  .end:not(:disabled)::after {
    content: '';
    position: absolute;
    left: 1.05rem;
    right: 0;
    bottom: 0.22rem;
    height: 1px;
    background: #c65f3c;
    opacity: 0.7;
  }

  .end:disabled {
    color: #8a6a55;
    opacity: 0.45;
    cursor: default;
  }

  .end:not(:disabled):hover {
    background: none;
    color: #c65f3c;
  }

  .end:not(:disabled):hover::after {
    opacity: 1;
  }

  .waiting {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.72rem;
    font-style: italic;
    color: #8a6a55;
  }

  /* Пока журнал играет, доска не принимает касаний, и курсор обычный. */
  .scene--held .cell,
  .scene--held .held {
    cursor: default;
  }

  .aside {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    min-width: 12rem;
    max-width: 16rem;
    font-size: 12px;
  }

  .chosen {
    padding: 0 0 0 0.85rem;
    border: none;
    border-left: 1px solid #d8c6b1;
  }

  .chosen-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.6rem;
    margin-bottom: 0.4rem;
  }

  .chosen-name {
    flex: 1;
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
    background: none;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 16px;
    text-align: left;
    color: inherit;
    cursor: pointer;
  }

  p.chosen-name {
    cursor: default;
  }

  button.chosen-name:hover {
    color: #6f3b24;
  }

  .chosen-leaf {
    flex-shrink: 0;
    margin: 0;
    padding: 0.12em 0.5em;
    font: inherit;
    font-size: 10px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #34251c;
    background: #f8f1e7;
    border: 1px solid #d8c6b1;
    cursor: pointer;
  }

  .chosen-leaf:hover {
    border-color: #6f3b24;
  }

  .chosen-kind {
    margin: 0 0 0.55rem;
    font-size: 10px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .chosen-kind .sep {
    margin: 0 0.35em;
    opacity: 0.55;
  }

  .chosen-stats {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.15rem 1rem;
    color: #5f4636;
  }

  .chosen-stats div {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .chosen-stats dt {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    min-width: 0;
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  /* Знак меряется строкой, а не точками: разбор набран десятью пикселями, и
     знак в четырнадцать стоял бы над словом вдвое выше него. */
  .chosen-stats dt :global(svg) {
    flex: 0 0 auto;
    opacity: 0.75;
  }

  .chosen-effect {
    margin: 0.65rem 0 0;
    padding-top: 0.5rem;
    border-top: 1px solid rgba(52, 37, 28, 0.12);
    font-family: Georgia, 'Fraunces', serif;
    font-size: 13px;
    font-style: italic;
    line-height: 1.45;
    color: #5f4636;
  }

  .chosen-riders {
    margin-top: 0.5rem;
    padding-top: 0.4rem;
    border-top: 1px solid rgba(52, 37, 28, 0.12);
    color: #5f4636;
  }

  .journal-label {
    margin-bottom: 0.4rem;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.82rem;
    font-style: italic;
    letter-spacing: 0.02em;
    text-transform: none;
    color: #8a6a55;
  }

  .journal-empty {
    font-style: italic;
    color: #8a6a55;
  }

  .journal-lines {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    color: #5f4636;
    line-height: 1.5;
  }

  .journal-open {
    font: inherit;
    text-align: left;
    border-bottom: 1px dotted rgba(52, 37, 28, 0.3);
    cursor: pointer;
  }

  .journal-open:hover {
    color: #c65f3c;
  }

  /* ── Печать ────────────────────────────────────────────────────────────── */

  /* ПОКРЫВАЛО исхода, и имя у него своё. Оно звалось `.seal-wrap` — тем же
     словом, что и оправа печати свода правил в шапке, — а правила Svelte при
     равном весе решает порядком: покрывало стоит в стилях ниже и назначало
     СВОЁ `position: absolute; inset: 0` ОБОИМ. Оправа в шапке от этого
     растягивалась на всю табличку, и кружок свода вставал ровно посреди слов
     «Ваш ход». Надпись была не тусклая — она была ПЕРЕКРЫТА. */
  .seal-veil {
    position: absolute;
    inset: 0;
    z-index: 6;
    background: rgba(248, 241, 231, 0.82);
  }

  /* Поле 3×6 выше окна, поэтому печать держится в виду, а не в середине доски:
     оттиск, за которым надо прокручивать, — это не оттиск. */
  .verdict {
    position: sticky;
    top: max(1.5rem, calc(50vh - 7rem));
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.85rem;
    width: max-content;
    max-width: 100%;
    margin: 0 auto;
    padding: 1.5rem 1.75rem;
    text-align: center;
    background: #f8f1e7;
    border: 1px solid #d8c6b1;
    outline: 1px solid #d8c6b1;
    outline-offset: 4px;
    box-shadow: 0 6px 28px rgba(52, 37, 28, 0.2);
    transform: rotate(-1deg);
  }

  .verdict--dim {
    opacity: 0.88;
    transform: none;
  }

  .seal-word {
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 20px;
    color: #34251c;
  }

  .seal-dust {
    margin-top: 0.4rem;
    font-size: 12px;
    color: #6f3b24;
  }

  .seal-doors {
    display: flex;
    gap: 1rem;
    justify-content: center;
    margin-top: 1rem;
  }

  .door {
    font-family: inherit;
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
    cursor: pointer;
  }

  .door:hover {
    color: #c65f3c;
  }

  /* Комната: печать накрывает зал целиком — не клетку поля. Панель лежит
     вдоль, чернила светлые на тёмном металле. Пергамент сюда не входит. */
  .scene.scene--fill .seal-veil {
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 1.6rem 2rem;
    background: rgba(6, 4, 3, 0.9);
    backdrop-filter: blur(2px);
  }

  .scene.scene--fill .verdict {
    position: static;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 1.6rem 2.4rem;
    width: min(58rem, 92%);
    max-width: 58rem;
    margin: 0;
    padding: 2rem 2.4rem;
    text-align: left;
    border: 2px solid #e0c078;
    outline-offset: 6px;
    transform: none;
  }

  .scene.scene--fill .verdict--dim {
    opacity: 1;
    transform: none;
  }

  .scene.scene--fill .verdict-copy {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.55rem;
    min-width: 0;
  }

  .scene.scene--fill .seal-word {
    font-size: clamp(2rem, 3.6vw, 2.7rem);
    line-height: 1.12;
    text-shadow: 0 1px 0 #1a1208;
  }

  .scene.scene--fill .seal-dust,
  .scene.scene--fill .seal-line {
    margin: 0;
    font-size: 1.22rem;
    font-weight: 600;
    line-height: 1.4;
  }

  .scene.scene--fill .seal-line--bar {
    font-style: italic;
    color: #ffe08a;
  }

  .scene.scene--fill .seal-record {
    margin: 0;
    font-size: 1.2rem;
    color: #ffe08a;
  }

  .scene.scene--fill .seal-doors {
    justify-content: flex-start;
    flex-wrap: wrap;
    gap: 0.85rem 1.1rem;
    margin-top: 0.85rem;
  }

  .scene.scene--fill .door {
    font-size: 0.78rem;
    letter-spacing: 0.14em;
    padding: 0.7rem 1.1rem 0.62rem;
    border: 1px solid #e0c078;
    background: #2a1c10;
  }

  .scene.scene--fill .door:hover {
    color: #ffffff;
    border-color: #ffe08a;
    background: #3a2818;
  }

  /* Обязательство, а не украшение. `stage()` при этой настройке не отдаёт ни
     одного жеста и ни одной копии, так что гасить здесь нечего — кроме того,
     что сцена рисует сама. */
  @media (prefers-reduced-motion: reduce) {
    .figure,
    .cell {
      transition: none;
    }

    .foot .held,
    .foot .held--picked,
    .foot .held--mine:hover:not(:disabled) {
      transform: none;
      transition: none;
    }
  }

  .cloth {
    min-width: 0;
    padding: 0.42rem;
    background: #f3e6d4;
    border: 1px solid #d8c6b1;
    outline: 1px solid #d8c6b1;
    outline-offset: 3px;
    box-shadow: 0 8px 22px rgba(52, 37, 28, 0.08);
  }

  .face {
    position: relative;
  }
</style>
