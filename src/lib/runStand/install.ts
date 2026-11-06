// Стенд забега: перехват запросов страницы `/battles/run`.
//
// Страница при `?stand=<сценарий>` работает на подставных данных: запросы к
// забегам, кошельку и маршрутам отвечает эта программа, остальное (карты,
// рамки, картинки) идёт на настоящий сервер. Страница при этом не меняется:
// видно ровно то, что увидит игрок. Ничего не пишется ни в базу, ни в
// аналитику; вход подставляется только в памяти этой вкладки.

import { api } from '$lib/api';
import { authStore } from '$lib/stores/auth.svelte';
import type { BattleCard, BattleRun, RunReward, RunSquadCard } from '$lib/types/api';
import { SCENARIOS, type StandState } from './scenarios';

type Json = Record<string, unknown>;

/** Стенд открыт по ссылке администратора: есть ключ админки или это разработка. */
export function standAllowed(): boolean {
  if (import.meta.env.DEV) return true;
  try {
    return !!(localStorage.getItem('gotiga_api_key') || sessionStorage.getItem('gotiga_api_key'));
  } catch {
    return false;
  }
}

const reply = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function installRunStand(id: string): Promise<boolean> {
  const scenario = SCENARIOS.find((s) => s.id === id);
  if (!scenario || !standAllowed() || typeof window === 'undefined') return false;

  const cards: BattleCard[] = await api.getBattleCards();
  const st: StandState = scenario.build(cards);

  // Подсказки новичку и звук живут в хранилище этого браузера: стенд их не трогает,
  // кроме подсказки первой волны, которую сценарий «Первая волна» должен показать.
  if (id === 'first') {
    try {
      localStorage.removeItem('gotiga_run_tips');
    } catch {
      // Хранилище закрыто — подсказка покажется как есть.
    }
  }

  // Вход только в памяти этой вкладки: запись в `localStorage` затёрла бы настоящий вход.
  if (st.signedIn) {
    authStore.sessionToken = 'stand-token';
    authStore.user = { id: 'stand-user', email: 'stand@example.com', displayName: 'Стенд' } as never;
  }

  // Гость, заведённый стендом, не должен остаться ключом гостя в браузере администратора.
  const setItem = Storage.prototype.setItem;
  Storage.prototype.setItem = function (key: string, value: string) {
    if (key === 'gotiga_battle_guest') return;
    setItem.call(this, key, value);
  };

  const byId = new Map(cards.map((c) => [c.id, c]));
  const real = window.fetch.bind(window);

  const squadCard = (c: BattleCard): RunSquadCard => ({
    cardId: c.id, slug: c.slug, health: c.health, maxHealth: c.health, dead: false, loaned: false, cell: null,
  });

  const apply = (r: BattleRun, reward: RunReward) => {
    if (reward.kind === 'dust') st.dust += reward.amount;
    if (reward.kind === 'freeSwap') r.freeSwaps += 1;
    if (reward.kind === 'healOne') r.healCharges += 1;
    if (reward.kind === 'healAll') for (const c of r.squad) if (!c.dead) c.health = Math.min(c.maxHealth, c.health + reward.amount);
    if (reward.kind === 'revive') {
      const down = r.squad.find((c) => c.dead);
      if (down) {
        down.dead = false;
        down.health = Math.max(1, Math.floor(down.maxHealth / 2));
      }
    }
  };

  window.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
    const method = (init?.method ?? (input instanceof Request ? input.method : 'GET')).toUpperCase();
    // Аналитика стенда не шлёт ничего: цифры игры не должны зависеть от проверки экрана.
    if (url.includes('/analytics/')) return new Response(null, { status: 204 });
    // Гость стенда настоящим гостем в базе не становится.
    if (url.includes('/auth/guest')) return reply({ sessionToken: 'stand-guest' });
    const at = url.indexOf('/battles/');
    if (at < 0) return real(input, init);
    const path = url.slice(at).split('?')[0];
    const isStand =
      /^\/battles\/(routes|daily|me|errands|runs|guest)/.test(path) || /^\/battles\/matches\/[^/]+\/replay/.test(path);
    if (!isStand) return real(input, init);

    if (st.delayMs) await wait(st.delayMs);
    if (st.broken && path.startsWith('/battles/runs')) return reply({ error: 'stand: server fault' }, 500);

    const body: Json = typeof init?.body === 'string' ? JSON.parse(init.body) : {};
    const r = st.run;

    if (method === 'GET') {
      if (path === '/battles/routes') return reply(st.routes);
      if (path === '/battles/daily') return reply(st.daily);
      if (path === '/battles/me') return reply({ dust: st.dust, feed: 0, owned: [], gifts: [] });
      if (path === '/battles/errands') return reply([]);
      if (path === '/battles/runs/current') return reply(r);
      if (/^\/battles\/runs\/[^/]+\/waves$/.test(path)) return reply(st.waves);
      if (/^\/battles\/runs\/[^/]+$/.test(path)) return reply(r);
      // Пересмотр: записанной партии движка у стенда нет, окно покажет «Партия не открылась».
      return reply({ error: 'stand: no recorded match' }, 404);
    }

    if (path === '/battles/runs') {
      // Новый забег: первая волна, свежий отряд.
      const fresh = SCENARIOS.find((s) => s.id === 'first')!.build(cards).run!;
      st.run = fresh;
      return reply(fresh);
    }
    if (!r) return reply({ error: 'stand: no run' }, 400);

    if (path.endsWith('/reward')) {
      const pick = r.reward[Number(body.index)];
      if (pick) {
        apply(r, pick);
        r.picks.push({ waveNo: r.waveNo, kind: pick.kind, amount: pick.amount });
      }
      r.phase = 'wave';
      r.reward = [];
      return reply(r);
    }
    if (path.endsWith('/heal')) {
      const c = r.squad.find((x) => x.cardId === body.cardId);
      if (c && r.healCharges > 0 && !c.dead && c.health < c.maxHealth) {
        c.health = c.maxHealth;
        r.healCharges -= 1;
      }
      return reply(r);
    }
    if (path.endsWith('/swap')) {
      const at2 = r.squad.findIndex((x) => x.cardId === body.cardId);
      const taken = new Set(r.squad.map((x) => x.cardId));
      const spare = cards.find((c) => !taken.has(c.id) && byId.has(c.id));
      if (at2 >= 0 && spare) {
        if (r.freeSwaps > 0) r.freeSwaps -= 1;
        else st.dust = Math.max(0, st.dust - (r.swapCost ?? 0));
        r.squad[at2] = squadCard(spare);
        r.swaps += 1;
        if (r.swapCost != null && r.freeSwaps === 0) r.swapCost += 5;
      }
      return reply(r);
    }
    if (path.endsWith('/trophy')) {
      const t = r.trophies.find((x) => x.cardId === body.cardId);
      const card = byId.get(String(body.cardId));
      if (t && card && !t.taken && st.dust >= t.priceDust) {
        st.dust -= t.priceDust;
        t.taken = true;
        if (r.squad.length < 6) r.squad.push(squadCard(card));
      }
      return reply(r);
    }
    if (path.endsWith('/abandon')) {
      r.state = 'lost';
      r.nextWave = null;
      return reply(r);
    }
    // Бой (начало волны, тренировка, ход) стенд не играет: движок живёт на сервере.
    return reply({ error: 'stand: the battle is not played here' }, 400);
  };
  return true;
}
