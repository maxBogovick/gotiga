<script lang="ts">
  // Расстановка перед встречей.
  //
  // Карты ставят на поле той самой встречи, в которой будут играть: с его
  // величиной (3–4 клетки поперёк, 3–5 вглубь у половины), местностью, фоном и
  // картами противника. Прежний стол на отдельной странице собирался на поле
  // 3 × 3, и на других полях карты вставали не там, где их видели.
  //
  // Правил поля здесь нет. Куда можно поставить карту, сколько карт и каких
  // рангов берёт колода и куда достаёт противник — называет сервер
  // (`cells`, `limits`, `threats`). Страница держит выбор человека, объясняет
  // отказ словами по этим числам и отправляет выбор целиком.
  //
  // Временные карты дома выбирает дом, а где им стоять — человек: у новичка
  // все карты временные, и без этого в окне ему нечего было бы делать.
  //
  // Сохранение идёт само, через паузу после правки и строго по одному
  // запросу за раз: два одновременных запроса сервер мог бы записать в
  // обратном порядке, и в бой ушла бы старая расстановка.
  import { onDestroy, tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t, lang, type TranslationKey, localized } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import {
    GROUND_KIND,
    GROUND_NAME,
    GROUND_SHORT,
    cardCopy,
    deckFaultLine,
    rulesApart,
    siegeLines,
  } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import BattleGroundMark from '$lib/components/BattleGroundMark.svelte';
  import BattleSheet from '$lib/components/BattleSheet.svelte';
  import type {
    BattleCard as BattleCardDto,
    BattleCell,
    BattleChallenge,
    BattleFrame,
    BattleGround,
    BattleLineup,
    BattleMe,
    DeckLimits,
  } from '$lib/types/api';

  let {
    challenge,
    cards,
    frames,
    starting = false,
    fault = null,
    onstart,
    onback,
  }: {
    challenge: BattleChallenge;
    /** Полка и снаряжение этюдов: стена противника рисуется своей картинкой. */
    cards: BattleCardDto[];
    frames: BattleFrame[];
    /** Страница уже начинает партию. */
    starting?: boolean;
    /** Отказ начала партии, если он пришёл. */
    fault?: string | null;
    /** Расстановка сохранена — можно начинать. */
    onstart: () => void;
    onback: () => void;
  } = $props();

  /** Пока сервер не ответил. Настоящие числа приходят в `lineup.limits`. */
  const HOUSE_LIMITS: DeckLimits = { board: 3, hand: 3, rankFive: 1, rankFour: 2 };
  const SAVE_PAUSE_MS = 350;
  const UNDO_DEPTH = 30;

  type Placed = { card: string; x: number; y: number };
  type Picked =
    | { kind: 'own'; card: string; from: 'drawer' | 'field' | 'hand' }
    | { kind: 'lent'; index: number };
  type Snapshot = { placed: Placed[]; held: string[]; lentCells: BattleCell[] };
  type Status = 'idle' | 'saving' | 'saved' | 'failed';

  let me = $state<BattleMe | null>(null);
  let lineup = $state<BattleLineup | null>(null);
  let loadFailed = $state(false);

  let placed = $state<Placed[]>([]);
  let held = $state<string[]>([]);
  /** Клетки временных карт по порядку мест — выбор человека. */
  let lentCells = $state<BattleCell[]>([]);
  let picked = $state<Picked | null>(null);
  let history = $state<Snapshot[]>([]);

  let status = $state<Status>('idle');
  /** Отказ правила словами: почему сюда нельзя. Гаснет со следующим жестом. */
  let notice = $state<string | null>(null);
  /** Колоду только что подобрал «Подобрать против»: гасится первой же правкой. */
  let suggested = $state<'races' | 'plain' | false>(false);
  let sheet = $state<BattleCardDto | null>(null);

  /** Клетка под указателем — только в ней рисуется карта-призрак. */
  let hoverKey = $state<string | null>(null);
  /** Клетка, которая держит Tab внутри поля (roving tabindex). */
  let focusKey = $state<string | null>(null);
  /** Карта из собрания или руки под указателем — для показа крупно. */
  let hoverCard = $state<string | null>(null);
  /** Карта противника, на которую навели: её зона удара. */
  let threatKey = $state<string | null>(null);
  let showThreats = $state(false);

  let fieldEl = $state<HTMLElement | null>(null);
  let startEl = $state<HTMLButtonElement | null>(null);

  const setup = $derived(challenge.setup);
  const field = $derived(setup.field ?? { width: 3, depth: 3 });
  const rows = $derived(field.depth * 2);
  /** Столбцы на экране: своя половина слева, противника справа — тот же
   *  разворот, что у поля в бою (`spots` в `BattleScene`). */
  const columns = $derived(Array.from({ length: rows }, (_, i) => rows - 1 - i));
  const lines = $derived(Array.from({ length: field.width }, (_, x) => x));
  const guestHalf = (y: number) => y >= field.depth;
  const limits = $derived(lineup?.limits ?? HOUSE_LIMITS);

  const byId = $derived(new Map(cards.map((c) => [c.id, c])));
  const bySlug = $derived(new Map(cards.map((c) => [c.slug, c])));
  const cardOf = (id: string | null | undefined) => (id ? (byId.get(id) ?? null) : null);
  const levelOf = (id: string) => me?.owned.find((o) => o.cardId === id)?.level ?? null;
  const key = (x: number, y: number) => `${x},${y}`;
  const same = (a: BattleCell, b: BattleCell) => a.x === b.x && a.y === b.y;

  const titleOf = (c: BattleChallenge) =>
    localized(c, 'title', $lang);
  const nameOf = (c: BattleCardDto) => cardCopy(c, $lang).title;

  const groundAt = $derived(
    new Map((setup.terrain ?? []).map((tile) => [key(tile.cell.x, tile.cell.y), tile.ground])),
  );
  const keeperAt = $derived(
    new Map(setup.keeperBoard.map((p) => [key(p.x, p.y), bySlug.get(p.card) ?? null])),
  );
  const openCells = $derived(new Set((lineup?.cells ?? []).map((c) => key(c.x, c.y))));

  // ── Расы противника ──────────────────────────────────────────────────────
  // Урон по расам решает исход встречи сильнее всего остального в колоде
  // (замер — `RACE_DAMAGE_MAX` в `battles.rs`), поэтому он назван здесь, где
  // выбирают карты, а не только на листе каждой карты по отдельности.
  const foes = $derived(
    [...setup.keeperBoard.map((p) => p.card), ...setup.keeperHand]
      .map((slug) => bySlug.get(slug))
      .filter((c): c is BattleCardDto => !!c?.raceId),
  );
  /** Расы противника с числом карт и именем. */
  const foeRaces = $derived.by(() => {
    const out = new Map<string, { name: string; count: number; card: BattleCardDto }>();
    for (const c of foes) {
      const id = c.raceId!;
      const seen = out.get(id);
      if (seen) seen.count += 1;
      else out.set(id, { name: localized(c, 'raceName', $lang), count: 1, card: c });
    }
    return out;
  });
  /** Что раса этой карты значит против этого противника. */
  function raceEdge(card: BattleCardDto) {
    const names = (ids: string[]) =>
      ids.filter((id) => foeRaces.has(id)).map((id) => foeRaces.get(id)!.name);
    const d = card.raceDamage;
    const strong = d && d.attack > 0 ? names(d.against) : [];
    const guard = d && d.defence > 0 ? names(d.from) : [];
    const weak = card.raceId
      ? [...foeRaces.values()]
          .filter((f) => {
            const fd = f.card.raceDamage;
            return !!fd && fd.attack > 0 && fd.against.includes(card.raceId!);
          })
          .map((f) => f.name)
      : [];
    return { strong, guard, weak, any: strong.length + guard.length + weak.length > 0 };
  }

  const hasStructures = $derived(
    setup.keeperBoard.some((p) => bySlug.get(p.card)?.kind === 'structure'),
  );
  /** Чем этот бой отличается от домашнего. Местность названа ниже поля
   *  строками земли, поэтому здесь её нет: дважды одно и то же — шум. */
  const apart = $derived([...rulesApart(setup.rules), ...siegeLines(hasStructures)]);
  const grounds = $derived([...new Set((setup.terrain ?? []).map((tile) => tile.ground))]);

  // ── Зоны удара противника ────────────────────────────────────────────────

  const zones = $derived(
    new Map(
      (lineup?.threats ?? []).map((z) => [
        key(z.cell.x, z.cell.y),
        new Set(z.cells.map((c) => key(c.x, c.y))),
      ]),
    ),
  );
  const allThreats = $derived(new Set([...zones.values()].flatMap((s) => [...s])));
  /** Клетки, которые сейчас помечены угрозой: зона наведённой карты или все. */
  const threatened = $derived(
    threatKey && zones.has(threatKey)
      ? (zones.get(threatKey) as Set<string>)
      : showThreats
        ? allThreats
        : new Set<string>(),
  );

  // ── Чтение ────────────────────────────────────────────────────────────────

  async function load() {
    const token = authStore.token;
    const id = challenge.id;
    if (!token) return;
    loadFailed = false;
    try {
      const [mine, laid] = await Promise.all([
        api.getBattleMe(token),
        api.getBattleLineup(token, id),
      ]);
      // Пока шёл запрос, могли открыть другую встречу.
      if (id !== challenge.id) return;
      me = mine;
      lay(laid);
      history = [];
      status = laid.laid ? 'saved' : 'idle';
      if (laid.laid) {
        // Повторная партия: расстановка прежняя, и «В бой» — одно нажатие.
        await tick();
        startEl?.focus({ preventScroll: true });
      }
    } catch {
      loadFailed = true;
    }
  }

  // Эффект, а не onMount: смена встречи или аккаунта перечитывает расстановку.
  $effect(() => {
    void challenge.id;
    void authStore.token;
    void load();
  });

  /** Разложить присланное сервером в то, что редактируется. Ушедшая карта
   *  (снятая с полки или проданная) не редактируется: держать её значило бы
   *  провалить следующее сохранение. Её место занимает временная. */
  function lay(next: BattleLineup) {
    lineup = next;
    placed = next.board
      .filter((s) => s.cardId && !s.gone && s.x != null && s.y != null)
      .map((s) => ({ card: s.cardId as string, x: s.x as number, y: s.y as number }));
    held = next.hand.filter((s) => s.cardId && !s.gone).map((s) => s.cardId as string);
    lentCells = next.board
      .filter((s) => (!s.cardId || s.gone) && s.lentCardId && s.x != null && s.y != null)
      .map((s) => ({ x: s.x as number, y: s.y as number }));
  }

  // ── Что где стоит ─────────────────────────────────────────────────────────

  const mineAt = (x: number, y: number) => placed.find((p) => p.x === x && p.y === y) ?? null;

  /** Временные карты на поле по порядку мест, как их присылает сервер. */
  const lentIds = $derived(
    (lineup?.board ?? [])
      .filter((s) => (!s.cardId || s.gone) && s.lentCardId)
      .map((s) => s.lentCardId as string),
  );

  /**
   * Где стоят временные карты сейчас.
   *
   * После ответа сервера это ровно его раскладка (`lay` берёт клетки из неё).
   * Между правкой и ответом — то же правило, по которому сервер раскладывает
   * (`loan_cells`): выбранные клетки по порядку, затем открытые в порядке
   * дома, без занятых своими картами. Ответ приходит через доли секунды и
   * поправляет, если что-то разошлось.
   */
  const lentShown = $derived.by(() => {
    if (!lineup) return [] as { card: string; x: number; y: number }[];
    const count = Math.min(lentIds.length, Math.max(0, limits.board - placed.length));
    const own = placed.map((p) => ({ x: p.x, y: p.y }));
    const cells: BattleCell[] = [];
    for (const c of [...lentCells, ...lineup.cells]) {
      if (cells.length >= count) break;
      if (!openCells.has(key(c.x, c.y))) continue;
      if (own.some((o) => same(o, c)) || cells.some((o) => same(o, c))) continue;
      cells.push({ x: c.x, y: c.y });
    }
    return cells.map((c, i) => ({ card: lentIds[i], x: c.x, y: c.y }));
  });

  const lentAt = (x: number, y: number) => {
    const i = lentShown.findIndex((l) => l.x === x && l.y === y);
    return i < 0 ? null : { ...lentShown[i], index: i };
  };

  /** Временные карты руки по порядку. Свои идут первыми, заём — за ними. */
  const lentHand = $derived(
    (lineup?.hand ?? [])
      .filter((s) => (!s.cardId || s.gone) && s.lentCardId)
      .map((s) => s.lentCardId as string),
  );

  function handAt(at: number): { card: string; lent: boolean } | null {
    if (at < held.length) return { card: held[at], lent: false };
    const lent = lentHand[at - held.length];
    return lent ? { card: lent, lent: true } : null;
  }

  /** Собрание за вычетом того, что уже стоит: карта лежит в одном месте. */
  const drawer = $derived(
    (me?.owned ?? [])
      .map((o) => byId.get(o.cardId))
      .filter((c): c is BattleCardDto => !!c && c.kind !== 'structure')
      .filter((c) => !placed.some((p) => p.card === c.id) && !held.includes(c.id)),
  );

  /** Карта на клетке поля, какая бы она ни была. */
  function cardAtCell(k: string): BattleCardDto | null {
    const [x, y] = k.split(',').map(Number);
    if (!guestHalf(y)) return keeperAt.get(k) ?? null;
    return cardOf(mineAt(x, y)?.card ?? lentAt(x, y)?.card);
  }

  // ── Правила выбора — по числам сервера ───────────────────────────────────

  const allChosen = () => [...placed.map((p) => p.card), ...held];

  /** Почему эта колода нарушает предел рангов, или ничего. */
  function rankFault(ids: string[]): TranslationKey | null {
    let five = 0;
    let four = 0;
    for (const id of ids) {
      const tier = cardOf(id)?.tier;
      if (tier === 5) five += 1;
      if (tier === 4) four += 1;
    }
    if (five > limits.rankFive) return 'battlesDeckFaultTooManyOfRankFive';
    if (four > limits.rankFour) return 'battlesDeckFaultTooManyOfRankFour';
    return null;
  }

  /** Сломает ли эта карта из собрания предел рангов, если её добавить. */
  const rankBlocked = (id: string) => rankFault([...allChosen(), id]) !== null;

  /** Можно ли положить взятое на клетку: `null` — можно, иначе причина. Пустая
   *  строка — нельзя и объяснять нечего (та же карта, своя половина не та). */
  function cellRefusal(x: number, y: number): TranslationKey | '' | null {
    if (!picked) return '';
    if (!guestHalf(y)) return 'battlesDeckFaultNotYourHalf';
    if (!openCells.has(key(x, y))) return 'battlesDeckFaultCellBlocked';
    const standing = mineAt(x, y);
    if (picked.kind === 'lent') {
      const here = lentAt(x, y);
      return here?.index === picked.index ? '' : null;
    }
    if (standing?.card === picked.card) return '';
    if (picked.from !== 'field' && !standing && placed.length >= limits.board) {
      return 'battlesDeckFaultTooManyOnBoard';
    }
    if (picked.from === 'drawer') {
      const own = picked.card;
      return rankFault([...allChosen().filter((id) => id !== standing?.card), own]);
    }
    return null;
  }

  function handRefusal(at: number): TranslationKey | '' | null {
    if (!picked) return '';
    if (picked.kind === 'lent') return 'battleLineupLentStaysOnField';
    const standing = at < held.length ? held[at] : null;
    if (standing === picked.card) return '';
    if (picked.from !== 'hand' && !standing && held.length >= limits.hand) {
      return 'battlesDeckFaultTooManyInHand';
    }
    if (picked.from === 'drawer') {
      const own = picked.card;
      return rankFault([...allChosen().filter((id) => id !== standing), own]);
    }
    return null;
  }

  const cellTakes = (x: number, y: number) => cellRefusal(x, y) === null;
  const handTakes = (at: number) => handRefusal(at) === null;

  // ── Жесты ─────────────────────────────────────────────────────────────────
  //
  // Нажать карту, потом нажать место: одинаково мышью, пальцем и клавишами
  // (WCAG 2.2, 2.5.7 — всё, что делается перетаскиванием, делается и без
  // него). Перетаскивание на компьютере — тот же путь, другими руками.

  function pickOwn(card: string, from: 'drawer' | 'field' | 'hand', toggle = true) {
    notice = null;
    picked =
      toggle && picked?.kind === 'own' && picked.card === card ? null : { kind: 'own', card, from };
    if (picked && from !== 'field') bringFieldIntoView();
  }

  function pickLent(index: number, toggle = true) {
    notice = null;
    picked = toggle && picked?.kind === 'lent' && picked.index === index ? null : { kind: 'lent', index };
  }

  /** На телефоне собрание и рука лежат под полем: взяв карту там, человек не
   *  видит, куда её класть. Поле подъезжает, если его не видно. */
  function bringFieldIntoView() {
    const el = fieldEl;
    if (!el) return;
    const r = el.getBoundingClientRect();
    if (r.top >= 0 && r.bottom <= window.innerHeight) return;
    const calm = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    el.scrollIntoView({ behavior: calm ? 'auto' : 'smooth', block: 'center' });
  }

  function refuse(why: TranslationKey | '' | null) {
    if (why) notice = $t(why);
  }

  function touchCell(x: number, y: number) {
    const standing = mineAt(x, y);
    const lent = lentAt(x, y);
    if (!picked) {
      if (standing) pickOwn(standing.card, 'field');
      else if (lent) pickLent(lent.index);
      return;
    }
    const why = cellRefusal(x, y);
    if (why !== null) {
      // Нажали на другую карту там, куда взятое не ляжет: берём нажатую —
      // так человек и переставляет. Иначе — говорим, почему нельзя.
      if (why === '' && (standing || lent)) {
        if (standing && !(picked.kind === 'own' && picked.card === standing.card)) {
          pickOwn(standing.card, 'field');
        } else if (lent && !(picked.kind === 'lent' && picked.index === lent.index)) {
          pickLent(lent.index);
        } else {
          picked = null;
        }
        return;
      }
      refuse(why);
      return;
    }
    const target = { x, y };
    const lentNow = lentShown.map((l) => ({ x: l.x, y: l.y }));

    if (picked.kind === 'lent') {
      const from = lentNow[picked.index];
      const nextLent = [...lentNow];
      nextLent[picked.index] = target;
      let nextPlaced = placed;
      if (standing) {
        // Своя карта меняется местами с временной.
        nextPlaced = placed.map((p) => (p === standing ? { ...p, x: from.x, y: from.y } : p));
      } else if (lent) {
        nextLent[lent.index] = from;
      }
      commit(nextPlaced, held, nextLent);
      return;
    }

    const own = picked;
    const origin = placed.find((p) => p.card === own.card) ?? null;
    let nextPlaced = placed.filter((p) => p !== standing && p.card !== own.card);
    let nextHeld = held.filter((id) => id !== own.card);
    let nextLent = lentNow;
    if (standing) {
      // Обмен: стоявшая карта уходит туда, откуда пришла взятая.
      if (own.from === 'field' && origin) {
        nextPlaced = [...nextPlaced, { card: standing.card, x: origin.x, y: origin.y }];
      } else if (own.from === 'hand') {
        nextHeld = held.map((id) => (id === own.card ? standing.card : id));
      }
    } else if (lent) {
      if (own.from === 'field' && origin) {
        // Временная карта встаёт на освободившуюся клетку.
        nextLent = lentNow.map((c, i) => (i === lent.index ? { x: origin.x, y: origin.y } : c));
      } else {
        // Своя карта занимает место временной: одной временной на поле меньше.
        nextLent = lentNow.filter((_, i) => i !== lent.index);
      }
    }
    nextPlaced = [...nextPlaced, { card: own.card, ...target }];
    commit(nextPlaced, nextHeld, nextLent);
  }

  function touchHand(at: number) {
    const standing = at < held.length ? held[at] : null;
    if (!picked) {
      if (standing) pickOwn(standing, 'hand');
      return;
    }
    const why = handRefusal(at);
    if (why !== null) {
      if (why === '' && standing) {
        if (picked.kind === 'own' && picked.card === standing) picked = null;
        else pickOwn(standing, 'hand');
        return;
      }
      refuse(why);
      return;
    }
    if (picked.kind !== 'own') return;
    const own = picked;
    const origin = placed.find((p) => p.card === own.card) ?? null;
    let nextPlaced = placed.filter((p) => p.card !== own.card);
    let nextHeld: string[];
    if (standing) {
      if (own.from === 'hand') {
        nextHeld = held.map((id) => (id === standing ? own.card : id === own.card ? standing : id));
      } else {
        // С поля — стоявшая в руке карта встаёт на освободившуюся клетку;
        // из собрания — возвращается в собрание.
        nextHeld = held.map((id) => (id === standing ? own.card : id));
        if (own.from === 'field' && origin) {
          nextPlaced = [...nextPlaced, { card: standing, x: origin.x, y: origin.y }];
        }
      }
    } else {
      nextHeld = [...held.filter((id) => id !== own.card), own.card];
    }
    commit(nextPlaced, nextHeld, lentShown.map((l) => ({ x: l.x, y: l.y })));
  }

  /** Вернуть взятую со стола карту в собрание. */
  function takeBack() {
    if (!picked) return;
    if (picked.kind === 'lent') {
      refuse('battleLineupLentStaysOnField');
      return;
    }
    if (picked.from === 'drawer') {
      picked = null;
      return;
    }
    const card = picked.card;
    commit(
      placed.filter((p) => p.card !== card),
      held.filter((id) => id !== card),
      lentShown.map((l) => ({ x: l.x, y: l.y })),
    );
  }

  /**
   * «Подобрать против»: из собрания — карты, чья раса сильна против рас
   * противника, на поле; следующие — в руку. Правил выбора здесь нет: расу
   * против расы называет `raceEdge` по числам из справочника, пределы рангов и
   * размеры поля и руки — сервер (`limits`, `cells`). Это один шаг в истории:
   * «Отменить» возвращает прежнюю расстановку целиком.
   */
  function suggest() {
    // Экземпляров одной карты может быть несколько (`card_copies`): на стол
    // карта идёт один раз.
    const owned = [...new Map((me?.owned ?? []).map((o) => [o.cardId, byId.get(o.cardId)])).values()].filter(
      (c): c is BattleCardDto => !!c && c.kind !== 'structure',
    );
    // Подсказка, а не оптимум: веса — порядок предпочтения, не результат замера.
    const worth = (c: BattleCardDto) => {
      const e = raceEdge(c);
      return e.strong.length * 2 + e.guard.length - e.weak.length * 2 + (c.power + c.health) / 20;
    };
    const chosen: string[] = [];
    for (const c of [...owned].sort((a, b) => worth(b) - worth(a))) {
      if (chosen.length >= limits.board + limits.hand) break;
      if (rankFault([...chosen, c.id])) continue;
      chosen.push(c.id);
    }
    const cells = (lineup?.cells ?? []).filter((c) => guestHalf(c.y));
    const onField = chosen.slice(0, Math.min(limits.board, cells.length));
    const inHand = chosen.slice(onField.length, onField.length + limits.hand);
    const next = onField.map((card, i) => ({ card, x: cells[i].x, y: cells[i].y }));
    // Клетки временных карт, выбранные человеком, сохраняются, кроме тех, что
    // заняли теперь свои.
    commit(
      next,
      inHand,
      lentCells.filter((l) => !next.some((p) => p.x === l.x && p.y === l.y)),
    );
    suggested = owned.some((c) => raceEdge(c).any) ? 'races' : 'plain';
  }

  function snapshot(): Snapshot {
    return {
      placed: placed.map((p) => ({ ...p })),
      held: [...held],
      lentCells: lentShown.map((l) => ({ x: l.x, y: l.y })),
    };
  }

  function commit(nextPlaced: Placed[], nextHeld: string[], nextLent: BattleCell[]) {
    suggested = false;
    history = [...history.slice(-(UNDO_DEPTH - 1)), snapshot()];
    placed = nextPlaced;
    held = nextHeld;
    lentCells = nextLent;
    picked = null;
    notice = null;
    scheduleSave();
  }

  function undo() {
    const last = history.at(-1);
    if (!last) return;
    history = history.slice(0, -1);
    suggested = false;
    placed = last.placed;
    held = last.held;
    lentCells = last.lentCells;
    picked = null;
    notice = null;
    scheduleSave();
  }

  // ── Перетаскивание (компьютер) ───────────────────────────────────────────

  let dropped = false;

  function dragOwn(e: DragEvent, card: string, from: 'drawer' | 'field' | 'hand') {
    dropped = false;
    picked = { kind: 'own', card, from };
    notice = null;
    e.dataTransfer?.setData('text/plain', card);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }

  function dragLent(e: DragEvent, index: number) {
    dropped = false;
    picked = { kind: 'lent', index };
    notice = null;
    e.dataTransfer?.setData('text/plain', `lent:${index}`);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }

  function dragEnd() {
    if (!dropped) picked = null;
    hoverKey = null;
  }

  function overCell(e: DragEvent, x: number, y: number) {
    hoverKey = key(x, y);
    if (cellTakes(x, y)) e.preventDefault();
  }

  function dropCell(e: DragEvent, x: number, y: number) {
    e.preventDefault();
    dropped = true;
    touchCell(x, y);
  }

  function overHand(e: DragEvent, at: number) {
    if (handTakes(at)) e.preventDefault();
  }

  function dropHand(e: DragEvent, at: number) {
    e.preventDefault();
    dropped = true;
    touchHand(at);
  }

  function overDrawer(e: DragEvent) {
    if (picked?.kind === 'own' && picked.from !== 'drawer') e.preventDefault();
  }

  function dropDrawer(e: DragEvent) {
    e.preventDefault();
    dropped = true;
    takeBack();
  }

  // ── Клавиши на поле (roving tabindex, WAI-ARIA grid) ─────────────────────

  const firstKey = $derived(
    lineup?.cells[0] ? key(lineup.cells[0].x, lineup.cells[0].y) : key(0, rows - 1),
  );
  const tabKey = $derived(focusKey ?? firstKey);

  function moveFocus(e: KeyboardEvent, x: number, y: number) {
    const col = columns.indexOf(y);
    let nx = x;
    let ncol = col;
    switch (e.key) {
      case 'ArrowLeft':
        ncol = Math.max(0, col - 1);
        break;
      case 'ArrowRight':
        ncol = Math.min(columns.length - 1, col + 1);
        break;
      case 'ArrowUp':
        nx = Math.max(0, x - 1);
        break;
      case 'ArrowDown':
        nx = Math.min(field.width - 1, x + 1);
        break;
      case 'Home':
        ncol = 0;
        break;
      case 'End':
        ncol = columns.length - 1;
        break;
      default:
        return;
    }
    e.preventDefault();
    const next = key(nx, columns[ncol]);
    focusKey = next;
    fieldEl?.querySelector<HTMLElement>(`[data-cell="${next}"]`)?.focus();
  }

  function enterCell(x: number, y: number) {
    const k = key(x, y);
    hoverKey = k;
    threatKey = guestHalf(y) ? null : k;
  }

  function leaveField() {
    hoverKey = null;
    threatKey = null;
  }

  // ── Сохранение: по одному запросу за раз ─────────────────────────────────

  let timer: ReturnType<typeof setTimeout> | null = null;
  /** Номер последней правки и номер последней отправленной. */
  let edition = 0;
  let sentEdition = 0;
  let pumping: Promise<void> | null = null;

  function scheduleSave() {
    edition += 1;
    status = 'saving';
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      void pump();
    }, SAVE_PAUSE_MS);
  }

  /** Отправлять, пока есть неотправленная правка. Следующий запрос уходит
   *  только после ответа на предыдущий: порядок записи на сервере — порядок
   *  правок. */
  function pump(): Promise<void> {
    if (pumping) return pumping;
    if (sentEdition >= edition) return Promise.resolve();
    pumping = (async () => {
      // Тело не должно закончиться синхронно: иначе `finally` обнулил бы
      // `pumping` раньше, чем его присвоят, и он остался бы занятым навсегда.
      await Promise.resolve();
      try {
        while (sentEdition < edition) {
          const token = authStore.token;
          if (!token) return;
          const sent = edition;
          sentEdition = sent;
          status = 'saving';
          try {
            const next = await api.saveBattleLineup(token, challenge.id, {
              board: placed.map((p) => ({ card: p.card, x: p.x, y: p.y })),
              hand: held,
              lent: lentShown.map((l) => ({ x: l.x, y: l.y })),
            });
            if (sent === edition) {
              lay(next);
              status = 'saved';
            } else if (lineup) {
              // Ответ на устаревшую правку: выбор человека не трогаем, но
              // зоны и заём берём — следующий ответ их уточнит.
              lineup = { ...lineup, threats: next.threats };
            }
          } catch (e) {
            if (String(e).includes('deck:')) {
              // Сервер отказал правилом: этот выбор в бой не пойдёт, и на
              // экране его не оставляем — возвращается последнее принятое.
              if (sent === edition) {
                notice = deckFaultLine(e, $t, 'battleLineupSaveFailed');
                if (lineup) lay(lineup);
                status = 'saved';
              }
            } else {
              // Сеть или сервер не ответили: выбор человека остаётся, его
              // можно отправить ещё раз.
              sentEdition = sent - 1;
              status = 'failed';
              return;
            }
          }
        }
      } finally {
        pumping = null;
      }
    })();
    return pumping;
  }

  function retry() {
    void pump();
  }

  /** Дождаться, пока последняя правка дойдёт до сервера. */
  async function flush(): Promise<boolean> {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
    while (sentEdition < edition || pumping) {
      if (!authStore.token) return false;
      await pump();
      if (status === 'failed') return false;
    }
    return true;
  }

  async function begin() {
    if (starting || !lineup) return;
    picked = null;
    if (await flush()) onstart();
  }

  function onBeforeUnload(e: BeforeUnloadEvent) {
    if (status === 'failed' || sentEdition < edition) {
      e.preventDefault();
    }
  }

  onDestroy(() => {
    // Ушли, не дождавшись паузы: правка всё равно дойдёт до сервера.
    if (timer) {
      clearTimeout(timer);
      timer = null;
      void pump();
    }
  });

  function onkey(e: KeyboardEvent) {
    if (sheet) return;
    const target = e.target as HTMLElement | null;
    const typing = !!target?.closest('input, textarea, [contenteditable="true"]');
    if (e.key === 'Escape' && picked) {
      picked = null;
      notice = null;
      e.stopPropagation();
      return;
    }
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && e.key.toLowerCase() === 'z' && !typing) {
      if (history.length) {
        e.preventDefault();
        undo();
      }
    }
  }

  // ── Предупреждения и подписи ─────────────────────────────────────────────

  /** Бой с пустым полем: карты руки выкладываются за ману, а её на первом
   *  ходу одна. Временные карты дома закрывают эту дыру, поэтому говорится
   *  только когда не встанет никто. */
  const fieldEmpty = $derived(!!lineup && placed.length === 0 && lentShown.length === 0);

  const onHazard = $derived(
    [...placed, ...lentShown].some((p) => {
      const g = groundAt.get(key(p.x, p.y));
      return !!g && GROUND_KIND[g] === 'hazard';
    }),
  );

  const pickedCard = $derived(
    picked
      ? picked.kind === 'own'
        ? cardOf(picked.card)
        : cardOf(lentShown[picked.index]?.card)
      : null,
  );

  /** Что показать крупно: карту под указателем, иначе взятую. */
  const previewCard = $derived(
    cardOf(hoverCard) ?? (hoverKey ? cardAtCell(hoverKey) : null) ?? pickedCard,
  );

  const groundWord = (g: BattleGround | undefined) => (g ? `${$t(GROUND_NAME[g])} — ` : '');

  function cellLabel(x: number, y: number): string {
    const k = key(x, y);
    const g = groundAt.get(k);
    const c = cardAtCell(k);
    const parts = [`${groundWord(g)}${c ? nameOf(c) : guestHalf(y) ? $t('battleLineupYourCell') : ''}`];
    if (guestHalf(y) && lentAt(x, y)) parts.push($t('battleLineupLentCell'));
    if (threatened.has(k)) parts.push($t('battleLineupThreatCell'));
    return parts.filter(Boolean).join(' — ');
  }
</script>

<svelte:window onkeydown={onkey} onbeforeunload={onBeforeUnload} />

{#snippet raceLines(card: BattleCardDto)}
  {@const edge = raceEdge(card)}
  {#if edge.any}
    <ul class="race-edge">
      {#if edge.strong.length}
        <li class="race-edge--strong">{$t('battleLineupRaceStrong')}: {edge.strong.join(', ')}</li>
      {/if}
      {#if edge.guard.length}
        <li>{$t('battleLineupRaceGuard')}: {edge.guard.join(', ')}</li>
      {/if}
      {#if edge.weak.length}
        <li class="race-edge--weak">{$t('battleLineupRaceWeak')}: {edge.weak.join(', ')}</li>
      {/if}
    </ul>
  {/if}
{/snippet}

<section class="lineup" aria-labelledby="lineup-title">
  <header class="head">
    <button type="button" class="back" onclick={onback}>{$t('battleLineupBack')}</button>
    <p class="eyebrow">{titleOf(challenge)}</p>
    <h2 class="title" id="lineup-title">{$t('battleLineupTitle')}</h2>
    <p class="rule">{$t('battleLineupHow')}</p>
    {#if apart.length}
      <p class="apart">
        {$t('battleRulesOwn')}:
        {#each apart as line, i (line.key)}{i > 0 ? ' · ' : ' '}{$t(line.key)}{line.amount === null
            ? ''
            : ` — ${line.amount}`}{/each}
      </p>
    {/if}
  </header>

  {#if loadFailed}
    <p class="fault">
      {$t('battleLineupLoadFailed')}
      <button type="button" class="link" onclick={load}>{$t('battleLineupRetry')}</button>
    </p>
  {:else if !lineup}
    <p class="quiet">…</p>
  {:else}
    <div class="stage" style="--cols:{rows}">
      <!-- Поле встречи целиком: своё слева, чужое справа, как в бою. -->
      <div class="board">
        <div class="board-head">
          <p class="halves">
            <span class="half-word half-word--guest">← {$t('battleLineupYourHalf')}</span>
            <span class="half-word half-word--keeper">{$t('battleLineupEnemyHalf')} →</span>
          </p>
          {#if zones.size}
            <label class="threat-toggle">
              <input type="checkbox" bind:checked={showThreats} />
              {$t('battleLineupThreats')}
            </label>
          {/if}
        </div>
        <div class="field-wrap">
          <div
            bind:this={fieldEl}
            class="field"
            class:field--pictured={!!setup.background}
            class:field--asking={!!picked}
            style="--cols:{rows}; --lines:{field.width}"
            style:--backdrop={setup.background ? `url("${setup.background}")` : undefined}
            role="grid"
            aria-label={$t('battleLineupField')}
            onpointerleave={leaveField}
          >
            {#each lines as x (x)}
              <div class="line" role="row">
                {#each columns as y (y)}
                  {@const k = key(x, y)}
                  {@const ground = groundAt.get(k)}
                  {#if guestHalf(y)}
                    {@const mine = mineAt(x, y)}
                    {@const lent = mine ? null : lentAt(x, y)}
                    {@const shown = cardOf(mine?.card ?? lent?.card)}
                    {@const takes = cellTakes(x, y)}
                    {@const open = openCells.has(k)}
                    {@const isPicked =
                      (!!mine && picked?.kind === 'own' && picked.card === mine.card) ||
                      (!!lent && picked?.kind === 'lent' && picked.index === lent.index)}
                    <div role="gridcell" class="slot" aria-selected={isPicked}>
                      <button
                        type="button"
                        class="cell cell--guest"
                        class:cell--seam={y === field.depth}
                        class:cell--open={takes}
                        class:cell--closed={!open}
                        class:cell--picked={isPicked}
                        class:cell--threat={threatened.has(k)}
                        data-cell={k}
                        tabindex={tabKey === k ? 0 : -1}
                        aria-label={cellLabel(x, y)}
                        draggable={!!shown}
                        ondragstart={(e) =>
                          mine ? dragOwn(e, mine.card, 'field') : lent ? dragLent(e, lent.index) : e.preventDefault()}
                        ondragend={dragEnd}
                        ondragover={(e) => overCell(e, x, y)}
                        ondrop={(e) => dropCell(e, x, y)}
                        onpointerenter={() => enterCell(x, y)}
                        onfocus={() => {
                          focusKey = k;
                          enterCell(x, y);
                        }}
                        onkeydown={(e) => moveFocus(e, x, y)}
                        onclick={() => touchCell(x, y)}
                      >
                        {#if ground}
                          <BattleGroundMark {ground} occupied={!!shown} />
                        {/if}
                        {#if shown}
                          <span class="card" class:card--lent={!mine}>
                            <BattleCard
                              card={shown}
                              {frames}
                              owned={true}
                              uniform
                              level={mine ? levelOf(mine.card) : null}
                              transition={false}
                              interactive={false}
                            />
                          </span>
                          {#if !mine}
                            <span class="slip">{$t('battlesTableLent')}</span>
                          {/if}
                        {:else if takes && hoverKey === k && pickedCard}
                          <span class="card card--ghost" aria-hidden="true">
                            <BattleCard
                              card={pickedCard}
                              {frames}
                              owned={true}
                              uniform
                              transition={false}
                              interactive={false}
                            />
                          </span>
                        {/if}
                      </button>
                    </div>
                  {:else}
                    {@const body = keeperAt.get(k)}
                    <div role="gridcell" class="slot">
                      <button
                        type="button"
                        class="cell cell--keeper"
                        class:cell--structure={body?.kind === 'structure'}
                        class:cell--aimed={threatKey === k && zones.has(k)}
                        data-cell={k}
                        tabindex={tabKey === k ? 0 : -1}
                        aria-label={cellLabel(x, y)}
                        onpointerenter={() => enterCell(x, y)}
                        onfocus={() => {
                          focusKey = k;
                          enterCell(x, y);
                        }}
                        onkeydown={(e) => moveFocus(e, x, y)}
                        onclick={() => {
                          if (picked) refuse('battlesDeckFaultNotYourHalf');
                          else if (body) sheet = body;
                        }}
                      >
                        {#if ground}
                          <BattleGroundMark {ground} occupied={!!body} />
                        {/if}
                        {#if body?.kind === 'structure'}
                          <img
                            class="structure-art"
                            src={body.artUrlOverride || body.artUrl}
                            alt=""
                            draggable="false"
                          />
                        {:else if body}
                          <span class="card">
                            <BattleCard
                              card={body}
                              {frames}
                              owned={true}
                              uniform
                              transition={false}
                              interactive={false}
                            />
                          </span>
                        {/if}
                      </button>
                    </div>
                  {/if}
                {/each}
              </div>
            {/each}
          </div>
        </div>
        {#if zones.size}
          <p class="hint">{$t('battleLineupThreatHint')}</p>
        {/if}
        {#if setup.keeperHand.length}
          <p class="keeper-hand">{$t('battleLineupEnemyHand')}: {setup.keeperHand.length}</p>
        {/if}
        {#if foeRaces.size}
          <p class="keeper-hand">
            {$t('battleLineupFoeRaces')}: {[...foeRaces.values()]
              .map((f) => (f.count > 1 ? `${f.name} ×${f.count}` : f.name))
              .join(', ')}
          </p>
        {/if}
        {#if grounds.length}
          <dl class="grounds" aria-label={$t('battleLineupGrounds')}>
            {#each grounds as g (g)}
              <div class="ground-line">
                <dt class="ground-name ground-name--{GROUND_KIND[g]}">{$t(GROUND_NAME[g])}</dt>
                <dd>{$t(GROUND_SHORT[g])}</dd>
              </div>
            {/each}
          </dl>
        {/if}
      </div>

      <!-- Карта крупно: то, на что навели, иначе то, что взято. В клетке при
           100 px карту не прочесть, а решать, куда ставить, надо по ней. -->
      <aside class="preview" aria-live="polite">
        {#if previewCard}
          <div class="preview-card">
            <BattleCard
              card={previewCard}
              {frames}
              owned={true}
              uniform
              transition={false}
              interactive={false}
            />
          </div>
          {@render raceLines(previewCard)}
          <button type="button" class="link link--block" onclick={() => (sheet = previewCard)}>
            {$t('battleLineupReadCard')}
          </button>
        {:else}
          <p class="quiet">{$t('battleLineupPreviewEmpty')}</p>
        {/if}
      </aside>
    </div>

    <!-- Рука, собрание и «В бой» — одна полоса, прилипшая к низу окна: карту
         берут в ней, а ставят на поле выше, и обе половины всегда на экране. -->
    <div class="tray">
    <div class="desk">
      <section class="band">
        <h3 class="band-title">{$t('battleLineupHand')}</h3>
        <div class="hand">
          {#each Array.from({ length: limits.hand }, (_, i) => i) as at (at)}
            {@const here = handAt(at)}
            {@const shown = cardOf(here?.card)}
            {@const own = here && !here.lent ? here.card : null}
            <button
              type="button"
              class="place"
              class:place--open={handTakes(at)}
              class:place--picked={!!own && picked?.kind === 'own' && picked.card === own}
              aria-pressed={!!own && picked?.kind === 'own' && picked.card === own}
              aria-label={shown ? nameOf(shown) : $t('battlesTablePut')}
              draggable={!!own}
              ondragstart={(e) => (own ? dragOwn(e, own, 'hand') : e.preventDefault())}
              ondragend={dragEnd}
              ondragover={(e) => overHand(e, at)}
              ondrop={(e) => dropHand(e, at)}
              onpointerenter={() => (hoverCard = here?.card ?? null)}
              onpointerleave={() => (hoverCard = null)}
              onclick={() => touchHand(at)}
            >
              {#if shown}
                <span class="card" class:card--lent={!own}>
                  <BattleCard
                    card={shown}
                    {frames}
                    owned={true}
                    uniform
                    level={own ? levelOf(own) : null}
                    transition={false}
                    interactive={false}
                  />
                </span>
                {#if !own}
                  <span class="slip">{$t('battlesTableLent')}</span>
                {/if}
              {:else}
                <span class="empty-word">{$t('battlesTableEmptySlot')}</span>
              {/if}
            </button>
          {/each}
        </div>
      </section>

      <section
        class="band band--drawer"
        class:band--return={picked?.kind === 'own' && picked.from !== 'drawer'}
        aria-label={$t('battlesTableDrawer')}
        ondragover={overDrawer}
        ondrop={dropDrawer}
      >
        <h3 class="band-title">{$t('battlesTableDrawer')}</h3>
        {#if !me?.owned.length}
          <p class="quiet">{$t('battlesTableDrawerEmpty')}</p>
          <p class="quiet"><a class="shelf-link" href="/battles">{$t('battlesTableToShelf')} →</a></p>
        {:else if !drawer.length}
          <p class="quiet">{$t('battleLineupAllPlaced')}</p>
        {:else}
          <div class="drawer">
            {#each drawer as card (card.id)}
              {@const blocked = rankBlocked(card.id)}
              <div class="pull-wrap">
                <button
                  type="button"
                  class="pull"
                  class:pull--picked={picked?.kind === 'own' && picked.card === card.id}
                  class:pull--blocked={blocked}
                  aria-pressed={picked?.kind === 'own' && picked.card === card.id}
                  aria-label={`${nameOf(card)}${blocked ? ` — ${$t('battleLineupRankMark')}` : ''}`}
                  title={blocked ? $t(rankFault([...allChosen(), card.id]) ?? 'battleLineupRankMark') : undefined}
                  draggable="true"
                  ondragstart={(e) => dragOwn(e, card.id, 'drawer')}
                  ondragend={dragEnd}
                  onpointerenter={() => (hoverCard = card.id)}
                  onpointerleave={() => (hoverCard = null)}
                  onclick={() => pickOwn(card.id, 'drawer')}
                >
                  <BattleCard
                    {card}
                    {frames}
                    owned={true}
                    uniform
                    level={levelOf(card.id)}
                    transition={false}
                    interactive={false}
                  />
                </button>
                {#if blocked}
                  <span class="rank-mark" aria-hidden="true">{$t('battleLineupRankMark')}</span>
                {/if}
                <div class="pull-edge">{@render raceLines(card)}</div>
                <button
                  type="button"
                  class="read"
                  aria-label={`${$t('battleLineupReadCard')}: ${nameOf(card)}`}
                  title={$t('battleLineupReadCard')}
                  onclick={() => (sheet = card)}>i</button
                >
              </div>
            {/each}
          </div>
        {/if}
      </section>
    </div>

    <footer class="foot">
      <div class="notes" aria-live="polite">
        {#if fault}
          <p class="fault" transition:fade={{ duration: 150 }}>{fault}</p>
        {:else if notice}
          <p class="fault" transition:fade={{ duration: 150 }}>{notice}</p>
        {:else if picked}
          <p class="ask">
            {picked.kind === 'lent' ? $t('battleLineupLentPicked') : $t('battleLineupWhere')}
            {#if picked.kind === 'own' && picked.from !== 'drawer'}
              <button type="button" class="link" onclick={takeBack}>{$t('battleLineupTakeBack')}</button>
            {/if}
            {#if pickedCard}
              <button type="button" class="link link--narrow" onclick={() => (sheet = pickedCard)}
                >{$t('battleLineupReadCard')}</button
              >
            {/if}
            <button type="button" class="link" onclick={() => (picked = null)}>{$t('battleLineupCancel')}</button>
          </p>
        {:else if suggested}
          <p class="quiet">
            {suggested === 'races' ? $t('battleLineupSuggested') : $t('battleLineupSuggestedPlain')}
          </p>
        {:else if fieldEmpty}
          <p class="warn">{$t('battlesTableFieldEmpty')}</p>
        {:else if onHazard}
          <p class="warn">{$t('battleLineupOnHazard')}</p>
        {:else if lineup.nothingToLend && !placed.length}
          <p class="quiet">{$t('battlesTableNothingToLend')}</p>
        {/if}
      </div>
      <div class="actions">
        <!-- Состояние сохранения стоит на своём месте всегда: автосохранение
             без видимого «Сохранено» — это работа, про которую не знаешь,
             цела ли она. -->
        <span class="status status--{status}" role="status">
          {#if status === 'saving'}
            {$t('battleLineupStatusSaving')}
          {:else if status === 'saved'}
            {$t('battleLineupStatusSaved')}
          {:else if status === 'failed'}
            {$t('battleLineupStatusFailed')}
            <button type="button" class="link" onclick={retry}>{$t('battleLineupRetry')}</button>
          {/if}
        </span>
        {#if foeRaces.size > 0 && (me?.owned.length ?? 0) > 0}
          <button type="button" class="undo" onclick={suggest}>{$t('battleLineupSuggest')}</button>
        {/if}
        <button type="button" class="undo" disabled={!history.length} onclick={undo}>
          {$t('battleLineupUndo')}
        </button>
        <button bind:this={startEl} type="button" class="start" disabled={starting} onclick={begin}>
          {starting ? $t('battleLineupStarting') : $t('battleLineupStart')}
        </button>
      </div>
    </footer>
    </div>
  {/if}
</section>

{#if sheet}
  <BattleSheet
    card={sheet}
    {frames}
    signedIn={true}
    owned={true}
    taking={false}
    onclose={() => (sheet = null)}
  />
{/if}

<style>
  .lineup {
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
    padding-bottom: 1rem;
  }

  .head {
    max-width: 46rem;
  }

  .back {
    margin-bottom: 0.5rem;
    font-size: 12px;
    letter-spacing: 0.08em;
    color: #6f5a48;
  }

  .back:hover {
    color: #34251c;
  }

  .eyebrow {
    margin: 0;
    font-size: 11px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .title {
    margin: 0.1rem 0 0.4rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(1.6rem, 3.4vw, 2.1rem);
    font-weight: 500;
    line-height: 1.1;
  }

  .rule,
  .apart {
    margin: 0.3rem 0 0;
    font-size: 14px;
    line-height: 1.55;
    color: #4a3628;
  }

  .apart {
    font-size: 13px;
    color: #6f3b24;
  }

  /* Поле и карта крупно рядом; на узком экране карта крупно не нужна —
     её заменяет «Подробнее о карте». */
  .stage {
    display: grid;
    gap: 1.5rem;
    align-items: start;
  }

  /* Колонка с картой стоит вплотную к полю, а не у края окна: поле 3 × 3
     узкое, и между ним и картой иначе лежала бы пустая половина экрана.
     Ширина поля — те же 100 px на клетку, что у `.cell`, плюс промежутки. */
  @media (min-width: 1100px) {
    .stage {
      grid-template-columns: minmax(0, calc(var(--cols) * 106px + 30px)) 15rem;
      justify-content: start;
      column-gap: 2rem;
    }
  }

  .preview {
    display: none;
  }

  @media (min-width: 1100px) {
    .preview {
      position: sticky;
      top: 5rem;
      display: block;
    }
  }

  .preview-card {
    width: 100%;
  }

  .board-head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.4rem 1rem;
    margin-bottom: 0.5rem;
  }

  .halves {
    display: flex;
    flex: 1;
    justify-content: space-between;
    gap: 1rem;
    margin: 0;
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
  }

  /* Те же два оттенка, что у половин в бою, приглушённые под бумагу. */
  .half-word--guest {
    color: #2f4a78;
  }

  .half-word--keeper {
    color: #7a2f25;
  }

  .threat-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 12px;
    color: #4a3628;
    cursor: pointer;
  }

  .threat-toggle input {
    accent-color: #8f2f22;
  }

  /* Глубокое поле на телефоне не помещается — оно прокручивается вбок, а не
     сжимает карты до неузнаваемости. */
  .field-wrap {
    container-type: inline-size;
    overflow-x: auto;
    padding: 18px 2px 4px;
  }

  .field {
    display: inline-flex;
    flex-direction: column;
    gap: 6px;
  }

  .field--pictured {
    background: var(--backdrop) center / cover no-repeat;
    padding: 8px;
  }

  .line {
    display: flex;
    gap: 6px;
  }

  .slot {
    display: contents;
  }

  /* Клетка — доля ширины поля, но не меньше 58 px (меньше карту не узнать) и
     не крупнее 100 px (поле 3 × 6 не должно раздуваться в плакат). Второй
     предел — высота окна: поле и полоса с картами внизу обязаны помещаться
     вместе, иначе карту берут внизу, а поле уходит за верхний край (20rem —
     полоса, подписи и отступы; 1.4 — отношение сторон карты). */
  .cell {
    position: relative;
    flex: none;
    width: clamp(
      58px,
      min(
        calc((100cqw - (var(--cols) - 1) * 6px - 30px) / var(--cols)),
        calc((100dvh - 20rem) / var(--lines) / 1.4 - 6px)
      ),
      100px
    );
    aspect-ratio: 5 / 7;
    padding: 0;
    border: 1px solid rgba(52, 37, 28, 0.18);
  }

  .cell--guest {
    background: rgba(47, 74, 120, 0.11);
    cursor: pointer;
  }

  .cell--keeper {
    background: rgba(122, 47, 37, 0.09);
    cursor: default;
  }

  .cell--keeper:has(.card),
  .cell--keeper:has(.structure-art) {
    cursor: help;
  }

  /* Шов между половинами. */
  .cell--seam {
    margin-right: 12px;
  }

  .cell--seam::after {
    content: '';
    position: absolute;
    top: -4px;
    bottom: -4px;
    right: -10px;
    border-right: 1px dashed rgba(52, 37, 28, 0.35);
    pointer-events: none;
  }

  .cell--guest:hover {
    border-color: rgba(52, 37, 28, 0.5);
  }

  .cell--closed {
    cursor: default;
  }

  /* Поле спрашивает клетку: открытые отмечены кромкой, остальные отступают. */
  .field--asking .cell--guest:not(.cell--open):not(.cell--picked) {
    opacity: 0.55;
  }

  .cell--open {
    border-color: #c65f3c;
    box-shadow: inset 0 0 0 1px #c65f3c;
  }

  .cell--picked {
    outline: 2px solid #34251c;
    outline-offset: 2px;
  }

  /* Угроза — штриховка поверх клетки, а не заливка: под ней должна читаться
     и земля, и карта. */
  .cell--threat::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 4;
    pointer-events: none;
    border: 2px solid rgba(143, 47, 34, 0.85);
    background: repeating-linear-gradient(
      135deg,
      rgba(143, 47, 34, 0.2) 0 5px,
      transparent 5px 10px
    );
  }

  /* На тёмной карте штриховка тонет — там угрозу несёт кромка и уголок. */
  .cell--threat:has(.card)::before {
    background: linear-gradient(225deg, rgba(143, 47, 34, 0.95) 0 14px, transparent 14px);
  }

  .cell--aimed {
    outline: 2px solid #8f2f22;
    outline-offset: 2px;
  }

  .cell:focus-visible,
  .place:focus-visible,
  .pull:focus-visible,
  .undo:focus-visible {
    outline: 2px solid #c65f3c;
    outline-offset: 2px;
  }

  .cell--structure {
    border-color: transparent;
    background: transparent;
  }

  .structure-art {
    position: absolute;
    inset: -3px -1px;
    z-index: 1;
    width: calc(100% + 2px);
    height: calc(100% + 6px);
    object-fit: fill;
    pointer-events: none;
  }

  .card {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: block;
    pointer-events: none;
  }

  .card--lent {
    opacity: 0.82;
  }

  .card--ghost {
    opacity: 0.35;
  }

  .slip {
    position: absolute;
    left: 50%;
    bottom: -0.55rem;
    z-index: 3;
    transform: translateX(-50%) rotate(-2deg);
    padding: 0.05rem 0.4rem;
    font-size: 9px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    white-space: nowrap;
    color: #34251c;
    background: #f3e7d4;
    border: 1px solid #d8c6b1;
    pointer-events: none;
  }

  .hint,
  .keeper-hand {
    margin: 0.6rem 0 0;
    font-size: 12px;
    color: #6f5a48;
  }

  .grounds {
    display: grid;
    gap: 0.25rem;
    margin: 0.8rem 0 0;
    font-size: 13px;
    line-height: 1.45;
  }

  .ground-line {
    display: flex;
    gap: 0.5rem;
  }

  .ground-line dd {
    margin: 0;
    color: #4a3628;
  }

  .ground-name {
    flex: none;
    font-weight: 600;
  }

  .ground-name::after {
    content: ' —';
    font-weight: 400;
  }

  .ground-name--hazard {
    color: #8f2f22;
  }

  .ground-name--boon {
    color: #3f5a2e;
  }

  /* Полоса карт и «В бой» держат низ окна: длинное собрание не уводит их за
     сгиб, а поле остаётся над ними. */
  .tray {
    position: sticky;
    bottom: 0;
    z-index: 5;
    margin: 0 -1.5rem;
    padding: 0 1.5rem;
    background: #f8f1e7;
    border-top: 1px solid #d8c6b1;
    box-shadow: 0 -6px 14px -10px rgba(52, 37, 28, 0.35);
  }

  /* Рука слева, собрание справа и прокручивается вбок: так полоса остаётся
     одной высоты, сколько бы карт ни было. */
  .desk {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
    gap: 1.2rem;
    padding-top: 0.6rem;
  }

  .band-title {
    margin: 0 0 0.4rem;
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #6f5a48;
  }

  /* Взятую с поля карту можно бросить обратно в собрание. */
  .band--return {
    outline: 1px dashed rgba(198, 95, 60, 0.6);
    outline-offset: 6px;
  }

  .hand {
    display: flex;
    gap: 0.6rem;
    padding: 0.5rem 0 0.4rem;
  }

  .place {
    position: relative;
    width: 84px;
    aspect-ratio: 5 / 7;
    padding: 0;
    border: 1px dashed rgba(52, 37, 28, 0.3);
    background: rgba(47, 74, 120, 0.05);
  }

  .place--open {
    border: 1px solid #c65f3c;
    box-shadow: inset 0 0 0 1px #c65f3c;
  }

  .place--picked {
    outline: 2px solid #34251c;
    outline-offset: 2px;
  }

  .empty-word {
    font-size: 11px;
    letter-spacing: 0.08em;
    color: #8a6a55;
  }

  .band--drawer {
    min-width: 0;
    padding-left: 1.2rem;
    border-left: 1px solid #d8c6b1;
  }

  .drawer {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 84px;
    gap: 0.7rem;
    overflow-x: auto;
    padding: 0.5rem 0.6rem 0.4rem 0;
  }

  /* Раса против этого противника: под большой картой рядом с полем, а в
     полосе — только там, где карты крупно нет и места под подпись хватает. */
  .pull-edge {
    display: none;
  }

  @media (min-width: 641px) and (max-width: 1099px) {
    .pull-edge {
      display: block;
    }
  }

  .preview .race-edge {
    margin-top: 0.7rem;
    font-size: 12px;
  }

  .pull-wrap {
    position: relative;
  }

  /* Раса против этого противника — под картой, мелко, словами. */
  .race-edge {
    margin: 0.7rem 0 0;
    padding: 0;
    list-style: none;
    font-size: 10px;
    line-height: 1.3;
    color: #5f4636;
  }
  .race-edge--strong {
    color: #8f2f22;
  }
  .race-edge--weak {
    color: #34251c;
    font-style: italic;
  }

  .pull {
    display: block;
    width: 100%;
    padding: 0;
    border: 1px solid transparent;
    transition: transform 0.15s ease;
  }

  .pull:hover {
    transform: translateY(-2px);
  }

  .pull--picked {
    outline: 2px solid #c65f3c;
    outline-offset: 2px;
  }

  /* Карта, которая сломала бы предел рангов: видна и берётся (ею можно
     заменить карту того же ранга), но отмечена словом. */
  .pull--blocked {
    opacity: 0.6;
  }

  .rank-mark {
    position: absolute;
    left: 50%;
    bottom: -0.55rem;
    transform: translateX(-50%);
    padding: 0.05rem 0.4rem;
    font-size: 9px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    white-space: nowrap;
    color: #8f2f22;
    background: #f8f1e7;
    border: 1px solid rgba(143, 47, 34, 0.4);
    pointer-events: none;
  }

  .read {
    position: absolute;
    top: -0.45rem;
    right: -0.45rem;
    z-index: 2;
    width: 1.5rem;
    height: 1.5rem;
    border-radius: 50%;
    font-family: Georgia, serif;
    font-size: 12px;
    font-style: italic;
    color: #f8f1e7;
    background: #6f3b24;
    border: 1px solid #f8f1e7;
  }

  .read:focus-visible {
    outline: 2px solid #c65f3c;
    outline-offset: 1px;
  }

  @media (prefers-reduced-motion: reduce) {
    .pull {
      transition: none;
    }

    .pull:hover {
      transform: none;
    }
  }

  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem 1rem;
    padding: 0.6rem 0 0.7rem;
    margin-top: 0.4rem;
    border-top: 1px dashed #d8c6b1;
  }

  .notes {
    min-width: 0;
    flex: 1 1 18rem;
  }

  .notes p {
    margin: 0;
    font-size: 13px;
    line-height: 1.45;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    margin-left: auto;
  }

  .status {
    min-width: 6.5rem;
    font-size: 12px;
    text-align: right;
    color: #6f5a48;
  }

  .status--failed {
    color: #8f2f22;
  }

  .undo {
    padding: 0.55rem 0.9rem;
    font-size: 12px;
    letter-spacing: 0.08em;
    color: #34251c;
    border: 1px solid #d8c6b1;
    background: transparent;
  }

  .undo:disabled {
    opacity: 0.4;
  }

  .start {
    flex: none;
    padding: 0.7rem 1.6rem;
    font-size: 13px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #f8f1e7;
    background: #6f3b24;
    border: 1px solid #6f3b24;
  }

  .start:hover:not(:disabled) {
    background: #34251c;
  }

  .start:disabled {
    opacity: 0.55;
  }

  .start:focus-visible {
    outline: 2px solid #c65f3c;
    outline-offset: 2px;
  }

  .quiet {
    color: #6f5a48;
    font-size: 13px;
  }

  .fault {
    color: #8f2f22;
  }

  .warn {
    color: #6f3b24;
  }

  .ask {
    color: #34251c;
  }

  .link {
    margin-left: 0.6rem;
    font-size: 12px;
    color: #6f3b24;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .link--block {
    display: block;
    margin: 0.7rem 0 0;
  }

  /* На широком экране карту крупно показывает колонка рядом с полем. */
  @media (min-width: 1100px) {
    .link--narrow {
      display: none;
    }
  }

  .shelf-link {
    color: #6f3b24;
  }

  /* Низкое окно: карты в полосе чуть мельче, чтобы поле осталось на виду. */
  @media (min-width: 641px) and (max-height: 760px) {
    .place {
      width: 70px;
    }

    .drawer {
      grid-auto-columns: 70px;
    }
  }

  @media (max-width: 640px) {
    .tray {
      margin: 0 -1rem;
      padding: 0 1rem;
    }

    .desk {
      gap: 0.7rem;
    }

    .band--drawer {
      padding-left: 0.7rem;
    }

    .status {
      min-width: 0;
    }

    /* Подсказки и отказы в полосе — не выше трёх строк (дальше прокрутка):
       длинная запись иначе съедает пол-экрана у того, что надо видеть. */
    .notes {
      flex-basis: 100%;
      max-height: 3.9em;
      overflow-y: auto;
    }

    .notes p {
      font-size: 12px;
      line-height: 1.3;
    }

    .actions {
      width: 100%;
      justify-content: flex-end;
    }

    .foot {
      gap: 0.3rem 1rem;
      padding: 0.4rem 0 0.5rem;
    }

    .empty-word {
      display: none;
    }

    .band-title {
      margin-bottom: 0.2rem;
      font-size: 10px;
      letter-spacing: 0.1em;
    }

    .band .quiet {
      margin: 0.2rem 0;
      font-size: 12px;
      line-height: 1.3;
    }

    /* Рука из трёх мест и собрание в одной полосе: карта на телефоне —
       64 px, прочесть её можно по «i». */
    .place {
      width: 58px;
    }

    .hand {
      gap: 0.4rem;
    }

    .drawer {
      grid-auto-columns: 64px;
      gap: 0.5rem;
    }
  }
</style>
