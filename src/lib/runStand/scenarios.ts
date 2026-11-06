// Сценарии стенда забега: готовые состояния экрана на подставных данных.
// Карты и картинки настоящие (с сервера), а забег, награды и кошелёк собираются
// здесь и живут в памяти страницы: база и сервер забега не используются.

import type {
  BattleCard,
  BattleDaily,
  BattleRoute,
  BattleRun,
  RunReward,
  RunSquadCard,
  RunTrophy,
  RunWaveField,
  RunWaveLog,
  RunWavePreview,
} from '$lib/types/api';

export type StandState = {
  routes: BattleRoute[];
  run: BattleRun | null;
  waves: RunWaveLog[];
  daily: BattleDaily | null;
  dust: number;
  /** Вошедший (true) или гость без входа (false). */
  signedIn: boolean;
  /** Каждый запрос ждёт столько миллисекунд. */
  delayMs: number;
  /** Запросы к забегам отвечают ошибкой сервера. */
  broken: boolean;
};

export type Scenario = {
  id: string;
  title: string;
  about: string;
  build: (cards: BattleCard[]) => StandState;
};

const route: BattleRoute = {
  slug: 'stand',
  titleEn: 'Stand route',
  titleRu: 'Стендовый маршрут',
  titleUk: 'Стендовий маршрут',
  titleEs: 'Ruta de prueba',
  titleDe: 'Testroute',
  titleIt: 'Percorso di prova',
  waves: 6,
  guestWaves: 2,
};

const same = (n: number) => Array.from({ length: n }, (_, i) => i);

/** Поле волны: две земли, двое врагов с начала, один выходит позже, зоны ударов. */
export function field(foes: string[]): RunWaveField {
  const open = same(3).flatMap((x) => same(3).map((i) => ({ x, y: 3 + i })));
  return {
    width: 3,
    depth: 3,
    terrain: [
      { cell: { x: 1, y: 4 }, ground: 'mire' },
      { cell: { x: 2, y: 1 }, ground: 'hill' },
    ],
    enemies: [
      { slug: foes[0], x: 0, y: 0 },
      { slug: foes[1], x: 1, y: 1 },
    ],
    structures: [],
    entrances: [{ round: 2, x: 2, y: 0, slug: foes[2] }],
    openCells: open,
    threats: [
      { cell: { x: 0, y: 0 }, cells: [{ x: 0, y: 1 }, { x: 0, y: 2 }, { x: 1, y: 0 }] },
      { cell: { x: 1, y: 1 }, cells: [{ x: 1, y: 2 }, { x: 1, y: 3 }, { x: 2, y: 1 }] },
    ],
  };
}

function preview(no: number, foes: string[], over: Partial<RunWavePreview> = {}): RunWavePreview {
  return {
    waveNo: no,
    minCards: 1,
    maxCards: 5,
    goal: no % 3 === 1 ? { kind: 'holdRounds', rounds: 5 } : null,
    raceHint: null,
    rewardDust: 20 + no * 5,
    enemies: foes,
    field: field(foes),
    ...over,
  };
}

function squad(cards: BattleCard[], hp: number[], loaned = false): RunSquadCard[] {
  return cards.slice(0, hp.length).map((c, i) => ({
    cardId: c.id,
    slug: c.slug,
    health: hp[i] === 0 ? 0 : Math.max(1, Math.round(c.health * hp[i])),
    maxHealth: c.health,
    dead: hp[i] === 0,
    loaned,
    cell: null,
  }));
}

function trophy(card: BattleCard, waveNo: number, over: Partial<RunTrophy> = {}): RunTrophy {
  return { cardId: card.id, slug: card.slug, waveNo, priceDust: 10 + card.tier * 10, taken: false, owned: false, onShelf: true, ...over };
}

function run(cards: BattleCard[], over: Partial<BattleRun> & { hp?: number[]; foes?: string[] }): BattleRun {
  const foes = over.foes ?? cards.slice(6, 9).map((c) => c.slug);
  const { hp = [1, 1, 1, 1, 1, 1], foes: _foes, ...rest } = over;
  void _foes;
  return {
    id: 'stand-run',
    route,
    waveNo: 1,
    wavesTotal: 6,
    state: 'active',
    squad: squad(cards, hp),
    swaps: 0,
    matchId: null,
    nextWave: preview(over.waveNo ?? 1, foes),
    phase: 'wave',
    reward: [],
    swapCost: 17,
    freeSwaps: 0,
    healCharges: 0,
    trophies: [],
    picks: [],
    bench: 2,
    paidDust: 0,
    practice: null,
    ...rest,
  };
}

const base = (over: Partial<StandState>): StandState => ({
  routes: [route],
  run: null,
  waves: [],
  daily: null,
  dust: 120,
  signedIn: true,
  delayMs: 0,
  broken: false,
  ...over,
});

const finished = (cards: BattleCard[], lostWave: number): { r: BattleRun; log: RunWaveLog[] } => {
  const foes = cards.slice(6, 9).map((c) => c.slug);
  const log: RunWaveLog[] = [];
  for (let n = 1; n < lostWave; n++) log.push({ matchId: `m${n}`, waveNo: n, outcome: 'player', rounds: 4 + n, practice: false });
  log.push({ matchId: `m${lostWave}`, waveNo: lostWave, outcome: 'keeper', rounds: 7, practice: false });
  const r = run(cards, {
    waveNo: lostWave,
    state: 'lost',
    hp: [0, 0, 0.2, 0, 0, 0],
    foes,
    nextWave: null,
    swapCost: null,
    trophies: [trophy(cards[6], 1), trophy(cards[7], 2, { owned: true }), trophy(cards[8], 2, { priceDust: 0 })],
    picks: [
      { waveNo: 1, kind: 'dust', amount: 20 },
      { waveNo: 2, kind: 'healOne', amount: 0 },
    ],
  });
  r.practice = { wave: preview(lostWave, foes), squad: squad(cards, [1, 0.8, 0.6, 1, 0.5, 1]) };
  return { r, log };
};

export const SCENARIOS: Scenario[] = [
  {
    id: 'empty',
    title: 'Нет забега',
    about: 'Вошедший игрок, забегов не было: список маршрутов и вход.',
    build: () => base({}),
  },
  {
    id: 'none',
    title: 'Нет маршрутов',
    about: 'Режим включён, но опубликованных маршрутов нет.',
    build: () => base({ routes: [] }),
  },
  {
    id: 'first',
    title: 'Первая волна',
    about: 'Подсказка новичку, поле с землёй, подкрепление «Позже», зоны ударов, здоровье карт в ряду.',
    build: (c) => base({ run: run(c, { waveNo: 1, hp: [1, 1, 1, 1, 1, 1] }) }),
  },
  {
    id: 'wounded',
    title: 'Раненые и павшая, заряд лечения',
    about: 'Меню карты: «Излечить», «Убрать с поля», «Заменить», «Подробности». У павшей — замена.',
    build: (c) =>
      base({ run: run(c, { waveNo: 3, hp: [1, 0.5, 0.8, 0.3, 0, 1], healCharges: 1, freeSwaps: 0 }) }),
  },
  {
    id: 'free-swap',
    title: 'Бесплатная замена',
    about: 'То же меню, но замена бесплатная; без пыли — «Не хватает пыли».',
    build: (c) => base({ dust: 3, run: run(c, { waveNo: 3, hp: [1, 0.5, 0.8, 0.3, 0, 1], freeSwaps: 1 }) }),
  },
  {
    id: 'no-bench',
    title: 'Нет карт на замену',
    about: 'Замена в меню остаётся серой с причиной.',
    build: (c) => base({ run: run(c, { waveNo: 3, hp: [1, 0.5, 0.8, 0.3, 0, 1], bench: 0 }) }),
  },
  {
    id: 'reward',
    title: 'Награда из трёх',
    about: 'Лечение (заряд), пыль, бесплатная замена; под каждой — что она даст.',
    build: (c) => {
      const reward: RunReward[] = [
        { kind: 'healOne', amount: 0 },
        { kind: 'dust', amount: 25 },
        { kind: 'freeSwap', amount: 0 },
      ];
      return base({
        run: run(c, { waveNo: 3, phase: 'reward', hp: [1, 0.5, 0.8, 0.3, 0, 1], reward, picks: [{ waveNo: 1, kind: 'dust', amount: 20 }] }),
        waves: [
          { matchId: 'm1', waveNo: 1, outcome: 'player', rounds: 5, practice: false },
          { matchId: 'm2', waveNo: 2, outcome: 'player', rounds: 6, practice: false },
        ],
      });
    },
  },
  {
    id: 'reward-big',
    title: 'Награда: лечение всем и возвращение',
    about: 'Другой набор наград: лечение всем, возвращение павшей, пыль.',
    build: (c) =>
      base({
        run: run(c, {
          waveNo: 4,
          phase: 'reward',
          hp: [0.4, 0.5, 0.8, 0.3, 0, 1],
          reward: [
            { kind: 'healAll', amount: 3 },
            { kind: 'revive', amount: 0 },
            { kind: 'dust', amount: 40 },
          ],
        }),
      }),
  },
  {
    id: 'loot',
    title: 'Добыча: трофеи и награды',
    about: 'Откройте «Добыча забега»: трофеи с ценой, взятый, закрытые, журнал волн.',
    build: (c) => {
      const r = run(c, {
        waveNo: 4,
        hp: [1, 0.7, 0.8, 0.9, 1, 1],
        freeSwaps: 1,
        trophies: [trophy(c[6], 1), trophy(c[7], 2, { taken: true }), trophy(c[8], 3, { onShelf: false }), trophy(c[9], 3, { priceDust: 0 })],
        picks: [
          { waveNo: 1, kind: 'dust', amount: 20 },
          { waveNo: 2, kind: 'healAll', amount: 3 },
          { waveNo: 3, kind: 'freeSwap', amount: 0 },
        ],
      });
      return base({
        run: r,
        waves: [1, 2, 3].map((n) => ({ matchId: `m${n}`, waveNo: n, outcome: 'player' as const, rounds: 4 + n, practice: false })),
      });
    },
  },
  {
    id: 'lost',
    title: 'Забег окончен (поражение)',
    about: 'Баннер, потери, журнал волн с «Посмотреть», кнопка «Сыграть эту волну ещё раз», трофеи.',
    build: (c) => {
      const { r, log } = finished(c, 3);
      return base({ run: r, waves: [...log, { matchId: 'mp', waveNo: 3, outcome: 'player', rounds: 6, practice: true }] });
    },
  },
  {
    id: 'practice',
    title: 'Тренировка проигранной волны',
    about: 'Нажмите «Сыграть эту волну ещё раз»: расстановка и подсказка про тренировку (сам бой в стенде не играется).',
    build: (c) => {
      const { r, log } = finished(c, 2);
      return base({ run: r, waves: log });
    },
  },
  {
    id: 'won',
    title: 'Маршрут пройден',
    about: 'Итог победы: баннер, полоса волн, трофеи, карточки маршрутов для нового забега.',
    build: (c) =>
      base({
        run: run(c, {
          waveNo: 6,
          state: 'won',
          hp: [0.6, 0.8, 1, 0.5, 0.9, 1],
          nextWave: null,
          swapCost: null,
          paidDust: 45,
          trophies: [trophy(c[6], 4), trophy(c[7], 5, { taken: true }), trophy(c[8], 6)],
          picks: [{ waveNo: 1, kind: 'dust', amount: 20 }, { waveNo: 2, kind: 'healAll', amount: 3 }],
        }),
        waves: [1, 2, 3, 4, 5, 6].map((n) => ({ matchId: `m${n}`, waveNo: n, outcome: 'player' as const, rounds: 4 + n, practice: false })),
      }),
  },
  {
    id: 'trial',
    title: 'Проба гостя пройдена',
    about: 'Гость прошёл пробные волны: «Проба пройдена» и ссылка ко входу.',
    build: (c) =>
      base({
        signedIn: false,
        run: run(c, { waveNo: 2, wavesTotal: 2, state: 'won', hp: [0.6, 0.8, 1], nextWave: null, swapCost: null, bench: 0 }),
      }),
  },
  {
    id: 'guest',
    title: 'Гость на входе',
    about: 'Не вошедший человек видит маршрут с пометкой, сколько волн ему открыто.',
    build: () => base({ signedIn: false }),
  },
  {
    id: 'daily',
    title: 'Маршрут дня',
    about: 'Блок маршрута дня над списком: общий счёт и кнопка «Сыграть».',
    build: () =>
      base({ daily: { date: '2026-11-05', route, started: 14, won: 3, mine: null } }),
  },
  {
    id: 'daily-done',
    title: 'Маршрут дня: уже сыгран',
    about: 'У игрока уже есть результат дня.',
    build: () =>
      base({ daily: { date: '2026-11-05', route, started: 14, won: 3, mine: { waveNo: 3, wavesTotal: 6, state: 'lost' } } }),
  },
  {
    id: 'slow',
    title: 'Медленный сервер',
    about: 'Каждый запрос ждёт 2,5 секунды: проверьте, как ведёт себя «…» и повторные нажатия.',
    build: (c) => base({ delayMs: 2500, run: run(c, { waveNo: 1, hp: [1, 1, 1, 1, 1, 1] }) }),
  },
  {
    id: 'fault',
    title: 'Ошибка сервера',
    about: 'Запросы к забегам отвечают 500: плавающее сообщение об ошибке.',
    build: (c) => base({ broken: true, run: run(c, { waveNo: 1 }) }),
  },
];

