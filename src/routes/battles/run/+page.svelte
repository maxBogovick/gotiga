<script lang="ts">
  // Забег: цепочка волн.
  //
  // Правил боя на странице нет, как и на странице этюдов: сервер присылает
  // состояние и список законных действий, страница показывает первое и шлёт
  // обратно одно из вторых. Здесь добавлено только то, что стоит между волнами:
  // отряд, награда из трёх, замена, трофей.
  import { onMount, onDestroy, tick, untrack } from 'svelte';
  import { t, lang, localized, type TranslationKey } from '$lib/i18n';
  import { api, ApiError } from '$lib/api';
  import { cardCopy } from '$lib/battles';
  import { authStore } from '$lib/stores/auth.svelte';
  import { interact } from '$lib/analytics';
  import BattleScene from '$lib/components/BattleScene.svelte';
  import BattleCardFace from '$lib/components/BattleCard.svelte';
  import BattleDoor from '$lib/components/BattleDoor.svelte';
  import BattleModes from '$lib/components/BattleModes.svelte';
  import RunPlacement from '$lib/components/RunPlacement.svelte';
  import CardPeek, { type PeekHover } from '$lib/components/CardPeek.svelte';
  import RunReplay from '$lib/components/RunReplay.svelte';
  import CardMenu, { type MenuItem, type MenuAt } from '$lib/components/CardMenu.svelte';
  import { installRunStand } from '$lib/runStand/install';
  import { cellKey, reconcile, safeLayout, threatened, toRequest, type Placement } from '$lib/runPlacement';
  import { matchChrome } from '$lib/stores/match-chrome.svelte';
  import { sfx, soundOn, setSound } from '$lib/sfx';
  import type {
    BattleAction,
    BattleCard,
    BattleMe,
    BattleFrame,
    BattleMatch,
    BattleRace,
    BattleDaily,
    RunWaveLog,
    BattleRoute,
    BattleRun,
    Foresight,
    Motion,
    RunReward,
    RunTrophy,
    WaveGoal,
  } from '$lib/types/api';

  let routes = $state<BattleRoute[]>([]);
  let run = $state<BattleRun | null>(null);
  let match = $state<BattleMatch | null>(null);
  let cards = $state<BattleCard[]>([]);
  let frames = $state<BattleFrame[]>([]);
  let motions = $state<Motion[]>([]);
  let races = $state<BattleRace[]>([]);
  let ready = $state(false);
  let busy = $state(false);
  /** Кошелёк в двери боя: после замены и трофея его надо перечитать. */
  let me = $state<BattleMe | null>(null);
  /** Первое нажатие «Сдать забег» спрашивает, второе сдаёт. */
  let abandonAsked = $state(false);
  let abandonTimer: ReturnType<typeof setTimeout> | null = null;
  let complaint = $state<string | null>(null);
  /** Что случилось после нажатия, словами: куда попала купленная карта. */
  let notice = $state<string | null>(null);
  /** Кто выходит в следующую волну. */
  let chosen = $state<string[]>([]);
  /** Где каждая выходящая карта встанет: клетки, которые игрок поправил или принял. */
  let cells = $state<Placement>({});
  /** Пыль, которую дала только что кончившаяся волна. */
  let paid = $state(0);
  /** Потери последней волны: кто ранен (и на сколько) и кто пал. */
  /** Факты о проигранной волне: условие, сколько карт вышло, сколько вышло ранеными. */
  let lostWhy = $state<string[]>([]);
  let report = $state<{ name: string; lost: number; fell: boolean }[] | null>(null);
  /** Выдвижная панель на экране забега: отряд или добыча. */
  let panel = $state<'squad' | 'loot' | null>(null);

  // Гость боя: ключ лежит в браузере, как и на странице этюдов.
  const GUEST_KEY = 'gotiga_battle_guest';
  let guestToken = $state<string | null>(null);
  const signedIn = $derived(authStore.isLoggedIn);
  const token = $derived(authStore.token ?? guestToken);

  function readGuest(): string | null {
    try {
      return localStorage.getItem(GUEST_KEY);
    } catch {
      return null;
    }
  }

  async function tokenForRun(route: BattleRoute): Promise<string | null> {
    if (authStore.token) return authStore.token;
    if (route.guestWaves == null) return null;
    if (guestToken) return guestToken;
    try {
      const made = await api.startBattleGuest();
      guestToken = made;
      try {
        localStorage.setItem(GUEST_KEY, made);
      } catch {
        // Хранилище закрыто — гость живёт до закрытия вкладки.
      }
      interact('battle_run_guest', route.slug);
      return made;
    } catch {
      complaint = $t('battleGuestUnavailable');
      return null;
    }
  }

  /** Отказ сервера словами: код `run:…` в теле ответа. */
  function faultLine(err: unknown): string {
    const text = err instanceof ApiError ? err.message : '';
    if (text.includes('run:off')) return $t('runFaultOff');
    if (text.includes('run:tooFew')) return $t('runFaultTooFew');
    if (text.includes('run:tooMany')) return $t('runFaultTooMany');
    if (text.includes('run:noBench')) return $t('runFaultNoBench');
    if (text.includes('run:cellBlocked') || text.includes('run:noRoom')) return $t('runFaultCell');
    if (text.includes('Not enough in the wallet')) return $t('runFaultNotEnough');
    if (text.includes('run:notOnTheShelf') || text.includes('run:notForSale')) return $t('runTrophyNotForSale');
    return $t('runFault');
  }

  /** Строка перевода с подстановкой `{имя}`. */
  const f = (key: TranslationKey, vars: Record<string, string | number>) =>
    Object.entries(vars).reduce((out, [k, v]) => out.replace(`{${k}}`, String(v)), $t(key));

  const loginHref = `/login?from=${encodeURIComponent('/battles/run')}`;

  const titleOf = (r: BattleRoute) => localized(r, 'title', $lang);
  const cardOf = (id: string) => cards.find((c) => c.id === id);
  const nameOf = (id: string) => {
    const c = cardOf(id);
    return c ? cardCopy(c, $lang).title : '';
  };
  const raceName = (slug: string | null) => {
    if (!slug) return '';
    const r = races.find((x) => x.slug === slug);
    return r ? localized(r, 'name', $lang) : '';
  };

  function goalLine(goal: WaveGoal | null): string {
    if (!goal) return $t('runGoalAll');
    if (goal.kind === 'holdRounds') return f('runGoalHold', { n: goal.rounds });
    if (goal.kind === 'defendStructure') return $t('runGoalDefend');
    return $t('runGoalKill');
  }

  function rewardLine(r: RunReward): string {
    switch (r.kind) {
      case 'dust':
        return f('runRewardDust', { n: r.amount });
      case 'healAll':
        return f('runRewardHealAll', { n: r.amount });
      case 'healOne':
        return $t('runRewardHealOne');
      case 'revive':
        return $t('runRewardRevive');
      default:
        return $t('runRewardFreeSwap');
    }
  }

  const PRIZE: Record<RunReward['kind'], { title: TranslationKey; tone: string }> = {
    dust: { title: 'runPrizeDust', tone: '#b08a3a' },
    healAll: { title: 'runPrizeHealAll', tone: '#8f2f22' },
    healOne: { title: 'runPrizeHealOne', tone: '#8f2f22' },
    revive: { title: 'runPrizeRevive', tone: '#c65f3c' },
    freeSwap: { title: 'runPrizeFreeSwap', tone: '#4a6b3a' },
  };

  /** Что награда сделает с этим отрядом: строки «карта · было → станет».
   *  Считается так же, как на сервере (`apply_reward`): самая раненая — та,
   *  у кого больше всего не хватает до полного, при равенстве первая в отряде. */
  function prizeDetail(r: RunReward, from: BattleRun): string[] {
    const alive = from.squad.filter((c) => !c.dead);
    const line = (c: { cardId: string; slug: string }, was: number, now: number) =>
      `${nameOf(c.cardId) || c.slug} · ${was} → ${now}`;
    switch (r.kind) {
      case 'healAll': {
        const hurt = alive.filter((c) => c.health < c.maxHealth);
        const rows = hurt.slice(0, 3).map((c) => line(c, c.health, Math.min(c.health + r.amount, c.maxHealth)));
        return hurt.length > 3 ? [...rows, `+${hurt.length - 3}`] : rows;
      }
      case 'healOne':
        return alive.some((c) => c.health < c.maxHealth) ? [$t('runPrizeCharge')] : [];
      case 'revive': {
        const down = from.squad.find((c) => c.dead);
        return down ? [line(down, 0, Math.max(Math.floor(down.maxHealth / 2), 1))] : [];
      }
      case 'freeSwap':
        return from.swapCost != null && from.freeSwaps === 0 ? [f('runPrizeSaves', { n: from.swapCost })] : [];
      default:
        return [];
    }
  }

  /** Живые карты отряда, сколько позволяет волна, — по умолчанию выходят они.
   *  Если карт больше, чем мест, берутся самые целые; порядок остаётся отрядным. */
  function defaultChosen(r: BattleRun): string[] {
    const max = r.nextWave?.maxCards ?? 0;
    const alive = r.squad.filter((c) => !c.dead);
    if (alive.length <= max) return alive.map((c) => c.cardId);
    const share = (c: (typeof alive)[number]) => c.health / Math.max(c.maxHealth, 1);
    const best = new Set(
      [...alive]
        .sort((a, b) => share(b) - share(a))
        .slice(0, max)
        .map((c) => c.cardId),
    );
    return alive.filter((c) => best.has(c.cardId)).map((c) => c.cardId);
  }

  function take(next: BattleRun | null) {
    // Сменился экран (волна, награда, итог): начинаем с его верха, а не с места,
    // где стоял палец на прошлом.
    const moved =
      !run || !next || run.id !== next.id || run.waveNo !== next.waveNo || run.phase !== next.phase || run.state !== next.state;
    // Выбор игрока живёт, пока та же волна и все выбранные карты на месте: после
    // трофея, не вошедшего в отряд, он не должен возвращаться к выбору по умолчанию.
    const kept =
      !moved && next && next.state === 'active'
        ? chosen.filter((id) => next.squad.some((c) => c.cardId === id && !c.dead))
        : [];
    const keepChoice = kept.length > 0 && kept.length === chosen.length;
    run = next;
    if (moved) panel = null;
    if (moved && ready) void tick().then(() => window.scrollTo({ top: 0 }));
    chosen = next && next.state === 'active' ? (keepChoice ? kept : defaultChosen(next)) : [];
  }

  // Сообщение об ошибке гаснет само: оно висит над страницей, а не в её теле.
  $effect(() => {
    if (!complaint) return;
    const timer = setTimeout(() => (complaint = null), 9000);
    return () => clearTimeout(timer);
  });
  $effect(() => {
    if (!notice) return;
    const timer = setTimeout(() => (notice = null), 7000);
    return () => clearTimeout(timer);
  });

  // Подсказка новичку на первой волне: показывается, пока игрок сам её не закроет.
  const TIPS_KEY = 'gotiga_run_tips';
  let tipsShut = $state(readTipsShut());
  function readTipsShut(): boolean {
    try {
      return localStorage.getItem(TIPS_KEY) === '1';
    } catch {
      return false;
    }
  }
  function shutTips() {
    tipsShut = true;
    interact('battle_run_tip_shut', run?.route.slug ?? null);
    try {
      localStorage.setItem(TIPS_KEY, '1');
    } catch {
      // Хранилище закрыто — подсказка вернётся в другой раз.
    }
  }

  // Подробности карты вне боя (список отряда и добычи): попап при наведении, лист по нажатию.
  let hovered = $state<PeekHover | null>(null);
  let looked = $state<BattleCard | null>(null);
  let lookedNote = $state<string | undefined>();
  function peekHover(e: Event, card: BattleCard | undefined, note?: string) {
    if (!card || looked) return;
    const r = (e.currentTarget as Element).getBoundingClientRect();
    const w = Math.min(260, window.innerWidth - 16);
    const left = Math.max(8, Math.min(r.left + r.width / 2 - w / 2, window.innerWidth - w - 8));
    hovered = r.top > 280 ? { card, left, bottom: window.innerHeight - r.top + 8, note } : { card, left, top: r.bottom + 8, note };
  }
  const peekLeave = () => (hovered = null);
  function peekOpen(card: BattleCard | undefined, note?: string) {
    if (!card) return;
    hovered = null;
    looked = card;
    lookedNote = note;
    interact('battle_run_peek', card.slug);
  }

  let sound = $state(soundOn());
  function toggleSound() {
    sound = !sound;
    setSound(sound);
    if (sound) sfx.lift();
  }

  /** Пыль на экране догоняет кошелёк за долю секунды: прибавку видно, а не просто подменили число. */
  let shownDust = $state(0);
  let dustSeen = false;
  $effect(() => {
    const to = me?.dust;
    if (to == null) return;
    const from = untrack(() => shownDust);
    if (!dustSeen || from === to || matchMedia('(prefers-reduced-motion: reduce)').matches) {
      dustSeen = true;
      shownDust = to;
      return;
    }
    const began = performance.now();
    let frame = 0;
    const step = (now: number) => {
      const k = Math.min(1, (now - began) / 700);
      shownDust = Math.round(from + (to - from) * k);
      if (k < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  });

  /** Плашка «Волна N из M» на пару секунд после начала волны. */
  let banner = $state<{ n: number; total: number } | null>(null);
  $effect(() => {
    if (!banner) return;
    const timer = setTimeout(() => (banner = null), 1900);
    return () => clearTimeout(timer);
  });

  async function readMe() {
    if (!authStore.token) return;
    try {
      me = await api.getBattleMe(authStore.token);
    } catch {
      // Кошелёк не прочесть — дверь покажет прежнее.
    }
  }

  onMount(async () => {
    // Стенд забега: страница на подставных данных по ссылке администратора (`?stand=…`, § 31).
    const standId = new URLSearchParams(location.search).get('stand');
    if (standId) await installRunStand(standId);
    guestToken = authStore.token ? null : readGuest();
    try {
      const [list, deck, gear, dressing, moving, kinds] = await Promise.all([
        api.getBattleRoutes(),
        api.getBattleCards(),
        api.getBattleEtudeCards(),
        api.getBattleFrames(),
        api.getBattleMotions(),
        api.getBattleRaces(),
      ]);
      routes = list;
      cards = [...deck, ...gear];
      frames = dressing.frames;
      motions = moving.motions;
      races = kinds;
    } catch {
      // Справочники не пришли: без них страница не нарисует ни отряд, ни врагов.
      complaint = $t('runFault');
    }
    const key = authStore.token ?? guestToken;
    if (key) {
      try {
        take(await api.getCurrentBattleRun(key));
        if (run?.matchId) match = await api.getBattleMatch(key, run.matchId);
      } catch {
        // Забега нет или ключ устарел — страница покажет список маршрутов.
      }
    }
    ready = true;
    void readMe();
    void loadDaily();
  });

  /** Забег идёт и партии нет: экран на всю высоту окна, без прокрутки. */
  const stageOn = $derived(ready && !match && !!run && run.state === 'active');

  $effect(() => {
    if (match || stageOn) matchChrome.cover();
    else matchChrome.uncover();
  });
  onDestroy(() => {
    autoStop();
    if (abandonTimer) clearTimeout(abandonTimer);
    matchChrome.uncover();
  });

  async function guarded(work: () => Promise<void>) {
    if (busy) return;
    busy = true;
    complaint = null;
    try {
      await work();
    } catch (err) {
      complaint = faultLine(err);
      await resync();
    } finally {
      busy = false;
    }
  }

  /** После отказа берём забег с сервера: страница могла отстать (второе окно,
   *  повтор запроса), и кнопки показывали бы то, чего уже нет. */
  async function resync() {
    const key = token;
    if (!key || !run || match) return;
    try {
      const fresh = await api.getBattleRun(key, run.id);
      if (fresh.state === run.state && fresh.waveNo === run.waveNo && fresh.phase === run.phase) return;
      take(fresh);
    } catch {
      // Забега не прочесть — оставляем то, что на экране.
    }
  }

  // Как прошли волны: список законченных партий и пересмотр одной из них.
  let waveLog = $state<RunWaveLog[]>([]);
  let replayOf = $state<RunWaveLog | null>(null);
  $effect(() => {
    const id = run?.id;
    run?.waveNo;
    run?.state;
    run?.phase;
    const key = token;
    if (!id || !key) return;
    let alive = true;
    api
      .getBattleRunWaves(key, id)
      .then((rows) => {
        if (alive) waveLog = rows;
      })
      .catch(() => {
        if (alive) waveLog = [];
      });
    return () => {
      alive = false;
    };
  });

  // Маршрут дня: только для вошедших, гостю заходить в него незачем.
  let daily = $state<BattleDaily | null>(null);
  async function loadDaily() {
    if (!authStore.token) {
      daily = null;
      return;
    }
    try {
      daily = await api.getBattleDaily(authStore.token);
    } catch {
      daily = null;
    }
  }
  const startDaily = () =>
    guarded(async () => {
      const key = authStore.token;
      if (!key || !daily) return;
      paid = 0;
      report = null;
      lostWhy = [];
      take(await api.startBattleRun(key, daily.route.slug, [], true));
      interact('battle_run_daily', daily.route.slug);
      void loadDaily();
    });

  const start = (route: BattleRoute) =>
    guarded(async () => {
      const key = await tokenForRun(route);
      if (!key) {
        complaint ??= $t('runFault');
        return;
      }
      paid = 0;
      report = null;
      lostWhy = [];
      take(await api.startBattleRun(key, route.slug));
      interact('battle_run_start', route.slug);
    });

  /** Тренировка проигранной волны: страница играет ту же волну, но в сервер уходит другая ручка. */
  let practicing = $state(false);
  function startPractice() {
    if (!run?.practice) return;
    const p = run.practice;
    practicing = true;
    take({ ...run, state: 'active', phase: 'wave', squad: p.squad, nextWave: p.wave, matchId: null, swapCost: null, freeSwaps: 0, reward: [], practice: null });
    interact('battle_run_practice', run?.route.slug ?? null, p.wave.waveNo);
  }
  async function endPractice() {
    practicing = false;
    if (!token || !run) return;
    take(await api.getBattleRun(token, run.id));
  }

  const beginWave = () =>
    guarded(async () => {
      if (!run || !token) return;
      const wave = run.waveNo;
      const placed = run.nextWave?.field ? toRequest(chosen, cells) : chosen.map((cardId) => ({ cardId }));
      match = practicing ? await api.practiceBattleRunWave(token, run.id, placed) : await api.beginBattleRunWave(token, run.id, placed);
      banner = { n: wave, total: run.wavesTotal };
      sfx.start();
      interact('battle_wave_start', `${run.route.slug}:${wave}`, wave);
    });

  async function play(action: BattleAction) {
    if (!match || !token || busy || auto) return;
    busy = true;
    complaint = null;
    const sent = match.seq;
    try {
      const next = await api.actInBattleMatch(token, match.id, sent, action);
      if (next.seq === sent && !next.outcome) complaint = $t('battleMovedOn');
      match = next;
      if (next.outcome) await finishWave(next);
    } catch {
      complaint = $t('battleActionLost');
    } finally {
      busy = false;
    }
  }

  /** Автобой включён: ходы за игрока делает бот сервера, пока человек не
   *  остановит. Выключается сам, когда волна кончилась. */
  let auto = $state(false);
  let autoTurn = 0;
  const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

  async function autoPlay() {
    if (!match || !token || auto) return;
    auto = true;
    const mine = ++autoTurn;
    interact('battle_run_auto', run?.route.slug ?? null, run?.waveNo ?? null);
    try {
      while (auto && mine === autoTurn && match && !match.outcome && token) {
        if (busy) {
          await pause(200);
          continue;
        }
        busy = true;
        complaint = null;
        const sent = match.seq;
        let shown = 0;
        try {
          const next = await api.autoActInBattleMatch(token, match.id, sent);
          if (next.seq === sent && !next.outcome) {
            complaint = $t('battleMovedOn');
            break;
          }
          match = next;
          shown = next.events.length;
          if (next.outcome) {
            await finishWave(next);
            break;
          }
        } catch {
          complaint = $t('battleActionLost');
          break;
        } finally {
          busy = false;
        }
        // Сцене нужно время показать ходы, прежде чем придут следующие.
        await pause(Math.min(5000, Math.max(450, 250 + shown * 400)));
      }
    } finally {
      if (mine === autoTurn) auto = false;
    }
  }

  const autoStop = () => {
    auto = false;
    autoTurn++;
  };

  /** Партия кончилась: забег уже ушёл дальше на сервере, страница берёт его
   *  оттуда. Ответ на ход мог не донести забег (сбой) — тогда спрашиваем. */
  async function finishWave(done: BattleMatch) {
    if (!token) return;
    const before = run;
    // Тренировка (или её партия, открытая заново после перезагрузки): забег не менялся,
    // страница берёт его таким, какой он, и только говорит, чем кончилась волна.
    if (practicing || before?.state === 'lost') {
      practicing = false;
      if (before) take(await api.getBattleRun(token, before.id));
      notice = done.outcome === 'player' ? $t('runPracticeWon') : $t('runPracticeLost');
      interact('battle_run_practice_end', before ? `${before.route.slug}:${before.waveNo}` : null, done.outcome === 'player' ? 1 : 0);
      return;
    }
    let next = done.run ?? null;
    if (!next && before) next = await api.getBattleRun(token, before.id);
    paid = next?.paidDust ?? 0;
    report = next && before
      ? before.squad.flatMap((was) => {
          const now = next.squad.find((c) => c.cardId === was.cardId);
          if (!now || was.dead) return [];
          const lost = Math.max(0, was.health - now.health);
          return lost > 0 || now.dead ? [{ name: nameOf(was.cardId) || was.slug, lost, fell: now.dead }] : [];
        })
      : null;
    lostWhy = [];
    if (before && next && next.state === 'lost') {
      const w = before.nextWave;
      const fielded = before.squad.filter((c) => !c.dead && chosen.includes(c.cardId));
      const wounded = fielded.filter((c) => c.health * 2 < c.maxHealth).length;
      if (w) lostWhy.push(f('runLostGoal', { goal: goalLine(w.goal) }));
      if (w && fielded.length < w.maxCards) lostWhy.push(f('runLostFewCards', { n: fielded.length, max: w.maxCards }));
      if (wounded > 0) lostWhy.push(f('runLostWounded', { n: wounded }));
    }
    take(next);
    if (before) {
      const won = done.outcome === 'player';
      interact('battle_wave_finish', `${before.route.slug}:${before.waveNo}`, won ? 1 : 0);
      if (next && next.state !== 'active') {
        interact('battle_run_end', `${before.route.slug}:${next.state}`, next.waveNo);
      }
    }
  }

  async function foresee(action: BattleAction): Promise<Foresight | null> {
    if (!token || !match) return null;
    try {
      return await api.foreseeBattleMatch(token, match.id, match.seq, action);
    } catch {
      return null;
    }
  }

  const toSheet = async () => {
    autoStop();
    match = null;
    void readMe();
    await tick();
    window.scrollTo({ top: 0 });
  };

  const abandon = () => {
    if (!abandonAsked) {
      abandonAsked = true;
      abandonTimer = setTimeout(() => (abandonAsked = false), 4000);
      return;
    }
    abandonAsked = false;
    practicing = false;
    if (abandonTimer) clearTimeout(abandonTimer);
    // Автобой останавливаем и ждём, пока уйдёт его запрос: иначе `guarded` молча
    // отбросил бы нажатие.
    autoStop();
    return (async () => {
      for (let i = 0; busy && i < 50; i++) await pause(100);
      await guarded(async () => {
        if (!run || !token) return;
        const before = run;
        take(await api.abandonBattleRun(token, before.id));
        match = null;
        interact('battle_run_end', `${before.route.slug}:abandoned`, before.waveNo);
      });
    })();
  };

  const pickReward = (index: number) =>
    guarded(async () => {
      if (!run || !token) return;
      take(await api.pickBattleRunReward(token, run.id, index));
      void readMe();
    });

  const swap = (cardId: string) =>
    guarded(async () => {
      if (!run || !token) return;
      take(await api.swapBattleRunCard(token, run.id, cardId, run.swaps));
      void readMe();
      interact('battle_swap', run?.route.slug ?? null, run?.swaps ?? null);
    });

  const heal = (cardId: string) =>
    guarded(async () => {
      if (!run || !token) return;
      const name = nameOf(cardId);
      take(await api.healBattleRunCard(token, run.id, cardId));
      notice = f('runHealed', { name });
      interact('battle_heal', run?.route.slug ?? null, run?.waveNo ?? null);
    });

  // Меню быстрых действий над картой отряда: нажатие по карте в ряду внизу.
  let menu = $state<{ cardId: string; at: MenuAt } | null>(null);
  function openMenu(e: Event, cardId: string) {
    const r = (e.currentTarget as Element).getBoundingClientRect();
    const w = Math.min(272, window.innerWidth - 16);
    const left = Math.max(8, Math.min(r.left + r.width / 2 - w / 2, window.innerWidth - w - 8));
    // Указатель смотрит на середину карты, но не выходит за скруглённые углы меню.
    const arrow = Math.max(22, Math.min(r.left + r.width / 2 - left, w - 22));
    const above = r.top > 330;
    menu = { cardId, at: above ? { left, bottom: window.innerHeight - r.top + 12, arrow, above } : { left, top: r.bottom + 12, arrow, above } };
    inspected = cardId;
    inspectedFoe = null;
    sfx.lift();
  }
  const menuItems = $derived.by((): MenuItem[] => {
    const r = run;
    const id = menu?.cardId;
    const c = r?.squad.find((x) => x.cardId === id);
    if (!r || !c) return [];
    const out: MenuItem[] = [];
    if (r.healCharges > 0 && !c.dead && c.health < c.maxHealth) {
      out.push({ key: 'heal', label: $t('runMenuHeal'), note: f('runMenuHealNote', { n: r.healCharges }), onclick: () => void heal(c.cardId) });
    }
    if (!c.dead && r.nextWave) {
      const on = chosen.includes(c.cardId);
      const full = !on && chosen.length >= r.nextWave.maxCards;
      out.push({
        label: on ? $t('runPlaceRemove') : $t('runMenuField'),
        key: on ? 'unfield' : 'field',
        note: full ? f('runMenuFull', { max: r.nextWave.maxCards }) : undefined,
        disabled: full,
        onclick: () => toggle(c.cardId),
      });
    }
    if (r.swapCost != null) {
      const free = r.freeSwaps > 0;
      const none = r.bench <= 0;
      const poor = !free && !!me && me.dust < r.swapCost;
      out.push({
        key: 'swap',
        label: free ? $t('runSwapFree') : $t('runMenuSwap'),
        price: free ? undefined : r.swapCost,
        note: none ? $t('runMenuNoBench') : poor ? $t('runFaultNotEnough') : $t('runMenuSwapNote'),
        disabled: none || poor,
        onclick: () => void swap(c.cardId),
      });
    }
    const face = cardOf(c.cardId);
    if (face) {
      out.push({ key: 'details', label: $t('runMenuDetails'), onclick: () => peekOpen(face, f('runHealth', { h: c.health, m: c.maxHealth })) });
    }
    return out;
  });

  const takeTrophy = (cardId: string) =>
    guarded(async () => {
      if (!run || !token) return;
      const seats = run.squad.length;
      const next = await api.takeBattleRunTrophy(token, run.id, cardId);
      const name = nameOf(cardId);
      const keep = chosen;
      take(next);
      // Выбор выходящих карт не сбрасываем: новая карта встаёт рядом, а не вместо.
      if (next.nextWave && next.squad.length > seats) {
        chosen = keep.filter((id) => next.squad.some((c) => c.cardId === id && !c.dead));
        notice = f('runTrophyJoined', { name });
        panel = null;
      } else {
        notice = f('runTrophyShelved', { name });
      }
      void readMe();
      interact('battle_trophy', run?.route.slug ?? null);
    });

  /** Снять карту с выхода в волну: она остаётся в отряде, просто не выходит. */
  function unfield(cardId: string) {
    if (!chosen.includes(cardId)) return;
    chosen = chosen.filter((id) => id !== cardId);
    inspected = null;
    sfx.lift();
  }
  /** Тащат свою карту с поля: ряд карт внизу становится местом, куда её отпустить. */
  let dragging = $state(false);
  function dropOnHand(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    const id = e.dataTransfer?.getData('application/x-gotiga-card');
    if (id) unfield(id);
  }

  function toggle(cardId: string) {
    if (!run?.nextWave) return;
    if (chosen.includes(cardId)) {
      chosen = chosen.filter((id) => id !== cardId);
    } else if (chosen.length < run.nextWave.maxCards) {
      chosen = [...chosen, cardId];
    } else {
      notice = f('runSquadFull', { max: run.nextWave.maxCards });
    }
  }

  // Клетки следуют за выбором: своя прежняя, запомненная с прошлой волны или
  // первая свободная в порядке сервера. Сама правка клеток этот ход не трогает.
  $effect(() => {
    const field = run?.nextWave?.field;
    const ids = chosen;
    if (!run || run.phase !== 'wave' || !field) return;
    const remembered = Object.fromEntries(run.squad.map((c) => [c.cardId, c.cell]));
    const current = untrack(() => $state.snapshot(cells)) as Placement;
    cells = reconcile(ids, current, remembered, field.openCells);
  });

  const resetCells = () => {
    const field = run?.nextWave?.field;
    if (!field) return;
    const health = Object.fromEntries(run!.squad.map((c) => [c.cardId, c.health / Math.max(c.maxHealth, 1)]));
    const danger = threatened(field.threats);
    cells = safeLayout(chosen, field.openCells, danger, health);
    notice = field.openCells.every((c) => danger.has(cellKey(c))) ? $t('runPlaceNoSafe') : $t('runPlaceResetDone');
  };

  /** Каждой выходящей карте есть клетка, если поле известно. */
  const placedOk = $derived(!run?.nextWave?.field || chosen.every((id) => !!cells[id]));

  /** Карта, которую игрок нажал последней: подробности справа от поля. */
  let inspected = $state<string | null>(null);
  /** Или вражеская карта на поле (слаг): нажатая последней из двух побеждает. */
  let inspectedFoe = $state<string | null>(null);
  /** Раунд, в котором выйдет нажатый враг-подкрепление; у стоящих с начала — пусто. */
  let inspectedRound = $state<number | null>(null);
  const shown = $derived.by(() => {
    if (!run) return null;
    const foe = inspectedFoe ? cards.find((c) => c.slug === inspectedFoe) : undefined;
    if (foe) return { foe: true, id: null as string | null, card: foe, health: foe.health, maxHealth: foe.health, round: inspectedRound };
    const id = inspected && run.squad.some((c) => c.cardId === inspected) ? inspected : (chosen[0] ?? null);
    const squad = run.squad.find((c) => c.cardId === id);
    const card = id ? cardOf(id) : undefined;
    return squad && card ? { foe: false, id, card, health: squad.health, maxHealth: squad.maxHealth, round: null } : null;
  });

  /** Меньше этого числа карт в волну не выйти (меньше, если живых осталось меньше). */
  const needCards = $derived(run?.nextWave ? Math.min(run.nextWave.minCards, run.squad.filter((c) => !c.dead).length) : 0);

  const fieldedOk = $derived(
    !!run?.nextWave && chosen.length >= needCards && chosen.length <= run.nextWave.maxCards && placedOk,
  );

  /** Почему «Начать волну» не нажать: словами под кнопкой, а не молчаливым серым. */
  const whyNot = $derived(
    !run?.nextWave ? '' : chosen.length < needCards ? f('runWhyFew', { n: needCards }) : !placedOk ? $t('runWhyNoCell') : '',
  );
</script>

<svelte:head>
  <title>{$t('runTitle')}</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (panel = null)} />

<div class="root" class:root--match={!!match} class:root--stage={stageOn}>
  {#if complaint}
    <p class="complaint" role="alert">
      <span>{complaint}</span>
      <button type="button" class="x" aria-label={$t('runClose')} onclick={() => (complaint = null)}>×</button>
    </p>
  {/if}
  {#if notice}
    <p class="notice" role="status">
      <span>{notice}</span>
      <button type="button" class="x" aria-label={$t('runClose')} onclick={() => (notice = null)}>×</button>
    </p>
  {/if}
  {#if match}
    <BattleScene
      {match}
      {cards}
      {frames}
      {motions}
      {busy}
      fill
      onact={play}
      onforesee={foresee}
      onnext={toSheet}
      doorLabels={{
        next: run?.state === 'active' ? $t('runContinue') : $t('runTitle'),
        leave: $t('runTitle'),
        nextAlways: true,
      }}
      guestLoginHref={!signedIn ? loginHref : undefined}
    />
    {#if !match.outcome}
      <div class="autobar">
        {#if auto}
          <button type="button" class="autobar-stop" onclick={autoStop}>{$t('runAutoStop')}</button>
        {:else}
          <button type="button" onclick={autoPlay} disabled={busy}>{$t('runAuto')}</button>
        {/if}
        <button type="button" class="autobar-quiet" onclick={abandon}>
          {abandonAsked ? $t('runAbandonConfirm') : $t('runAbandon')}
        </button>
      </div>
    {/if}
  {:else if stageOn && run}
    {@render stage(run)}
  {:else}
    <main class="sheet">
      <BattleDoor {me}>
        <BattleModes current="run" runOpen={true} base="/battles/etude" />
      </BattleDoor>
      <h1>{$t('runTitle')}</h1>

      {#if !ready}
        <p class="note">…</p>
      {:else}
        {#if run}
          {@const kind = run.state !== 'won' ? 'lost' : run.wavesTotal < run.route.waves ? 'trial' : 'won'}
          <section
            class="finale"
            aria-labelledby="finale-h"
            style:--tone={kind === 'lost' ? '#8f2f22' : kind === 'trial' ? '#b08a3a' : '#4a6b3a'}
          >
            <span class="seal seal--big" aria-hidden="true">
              <svg viewBox="0 0 24 24" width="40" height="40" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                {#if kind === 'lost'}
                  <path d="M6 21V4M6 5h11l-3 4 3 4H6" />
                {:else}
                  <path d="M5 12.5l4.5 4.5L19 7.5" />
                {/if}
              </svg>
            </span>
            <div class="finale-body">
              <p class="kicker">{titleOf(run.route)}</p>
              <h2 id="finale-h">
                {kind === 'lost' ? $t('runLost') : kind === 'trial' ? $t('runTrialDone') : $t('runWon')}
              </h2>
              <div class="head">
                {@render stepsBar(run, true)}
                <span class="small">{f('runWaveOf', { n: run.waveNo, total: run.wavesTotal })}</span>
              </div>
              {#if paid > 0}<p class="dust-chip">{f('runPaid', { n: paid })}</p>{/if}
              {#if kind === 'lost' && lostWhy.length}
                <div class="report">
                  <b>{$t('runLostWhyTitle')}</b>
                  <ul>{#each lostWhy as why (why)}<li>{why}</li>{/each}</ul>
                </div>
              {/if}
              {#if kind === 'lost'}{@render lossReport()}{/if}
              {@render waveLogList()}
              {#if kind === 'lost' && run.practice}
                <button type="button" class="act" disabled={busy} onclick={startPractice}>{$t('runPractice')}</button>
                <p class="small">{$t('runPracticeAbout')}</p>
              {/if}
              {#if !signedIn}
                <p class="note">{$t('runGuestNote')}</p>
                <a class="act link" href={loginHref}>{$t('battleGuestSealLink')}</a>
              {/if}
              {#if signedIn || routes.some((r) => r.guestWaves != null)}
                <div class="finale-cta">
                  {#if routes.length === 1}
                    <button type="button" class="act" disabled={busy} onclick={() => start(routes[0])}>{$t('runAgain')}</button>
                  {:else}
                    <a class="act link" href="#routes">{$t('runNewRun')}</a>
                  {/if}
                </div>
              {/if}
            </div>
          </section>
          {@render trophies(run)}
          {#if routes.length > 1}<h2 id="routes">{$t('runNewRun')}</h2>{/if}
        {:else}
          <p class="lead">{$t('runIntro')}</p>
          <ol class="how">
            {#each [['squad', 'runSquad', 'runHowSquadText', '#6f3b24'], ['waves', 'runHowWaves', 'runHowWavesText', '#8f2f22'], ['reward', 'runHowReward', 'runHowRewardText', '#b08a3a'], ['trophy', 'runTrophies', 'runHowTrophyText', '#4a6b3a']] as [icon, title, text, tone], i (icon)}
              <li style:--tone={tone} style:--i={i}>
                <span class="seal seal--sm" aria-hidden="true">{@render glyph(icon)}</span>
                <span class="how-title">{$t(title as TranslationKey)}</span>
                <span class="small">{$t(text as TranslationKey)}</span>
              </li>
            {/each}
          </ol>
        {/if}

        {#if daily}
          <section class="daily" aria-labelledby="daily-h">
            <div class="daily-main">
              <p class="kicker">{$t('runDaily')}</p>
              <h2 id="daily-h">{titleOf(daily.route)}</h2>
              <p class="small">{$t('runDailyAbout')}</p>
              <p class="small">{f('runDailyStats', { a: daily.started, b: daily.won })}</p>
              {#if daily.mine}
                <p class="small daily-mine">
                  {daily.mine.state === 'won' ? $t('runDailyWon') : f('runDailyMine', { n: daily.mine.waveNo, total: daily.mine.wavesTotal })}
                </p>
              {/if}
            </div>
            {#if !daily.mine}
              <button type="button" class="act" disabled={busy} onclick={startDaily}>{$t('runDailyPlay')}</button>
            {/if}
          </section>
        {/if}

        <!-- Маршрут один — кнопка «Начать заново» уже в баннере итога. -->
        {#if !(run && routes.length === 1 && (signedIn || routes[0].guestWaves != null))}
        {#if routes.length === 0}
          <p class="note">{$t('runNone')}</p>
        {/if}
        <ul class="routes">
          {#each routes as route (route.slug)}
            <li>
              <div class="route-main">
                <span class="route-name">{titleOf(route)}</span>
                <ol class="steps" aria-hidden="true">
                  {#each Array.from({ length: route.waves }, (_, i) => i + 1) as n (n)}
                    <li class:free={!signedIn && route.guestWaves != null && n <= route.guestWaves}></li>
                  {/each}
                </ol>
                <span class="small">
                  {f('runRouteWaves', { n: route.waves })}
                  {#if !signedIn && route.guestWaves != null} · {f('runRouteGuest', { n: route.guestWaves })}{/if}
                </span>
              </div>
              {#if signedIn || route.guestWaves != null}
                <button type="button" class="act" disabled={busy} onclick={() => start(route)}>
                  {run ? $t('runAgain') : $t('runStart')}
                </button>
              {/if}
            </li>
          {/each}
        </ul>
        {/if}
      {/if}
    </main>
  {/if}
</div>

{#if banner}
  <div class="banner" role="status"><span>{f('runWaveOf', { n: banner.n, total: banner.total })}</span></div>
{/if}

{#snippet waveLogList()}
  {#if waveLog.length}
    <div class="wave-log">
      <b>{$t('runLog')}</b>
      <ul>
        {#each waveLog as w (w.matchId)}
          <li>
            <span class="wl-name">{f('runLogWave', { n: w.waveNo })}{w.practice ? ` · ${$t('runLogPractice')}` : ''}</span>
            <span class="wl-out" class:wl-out--won={w.outcome === 'player'}>{w.outcome === 'player' ? $t('runLogWon') : $t('runLogLost')}</span>
            <button type="button" class="quiet" onclick={() => (replayOf = w)}>{$t('runLogWatch')}</button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
{/snippet}

{#snippet lossReport()}
  {#if report}
    <div class="report">
      <b>{$t('runReportTitle')}</b>
      {#if report.length}
        <ul>
          {#each report as x (x.name)}
            <li class:fell={x.fell}>{x.fell ? `${x.name}: ${$t('runFallen')}` : f('runReportHurt', { name: x.name, n: x.lost })}</li>
          {/each}
        </ul>
      {:else}
        <span>{$t('runReportNone')}</span>
      {/if}
    </div>
  {/if}
{/snippet}

{#snippet stage(r: BattleRun)}
  {@const got = (tr: RunTrophy) => tr.taken || tr.owned}
  {@const takeable = r.trophies.filter((tr) => !got(tr) && tr.priceDust > 0 && tr.onShelf).length}
  <div class="stage">
    <h1 class="sr">{$t('runTitle')}</h1>
    <header class="topbar">
      <a class="back" href="/battles">‹ {$t('battlesRoomShelf')}</a>
      <div class="route">
        <span class="route-title">{titleOf(r.route)}</span>
        {@render stepsBar(r, false)}
      </div>
      <div class="tools">
        {#if me}<span class="dust" title={$t('battlesYourDust')}><b>{shownDust}</b> {$t('battleGreetDust')}</span>{/if}
        <button type="button" class="tool" class:tool--on={sound} aria-pressed={sound} onclick={toggleSound}>
          {$t('runSound')}
        </button>
        <button type="button" class="tool" class:tool--on={panel === 'squad'} aria-expanded={panel === 'squad'} onclick={() => (panel = panel === 'squad' ? null : 'squad')}>
          {$t('runSquad')}
        </button>
        <button type="button" class="tool" class:tool--on={panel === 'loot'} aria-expanded={panel === 'loot'} onclick={() => (panel = panel === 'loot' ? null : 'loot')}>
          {$t('runLoot')}
          {#if takeable > 0}<span class="badge badge--gold" title={$t('runTrophies')}>{f('runBadgeTrophies', { n: takeable })}</span>{/if}
          {#if r.freeSwaps > 0}<span class="badge" title={$t('runSwapFree')}>{f('runBadgeSwaps', { n: r.freeSwaps })}</span>{/if}
        </button>
        {#if practicing}
          <button type="button" class="tool tool--quiet" disabled={busy} onclick={endPractice}>{$t('runPracticeExit')}</button>
        {:else}
          <button type="button" class="tool tool--quiet" disabled={busy} onclick={abandon}>
            {abandonAsked ? $t('runAbandonConfirm') : $t('runAbandon')}
          </button>
        {/if}
      </div>
    </header>
    {#if !signedIn}
      <p class="guestbar">
        {$t('runGuestNote')}
        <a href={loginHref}>{$t('battleGuestSealLink')}</a>
      </p>
    {/if}

    {#if r.phase === 'reward'}
      <section class="reward" aria-labelledby="reward-h">
        <p class="kicker">{f('runCleared', { n: Math.max(r.waveNo - 1, 1) })}</p>
        <h2 id="reward-h">{$t('runPickReward')}</h2>
        {#if paid > 0}<p class="dust-chip">{f('runPaid', { n: paid })}</p>{/if}
        {@render lossReport()}
        <ul class="prizes">
          {#each r.reward as rw, i (i)}
            {@const info = PRIZE[rw.kind]}
            {@const detail = prizeDetail(rw, r)}
            <li style:--tone={info.tone} style:--i={i}>
              <button type="button" class="prize" disabled={busy} onclick={() => pickReward(i)}>
                <span class="seal" aria-hidden="true">{@render glyph(rw.kind)}</span>
                <span class="prize-title">{$t(info.title)}</span>
                {#if rw.kind === 'dust' || rw.kind === 'healAll'}
                  <span class="prize-big">+{rw.amount}</span>
                {/if}
                {#if rw.kind === 'dust'}
                  <span class="prize-text">{$t('runPrizeDustAbout')}</span>
                {:else if rw.kind === 'healAll'}
                  <span class="prize-text">{$t('runPrizeHealAllAbout')}</span>
                {:else if rw.kind !== 'freeSwap' || !detail.length}
                  <span class="prize-text">{rewardLine(rw)}</span>
                {/if}
                {#if detail.length}
                  <span class="prize-detail">
                    {#each detail as row (row)}<span>{row}</span>{/each}
                  </span>
                {/if}
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {:else if r.nextWave}
      {@const w = r.nextWave}
      <section class="wavebar" aria-label={$t('runNextWave')}>
        <span class="wave-no" aria-hidden="true"><b>{w.waveNo}</b><span>/ {r.wavesTotal}</span></span>
        <span class="goal">
          <span class="goal-icon" aria-hidden="true">
            {@render glyph(!w.goal ? 'all' : w.goal.kind === 'holdRounds' ? 'hold' : w.goal.kind === 'defendStructure' ? 'defend' : 'kill')}
          </span>
          {goalLine(w.goal)}
        </span>
        <span class="chips">
          <span class="chip">{f('runCardsRange', { min: w.minCards, max: w.maxCards })}</span>
          {#if r.healCharges > 0}<span class="chip chip--heal">{f('runHealCharges', { n: r.healCharges })}</span>{/if}
          {#if w.raceHint && raceName(w.raceHint)}<span class="chip">{f('runRaceHint', { race: raceName(w.raceHint) })}</span>{/if}
          {#if signedIn && w.rewardDust > 0}<span class="chip chip--gold">+{f('runRewardDust', { n: w.rewardDust })}</span>{/if}
        </span>
      </section>

      {#if practicing}
        <p class="tip" role="note"><span>{$t('runPracticeAbout')}</span></p>
      {:else if w.waveNo === 1 && !tipsShut}
        <p class="tip" role="note">
          <span>{$t('runTipText')}</span>
          <button type="button" class="tip-ok" onclick={shutTips}>{$t('runTipGot')}</button>
        </p>
      {/if}

      <div class="field">
        <div class="field-main">
          {#if w.field}
            <RunPlacement compact quiet={practicing || (w.waveNo === 1 && !tipsShut)} field={w.field} {chosen} bind:cells {cards} onreset={resetCells} onremove={unfield} ondrag={(on) => (dragging = on)} onpick={(id) => id && ((inspected = id), (inspectedFoe = null))} onlook={(slug, round) => ((inspectedFoe = slug), (inspectedRound = round ?? null), interact('battle_run_look', slug, round ?? null))} />
          {:else}
            <p class="note">{$t('runPickWho')}</p>
          {/if}
        </div>
        {#if shown}
          {@const sc = shown.card}
          <aside class="inspect" class:inspect--foe={shown.foe} aria-label={shown.foe ? $t('runFoeNow') : $t('runCardNow')}>
            <span class="kicker">{shown.foe ? $t('runFoeNow') : $t('runCardNow')}</span>
            <h2>{cardCopy(sc, $lang).title}</h2>
            {#if shown.id && chosen.includes(shown.id)}
              <button type="button" class="quiet" onclick={() => unfield(shown.id!)}>{$t('runPlaceRemove')}</button>
            {/if}
            <span class="face" aria-hidden="true">
              <BattleCardFace card={sc} {frames} owned={true} uniform transition={false} interactive={false} />
            </span>
            <dl class="stats">
              <div><dt>{$t('battlesPowerLabel')}</dt><dd>{sc.power}</dd></div>
              <div><dt>{$t('battleStatArmour')}</dt><dd>{sc.armor}</dd></div>
              <div><dt>{$t('battleStatWard')}</dt><dd>{sc.ward}</dd></div>
              <div><dt>{$t('battleStatReach')}</dt><dd>{sc.reach}</dd></div>
            </dl>
            <p class="small">{f('runHealth', { h: shown.health, m: shown.maxHealth })}</p>
            {#if shown.round}<p class="effect effect--later">{f('runPlaceEntrance', { n: shown.round })}</p>{/if}
            {#if cardCopy(sc, $lang).effect}<p class="effect">{cardCopy(sc, $lang).effect}</p>{/if}
          </aside>
        {/if}
      </div>

      {#if shown}
        {@const sc = shown.card}
        <p class="inspect-line" class:inspect-line--foe={shown.foe}>
          <b>{cardCopy(sc, $lang).title}</b>
          {$t('battlesPowerLabel')} {sc.power} · {$t('battleStatArmour')} {sc.armor} · {$t('battleStatWard')} {sc.ward} · {$t('battleStatReach')} {sc.reach} · {f('runHealth', { h: shown.health, m: shown.maxHealth })}
          {#if shown.round}<br />{f('runPlaceEntrance', { n: shown.round })}{/if}
        </p>
      {/if}

      <footer class="hand" class:hand--drop={dragging} ondragover={(e) => dragging && e.preventDefault()} ondrop={dropOnHand}>
        {#if dragging}<p class="drop-note" role="status">{$t('runDropHere')}</p>{/if}
        {#if chosen.length}
          <div class="dropzone" class:dropzone--hot={dragging} aria-hidden="true">
            <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v12M6 12l6 6 6-6" /></svg>
            <b>{$t('runDropZone')}</b>
            <span>{$t('runDropZoneHow')}</span>
          </div>
        {/if}
        <ul class="hand-cards" aria-label={$t('runPickWho')}>
          {#each r.squad as c (c.cardId)}
            {@const face = cardOf(c.cardId)}
            {@const on = chosen.includes(c.cardId)}
            {@const pct = Math.max(0, Math.min(100, Math.round((c.health / Math.max(c.maxHealth, 1)) * 100)))}
            <li class:dead={c.dead} class:on>
              <button
                type="button"
                class="hcard"
                aria-haspopup="menu"
                aria-pressed={c.dead ? undefined : on}
                aria-label={`${nameOf(c.cardId) || c.slug}, ${f('runHealth', { h: c.health, m: c.maxHealth })}`}
                title={nameOf(c.cardId) || c.slug}
                onclick={(e) => openMenu(e, c.cardId)}
              >
                {#if face}
                  <span class="face">
                    <BattleCardFace card={face} {frames} owned={true} uniform transition={false} interactive={false} />
                  </span>
                {:else}
                  <span class="card-name">{nameOf(c.cardId) || c.slug}</span>
                {/if}
                {#if on}<span class="tick" aria-hidden="true">✓</span>{/if}
              </button>
              {#if c.dead}
                <span class="hp-num">{$t('runFallen')}</span>
              {:else}
                <span class="hp-bar" aria-hidden="true"><i style:width="{pct}%" class:low={pct <= 34} class:mid={pct > 34 && pct <= 66}></i></span>
                <span class="hp-num" class:low={pct <= 34} class:mid={pct > 34 && pct <= 66} aria-hidden="true"><svg class="heart" viewBox="0 0 24 24" width="12" height="12" aria-hidden="true"><path d="M12 21s-7.5-4.6-9.6-9.2C.9 8.4 3 5 6.4 5c2 0 3.6 1.1 4.6 2.7h2C14 6.1 15.6 5 17.6 5 21 5 23.1 8.4 21.6 11.8 19.5 16.4 12 21 12 21z" fill="currentColor" /></svg><b>{c.health}</b><span>/{c.maxHealth}</span></span>
              {/if}
            </li>
          {/each}
        </ul>
        <p class="hand-note">
          {$t('runHandHealth')}<br />
          {#if chosen.length}{$t('runRemoveHow')}<span class="drag-how"> {$t('runRemoveDrag')}</span>{/if}
        </p>
        <div class="go">
          <span class="small">{f('runChosen', { n: chosen.length, max: w.maxCards })}</span>
          {#if whyNot}<span class="small why" id="why-not" role="status">{whyNot}</span>{/if}
          <button type="button" class="act" disabled={busy || !fieldedOk} aria-describedby={whyNot ? 'why-not' : undefined} onclick={beginWave}>{$t('runBeginWave')}</button>
        </div>
      </footer>
    {/if}

    {#if panel}
      <button type="button" class="scrim" aria-label={$t('runClose')} onclick={() => (panel = null)}></button>
      <aside class="panel" aria-label={panel === 'squad' ? $t('runSquad') : $t('runLoot')}>
        <span class="grab" aria-hidden="true"></span>
        <button type="button" class="panel-x" aria-label={$t('runClose')} onclick={() => (panel = null)}>×</button>
        {#if panel === 'squad'}
          <h2>{$t('runSquad')}</h2>
          {@render squadGrid(r, false)}
        {:else}
          <h2>{$t('runLoot')}</h2>
          <h3 class="sub">{$t('runLootRewards')}</h3>
          {#if r.picks.length}
            <ul class="picks">
              {#each r.picks as pk (pk.waveNo)}
                {@const info = PRIZE[pk.kind]}
                <li style:--tone={info.tone}>
                  <span class="seal seal--xs" aria-hidden="true">{@render glyph(pk.kind)}</span>
                  <span>{$t(info.title)}{pk.kind === 'dust' || pk.kind === 'healAll' ? ` +${pk.amount}` : ''}</span>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="small">{$t('runLootNone')}</p>
          {/if}
          {#if r.freeSwaps > 0}<p class="chip chip--gold chip--inline">{f('runFreeSwaps', { n: r.freeSwaps })}</p>{/if}
          {@render waveLogList()}
          {@render trophies(r)}
        {/if}
      </aside>
    {/if}
  </div>
{/snippet}

{#snippet squadGrid(r: BattleRun, pick: boolean)}
  <ul class="grid">
    {#each r.squad as c (c.cardId)}
      {@const face = cardOf(c.cardId)}
      {@const on = pick && chosen.includes(c.cardId)}
      <li class:dead={c.dead} class:on>
        <button
          type="button"
          class="pick"
          class:pick--look={!pick}
          disabled={pick && c.dead}
          aria-pressed={pick && !c.dead ? on : undefined}
          aria-label={nameOf(c.cardId) || c.slug}
          onmouseenter={(e) => peekHover(e, face, f('runHealth', { h: c.health, m: c.maxHealth }))}
          onmouseleave={peekLeave}
          onclick={() => (pick ? toggle(c.cardId) : peekOpen(face, f('runHealth', { h: c.health, m: c.maxHealth })))}
        >
          {#if face}
            <span class="face">
              <BattleCardFace card={face} {frames} owned={true} uniform transition={false} interactive={false} />
            </span>
          {:else}
            <span class="card-name">{nameOf(c.cardId) || c.slug}</span>
          {/if}
          {#if on}<span class="tick" aria-hidden="true">✓</span>{/if}
        </button>
        <span class="card-name">{nameOf(c.cardId) || c.slug}</span>
        {#if c.dead}
          <span class="small">{$t('runFallen')}</span>
        {:else}
          {@const pct = Math.max(0, Math.min(100, Math.round((c.health / Math.max(c.maxHealth, 1)) * 100)))}
          <span class="hp" role="img" aria-label={f('runHealth', { h: c.health, m: c.maxHealth })}>
            <span class="hp-bar"><i style:width="{pct}%" class:low={pct <= 34} class:mid={pct > 34 && pct <= 66}></i></span>
            <span class="hp-num" class:low={pct <= 34} class:mid={pct > 34 && pct <= 66}><svg class="heart" viewBox="0 0 24 24" width="12" height="12" aria-hidden="true"><path d="M12 21s-7.5-4.6-9.6-9.2C.9 8.4 3 5 6.4 5c2 0 3.6 1.1 4.6 2.7h2C14 6.1 15.6 5 17.6 5 21 5 23.1 8.4 21.6 11.8 19.5 16.4 12 21 12 21z" fill="currentColor" /></svg><b>{c.health}</b><span>/{c.maxHealth}</span></span>
          </span>
        {/if}
        {#if c.loaned && r.squad.some((x) => !x.loaned)}<span class="chip chip--tag">{$t('runLoaned')}</span>{/if}
        {#if r.healCharges > 0 && !c.dead && c.health < c.maxHealth}
          <button type="button" class="quiet quiet--heal" disabled={busy} onclick={() => heal(c.cardId)}>{$t('runMenuHeal')}</button>
        {/if}
        {#if r.swapCost != null && r.bench > 0}
          <button type="button" class="quiet" disabled={busy} onclick={() => swap(c.cardId)}>
            {r.freeSwaps > 0 ? $t('runSwapFree') : f('runSwapDo', { n: r.swapCost })}
          </button>
        {/if}
      </li>
    {/each}
  </ul>
  {#if r.swapCost != null}<p class="small">{r.bench > 0 ? $t('runSwapAbout') : $t('runSwapNoBench')}</p>{/if}
{/snippet}

{#snippet glyph(name: string)}
  <svg viewBox="0 0 24 24" width="26" height="26" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    {#if name === 'squad'}
      <rect x="3" y="7" width="10" height="14" rx="1" /><rect x="9" y="3" width="10" height="14" rx="1" />
    {:else if name === 'waves'}
      <path d="M3 9c3-3 5 3 9 0s5-3 9 0M3 15c3-3 5 3 9 0s5-3 9 0" />
    {:else if name === 'reward'}
      <path d="M12 3l2.6 5.6 6.1.7-4.5 4.2 1.2 6L12 16.5 6.6 19.5l1.2-6L3.3 9.3l6.1-.7z" />
    {:else if name === 'trophy'}
      <path d="M8 4h8v5a4 4 0 0 1-8 0zM8 6H5v1a3 3 0 0 0 3 3M16 6h3v1a3 3 0 0 1-3 3M12 13v4M8 20h8" />
    {:else if name === 'dust'}
      <circle cx="8" cy="8" r="1.5" fill="currentColor" /><circle cx="16" cy="6" r="1.2" fill="currentColor" /><circle cx="12" cy="13" r="1.8" fill="currentColor" /><circle cx="6" cy="17" r="1.2" fill="currentColor" /><circle cx="18" cy="16" r="1.5" fill="currentColor" />
    {:else if name === 'healAll'}
      <path d="M12 20s-7-4.4-8.8-8.6C1.9 8.3 3.6 5 6.8 5c1.9 0 3.3 1 5.2 2.8C13.9 6 15.3 5 17.2 5c3.2 0 4.9 3.3 3.6 6.4C19 15.6 12 20 12 20z" /><path d="M12 10v5M9.5 12.5h5" />
    {:else if name === 'healOne'}
      <path d="M12 21s-8-5-10-10C.7 7.5 3 4 6.5 4c2 0 3.7 1 5.5 3 1.8-2 3.5-3 5.5-3C21 4 23.3 7.5 22 11c-2 5-10 10-10 10z" />
    {:else if name === 'revive'}
      <path d="M12 3c.8 3.2 5 5.2 5 10a5 5 0 0 1-10 0c0-2.2 1-3.4 2.2-4.4.2 1.8 1 2.8 2 3C10.5 8.5 10.7 5.5 12 3z" />
    {:else if name === 'freeSwap'}
      <path d="M4 8h13l-3-3M20 16H7l3 3" />
    {:else if name === 'all'}
      <path d="M5 19L17 7M14 5h5v5M19 19L7 7M10 5H5v5" />
    {:else if name === 'hold'}
      <path d="M7 4h10M7 20h10M8 4c0 5 4 5 4 8s-4 3-4 8M16 4c0 5-4 5-4 8s4 3 4 8" />
    {:else if name === 'defend'}
      <path d="M12 3l7 3v6c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9V6z" />
    {:else}
      <circle cx="12" cy="12" r="8" /><circle cx="12" cy="12" r="3" /><path d="M12 2v4M12 18v4M2 12h4M18 12h4" />
    {/if}
  </svg>
{/snippet}

{#snippet stepsBar(r: BattleRun, ended: boolean)}
  <ol class="steps" aria-label={f('runWaveOf', { n: r.waveNo, total: r.wavesTotal })}>
    {#each Array.from({ length: r.wavesTotal }, (_, i) => i + 1) as n (n)}
      <li
        class:done={ended ? r.state === 'won' || n < r.waveNo : n < r.waveNo}
        class:now={!ended && n === r.waveNo}
        class:fail={ended && r.state !== 'won' && n === r.waveNo}
      ></li>
    {/each}
  </ol>
{/snippet}

{#snippet trophies(r: BattleRun)}
  {@const done = (t: RunTrophy) => t.taken || t.owned}
  {@const open = r.trophies.filter((t) => !done(t) && t.priceDust > 0 && t.onShelf)}
  {@const had = r.trophies.filter(done)}
  {@const closed = r.trophies.filter((t) => !done(t) && !(t.priceDust > 0 && t.onShelf))}
  {#if open.length || had.length}
    <section aria-labelledby="trophy-h">
      <h2 id="trophy-h">{$t('runTrophies')}</h2>
      <p class="small">{$t('runTrophyAbout')}</p>
      <ul class="grid">
        {#each [...open, ...had] as tr (tr.cardId)}
          {@const face = cardOf(tr.cardId)}
          <li class:taken={done(tr)}>
            <button
              type="button"
              class="trophy-face trophy-face--look"
              aria-label={nameOf(tr.cardId) || tr.slug}
              onmouseenter={(e) => peekHover(e, face, f('runTrophyTake', { n: tr.priceDust }))}
              onmouseleave={peekLeave}
              onclick={() => peekOpen(face, done(tr) ? $t('runTrophyTaken') : f('runTrophyTake', { n: tr.priceDust }))}
            >
              {#if face}
                <span class="face">
                  <BattleCardFace card={face} {frames} owned={true} uniform transition={false} interactive={false} />
                </span>
              {/if}
              {#if done(tr)}<span class="tick" aria-hidden="true">✓</span>{/if}
            </button>
            <span class="card-name">{nameOf(tr.cardId) || tr.slug}</span>
            {#if done(tr)}
              <span class="small">{$t('runTrophyTaken')}</span>
            {:else if r.swapCost != null}
              {@const short = me != null && me.dust < tr.priceDust}
              <button type="button" class="quiet buy" disabled={busy || short} onclick={() => takeTrophy(tr.cardId)}>
                {f('runTrophyTake', { n: tr.priceDust })}
              </button>
              {#if short}<span class="small">{$t('runFaultNotEnough')}</span>{/if}
            {/if}
          </li>
        {/each}
      </ul>
    </section>
  {/if}
  {#if closed.length}
    <section aria-labelledby="closed-h">
      <details class="closed" open={!open.length && !had.length}>
        <summary id="closed-h">{$t('runTrophyClosed')} · {closed.length}</summary>
        <p class="small">{$t('runTrophyClosedAbout')}</p>
        <ul class="closed-list">
          {#each closed as tr (tr.cardId)}
            {@const face = cardOf(tr.cardId)}
            <li>
              {#if face}
                <span class="face face--small" aria-hidden="true">
                  <BattleCardFace card={face} {frames} owned={true} uniform transition={false} interactive={false} />
                </span>
              {/if}
              <span class="small">{nameOf(tr.cardId) || tr.slug}</span>
              <span class="small closed-why">{tr.onShelf ? $t('runTrophyNotForSale') : $t('runTrophyNotOnShelf')}</span>
            </li>
          {/each}
        </ul>
      </details>
    </section>
  {/if}
{/snippet}

{#if replayOf && token}
  <RunReplay
    matchId={replayOf.matchId}
    {token}
    title="{f('runLogWave', { n: replayOf.waveNo })}{replayOf.practice ? ` · ${$t('runLogPractice')}` : ''}"
    {cards}
    {frames}
    {motions}
    onclose={() => (replayOf = null)}
  />
{/if}

{#if menu && menuItems.length}
  <CardMenu
    title={nameOf(menu.cardId)}
    art={(() => {
      const face = cardOf(menu.cardId);
      return face?.artUrlOverride || face?.artUrl || null;
    })()}
    health={(() => {
      const c = run?.squad.find((x) => x.cardId === menu?.cardId);
      return c && !c.dead ? { h: c.health, m: c.maxHealth } : null;
    })()}
    items={menuItems}
    at={menu.at}
    onclose={() => (menu = null)}
  />
{/if}

<CardPeek hover={hovered} look={looked} {frames} note={lookedNote} onclose={() => (looked = null)} />

<style>
  /* Автобой: одна кнопка поверх сцены, в левом углу: справа у сцены своя кнопка. */
  .autobar {
    position: fixed;
    top: 0.75rem;
    left: 0.75rem;
    z-index: 60;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.4rem;
  }
  .autobar button {
    min-height: 44px;
    padding: 0.45rem 1rem;
    border: 1px solid #6f3b24;
    background: rgba(248, 241, 231, 0.95);
    color: #6f3b24;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.78rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    cursor: pointer;
  }
  .autobar button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .autobar .autobar-stop {
    background: #6f3b24;
    color: #f8f1e7;
  }
  .autobar .autobar-quiet {
    min-height: 36px;
    font-size: 0.68rem;
    background: rgba(248, 241, 231, 0.8);
    border-color: rgba(111, 59, 36, 0.5);
  }
  /* На узком экране шапка сцены занимает всю ширину: кнопка встаёт под неё. */
  @media (max-width: 44rem) {
    .autobar {
      top: 7.5rem;
    }
  }
  /* Партия занимает окно целиком, как на странице этюдов: сцена берёт всю высоту. */
  .root--match {
    height: 100dvh;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    background: #0b0806;
    color: #e6dcc8;
  }
  .root--match :global(.room) {
    flex: 1 1 auto;
    min-height: 0;
  }
  .kicker {
    margin: 1.5rem 0 0;
    font-size: 0.72rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #7a5a45;
  }
  .dust-chip {
    display: inline-block;
    margin: 0 0 0.5rem;
    padding: 0.15rem 0.7rem;
    border: 1px solid #b08a3a;
    border-radius: 999px;
    color: #6f3b24;
    font-size: 0.85rem;
    background: rgba(176, 138, 58, 0.1);
  }
  ul.prizes {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(12.5rem, 1fr));
    gap: 1rem;
    margin: 1rem 0 0;
  }
  ul.prizes li {
    padding: 0;
    border: 0;
    display: block;
    opacity: 0;
    transform: translateY(14px);
    animation: prize-in 520ms cubic-bezier(0.2, 0.7, 0.2, 1) forwards;
    animation-delay: calc(var(--i) * 110ms);
  }
  @keyframes prize-in {
    to {
      opacity: 1;
      transform: none;
    }
  }
  .prize {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.35rem;
    width: 100%;
    height: 100%;
    padding: 1.6rem 1rem 1.1rem;
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    background: linear-gradient(180deg, #fcf7ee, #f4ead9);
    color: #34251c;
    font: inherit;
    text-align: center;
    cursor: pointer;
    box-shadow:
      inset 0 0 0 4px #fcf7ee,
      inset 0 0 0 5px #d8c6b1,
      0 1px 0 rgba(52, 37, 28, 0.06);
    transition:
      transform 180ms ease,
      box-shadow 180ms ease,
      border-color 180ms ease;
  }
  .prize:hover:not(:disabled),
  .prize:focus-visible {
    transform: translateY(-5px);
    border-color: var(--tone);
    box-shadow:
      inset 0 0 0 4px #fcf7ee,
      inset 0 0 0 5px var(--tone),
      0 14px 24px -12px rgba(52, 37, 28, 0.45);
  }
  .prize:active:not(:disabled) {
    transform: translateY(-1px);
  }
  .prize:disabled {
    opacity: 0.6;
  }
  /* Печать: круг сургуча своего цвета, значок вдавлен. */
  .seal {
    display: grid;
    place-items: center;
    width: 3.6rem;
    height: 3.6rem;
    margin-bottom: 0.35rem;
    border-radius: 50%;
    color: #f8f1e7;
    background: radial-gradient(circle at 35% 30%, color-mix(in srgb, var(--tone) 70%, #fff 30%), var(--tone) 60%, color-mix(in srgb, var(--tone) 70%, #000 30%));
    box-shadow:
      0 0 0 3px rgba(248, 241, 231, 0.9),
      0 0 0 4px var(--tone),
      0 4px 8px -2px rgba(52, 37, 28, 0.4);
  }
  .prize-title {
    font-family: Fraunces, Georgia, serif;
    font-size: 1.1rem;
    font-weight: 600;
    color: #34251c;
  }
  .prize-big {
    font-family: Fraunces, Georgia, serif;
    font-size: 2rem;
    line-height: 1;
    /* Тон чуть темнее печати: золото на пергаменте без этого не набирает контраст. */
    color: color-mix(in srgb, var(--tone) 75%, #000);
  }
  .prize-text {
    font-size: 0.9rem;
    color: #6f3b24;
    line-height: 1.35;
  }
  .prize-detail {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    margin-top: auto;
    padding-top: 0.6rem;
    border-top: 1px dashed #d8c6b1;
    width: 100%;
    font-size: 0.82rem;
    color: #34251c;
  }
  @media (max-width: 36rem) {
    ul.prizes {
      grid-template-columns: 1fr;
      gap: 0.75rem;
    }
    .prize {
      display: grid;
      grid-template-columns: auto 1fr;
      column-gap: 1rem;
      text-align: left;
      align-items: center;
      padding: 1rem;
    }
    .seal {
      grid-row: 1 / span 4;
      margin: 0;
    }
    .prize-detail {
      grid-column: 2;
      margin-top: 0.3rem;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    ul.prizes li {
      animation: none;
      opacity: 1;
      transform: none;
    }
    .prize {
      transition: none;
    }
  }
  .steps li.fail {
    background: #8f2f22;
    border-color: #8f2f22;
  }
  .finale {
    display: flex;
    gap: 1.5rem;
    align-items: center;
    margin: 1.25rem 0 0.5rem;
    padding: 1.5rem 1.5rem;
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    background: linear-gradient(180deg, #fcf7ee, #f4ead9);
    box-shadow:
      inset 0 0 0 4px #fcf7ee,
      inset 0 0 0 5px #d8c6b1;
  }
  .finale .seal {
    flex: none;
    margin: 0;
  }
  .seal--big {
    width: 5.5rem;
    height: 5.5rem;
  }
  .finale h2 {
    margin: 0.1rem 0 0.6rem;
    font-size: 1.6rem;
  }
  .finale .kicker {
    margin: 0;
  }
  .finale-cta {
    margin-top: 0.9rem;
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .finale .note {
    margin: 0.9rem 0 0.6rem;
  }
  a.act.link {
    display: inline-flex;
    align-items: center;
    text-decoration: none;
  }
  .trophy-face {
    position: relative;
    display: block;
    width: 100%;
  }
  .trophy-face--look {
    padding: 0;
    border: 0;
    background: none;
    cursor: zoom-in;
    text-align: left;
  }
  .grid li:not(.on):not(.dead) .pick.pick--look:not(:disabled) {
    opacity: 1;
    cursor: zoom-in;
  }
  .grid li.taken .face {
    opacity: 0.55;
  }
  .grid li .quiet {
    align-self: flex-start;
    min-height: 36px;
    padding: 0.2rem 0.7rem;
    font-size: 0.8rem;
  }
  .picks {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 0.9rem;
    margin: 0 0 0.4rem;
  }
  ul.picks li {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0;
    border: 0;
    font-size: 0.88rem;
  }
  .seal--xs {
    width: 1.9rem;
    height: 1.9rem;
    margin: 0;
    box-shadow:
      0 0 0 2px rgba(248, 241, 231, 0.9),
      0 0 0 3px var(--tone);
  }
  .seal--xs :global(svg) {
    width: 16px;
    height: 16px;
  }
  .closed {
    margin: 1.25rem 0 0;
    color: #6f3b24;
  }
  .closed summary {
    cursor: pointer;
    font-family: Fraunces, Georgia, serif;
    font-size: 1rem;
  }
  .closed-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.9rem;
    margin: 0.6rem 0 0;
  }
  ul.closed-list li {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.2rem;
    width: 5.5rem;
    padding: 0;
    border: 0;
    text-align: center;
  }
  .closed-list .face {
    opacity: 0.7;
    filter: grayscale(0.5);
  }
  .closed-why {
    color: #7a5a45;
    font-size: 0.78rem;
  }
  .quiet.buy {
    border-color: #b08a3a;
    background: rgba(176, 138, 58, 0.1);
  }
  @media (max-width: 36rem) {
    .finale {
      flex-direction: column;
      text-align: center;
      padding: 1.25rem 1rem;
    }
    .finale .head {
      justify-content: center;
    }
    .finale-cta {
      justify-content: center;
    }
  }
  .lead {
    max-width: 38rem;
    font-size: 1.1rem;
    line-height: 1.55;
    color: #6f3b24;
  }
  ol.how {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(9.5rem, 1fr));
    gap: 1rem;
    list-style: none;
    margin: 1.5rem 0 2rem;
    padding: 0;
  }
  .how li {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 1rem 0.9rem;
    border: 1px solid #d8c6b1;
    background: rgba(252, 247, 238, 0.7);
    opacity: 0;
    transform: translateY(10px);
    animation: prize-in 480ms ease forwards;
    animation-delay: calc(var(--i) * 90ms);
  }
  .how-title {
    font-family: Fraunces, Georgia, serif;
    font-weight: 600;
    font-size: 1.05rem;
  }
  .seal--sm {
    width: 2.8rem;
    height: 2.8rem;
    margin-bottom: 0.3rem;
  }
  ul.routes li {
    justify-content: space-between;
    gap: 1rem 1.5rem;
  }
  .route-main {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .routes .steps li {
    width: 1.5rem;
    height: 0.45rem;
    padding: 0;
    border: 1px solid #6f3b24;
    margin: 0;
    display: block;
  }
  .routes .steps li.free {
    background: #b08a3a;
    border-color: #b08a3a;
  }
  .goal {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0 0 0.6rem;
    font-weight: 600;
  }
  .goal-icon {
    display: grid;
    place-items: center;
    width: 2rem;
    height: 2rem;
    border: 1px solid #6f3b24;
    border-radius: 50%;
    color: #6f3b24;
    flex: none;
  }
  .goal-icon :global(svg) {
    width: 18px;
    height: 18px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin: 0;
  }
  .chip {
    padding: 0.15rem 0.7rem;
    border: 1px solid #d8c6b1;
    border-radius: 999px;
    font-size: 0.82rem;
    color: #6f3b24;
    background: rgba(255, 255, 255, 0.5);
  }
  .chip--tag {
    align-self: flex-start;
    padding: 0 0.5rem;
    font-size: 0.72rem;
  }
  .chip--gold {
    border-color: #b08a3a;
    background: rgba(176, 138, 58, 0.12);
  }
  @media (prefers-reduced-motion: reduce) {
    .how li {
      animation: none;
      opacity: 1;
      transform: none;
    }
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem 1rem;
  }
  .steps {
    display: flex;
    gap: 0.3rem;
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .steps li {
    width: 1.1rem;
    height: 0.4rem;
    border: 1px solid #6f3b24;
    background: transparent;
  }
  .steps li.done {
    background: #6f3b24;
  }
  .steps li.now {
    background: #c65f3c;
    border-color: #c65f3c;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr));
    gap: 1.25rem 1rem;
    margin: 0.75rem 0;
  }
  .grid li {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.3rem;
    padding: 0;
    border: 0;
  }
  .grid .face {
    width: 100%;
  }
  .pick {
    position: relative;
    padding: 0;
    border: 2px solid transparent;
    background: none;
    width: 100%;
    cursor: pointer;
    text-align: left;
  }
  .pick:disabled {
    opacity: 1;
    cursor: default;
  }
  .grid li.on .pick {
    border-color: #c65f3c;
    box-shadow: 0 0 0 2px rgba(198, 95, 60, 0.25);
  }
  .grid li:not(.on):not(.dead) .pick:not(:disabled) {
    opacity: 0.8;
  }
  .grid li.dead .face {
    opacity: 0.45;
    filter: grayscale(0.8);
  }
  .grid li.dead .card-name {
    text-decoration: line-through;
    opacity: 0.6;
  }
  .tick {
    position: absolute;
    top: 0.3rem;
    right: 0.3rem;
    width: 1.5rem;
    height: 1.5rem;
    display: grid;
    place-items: center;
    background: #c65f3c;
    color: #f8f1e7;
    border-radius: 50%;
    font-size: 0.85rem;
  }

  .sheet {
    max-width: 56rem;
    margin: 0 auto;
    /* 1.5rem: дверь боя уходит в поля на столько же (BattleDoor). */
    padding: 1.5rem 1.5rem 4rem;
    color: #34251c;
  }
  h1 {
    font-family: Fraunces, Georgia, serif;
    font-weight: 600;
    margin: 0.5rem 0 1rem;
  }
  h2 {
    font-family: Fraunces, Georgia, serif;
    font-size: 1.15rem;
    margin: 1.75rem 0 0.5rem;
  }
  .note,
  .small {
    color: #6f3b24;
    font-size: 0.92rem;
    line-height: 1.5;
  }
  .complaint {
    position: fixed;
    left: 50%;
    bottom: 6rem;
    transform: translateX(-50%);
    width: min(92vw, 38rem);
    z-index: 90;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
    box-shadow: 0 10px 24px -10px rgba(52, 37, 28, 0.5);
    border: 1px solid rgba(143, 47, 34, 0.35);
    border-left: 4px solid #8f2f22;
    padding: 0.6rem 0.9rem;
    background: #f6e6e1;
    color: #6f1f16;
  }
  .notice {
    position: fixed;
    left: 50%;
    bottom: 9.5rem;
    transform: translateX(-50%);
    width: min(92vw, 38rem);
    z-index: 95;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
    margin: 0;
    padding: 0.6rem 0.9rem;
    border: 1px solid rgba(74, 107, 58, 0.45);
    border-left: 4px solid #4a6b3a;
    background: #eaf0e1;
    color: #2c4422;
    box-shadow: 0 10px 24px -10px rgba(52, 37, 28, 0.5);
  }
  .complaint .x,
  .notice .x {
    flex: none;
    width: 2rem;
    height: 2rem;
    border: 0;
    background: none;
    color: inherit;
    font-size: 1.4rem;
    line-height: 1;
    cursor: pointer;
  }
  @media (max-width: 64rem) {
    .complaint {
      bottom: 9.5rem;
    }
  }
  .guestbar {
    margin: 0.75rem 0 0;
    padding: 0.55rem 0.9rem;
    border-left: 4px solid #b08a3a;
    background: rgba(176, 138, 58, 0.1);
    font-size: 0.9rem;
    color: #6f3b24;
  }
  .guestbar a {
    margin-left: 0.4rem;
    color: #6f3b24;
    font-weight: 600;
  }
  .hp {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
  }
  .hp-bar {
    flex: 1 1 auto;
    height: 8px;
    min-height: 8px;
    overflow: hidden;
    border-radius: 999px;
    background: #e8dccb;
    box-shadow: inset 0 1px 2px rgba(52, 37, 28, 0.3);
  }
  .hp-bar i {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: linear-gradient(#7aa561, #4a6b3a);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.35);
    transition: width 0.45s ease;
  }
  .hp-bar i.mid {
    background: linear-gradient(#dbb85f, #b08a3a);
  }
  .hp-bar i.low {
    background: linear-gradient(#c75a49, #8f2f22);
  }
  .hp-num {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    color: #4a6b3a;
    font-size: 0.82rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .hp-num.mid {
    color: #8a6a22;
  }
  .hp-num.low {
    color: #8f2f22;
  }
  .hp-num b {
    font-family: Fraunces, Georgia, serif;
    font-size: 1.05em;
    font-weight: 700;
  }
  .hp-num span {
    color: #8a6a55;
    font-size: 0.9em;
  }
  .hp-num .heart {
    flex: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .hp-bar i {
      transition: none;
    }
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0.5rem 0;
  }
  ul.routes li {
    border: 1px solid #d8c6b1;
    background: rgba(248, 241, 231, 0.6);
    padding: 0.9rem 1rem;
    margin-bottom: 0.75rem;
    justify-content: space-between;
  }
  .routes .route-name {
    font-family: Fraunces, Georgia, serif;
    font-size: 1.1rem;
  }
  .routes li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem 1rem;
    padding: 0.6rem 0;
    border-bottom: 1px solid #d8c6b1;
  }
  /* Лицо карты заполняет контейнер и читает его ширину само. */
  .face {
    display: block;
    width: 6.5rem;
    flex: none;
  }
  .face--small {
    width: 5.5rem;
  }
  .card-name,
  .route-name {
    font-weight: 600;
  }
  .act,
  .quiet {
    font: inherit;
    cursor: pointer;
    border: 1px solid #6f3b24;
    background: #a94e2b;
    color: #f8f1e7;
    padding: 0.45rem 0.9rem;
    min-height: 44px;
    border-radius: 2px;
  }
  .quiet {
    background: transparent;
    color: #6f3b24;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  button:focus-visible {
    outline: 2px solid #6f3b24;
    outline-offset: 2px;
  }
  /* Экран забега: одно окно, без прокрутки страницы. Сверху полоса маршрута,
     в середине поле, снизу отряд и кнопка начала волны. */
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
  .root--stage .complaint {
    bottom: 9.5rem;
  }
  .stage {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    padding: 0.5rem 0.75rem max(1.25rem, env(safe-area-inset-bottom, 0px));
    gap: 0.5rem;
    background: #f8f1e7;
    color: #34251c;
  }
  .topbar {
    display: flex;
    align-items: center;
    gap: 0.5rem 1rem;
    flex-wrap: wrap;
    min-height: 2.75rem;
  }
  .back {
    color: #6f3b24;
    font-size: 0.78rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    text-decoration: none;
    min-height: 44px;
    display: inline-flex;
    align-items: center;
  }
  .route {
    display: flex;
    align-items: center;
    gap: 0.4rem 0.8rem;
    flex: 1;
    min-width: 0;
  }
  .route-title {
    font-family: Fraunces, Georgia, serif;
    font-weight: 600;
    font-size: 1.05rem;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-left: auto;
  }
  .dust {
    margin-right: 0.4rem;
    font-size: 0.85rem;
    color: #6f3b24;
    white-space: nowrap;
  }
  .dust b {
    font-family: Fraunces, Georgia, serif;
    font-size: 1.15rem;
    color: #34251c;
  }
  .tool {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    min-height: 44px;
    padding: 0.2rem 0.8rem;
    border: 1px solid #6f3b24;
    background: rgba(255, 255, 255, 0.5);
    color: #6f3b24;
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .tool--on {
    background: #6f3b24;
    color: #f8f1e7;
  }
  /* «Сдать забег» стоит особняком: между ним и полезными кнопками зазор,
     чтобы не нажать его вместо «Добыча забега». */
  .tool--quiet {
    border-color: #d8c6b1;
    font-size: 0.78rem;
    margin-left: 1.2rem;
  }
  .badge {
    text-transform: none;
    letter-spacing: 0;
    min-width: 1.25rem;
    padding: 0 0.35rem;
    border-radius: 999px;
    background: #6f3b24;
    color: #f8f1e7;
    font-size: 0.75rem;
    line-height: 1.35rem;
    text-align: center;
  }
  .badge--gold {
    background: #8a6a1f;
  }
  .tool--on .badge {
    background: #f8f1e7;
    color: #6f3b24;
  }
  .stage .guestbar {
    margin: 0;
    padding: 0.3rem 0.8rem;
    font-size: 0.82rem;
  }
  .wave-log {
    margin: 0.6rem 0;
    font-size: 0.9rem;
  }
  .wave-log ul {
    display: grid;
    gap: 0.3rem;
    margin: 0.3rem 0 0;
    padding: 0;
    list-style: none;
  }
  .wave-log li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.3rem 0.8rem;
  }
  .wl-name {
    min-width: 9rem;
  }
  .wl-out {
    color: #8f2f22;
  }
  .wl-out--won {
    color: #4a6b3a;
  }
  .chip--heal {
    border-color: #3d5a2f;
    background: #e6efdc;
    color: #3d5a2f;
  }
  .quiet--heal {
    border-color: #3d5a2f;
    color: #3d5a2f;
  }
  .daily {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem 1rem;
    margin: 0 0 1rem;
    padding: 0.8rem 1rem;
    border: 1px solid #d8c6b1;
    border-left: 3px solid #b08a3a;
    background: #fcf7ee;
  }
  .daily h2 {
    margin: 0.1rem 0 0.3rem;
    font-family: Fraunces, Georgia, serif;
    font-size: 1.2rem;
  }
  .daily p {
    margin: 0.15rem 0 0;
  }
  .daily-mine {
    color: #6f3b24;
    font-weight: 600;
  }
  .report {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.2rem 0.8rem;
    margin: 0.5rem 0;
    font-size: 0.88rem;
  }
  .report ul {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem 0.8rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .report .fell {
    color: #8f2f22;
  }
  .tip {
    display: flex;
    flex: none;
    align-items: center;
    gap: 0.6rem;
    margin: 0;
    padding: 0.3rem 0.6rem;
    border: 1px solid #d8c6b1;
    border-left: 3px solid #c65f3c;
    background: #fcf7ee;
    font-size: 0.85rem;
    line-height: 1.35;
  }
  .tip span {
    flex: 1;
  }
  .tip-ok {
    flex: none;
    padding: 0.2rem 0.7rem;
    border: 1px solid #6f3b24;
    background: transparent;
    color: #6f3b24;
    font: inherit;
    cursor: pointer;
  }
  .wavebar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.4rem 1rem;
    padding: 0 0.25rem;
  }
  .wave-no {
    display: flex;
    align-items: baseline;
    gap: 0.25rem;
    font-family: Fraunces, Georgia, serif;
  }
  .wave-no b {
    font-size: 2rem;
    line-height: 1;
    color: #c65f3c;
  }
  .wave-no span {
    font-size: 0.85rem;
    color: #7a5a45;
  }
  .goal {
    margin: 0;
    font-size: 0.95rem;
  }
  .field {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 0.75rem;
  }
  .field-main {
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  .field-main > .note {
    text-align: center;
  }
  /* Подробности выбранной карты: только на широком экране, на телефоне места нет. */
  .inspect {
    display: none;
  }
  @media (min-width: 64rem) {
    .inspect {
      display: flex;
      flex-direction: column;
      gap: 0.4rem;
      flex: none;
      width: 15rem;
      overflow-y: auto;
      padding: 0.7rem 0.8rem;
      border: 1px solid #d8c6b1;
      background: #fcf7ee;
    }
    .inspect h2 {
      margin: 0;
      font-size: 1.15rem;
    }
    .inspect .face {
      width: 8rem;
      align-self: center;
    }
    .inspect--foe {
      border-color: #8f2f22;
    }
    .inspect .stats {
      display: grid;
      grid-template-columns: repeat(2, 1fr);
      gap: 0.2rem 0.6rem;
      margin: 0.2rem 0 0;
    }
    .inspect .stats div {
      display: flex;
      align-items: baseline;
      justify-content: space-between;
      gap: 0.4rem;
      border-bottom: 1px solid #e6d8c4;
    }
    .inspect dt {
      font-size: 0.68rem;
      letter-spacing: 0.06em;
      text-transform: uppercase;
      color: #7a5a45;
    }
    .inspect dd {
      margin: 0;
      font-family: Fraunces, Georgia, serif;
      font-size: 1.3rem;
    }
    .inspect .effect {
      margin: 0;
      font-size: 0.85rem;
      line-height: 1.4;
    }
  }
  /* Рука, число выбранных и кнопка — одна полоса по центру под полем: кнопка
     стоит рядом с картами, а не в дальнем углу окна. */
  .hand {
    position: relative;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: end;
    gap: 1rem;
    padding-top: 0.5rem;
    border-top: 1px solid #d8c6b1;
  }
  .hand-cards {
    grid-column: 2;
  }
  .go {
    grid-column: 3;
    grid-row: 1 / span 2;
    justify-self: end;
  }
  .hand-note {
    grid-column: 2;
    margin: 0;
    font-size: 0.78rem;
    text-align: center;
    color: #6f3b24;
  }
  /* Подробности выбранной карты одной строкой: на широком экране их показывает боковая панель. */
  .inspect-line {
    flex: none;
    margin: 0;
    padding: 0.3rem 0.6rem;
    border: 1px solid #d8c6b1;
    background: #fcf7ee;
    font-size: 0.85rem;
    line-height: 1.35;
    text-align: center;
  }
  .inspect-line--foe {
    border-color: #8f2f22;
  }
  .inspect-line b {
    margin-right: 0.4rem;
    font-family: Fraunces, Georgia, serif;
  }
  @media (min-width: 64rem) {
    .inspect-line {
      display: none;
    }
  }
  .hand-cards {
    display: flex;
    gap: 0.5rem;
    flex: 0 1 auto;
    min-width: 0;
    margin: 0;
    padding: 0.35rem 0.25rem 0.3rem;
    overflow-x: auto;
  }
  /* Место, куда отпустить карту, чтобы убрать её с поля: видно всегда, пока на поле есть карты. */
  .dropzone {
    display: none;
    grid-column: 1;
    align-self: center;
    justify-self: start;
    flex-direction: column;
    align-items: center;
    gap: 0.15rem;
    max-width: 11rem;
    padding: 0.6rem 0.8rem;
    border: 2px dashed #c9b49a;
    background: rgba(255, 255, 255, 0.35);
    color: #6f3b24;
    font-size: 0.8rem;
    line-height: 1.25;
    text-align: center;
  }
  .dropzone b {
    font-family: Fraunces, Georgia, serif;
    font-size: 0.95rem;
  }
  .dropzone--hot {
    border-color: #8f2f22;
    background: rgba(143, 47, 34, 0.1);
    color: #8f2f22;
  }
  @media (hover: hover) and (pointer: fine) and (min-width: 48rem) {
    .dropzone {
      display: flex;
    }
  }
  /* Перетаскивание есть только там, где есть мышь. */
  .drag-how {
    display: none;
  }
  @media (hover: hover) and (pointer: fine) {
    .drag-how {
      display: inline;
    }
  }
  .hand--drop {
    outline: 2px dashed #8f2f22;
    outline-offset: -4px;
    background: rgba(143, 47, 34, 0.06);
  }
  .drop-note {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0;
    background: rgba(252, 247, 238, 0.88);
    color: #8f2f22;
    font-weight: 600;
    pointer-events: none;
  }
  .hand-cards li {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0;
    border: 0;
    width: 5rem;
  }
  .hcard {
    position: relative;
    padding: 0;
    border: 2px solid transparent;
    background: none;
    cursor: pointer;
    transition: transform 160ms ease;
  }
  .hcard:disabled {
    opacity: 1;
    cursor: default;
  }
  .hand-cards li.on .hcard {
    border-color: #c65f3c;
    box-shadow: 0 0 0 2px rgba(198, 95, 60, 0.25);
    transform: translateY(-4px);
  }
  .hand-cards li:not(.on):not(.dead) .hcard {
    opacity: 0.75;
  }
  .hand-cards li.dead .face {
    opacity: 0.45;
    filter: grayscale(0.8);
  }
  .hand-cards .face {
    width: 100%;
  }
  .hand-cards .hp-bar {
    display: block;
    flex: none;
    width: 100%;
  }
  .go {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    text-align: center;
    gap: 0.3rem;
    flex: none;
    padding-bottom: 0.3rem;
  }
  .go .why {
    max-width: 14rem;
    align-self: center;
  }
  .hand-cards .hp-num {
    justify-content: center;
    font-size: 0.8rem;
  }
  .hand-cards li.dead .hp-num {
    justify-content: center;
    color: #8f2f22;
    font-style: italic;
  }
  .go .why {
    color: #8f2f22;
  }
  .go .act {
    min-width: 9rem;
    min-height: 52px;
    font-family: Georgia, 'Fraunces', serif;
    letter-spacing: 0.06em;
  }
  /* Награда: выбор из трёх на весь экран. */
  .reward {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 0.5rem 0;
  }
  .reward h2 {
    margin: 0.2rem 0 0.8rem;
    font-size: 1.9rem;
  }
  .reward .kicker {
    margin: 0;
  }
  .reward ul.prizes {
    width: min(100%, 52rem);
    margin: 0.5rem 0;
  }
  /* Панель: справа на широком экране, снизу листом на телефоне. */
  .scrim {
    position: absolute;
    inset: 0;
    z-index: 20;
    border: 0;
    background: rgba(27, 20, 16, 0.4);
    cursor: default;
  }
  .panel {
    position: absolute;
    z-index: 21;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(27rem, 92vw);
    overflow-y: auto;
    padding: 1rem 1.1rem 2rem;
    background: #f8f1e7;
    border-left: 1px solid #d8c6b1;
    box-shadow: -14px 0 28px -16px rgba(52, 37, 28, 0.5);
  }
  .panel h2 {
    margin: 0.2rem 0 0.5rem;
  }
  .panel .sub {
    margin: 0.4rem 0;
    font-size: 0.72rem;
    font-weight: 400;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #7a5a45;
  }
  .panel-x {
    position: absolute;
    top: 0.4rem;
    right: 0.5rem;
    width: 2.75rem;
    height: 2.75rem;
    border: 0;
    background: none;
    color: #6f3b24;
    font-size: 1.7rem;
    cursor: pointer;
  }
  .grab {
    display: none;
  }
  .chip--inline {
    display: inline-block;
    margin: 0.3rem 0 0;
  }
  @media (max-width: 48rem) {
    .stage {
      padding: 0.35rem 0.5rem max(1rem, env(safe-area-inset-bottom, 0px));
      gap: 0.35rem;
    }
    .route-title {
      display: none;
    }
    .dust {
      display: none;
    }
    .tool {
      padding: 0.2rem 0.55rem;
      font-size: 0.8rem;
    }
    .wavebar {
      padding: 0.35rem 0.6rem;
    }
    .wave-no b {
      font-size: 1.5rem;
    }
    .hand {
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 0.5rem;
    }
    .hand-cards,
    .hand-note {
      grid-column: 1;
    }
    .go {
      grid-column: 2;
    }
    .hand-cards li {
      width: 3.4rem;
    }
    .go .act {
      min-width: 6.5rem;
    }
    .panel {
      top: auto;
      left: 0;
      width: auto;
      max-height: 78dvh;
      border-left: 0;
      border-top: 1px solid #d8c6b1;
      border-radius: 14px 14px 0 0;
      box-shadow: 0 -14px 28px -16px rgba(52, 37, 28, 0.5);
    }
    .grab {
      display: block;
      width: 2.5rem;
      height: 4px;
      margin: 0 auto 0.4rem;
      border-radius: 2px;
      background: #d8c6b1;
    }
    .reward h2 {
      font-size: 1.5rem;
    }
  }
  @media (max-height: 44rem) {
    .hand-cards li {
      width: 3.4rem;
    }
  }
  /* Плашка волны: полоса проезжает через экран и уходит. */
  .banner {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: center;
    pointer-events: none;
  }
  .banner span {
    width: 100%;
    padding: 0.9rem 1rem;
    background: #c65f3c;
    color: #fff;
    font-family: Fraunces, Georgia, serif;
    font-size: clamp(1.4rem, 4vw, 2.2rem);
    letter-spacing: 0.08em;
    text-align: center;
    animation: banner 1.9s ease-in-out both;
  }
  @keyframes banner {
    0% {
      transform: translateX(-100%);
    }
    18%,
    76% {
      transform: translateX(0);
    }
    100% {
      transform: translateX(100%);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .hcard,
    .hand-cards li {
      transition: none;
    }
    .banner span {
      animation: banner-fade 1.9s ease-in-out both;
    }
    @keyframes banner-fade {
      0%,
      100% {
        opacity: 0;
      }
      15%,
      80% {
        opacity: 1;
      }
    }
  }
</style>
