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
  import { fade } from 'svelte/transition';
  import { t, lang, type TranslationKey } from '$lib/i18n';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import BattleSheet from '$lib/components/BattleSheet.svelte';
  import BattleMotionStage from '$lib/components/BattleMotionStage.svelte';
  import WaxSeal from '$lib/components/WaxSeal.svelte';
  import {
    DEFAULT_ASPECT,
    BODY_STAT_LABELS,
    bodyPassport,
    cardCopy,
    channelLabelKey,
    frameForCard,
    kindLabelKey,
    MOTION_MS_MAX,
    motionFor,
    motionSpan,
    motionWound,
    occasionOf,
    stage,
    struckOf,
    WARD_MOTION,
    type HitWear,
    type ScrapFly,
    type Staged,
    type StruckKind,
  } from '$lib/battles';
  import type {
    BattleAction,
    BattleCard as BattleCardDto,
    BattleCell,
    BattleEvent,
    BattleFrame,
    BattleMatch,
    BattleMatchState,
    BattleUnit,
    Foresight,
    Motion,
  } from '$lib/types/api';

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
  } = $props();

  const WIDTH = 3;
  const DEPTH = 6;
  /** Комната, в которой стол ложится вдоль: шесть портретов в ряд на более
   *  узкой — штампы. Совпадает с `.scene--along`, не с контейнерным 900 px:
   *  три колонки сцены появляются раньше, чем доске хватает ширины. */
  const ALONG = 1100;

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

  let byCell = $derived(
    new Map(position.board.map((s) => [`${s.cell.x},${s.cell.y}`, s.unit])),
  );
  const unitAt = (x: number, y: number): BattleUnit | null => {
    const id = byCell.get(`${x},${y}`);
    return id === undefined ? null : (position.units[id] ?? null);
  };
  const cellOf = (unit: number): BattleCell | null => {
    const spot = position.board.find((s) => s.unit === unit);
    return spot ? spot.cell : null;
  };

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

  /** Тела, по которым выбранное может ударить или которые может залечить. */
  let openUnits = $derived.by(() => {
    const out = new Map<number, 'attack' | 'mend'>();
    if (picked?.kind !== 'unit' || !mine) return out;
    for (const a of legal) {
      if (typeof a === 'string') continue;
      if ('attack' in a && a.attack.attacker === picked.id) out.set(a.attack.target, 'attack');
      if ('mend' in a && a.mend.healer === picked.id) out.set(a.mend.target, 'mend');
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
    if (here && openUnits.has(here.id) && picked?.kind === 'unit') {
      return openUnits.get(here.id) === 'attack'
        ? { attack: { attacker: picked.id, target: here.id } }
        : { mend: { healer: picked.id, target: here.id } };
    }
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

      // Готовое своё тело — выбор для хода. Подсветка клеток только у него.
      if (here && here.owner === me && ready.has(here.id)) {
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

  function tapHand(index: number) {
    if (!mine || !playableHand.has(index)) return;
    picked = picked?.kind === 'hand' && picked.index === index ? null : { kind: 'hand', index };
  }

  function onkey(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
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
    if ('moved' in e) {
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

    // Выставление и лечение: тело должно существовать (или уже быть залечено),
    // прежде чем ему что-то показывать.
    transcribe(pos, e);
    live = copy(pos);
    if ('played' in e || 'healed' in e) {
      const striker = 'healed' in e ? e.healed.by : null;
      const target = 'played' in e ? e.played.unit : e.healed.target;
      const motion = motionOf(pos, striker ?? target, e);
      // Выставление показывается на самом вышедшем теле, а не на бьющем:
      // у него нет ни автора, ни цели, есть только оно само.
      put(pos, motion, 'played' in e ? target : striker, 'played' in e ? null : target);
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
      if ('turnEnded' in e || 'finished' in e) {
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
      if (!('turnEnded' in e) && !('finished' in e)) {
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
      // снимок сервера и есть истина, играть нечего.
      run++;
      running = false;
      playing = false;
      live = null;
      before = match.state;
      return;
    }

    void playThrough(before, events);
  }

  // ── Журнал ────────────────────────────────────────────────────────────────
  const STEP_KEY: Record<string, TranslationKey> = {
    immunity: 'battleStepImmunity',
    pointBlank: 'battleStepPointBlank',
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

  type Line = {
    text: string;
    trail: { step: string; from: number; to: number }[];
    total: number;
    head: string;
  };
  let journal = $derived.by<Line[]>(() =>
    told.map((e) => {
      const bare = (text: string): Line => ({ text, trail: [], total: 0, head: '' });
      if ('played' in e) return bare(`${nameOf(e.played.unit)} — ${$t('battleLogPlayed')}`);
      if ('moved' in e) return bare(`${nameOf(e.moved.unit)} — ${$t('battleLogMoved')}`);
      if ('healed' in e)
        return bare(`${nameOf(e.healed.target)} — ${$t('battleLogHealed')} ${e.healed.amount}`);
      if ('died' in e) return bare(`${nameOf(e.died.target)} — ${$t('battleLogDied')}`);
      if ('immune' in e) return bare(`${nameOf(e.immune.target)} — ${$t('battleLogImmune')}`);
      if ('turnEnded' in e)
        return bare(
          e.turnEnded.side === me ? $t('battleLogTurnYours') : $t('battleLogTurnKeeper'),
        );
      if ('damaged' in e) {
        const d = e.damaged;
        const total = d.toHealth + d.toShield;
        return {
          text: `${nameOf(d.target)} — ${$t('battleLogDamaged')} ${total}`,
          trail: d.trail,
          total,
          head: d.by == null ? '' : `${nameOf(d.by)} → ${nameOf(d.target)}`,
        };
      }
      return bare('');
    }).filter((l) => l.text),
  );

  let open = $state<number | null>(null);

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
  let along = $state(false);

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
      along = w >= ALONG;
    };
    apply(el.getBoundingClientRect().width);
    const ro = new ResizeObserver((entries) => {
      apply(entries[0]?.contentRect.width ?? 0);
    });
    ro.observe(el);
    return () => ro.disconnect();
  });

  const rows = Array.from({ length: DEPTH }, (_, y) => y);
  const cols = Array.from({ length: WIDTH }, (_, x) => x);
  /** Порядок клеток на экране. Движок не знает про это: x и y те же. */
  let spots = $derived(
    along
      ? cols.flatMap((x) => rows.map((y) => ({ x, y })))
      : rows.flatMap((y) => cols.map((x) => ({ x, y }))),
  );
</script>

<svelte:window onkeydown={onkey} />

<!-- Руки пишутся один раз и ставятся дважды: на широком экране — в боковую
     колонку, на узком — по краям доски. Не два куска разметки, а один снипет
     в двух местах: доска, которая рисует свою руку иначе, чем полка, начнёт
     врать ровно про то, что на ней проверяют. -->
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

{#snippet ownHand()}
  {#if hand.length}
    <p class="hand-label">{$t('battleHandYours')}</p>
    <div class="hand">
      {#each hand as held, i (i)}
        {@const dto = dtoOf(held.name)}
        <button
          type="button"
          disabled={!mine || !playableHand.has(i)}
          onclick={() => tapHand(i)}
          class="held held--mine"
          class:held--picked={picked?.kind === 'hand' && picked.index === i}
          class:held--dim={!playableHand.has(i)}
          style="--i:{i}; --n:{hand.length}"
        >
          {#if dto}
            <BattleCard card={dto} {frames} owned={true} transition={false} interactive={false} />
          {:else}
            <span class="held-name">{titleOf(held.name)}</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
{/snippet}

<!-- Комната и стол — разные элементы: элемент не может спрашивать свой
     собственный контейнер, и запрос ниже молча мерил бы страницу. -->
<div class="room" bind:this={roomEl}>
<div class="scene" class:scene--held={playing} class:scene--along={along}>
  <!-- Слева: круг, чей ход, мана обеих сторон. -->
  <aside class="ledger">
    <p class="ledger-turn">
      {position.active === me ? $t('battleWhoseTurnYours') : $t('battleWhoseTurnKeeper')}
    </p>
    <p class="ledger-line">{$t('battleRound')} {position.round}</p>
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
    <p class="ledger-line">
      {$t('battleManaYours')}
      <span class="num">
        {#if position.active === 'player'}{position.player.mana}/{position.player.manaMax}
        {:else}{position.player.manaMax}{/if}
      </span>
    </p>
    <p class="ledger-line">
      {$t('battleManaKeeper')}
      <span class="num">
        {#if position.active === 'keeper'}{position.keeper.mana}/{position.keeper.manaMax}
        {:else}{position.keeper.manaMax}{/if}
      </span>
    </p>
    <p class="ledger-note">{$t('battleManaNote')}</p>

    <!-- Рука хранителя в этой колонке на широком экране: тогда под доской
         остаются только свои карты и ход, и они помещаются в окно. -->
    <div class="ledger-hands">
      {@render keeperHand()}
    </div>
  </aside>

  <div class="table" bind:this={tableEl} style="--room:{room}px">
    <div class="table-hand table-hand--theirs">{@render keeperHand()}</div>

    <div class="field" bind:this={fieldEl}>
      <div class="cloth">
      <div class="face">
      <div class="grid">
        {#each spots as { x, y } (`${x},${y}`)}
          {@const here = unitAt(x, y)}
          {@const open2 = openCells.has(`${x},${y}`)}
          {@const target = here ? openUnits.get(here.id) : undefined}
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
            aria-label={here ? titleOf(here.card.name) : `${x},${y}`}
            class="cell"
            class:cell--omen={!!willGet}
            class:cell--open={open2}
            class:cell--picked={picked?.kind === 'unit' && here?.id === picked.id}
            class:cell--attack={target === 'attack'}
            class:cell--mend={target === 'mend'}
            class:cell--live={mine && here?.owner === me && ready.has(here.id)}
          >
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
                      wearSeed={here.id}
                      struck={acting?.target === here.id ? acting.struck : null}
                      scrap={flying?.unit === here.id ? flying : null}
                    />
                  {:else}
                    <span class="figure-name">{titleOf(here.card.name)}</span>
                  {/if}
                </span>

                <!-- Число — только у раненых. Целое тело сообщает о себе тем,
                     что молчит. -->
                {#if here.health.current < here.health.max}
                  <span class="wound">{here.health.current}</span>
                {/if}

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

                {#if here.statuses.length}
                  <span class="nicks" aria-hidden="true">
                    {#each here.statuses as st, i (i)}<i class="nick"></i>{/each}
                  </span>
                {/if}
              {/if}
            </button>
          {/each}
        <span class="midline" aria-hidden="true"></span>
      </div>

      <!-- Нарисованное: стрела в полёте, вспышка на цели, полоса кадров.
           Лежит в тех же `inset: 0` от поля, что печать, — поле шире
           доски увело бы всё вбок. -->
      <BattleMotionStage motes={acting?.play.motes ?? []} />

      <!-- Исход. Сургучная печать, тот же оттиск, что при получении карты. -->
      {#if position.outcome && !playing}
        <div class="seal-wrap" transition:fade={{ duration: 200 }}>
          <div class="verdict" class:verdict--dim={position.outcome !== 'player'}>
            <WaxSeal size="6.5rem" dim={position.outcome !== 'player'} />
            <p class="seal-word">
              {position.outcome === 'player'
                ? $t('battleWonByPlayer')
                : position.outcome === 'keeper'
                  ? $t('battleWonByKeeper')
                  : $t('battleDrawn')}
            </p>
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
      {/if}
      </div>
      </div>
    </div>

    <!-- Своя рука и ход — ближний край стола: веер карт и фраза хода
         в одной полосе, чтобы оба оставались в окне. -->
    <div class="foot">
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

  <aside class="aside">
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
        <dl class="chosen-stats">
          <div>
            <dt>{$t('battlesHealthLabel')}</dt>
            <dd class="num">{chosen.health.current}/{chosen.health.max}</dd>
          </div>
          {#if chosenDto}
            <div><dt>{$t('battlesCostLabel')}</dt><dd class="num">{chosenDto.cost}</dd></div>
            <div><dt>{$t('battlesPowerLabel')}</dt><dd class="num">{chosenDto.power}</dd></div>
            {#each chosenPass.filter((row) => row.field !== 'health') as row (row.field)}
              <div>
                <dt>{$t(BODY_STAT_LABELS[row.field])}</dt>
                <dd class="num">{row.value}</dd>
              </div>
            {/each}
          {:else}
            <div><dt>{$t('battlesPowerLabel')}</dt><dd class="num">{chosen.power}</dd></div>
            {#if chosen.armor > 0}<div><dt>{$t('battleStatArmour')}</dt><dd class="num">{chosen.armor}</dd></div>{/if}
            {#if chosen.ward > 0}<div><dt>{$t('battleStatWard')}</dt><dd class="num">{chosen.ward}</dd></div>{/if}
            {#if chosen.reach > 0}<div><dt>{$t('battleStatReach')}</dt><dd class="num">{chosen.reach}</dd></div>{/if}
            {#if chosen.step > 0}<div><dt>{$t('battleStatStep')}</dt><dd class="num">{chosen.step}</dd></div>{/if}
            {#if chosen.mend > 0}<div><dt>{$t('battleStatMend')}</dt><dd class="num">{chosen.mend}</dd></div>{/if}
          {/if}
        </dl>
        {#if chosenFace?.effect}
          <p class="chosen-effect">{chosenFace.effect}</p>
        {/if}
        {#if chosen.statuses.length}
          <ul class="chosen-riders">
            {#each chosen.statuses as st, i (i)}
              <li>{st.name} <span class="num">{st.amount}</span> · {$t('battleStatusTurns')} {st.turns}</li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}

    <div class="journal" class:journal--vacant={!journal.length}>
      <p class="journal-label">{$t('battleJournal')}</p>
      {#if !journal.length}
        <p class="journal-empty">{$t('battleJournalEmpty')}</p>
      {:else}
        <ul class="journal-lines">
          {#each journal as line, i (i)}
            <li>
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
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </aside>
</div>
</div>

{#if sheet}
  <BattleSheet card={sheet} {frames} taking={false} onclose={() => (sheet = null)} />
{/if}

<style>
  .room {
    container-type: inline-size;
  }

  .scene {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
    color: #34251c;
  }

  /* Три колонки только там, где они помещаются. Стол хранителя рисует ту же
     сцену в 24rem, и там она складывается в одну колонку сама. */
  @container (min-width: 900px) {
    .scene {
      display: grid;
      grid-template-columns: 13rem minmax(0, 26rem) minmax(0, 1fr);
      align-items: start;
      gap: 1.1rem 1.35rem;
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
      width: 4.65rem;
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
  .scene.scene--along {
    display: grid;
    grid-template-columns: 11rem minmax(0, 1fr) minmax(12rem, 16rem);
    align-items: start;
    gap: 1rem 1.25rem;
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
  }

  /* Шесть колонок 3:4 дают H ≈ ⅔·W. Ширину берём из остатка высоты, иначе
     поле занимает окно целиком, а рука и ход уезжают под складку. */
  .scene--along .field,
  .scene--along .foot {
    width: min(100%, 42rem, calc((100dvh - var(--room, 16rem)) * 1.5));
    margin-inline: auto;
  }

  .scene--along .grid {
    grid-template-columns: repeat(6, 1fr);
  }

  .scene--along .midline {
    left: 50%;
    right: auto;
    top: 0;
    bottom: 0;
    border-top: none;
    border-left: 1px dashed rgba(52, 37, 28, 0.2);
  }

  .ledger {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 13px;
    letter-spacing: 0.04em;
    color: #8a6a55;
  }

  .ledger-turn {
    margin: 0 0 0.15rem;
    font-size: 1.05rem;
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
    grid-template-columns: repeat(3, 1fr);
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

  .cell:not(:has(.figure)):not(.cell--open):not(.cell--attack):not(.cell--mend):not(.cell--picked) {
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
  .cell--mend::before {
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

  /* Ходил — приглушение, не слово. */
  /* Каждая рамка носит своё отношение сторон, а клетки одинаковы. Ширина
     считается, а не выводится из содержимого: клетка 3:4 (см. `.cell`), значит
     её высота — 133⅓% ширины, и карта ростом в клетку шириной `133⅓% × отношение`.
     Шире клетки она не станет — тогда её держит `100%`, а высоту досчитает
     собственный `aspect-ratio`.

     Так, а не `height: 100%; width: auto`: карта — контейнер (`container-type`),
     и её содержимое меряется в `cqi`. Позволить такому боксу мерить свою ширину
     по содержимому — значит замкнуть круг: ширина из содержимого, содержимое из
     ширины. Широкая рамка в этом круге вырастала в полтора раза больше клетки и
     ложилась поверх соседей. Процент от клетки круг разрывает. */
  .figure > :global(.slot) {
    width: min(100%, calc(133.3333% * var(--fit, 0.714)));
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

  .wound {
    position: absolute;
    left: 4px;
    bottom: 4px;
    z-index: 2;
    padding: 0 3px;
    background: #f8f1e7;
    border: 1px solid rgba(52, 37, 28, 0.25);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: #34251c;
  }

  /* ── Предвестие ────────────────────────────────────────────────────────
     Метка, а не всплывающая подсказка: она стоит на теле, к которому
     относится, и не закрывает собой доску. Своё число тёмное, чужое —
     цветом дома, потому что складывать их нельзя. */
  .omen {
    position: absolute;
    right: 3px;
    bottom: 3px;
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
    display: block;
    width: 4.6rem;
    margin-inline: 0;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
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
    width: 4.5rem;
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
    width: 5.45rem;
    flex: 0 0 auto;
    margin-inline: calc(-0.42rem - 0.1rem * var(--n, 3));
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
    font-size: 1.22rem;
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
    min-width: 0;
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
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #8a6a55;
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

  .seal-wrap {
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
