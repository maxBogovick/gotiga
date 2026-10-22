<script lang="ts">
  import { figurineNameIn } from '$lib/workText';
  // Этюды хранителя.
  //
  // Расстановка составлена рукой — обе стороны сразу, как шахматный этюд. Гость
  // играет за свою половину против хранителя. Переигрывать можно сколько угодно:
  // пыль даётся за этюд и однажды, иначе самый лёгкий стал бы мельницей.
  //
  // Ни одного правила боя на этой странице нет. Сервер присылает состояние и
  // список законных действий, страница показывает первое и отправляет обратно
  // одно из вторых.
  import { onMount, onDestroy } from 'svelte';
  import { pushState, replaceState } from '$app/navigation';
  import { page } from '$app/state';
  import { fade } from 'svelte/transition';
  import { t, lang, brandName, localized } from '$lib/i18n';
  import { api, ApiError } from '$lib/api';
  import { HOUSE_RULES, deckFaultLine, rulesApart, siegeLines, terrainLines } from '$lib/battles';
  import { authStore } from '$lib/stores/auth.svelte';
  import { interact } from '$lib/analytics';
  import BattleScene from '$lib/components/BattleScene.svelte';
  import BattleDoor from '$lib/components/BattleDoor.svelte';
  import BattleLineup from '$lib/components/BattleLineup.svelte';
  import EtudeMap, { type EtudeKind } from '$lib/components/EtudeMap.svelte';
  import EtudeRoad from '$lib/components/EtudeRoad.svelte';
  import { loadWorld, heroesOfEtude, talesOfWork, workHref, type World } from '$lib/world';
  import { leafHref } from '$lib/gazette';
  import { matchChrome } from '$lib/stores/match-chrome.svelte';
  import type {
    BattleAction,
    BattleBodyCard,
    BattleCard,
    BattleChallenge,
    BattleFrame,
    BattleMatch,
    BattleUnit,
    Foresight,
    Motion,
  } from '$lib/types/api';

  let challenges = $state<BattleChallenge[]>([]);
  // Два режима одной карты: обучение — этюды с заданной расстановкой, прохождение
  // — встречи, где гость приводит свои карты. Режим в адресе (`?mode=`), а не в
  // хранилище браузера: ссылку на прохождение можно отправить.
  //
  // Адрес вида `?play=<бой>` режима не называет (ссылка из входа несёт один бой),
  // и тогда режим берётся у этого боя: иначе, выйдя из встречи, человек
  // оказывался бы на карте обучения.
  let mode = $derived.by<'training' | 'campaign'>(() => {
    const named = page.url.searchParams.get('mode');
    if (named === 'campaign' || named === 'training') return named;
    const id = page.url.searchParams.get('play');
    const found = id ? challenges.find((c) => c.id === id) : null;
    return found?.playerSide === 'deck' ? 'campaign' : 'training';
  });
  let shown = $derived(challenges.filter((c) => (c.playerSide === 'deck') === (mode === 'campaign')));
  let cards = $state<BattleCard[]>([]);
  let frames = $state<BattleFrame[]>([]);
  /** Свод движений. Пустой — комната играет умолчания дома. */
  let motions = $state<Motion[]>([]);
  let match = $state<BattleMatch | null>(null);
  let busy = $state(false);
  let complaint = $state<string | null>(null);
  /** Виноват ли состав карт, а не ход: только тогда у жалобы есть дверь —
   *  вернуться к расстановке. */
  let deckAtFault = $state(false);
  /** Фон поля партии: его задал этюд, а партия о нём не знает. */
  const backdropOf = (m: BattleMatch) =>
    challenges.find((c) => c.id === m.challengeId)?.setup.background ?? null;

  /** Какой этюд играется — чтобы «начать этот заново» знал, какой «этот». */
  let taken = $state<BattleChallenge | null>(null);
  /**
   * Встреча, для которой открыто окно расстановки. Во встрече гость приводит
   * свои карты, и ставит он их перед каждым новым боем на поле этой встречи,
   * а не на отдельной странице: поля у встреч разные.
   */
  //
  // Окно — запись истории без перехода (`pushState`): «назад» в браузере и
  // жест «назад» на телефоне закрывают его, а не уводят с этюдов целиком.
  const arranging = $derived(
    page.state.arrange ? (challenges.find((c) => c.id === page.state.arrange) ?? null) : null,
  );
  /** Начать бой заново, а не продолжить открытый: окно открыто из «ещё раз». */
  const arrangingAgain = $derived(!!page.state.arrangeFresh);
  /** Отказ начала партии — его показывает окно расстановки. */
  let arrangeFault = $state<string | null>(null);
  /** onMount finished — `?play=` must not fire before the challenges are read. */
  let ready = $state(false);
  let openedPlay = false;
  /** `?chamber=1` already dressed a preview — leaving must not dress it again. */
  let openedChamber = false;
  /** Спросить, оставить открытую партию или отдать поле. */
  let leaving = $state(false);

  let signedIn = $derived(authStore.isLoggedIn);

  // Гость боя: без входа играются бои обучения (расстановка задана заранее).
  // Имя заводит сервер нажатием «Начать бой», ключ лежит в браузере — встреча со
  // своей колодой и награда остаются за вошедшими.
  const GUEST_KEY = 'gotiga_battle_guest';
  let guestToken = $state<string | null>(null);
  /** Чем играет эта вкладка: именем или, если его нет, гостевым ключом. */
  const playToken = $derived(authStore.token ?? guestToken);
  const canPlay = (c: BattleChallenge) => signedIn || c.playerSide !== 'deck';

  function readGuest(): string | null {
    try {
      return localStorage.getItem(GUEST_KEY);
    } catch {
      return null;
    }
  }

  /** Ключ для партии: имя, а если его нет и бой гостю открыт — гостевой.
   *  Нет ни того ни другого — `null`, и страница ведёт ко входу. */
  async function tokenFor(c: BattleChallenge): Promise<string | null> {
    if (authStore.token) return authStore.token;
    if (c.playerSide === 'deck') return null;
    if (guestToken) return guestToken;
    let made: string;
    try {
      made = await api.startBattleGuest();
    } catch {
      complaint = $t('battleGuestUnavailable');
      return null;
    }
    guestToken = made;
    try {
      localStorage.setItem(GUEST_KEY, made);
    } catch {
      // Хранилище закрыто — гость живёт до закрытия вкладки.
    }
    interact('etude_guest', c.slug);
    // Метки и порядок карты считаются по этому имени. Не перечиталось — бой всё
    // равно начинается, список обновится со следующим заходом.
    const again = await api.getBattleChallenges(made);
    if (again.length) challenges = again;
    return made;
  }

  // Вошёл по-настоящему — гостевой ключ больше не нужен.
  $effect(() => {
    if (!authStore.token) return;
    guestToken = null;
    try {
      localStorage.removeItem(GUEST_KEY);
    } catch {
      // ничего
    }
  });

  /** Сколько разных карт у человека: «Прохождение» играется своей колодой, и
   *  без этого числа непонятно, что делать на первой встрече. */
  let owned = $state<number | null>(null);
  $effect(() => {
    const token = authStore.token;
    if (!token) {
      owned = null;
      return;
    }
    let live = true;
    api
      .getBattleMe(token)
      .then((me) => live && (owned = new Set(me.owned.map((o) => o.cardId)).size))
      .catch(() => live && (owned = null));
    return () => (live = false);
  });

  /** Все бои открытого режима выиграны — прохождению пора сказать, что дальше. */
  const allWon = $derived(shown.length > 0 && shown.every((c) => c.marks?.won));

  /** Предыдущий бой того же режима, если он ещё не выигран. Совет, а не замок:
   *  порядок держит карта (`current`), правила сервер не меняет. */
  function unwonBefore(c: BattleChallenge): BattleChallenge | null {
    const at = shown.findIndex((x) => x.id === c.id);
    const prev = at > 0 ? shown[at - 1] : null;
    return signedIn && prev && !prev.marks?.won ? prev : null;
  }


  /** Уже 760 px вместо карты стоит дорога (`EtudeRoad`): карта там видна на
   *  треть, а сжатая — печатает подписи в шесть точек. Карту на телефоне не
   *  монтируют вовсе: её картинку незачем считать. */
  let narrow = $state(false);
  $effect(() => {
    const query = window.matchMedia('(max-width: 760px)');
    narrow = query.matches;
    const follow = (e: MediaQueryListEvent) => (narrow = e.matches);
    query.addEventListener('change', follow);
    return () => query.removeEventListener('change', follow);
  });

  const titleOf = (c: BattleChallenge) => localized(c, 'title', $lang);
  const noteOf = (c: BattleChallenge) => localized(c, 'note', $lang);
  /** Чем правила этого боя отличаются от домашних. Пусто — обычный бой, и
   *  говорить нечего. */
  const apart = (c: BattleChallenge) => [
    ...rulesApart(c.setup.rules),
    ...terrainLines(c.setup.terrain),
    ...siegeLines(
      [...c.setup.keeperBoard, ...c.setup.playerBoard].some(
        (p) => cards.find((card) => card.slug === p.card)?.kind === 'structure',
      ),
    ),
  ];

  /** Что стоит против гостя во встрече: сколько карт и каких рас. Читателю
   *  нужно это решить ДО входа в расстановку — кого приводить. */
  function foeLine(c: BattleChallenge): string {
    const bySlug = new Map(cards.map((card) => [card.slug, card]));
    const races = [
      ...new Set(
        [...c.setup.keeperBoard.map((p) => p.card), ...c.setup.keeperHand]
          .map((slug) => bySlug.get(slug))
          .filter((card): card is BattleCard => !!card?.raceId)
          .map((card) => localized(card, 'raceName', $lang)),
      ),
    ];
    return $t('battleFoeLine')
      .replace('{n}', String(c.setup.keeperBoard.length))
      .replace('{h}', String(c.setup.keeperHand.length))
      .replace('{races}', races.length ? races.join(', ') : $t('battleFoeNoRace'));
  }

  function fieldLine(c: BattleChallenge): string {
    const f = c.setup.field ?? { width: 3, depth: 3 };
    return $t('battleFieldLine').replace('{w}', String(f.width)).replace('{d}', String(f.depth));
  }

  /** Род знака на карте: город — там, где на поле стоят строения. */
  function kindOf(c: BattleChallenge): EtudeKind {
    const built = [...c.setup.keeperBoard, ...c.setup.playerBoard].some(
      (p) => cards.find((card) => card.slug === p.card)?.kind === 'structure',
    );
    if (built) return 'city';
    return c.playerSide === 'deck' ? 'meeting' : 'study';
  }

  function loginFrom(challenge?: BattleChallenge) {
    const dest = challenge ? `/battles/etude?play=${challenge.id}` : '/battles/etude';
    return `/login?from=${encodeURIComponent(dest)}`;
  }

  // Работы и байки героинь этюда: из этюда ведёт дорога к их историям.
  let world = $state<World | null>(null);
  onMount(() => {
    loadWorld()
      .then((w) => (world = w))
      .catch(() => {});
  });
  const heroesOf = (c: BattleChallenge) =>
    world
      ? heroesOfEtude(world, c)
          .slice(0, 4)
          .map((card) => ({
            card,
            name: localized(card, 'title', $lang) || figurineNameIn(card, $lang),
            work: workHref(card),
            tale: talesOfWork(world!, card.figurineId)[0] ?? null,
          }))
      : [];

  onMount(async () => {
    guestToken = authStore.token ? null : readGuest();
    const [got, deck, gear, dressing, moving] = await Promise.all([
      api.getBattleChallenges(authStore.token ?? guestToken),
      api.getBattleCards(),
      api.getBattleEtudeCards(),
      api.getBattleFrames(),
      api.getBattleMotions(),
    ]);
    challenges = got;
    // Полка и снаряжение этюдов — один словарь для сцены: стена, которой нет
    // на полке, всё равно стоит в городе со своей картинкой.
    cards = [...deck, ...gear];
    frames = dressing.frames;
    motions = moving.motions;
    ready = true;
  });

  $effect(() => {
    if (match) matchChrome.cover();
    else matchChrome.uncover();
  });

  onDestroy(() => matchChrome.uncover());

  $effect(() => {
    const id = page.url.searchParams.get('play');
    if (!ready || openedPlay || !id || !signedIn || busy || match || arranging) return;
    const found = challenges.find((c) => c.id === id);
    if (!found) return;
    openedPlay = true;
    void takeUp(found);
  });

  /** Живая комната без входа: только в dev, `?chamber=1`. Расстановка — с этюда. */
  $effect(() => {
    if (!import.meta.env.DEV) return;
    if (!ready || match || busy) return;
    if (page.url.searchParams.get('chamber') !== '1') return;
    if (openedChamber) return;
    openedChamber = true;
    const study =
      challenges.find((c) => c.playerSide === 'scripted') ?? challenges[0] ?? null;
    if (!study) return;
    taken = study;
    match = dressStudy(study, cards, page.url.searchParams.get('seal') === '1');
  });

  function bodyOf(card: BattleCard): BattleBodyCard {
    return {
      name: card.slug,
      cost: card.cost,
      health: card.health,
      power: card.power,
      armor: card.armor,
      ward: card.ward,
      reach: card.reach,
      step: card.step,
      mend: card.mend,
      channel: card.attackChannel,
    };
  }

  function dressStudy(study: BattleChallenge, deck: BattleCard[], sealed = false): BattleMatch {
    const find = (slug: string) => deck.find((c) => c.slug === slug);
    const units: BattleUnit[] = [];
    const board: { cell: { x: number; y: number }; unit: number }[] = [];
    let next = 0;
    let playerId = -1;
    let keeperId = -1;
    const put = (
      slug: string,
      x: number,
      y: number,
      owner: 'player' | 'keeper',
    ) => {
      const card = find(slug);
      if (!card) return;
      const id = next++;
      const body = bodyOf(card);
      units[id] = {
        id,
        owner,
        reach: body.reach,
        step: body.step,
        mend: body.mend,
        channel: body.channel,
        acted: false,
        moved: false,
        retaliated: false,
        card: body,
        health: { current: body.health, max: body.health },
        power: body.power,
        armor: body.armor,
        ward: body.ward,
        shield: 0,
        statuses: [],
        immune: null,
      };
      board.push({ cell: { x, y }, unit: id });
      if (owner === 'player' && playerId < 0) playerId = id;
      if (owner === 'keeper' && keeperId < 0) keeperId = id;
    };
    for (const p of study.setup.playerBoard) put(p.card, p.x, p.y, 'player');
    for (const p of study.setup.keeperBoard) put(p.card, p.x, p.y, 'keeper');
    const handOf = (slugs: string[]) =>
      slugs.map((s) => find(s)).filter(Boolean).map((c) => bodyOf(c as BattleCard));
    const playerHand = handOf(study.setup.playerHand);
    const occupied = new Set(board.map((s) => `${s.cell.x},${s.cell.y}`));
    const legal: BattleAction[] = ['endTurn'];
    for (const spot of board) {
      const u = units[spot.unit];
      if (!u || u.owner !== 'player') continue;
      legal.push({
        move: {
          unit: u.id,
          to: { x: (spot.cell.x + 1) % 3, y: spot.cell.y },
        },
      });
      for (const other of board) {
        const t = units[other.unit];
        if (t && t.owner === 'keeper') {
          legal.push({ attack: { attacker: u.id, target: t.id } });
          break;
        }
      }
    }
    for (let i = 0; i < playerHand.length; i++) {
      for (let x = 0; x < 3; x++) {
        for (const y of [3, 4, 5]) {
          if (!occupied.has(`${x},${y}`)) {
            legal.push({ play: { handIndex: i, cell: { x, y } } });
            occupied.add(`${x},${y}`);
            break;
          }
        }
      }
    }
    const events: BattleMatch['events'] = [];
    if (playerId >= 0 && board[0]) {
      const from = board.find((s) => s.unit === playerId)?.cell ?? { x: 0, y: 4 };
      events.push({
        moved: { unit: playerId, from, to: { x: from.x, y: from.y } },
      });
    }
    if (playerId >= 0 && keeperId >= 0) {
      events.push({
        damaged: {
          target: keeperId,
          by: playerId,
          toHealth: 2,
          toShield: 0,
          channel: 'physical',
          source: 'attack',
          trail: [{ step: 'pointBlank', from: 3, to: 2 }],
        },
      });
    }
    events.push({ turnEnded: { side: 'player', round: 1 } });
    return {
      id: 'chamber-preview',
      challengeId: study.id,
      seq: 0,
      state: {
        units,
        board,
        player: { hand: playerHand, mana: 3, manaMax: 5 },
        keeper: {
          hand: handOf(study.setup.keeperHand),
          mana: 2,
          manaMax: 2,
        },
        round: 1,
        active: 'player',
        outcome: sealed ? 'player' : null,
        rules: study.setup.rules ?? HOUSE_RULES,
        openingAttacksUsed: 0,
        actsThisTurn: 0,
      },
      legalActions: legal,
      events,
      outcome: sealed ? 'player' : null,
      rewardDust: sealed ? 12 : 0,
      marks: sealed
        ? { acts: 16, bodiesLost: 2, bestKnown: 9, record: false }
        : undefined,
    };
  }

  /** Встреча, перед которой карты расставляют: открытой партии на ней нет. */
  const needsLineup = (c: BattleChallenge) => c.playerSide === 'deck' && !c.openMatchId;

  function playWord(challenge: BattleChallenge) {
    if (challenge.openMatchId) return $t('battleStudyContinue');
    if (needsLineup(challenge)) return $t('battleLineupOpen');
    return $t('battleStudyPlay');
  }

  /** Почему партия не началась. Отказ стола (`deck:notYours`) приходит сюда
   *  тем же четырёхсотым, что и при сохранении колоды: колода, собранная из
   *  карт, которых у гостя больше нет, начать партию не может, и сказать об
   *  этом надо теми же словами, а не общим «ход потерян» — по нему не понять
   *  ни что сломалось, ни куда идти чинить. */
  function takeUpComplaint(e: unknown): string {
    deckAtFault = String(e).includes('deck:');
    if (String(e).includes('nothingToBring')) return $t('battleNothingToBring');
    if (deckAtFault) return deckFaultLine(e, $t, 'battleActionLost');
    return $t('battleActionLost');
  }

  /** Гостевой ключ мог остыть (имя убрано через месяц): тогда он забывается и
   *  заводится новый, один раз. */
  function forgetGuest() {
    guestToken = null;
    try {
      localStorage.removeItem(GUEST_KEY);
    } catch {
      // ничего
    }
  }

  async function takeUp(challenge: BattleChallenge) {
    if (needsLineup(challenge)) {
      if (!authStore.token) return;
      arrange(challenge, false);
      return;
    }
    busy = true;
    complaint = null;
    deckAtFault = false;
    taken = challenge;
    try {
      let token = await tokenFor(challenge);
      if (!token) return;
      try {
        match = await api.beginBattleMatch(token, challenge.id);
      } catch (e) {
        // Не вошедший и не принятый: ключ гостя остыл.
        if (authStore.token || !guestToken || !(e instanceof ApiError && e.status === 401)) throw e;
        forgetGuest();
        token = await tokenFor(challenge);
        if (!token) throw e;
        match = await api.beginBattleMatch(token, challenge.id);
      }
      noteStart(challenge, !!challenge.openMatchId);
    } catch (e) {
      complaint = takeUpComplaint(e);
    } finally {
      busy = false;
    }
  }

  /** Открыть окно расстановки перед встречей. */
  function arrange(challenge: BattleChallenge, fresh: boolean) {
    complaint = null;
    deckAtFault = false;
    arrangeFault = null;
    leaving = false;
    match = null;
    taken = challenge;
    pushState('', { arrange: challenge.id, arrangeFresh: fresh });
  }

  /** Расстановка сохранена — начать бой: новый, продолженный или заново. */
  async function startArranged() {
    const token = authStore.token;
    const challenge = arranging;
    if (!token || !challenge) return;
    busy = true;
    arrangeFault = null;
    try {
      const next = arrangingAgain
        ? await api.restartBattleMatch(token, challenge.id)
        : await api.beginBattleMatch(token, challenge.id);
      taken = challenge;
      match = next;
      noteStart(challenge, !arrangingAgain && !!challenge.openMatchId);
      // Бой начался: запись окна заменяется, и «назад» из боя не открывает
      // расстановку уже начатой партии.
      replaceState('', {});
      challenges = await api.getBattleChallenges(token);
    } catch (e) {
      arrangeFault = takeUpComplaint(e);
    } finally {
      busy = false;
    }
  }

  function closeArranging() {
    arrangeFault = null;
    taken = null;
    history.back();
  }

  /** Тот же этюд с начала. Открытая партия сбрасывается, не считается проигрышем.
   *  Во встрече перед новым боем снова открывается расстановка — с прежней
   *  расстановкой этой встречи. */
  async function again() {
    const token = playToken;
    if (!token || !taken) return;
    if (taken.playerSide === 'deck') {
      arrange(taken, true);
      return;
    }
    busy = true;
    complaint = null;
    deckAtFault = false;
    leaving = false;
    try {
      match = await api.restartBattleMatch(token, taken.id);
      noteStart(taken, false);
      challenges = await api.getBattleChallenges(token);
    } catch (e) {
      complaint = takeUpComplaint(e);
    } finally {
      busy = false;
    }
  }

  function previewAct(action: BattleAction) {
    const now = match;
    if (!now || now.id !== 'chamber-preview') return;
    const from = now.state.board.find((s) =>
      typeof action !== 'string' && 'move' in action ? s.unit === action.move.unit : false,
    );
    if (action === 'endTurn') {
      match = {
        ...now,
        seq: now.seq + 1,
        events: [{ turnEnded: { side: 'player', round: now.state.round } }],
        state: { ...now.state, active: 'keeper' },
        legalActions: [],
      };
      return;
    }
    if (typeof action !== 'string' && 'move' in action) {
      match = {
        ...now,
        seq: now.seq + 1,
        events: [
          {
            moved: {
              unit: action.move.unit,
              from: from?.cell ?? action.move.to,
              to: action.move.to,
            },
          },
        ],
        state: {
          ...now.state,
          board: now.state.board.map((s) =>
            s.unit === action.move.unit ? { ...s, cell: action.move.to } : s,
          ),
        },
      };
      return;
    }
    if (typeof action !== 'string' && 'attack' in action) {
      const target = now.state.units[action.attack.target];
      const hp = target ? Math.min(2, target.health.current) : 2;
      match = {
        ...now,
        seq: now.seq + 1,
        events: [
          {
            damaged: {
              target: action.attack.target,
              by: action.attack.attacker,
              toHealth: hp,
              toShield: 0,
              channel: 'physical',
              source: 'strike',
              trail: [{ step: 'floor', from: hp + 1, to: 1 }],
            },
          },
        ],
      };
    }
  }

  async function play(action: BattleAction) {
    const token = playToken;
    if (!match) return;
    if (match.id === 'chamber-preview') {
      previewAct(action);
      return;
    }
    if (!token) return;
    busy = true;
    complaint = null;
    deckAtFault = false;
    const sent = match.seq;
    try {
      // `seq` — то, что делает двойной щелчок безвредным: сервер отвечает на
      // повтор той же доской, а не играет ход дважды.
      const next = await api.actInBattleMatch(token, match.id, sent, action);
      // Тот же номер назад — значит, ход не был применён: партия ушла вперёд
      // из другой вкладки. Доска перерисовывается тем, что прислал сервер, и
      // кнопки «обновить» здесь нет: она уже обновлена.
      if (next.seq === sent && !next.outcome) complaint = $t('battleMovedOn');
      match = next;
      if (next.outcome) {
        noteFinish(next);
        challenges = await api.getBattleChallenges(token);
      }
    } catch {
      // Позиция не меняется, выбор не снимается: повторить можно тем же
      // касанием — `seq` делает повтор безвредным.
      complaint = $t('battleActionLost');
    } finally {
      busy = false;
    }
  }

  /**
   * «Если сделать это и на этом закончить ход — чем ответит хранитель».
   *
   * Считает сервер: сцена не знает ни одного правила, а страница — ни одного
   * адреса сверх этого вызова. Ничего не пишет и ни на что не влияет, поэтому
   * молчаливый отказ здесь — правильный ответ: предвестие, которое не пришло,
   * должно просто не появиться, а не сообщать об ошибке посреди партии.
   */
  async function foresee(action: BattleAction): Promise<Foresight | null> {
    if (match?.id === 'chamber-preview') return null;
    const token = playToken;
    if (!token || !match) return null;
    try {
      return await api.foreseeBattleMatch(token, match.id, match.seq, action);
    } catch {
      return null;
    }
  }

  // ── Замер: где люди останавливаются ──────────────────────────────────────
  // Имя события и его слаг — слова, а не текст пользователя. `once = false`:
  // повторный бой того же этюда — новое событие, иначе «ещё раз» не считалось
  // бы вовсе. Что человек выбрал в расстановке, в замер не уходит.
  const modeOf = (c: BattleChallenge) => (c.playerSide === 'deck' ? 'campaign' : 'training');

  function noteStart(c: BattleChallenge, resumed: boolean) {
    interact(resumed ? 'etude_resume' : 'etude_start', `${modeOf(c)}:${c.slug}`, null, false);
  }

  function noteFinish(m: BattleMatch) {
    const c = taken ?? challenges.find((x) => x.id === m.challengeId);
    if (!c || !m.outcome) return;
    interact('etude_finish', `${modeOf(c)}:${c.slug}:${m.outcome}`, m.marks?.acts ?? null, false);
  }

  /** `keep` — оставил открытую партию на потом, `yield` — отдал поле. */
  function noteLeave(how: 'keep' | 'yield', open: BattleMatch | null = match) {
    if (!open || open.outcome || !taken) return;
    interact('etude_leave', `${modeOf(taken)}:${taken.slug}:${how}`, open.state.round, false);
  }

  /** Следующий бой в том же режиме, в порядке карты. Последний — без следующего. */
  const nextChallenge = $derived.by(() => {
    if (!taken) return null;
    const deck = taken.playerSide === 'deck';
    const same = challenges.filter((c) => (c.playerSide === 'deck') === deck);
    const at = same.findIndex((c) => c.id === taken?.id);
    return at >= 0 ? (same[at + 1] ?? null) : null;
  });

  function goNext() {
    const next = nextChallenge;
    if (!next) return;
    putBack();
    void takeUp(next);
  }

  function askLeave() {
    if (match && !match.outcome) {
      leaving = true;
      return;
    }
    putBack();
  }

  function keepMatch() {
    leaving = false;
    noteLeave('keep');
    putBack();
  }

  async function giveField() {
    const token = playToken;
    if (!token || !match) return;
    busy = true;
    leaving = false;
    complaint = null;
    deckAtFault = false;
    try {
      const left = match;
      match = await api.yieldBattleMatch(token, match.id);
      noteLeave('yield', left);
      challenges = await api.getBattleChallenges(token);
    } catch {
      complaint = $t('battleActionLost');
    } finally {
      busy = false;
    }
  }

  function putBack() {
    match = null;
    taken = null;
    complaint = null;
    deckAtFault = false;
    leaving = false;
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape' && leaving) {
      leaving = false;
      e.stopPropagation();
    }
  }
</script>

<svelte:window onkeydown={onkey} />

<svelte:head>
  <title>{$t('battleStudies')} — {$brandName}</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<div class="root" class:root--match={!!match}>
  <div class="grain" aria-hidden="true"></div>
  <div class="page" class:page--match={!!match} class:page--map={!match && !arranging}>
    {#if !match && !arranging}
      <!-- Режимы — второй уровень той же навигации, поэтому стоят в строке
           двери, а не отдельной полосой: всё, что над картой, отнимает у неё
           высоту первого экрана. -->
      <BattleDoor>
        {#if ready && challenges.length}
          <nav class="modes" aria-label={$t('battleModesLabel')}>
            <a
              href="?mode=training"
              class:on={mode === 'training'}
              aria-current={mode === 'training' ? 'page' : undefined}
              data-sveltekit-noscroll
              data-sveltekit-replacestate>{$t('battleModeTraining')}</a
            >
            <a
              href="?mode=campaign"
              class:on={mode === 'campaign'}
              aria-current={mode === 'campaign' ? 'page' : undefined}
              data-sveltekit-noscroll
              data-sveltekit-replacestate>{$t('battleModeCampaign')}</a
            >
          </nav>
        {/if}
        {#if !signedIn}
          <a class="sign" href={loginFrom()}
            >{mode === 'training' ? $t('battleStudySignInKeep') : $t('battleStudySignIn')}</a
          >
        {/if}
      </BattleDoor>

      <!-- Заглавие напечатано на самой карте, в картуше; здесь оно для чтения
           экрана и для заголовка вкладки. -->
      <h1 class="quiet">{$t('battleStudies')}</h1>
    {/if}

    {#if complaint}
      <p class="fault" transition:fade={{ duration: 150 }}>
        {complaint}
        <!-- Жалоба на стол без двери к столу — это тупик: гость узнаёт, что
             колода не годится, ровно там, где её нельзя поправить. -->
        {#if deckAtFault && taken?.playerSide === 'deck'}
          <button type="button" class="fault-door" onclick={() => taken && arrange(taken, true)}
            >{$t('battleLineupRearrange')}</button
          >
        {/if}
      </p>
    {/if}

    {#if arranging}
      <BattleLineup
        challenge={arranging}
        {cards}
        {frames}
        starting={busy}
        fault={arrangeFault}
        onstart={startArranged}
        onback={closeArranging}
      />
    {:else if match}
      <BattleScene
        {match}
        backdrop={backdropOf(match)}
        {cards}
        {frames}
        {motions}
        {busy}
        fill
        onact={play}
        onforesee={foresee}
        onleave={putBack}
        onexit={askLeave}
        onreplay={taken ? again : undefined}
        onnext={nextChallenge ? goNext : undefined}
        guestLoginHref={!signedIn ? loginFrom() : undefined}
      />

    {:else}
      {#if ready && !challenges.length}
        <p class="empty">{$t('battleStudiesEmpty')}</p>
      {/if}
      {#if ready && challenges.length && !shown.length}
        <p class="empty">{mode === 'campaign' ? $t('battleCampaignEmpty') : $t('battleStudiesEmpty')}</p>
      {/if}
      {#if ready && shown.length}
        {#snippet sheet(challenge: BattleChallenge, inRow = false)}
            <div class="row">
              <div class="copy">
                <!-- В строке дороги название уже стоит над листом. -->
                {#if !inRow}<span class="name">{titleOf(challenge)}</span>{/if}
                {#if noteOf(challenge)}
                  <span class="note">{noteOf(challenge)}</span>
                {/if}
                <!-- Два рода записей на одной полке: этюд расставлен рукой
                     целиком, во встречу вы приводите своё. -->
                <span class="kind">
                  {challenge.playerSide === 'deck' ? $t('battleMeetingNote') : $t('battleStudyNote')}
                </span>
                {#if challenge.playerSide === 'deck'}
                  <span class="foe">{foeLine(challenge)} {fieldLine(challenge)}</span>
                  {#if unwonBefore(challenge)}
                    <span class="foe"
                      >{$t('battleAdvisePrev').replace('{title}', titleOf(unwonBefore(challenge)!))}</span
                    >
                  {/if}
                {/if}
                <!-- Чем этот бой отличается от соседнего. Названо только
                     отличие: свод, в котором перечислены все десять правил,
                     не сообщает ничего — читатель обязан помнить наизусть,
                     какие из них обычные. -->
                {#if apart(challenge).length}
                  <span class="apart">
                    {$t('battleRulesOwn')}:
                    {#each apart(challenge) as line, i (line.key)}{i > 0
                        ? ' · '
                        : ' '}{$t(line.key)}{line.amount === null
                        ? ''
                        : ` — ${line.amount}`}{/each}
                  </span>
                {/if}
                <!-- Три печати вместо одной. Победа была двоичной: прошёл и
                     забыл. «За пять дел вместо семи» — утверждение о человеке,
                     и оно не зависит от того, кому досталась монета. -->
                {#if challenge.marks}
                  <span class="marks">
                    <!-- «Доведено» — печать проигравшего: победа говорит то же
                         самое и громче, и две печати рядом читались бы как две
                         разные вещи. -->
                    {#if challenge.marks.finished && !challenge.marks.won}
                      <span class="mark mark--done">{$t('battleMarkFinished')}</span>
                    {/if}
                    {#if challenge.marks.won}
                      <span class="mark mark--won">{$t('battleMarkWon')}</span>
                    {/if}
                    {#if challenge.marks.clean}
                      <span class="mark mark--clean">{$t('battleMarkClean')}</span>
                    {/if}
                    {#if challenge.marks.yourBest != null}
                      <span class="line"
                        >{$t('battleMarkYourLine')} — {challenge.marks.yourBest}</span
                      >
                    {/if}
                    {#if challenge.marks.bestKnown != null}
                      <span class="line line--bar"
                        >{$t('battleMarkBestLine')} — {challenge.marks.bestKnown}</span
                      >
                    {/if}
                  </span>
                {/if}
                {#if heroesOf(challenge).length}
                  <span class="heroes">
                    {$t('worldEtudeHeroes')}:
                    {#each heroesOf(challenge) as hero, i (hero.card.id)}{i > 0 ? ' · ' : ' '}{#if hero.work}<a
                          href={hero.work}
                          onclick={() => interact('world_link', 'etude_to_work')}>{hero.name}</a
                        >{:else}{hero.name}{/if}{#if hero.tale}{' '}(<a
                          href={leafHref(hero.tale, 'etude')}
                          onclick={() => interact('world_link', 'etude_to_tale')}>{$t('worldTale')}</a
                        >){/if}{/each}
                  </span>
                {/if}
              </div>
              {#if !signedIn && challenge.playerSide !== 'deck'}
                <span class="reward">{$t('battleGuestNoReward')}</span>
              {:else if challenge.rewardDust > 0 || challenge.rewardFinishDust > 0}
                <span class="reward">
                  {#if challenge.alreadyPaid}
                    {$t('battleStudyPaid')}
                  {:else}
                    {#if challenge.rewardDust > 0}
                      {challenge.rewardDust} {$t('battleStudyReward')}
                    {/if}
                    <!-- Сказано ЗАРАНЕЕ, а не после проигрыша: человек, знающий,
                         что за доведённую до конца партию платят, садится за
                         доску иначе, чем тот, кто думает, что играет за всё или
                         ничего. -->
                    {#if challenge.rewardFinishDust > 0}
                      <span class="reward-finish"
                        >{challenge.rewardFinishDust} {$t('battleStudyFinishReward')}</span
                      >
                    {/if}
                  {/if}
                </span>
              {/if}
              {#if canPlay(challenge)}
                <button
                  type="button"
                  class="play"
                  disabled={busy}
                  onclick={() => takeUp(challenge)}
                >{playWord(challenge)}</button>
              {:else}
                <a
                  class="play play--door"
                  href={loginFrom(challenge)}
                  onclick={() => interact('etude_login_wall', `${modeOf(challenge)}:${challenge.slug}`)}
                  >{$t('battleStudySignInStart')}</a
                >
              {/if}
            </div>
        {/snippet}
        {#if narrow}
          <EtudeRoad challenges={shown} {kindOf} {titleOf} {sheet} />
        {:else}
          <EtudeMap challenges={shown} {kindOf} {titleOf} {sheet} />
        {/if}
        <!-- Под картой, а не над ней: прочесть это можно и после того, как
             карту увидели. Общее правило (`battleStudiesRule`) повторяло
             подсказку обоих режимов — осталась подсказка открытого. -->
        <p class="page-rule">
          {mode === 'campaign' ? $t('battleModeCampaignHint') : $t('battleModeTrainingHint')}
        </p>
        {#if mode === 'campaign' && owned !== null}
          <p class="page-rule">
            {owned === 0
              ? $t('battleCampaignNoCards')
              : $t('battleCampaignOwned').replace('{n}', String(owned))}
            <a class="shelf-link" href="/battles">{$t('battleCampaignShelf')}</a>
          </p>
        {/if}
        {#if mode === 'campaign' && allWon}
          <p class="page-rule">
            {$t('battleCampaignDone')}
            <a class="shelf-link" href="/battles">{$t('battleCampaignShelf')}</a>
          </p>
        {/if}
      {/if}
    {/if}
  </div>
</div>

{#if leaving}
  <div
    class="veil"
    role="presentation"
    onclick={() => (leaving = false)}
    transition:fade={{ duration: 200 }}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="ask"
      role="dialog"
      aria-modal="true"
      aria-label={$t('battleLeave')}
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
    >
      <p class="ask-word">{$t('battleLeaveAsk')}</p>
      <p class="ask-doors">
        <button type="button" class="ask-keep" onclick={keepMatch}>{$t('battleLeaveKeep')}</button>
        <button type="button" class="ask-yield" onclick={giveField}>{$t('battleLeaveYield')}</button>
      </p>
    </div>
  </div>
{/if}

<style>
  .root {
    position: relative;
    min-height: 100vh;
    background: #f8f1e7;
    color: #34251c;
  }

  .root--match {
    height: 100dvh;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    background: #0b0806;
    color: #e6dcc8;
  }

  .root--match .grain {
    display: none;
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
    max-width: 1560px;
    margin: 0 auto;
    padding: 3rem 1.5rem 6rem;
  }

  .page--match {
    flex: 1 1 auto;
    min-height: 0;
    max-width: none;
    width: 100%;
    padding: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
  }

  .page--match :global(.room) {
    flex: 1 1 auto;
    min-height: 0;
  }

  .page-rule {
    max-width: 56ch;
    margin: 0.5rem auto 0;
    text-align: center;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1rem;
    line-height: 1.6;
    opacity: 0.74;
  }

  .shelf-link {
    color: #6f3b24;
    border-bottom: 1px solid rgba(111, 59, 36, 0.45);
    text-decoration: none;
    white-space: nowrap;
  }

  .shelf-link:hover {
    color: #c65f3c;
    border-bottom-color: #c65f3c;
  }

  .fault {
    margin: 0 0 1.25rem;
    font-size: 0.9rem;
    color: #8f2f22;
  }

  .fault-door {
    margin-left: 0.6rem;
    color: #6f3b24;
    text-decoration: none;
    border-bottom: 1px solid rgba(111, 59, 36, 0.45);
  }

  .fault-door:hover {
    color: #c65f3c;
    border-bottom-color: #c65f3c;
  }

  .empty {
    margin: 1.5rem 0 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.95rem;
    font-style: italic;
    color: #5f4636;
  }

  /* Первый экран отдан карте: страница не держит над ней ни отступа, ни
     полос. Сколько остаётся над картой, знает `--map-above`; по нему карта
     вписывает свою высоту в окно (EtudeMap). */
  .page--map {
    --map-above: 9.5rem;
    padding-top: 0.75rem;
  }

  .page--map :global(.door) {
    margin-bottom: 0.5rem;
  }

  /* У вошедшего в строке ещё кошелёк и три кнопки: на планшете ей
     тесно, и она переносится, а не уезжает за край. */
  .page--map :global(.door .row) {
    flex-wrap: wrap;
    row-gap: 0.4rem;
  }

  .page--map :global(.door-mark) {
    margin-top: -0.5rem;
  }

  /* Вход для гостя — в правом конце строки, там, где у вошедшего кошелёк. */
  .sign {
    margin-left: auto;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.9rem;
    font-style: italic;
    color: #5f4636;
    text-decoration: underline;
    text-underline-offset: 0.18em;
  }

  .sign:hover {
    color: #c65f3c;
  }

  /* Режимы — переключатель из двух половин в строке двери, после комнат:
     это второй уровень той же навигации. Обычные ссылки: у каждого режима
     свой адрес. */
  .modes {
    display: inline-flex;
    margin-left: 0.4rem;
    border: 1px solid #d8c6b1;
  }

  .modes a {
    padding: 0.32rem 0.85rem;
    color: #6f3b24;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.74rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    text-decoration: none;
  }

  .modes a + a {
    border-left: 1px solid #d8c6b1;
  }

  .modes a:hover:not(.on) {
    color: #c65f3c;
  }

  .modes a.on {
    background: #6f3b24;
    color: #f8f1e7;
  }

  /* На телефоне строка двери переносится (BattleDoor: комнаты и кошелёк,
     разрыв, кнопки). Режимы встают отдельной строкой во всю ширину после
     разрыва: рядом с кошельком им места нет, и кошелёк рвался пополам. */
  @media (max-width: 40rem) {
    .modes {
      order: 2;
      flex-basis: 100%;
      margin-left: 0;
    }

    .modes a {
      flex: 1;
      text-align: center;
    }
  }

  .quiet {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  /* Листок этюда на карте: строка стала столбцом. */
  .row {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.8rem;
  }

  .copy {
    min-width: 0;
  }

  .name {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.35rem;
    line-height: 1.25;
  }

  .note,
  .kind,
  .foe {
    display: block;
    margin-top: 0.2rem;
    font-size: 0.78rem;
    line-height: 1.45;
    color: #8a6a55;
  }

  .kind {
    font-style: italic;
  }

  .heroes {
    display: block;
    margin-top: 0.25rem;
    font-size: 0.72rem;
    line-height: 1.5;
    color: #5b4638;
  }
  .heroes a {
    color: #6f3b24;
    text-decoration: none;
    border-bottom: 1px solid rgba(198, 95, 60, 0.35);
  }
  .heroes a:hover {
    color: #34251c;
  }

  .apart {
    display: block;
    margin-top: 0.25rem;
    font-size: 0.72rem;
    line-height: 1.5;
    color: #6f3b24;
  }

  /* Печати. Не значки и не медали: три коротких слова в строку — комната,
     которая хвалит громче, чем говорит, перестаёт быть этой комнатой. */
  .marks {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.35rem 0.6rem;
    margin-top: 0.35rem;
  }

  .mark {
    padding: 0.08rem 0.4rem;
    font-size: 0.66rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #6f3b24;
    border: 1px solid #d8c6b1;
  }

  .mark--won {
    color: #c65f3c;
    border-color: #c65f3c;
  }

  .mark--clean {
    color: #f8f1e7;
    background: #6f3b24;
    border-color: #6f3b24;
  }

  .line {
    font-size: 0.7rem;
    font-variant-numeric: tabular-nums;
    color: #8a6a55;
  }

  /* Планка дома. Она и есть причина вернуться, поэтому читается как чужой
     результат, а не как своя строка. */
  .line--bar {
    font-style: italic;
  }

  .reward-finish {
    display: block;
    opacity: 0.75;
  }

  .reward {
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
    color: #8a6a55;
  }

  .play {
    padding: 0.4rem 0.7rem;
    font: inherit;
    font-size: 0.68rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #6f3b24;
    background: none;
    border: 1px solid #d8c6b1;
    text-decoration: none;
    white-space: nowrap;
    cursor: pointer;
  }

  .play:hover:not(:disabled) {
    color: #c65f3c;
    border-color: #c65f3c;
  }

  .play:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .play--door {
    display: inline-block;
  }

  .leave-row {
    display: none;
  }

  .page--match .fault {
    position: absolute;
    top: 4.6rem;
    left: 50%;
    z-index: 30;
    transform: translateX(-50%);
    margin: 0;
    padding: 0.35rem 0.8rem;
    background: rgba(18, 10, 8, 0.88);
    border: 1px solid #8a7040;
    color: #f0c8b0;
  }

  .veil {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    background: rgba(8, 6, 5, 0.72);
    backdrop-filter: blur(2px);
  }

  .ask {
    max-width: 28rem;
    padding: 1.6rem 1.7rem 1.4rem;
    background: #1a1410;
    border: 1px solid #8a7040;
    outline: 1px solid #8a7040;
    outline-offset: 4px;
    transform: rotate(-1deg);
    color: #e6dcc8;
  }

  .ask-word {
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.05rem;
    line-height: 1.5;
    color: #e6dcc8;
  }

  .ask-doors {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem 1.4rem;
    margin: 1.1rem 0 0;
  }

  .ask-keep,
  .ask-yield {
    padding: 0;
    font: inherit;
    font-size: 0.68rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    background: none;
    border: 0;
    cursor: pointer;
  }

  .ask-keep {
    color: #e8c96a;
  }

  .ask-yield {
    color: #a09070;
  }

  .ask-keep:hover,
  .ask-yield:hover {
    color: #c65f3c;
  }
</style>
