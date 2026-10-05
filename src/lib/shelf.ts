// Полка карт: что на ней стоит, в каком порядке и под какими заголовками.
//
// Чистые функции, без страницы: полка спрашивает их в `$derived`, и порядок
// карт на ней не зависит от того, где написан запрос. Правило отбора то же,
// что у всей комнаты: полка не торопит и ничего не обещает («осталось N»,
// «скидка»), поэтому здесь нет ни «популярного», ни «новинок» — только то, что
// человек назвал сам: слово, расу, ранг, тип и порядок.

import type { Lang } from '$lib/i18n';
import type { BattleCard, BattleCardKind, BattleMe } from '$lib/types/api';
import { cardCopy, headerCopy, pricesOf } from '$lib/battles';

/** Чьи карты: все, только свои или те, что можно взять сейчас. */
export type ShelfScope = 'all' | 'mine' | 'can';

/** `keeper` — порядок, в котором автор расставил полку руками. */
export type ShelfSort = 'tier' | 'keeper' | 'price' | 'name';

export interface ShelfQuery {
  text: string;
  scope: ShelfScope;
  raceId: string | null;
  tier: number | null;
  kind: BattleCardKind | null;
  sort: ShelfSort;
}

export const SHELF_DEFAULT: ShelfQuery = {
  text: '',
  scope: 'all',
  raceId: null,
  tier: null,
  kind: null,
  sort: 'tier',
};

/** Что-нибудь выбрано, кроме порядка: порядок — не отбор, и сбрасывать нечего. */
export function shelfNarrowed(q: ShelfQuery): boolean {
  return (
    q.text.trim() !== '' ||
    q.scope !== 'all' ||
    q.raceId !== null ||
    q.tier !== null ||
    q.kind !== null
  );
}

/** Хватает ли монет хотя бы на одну из цен карты. Ответ сервера всё равно
 *  главнее: здесь только то, что позволяет не предлагать несбыточное. */
export function canAfford(card: BattleCard, me: Pick<BattleMe, 'dust' | 'feed'> | null): boolean {
  if (!me) return false;
  return pricesOf(card).some((p) => (p.coin === 'dust' ? me.dust : me.feed) >= p.amount);
}

/** Какие расы, ранги и типы стоят на полке: чипы печатаются только для них,
 *  иначе половина чипов отвечала бы «ничего». */
export interface ShelfFacets {
  races: { id: string; name: string }[];
  tiers: number[];
  kinds: BattleCardKind[];
}

const KIND_ORDER: BattleCardKind[] = ['unit', 'spell', 'relic'];

export function shelfFacets(cards: BattleCard[], lang: Lang): ShelfFacets {
  const races = new Map<string, string>();
  const tiers = new Set<number>();
  const kinds = new Set<BattleCardKind>();
  for (const card of cards) {
    tiers.add(card.tier);
    kinds.add(card.kind);
    if (card.raceId) {
      const name = headerCopy(card, lang).race;
      // Раса без имени на этом языке чипа не получает: подпись была бы пустой.
      if (name && !races.has(card.raceId)) races.set(card.raceId, name);
    }
  }
  return {
    races: [...races].map(([id, name]) => ({ id, name })).sort((a, b) => a.name.localeCompare(b.name, lang)),
    tiers: [...tiers].sort((a, b) => a - b),
    kinds: KIND_ORDER.filter((k) => kinds.has(k)),
  };
}

function matchesText(card: BattleCard, needle: string, lang: Lang): boolean {
  const copy = cardCopy(card, lang);
  const head = headerCopy(card, lang);
  const hay = [copy.title, card.figurineName ?? '', head.race, head.type]
    .join(' ')
    .toLowerCase();
  return hay.includes(needle);
}

/**
 * Цена для порядка «по цене». Монеты разной породы не сравниваются: сначала
 * карты за пыль, потом только за корм, потом без цены, а внутри породы —
 * по цене в её монете. Вторая цена карты в счёт не идёт: «10 пыли и 1 корм»
 * стоит там же, где «10 пыли», иначе добавленный корм переставлял бы карту.
 */
function priceKey(card: BattleCard): [number, number] {
  if (card.priceDust != null && card.priceDust > 0) return [0, card.priceDust];
  if (card.priceFeed != null && card.priceFeed > 0) return [1, card.priceFeed];
  return [2, 0];
}

/**
 * Отобрать и упорядочить. Порядок устойчивый: равные карты остаются в том
 * порядке, в каком их расставил автор, — сортировка не перемешивает полку.
 */
export function shelfCards(
  cards: BattleCard[],
  q: ShelfQuery,
  lang: Lang,
  me: Pick<BattleMe, 'dust' | 'feed'> | null,
  held: ReadonlySet<string>,
): BattleCard[] {
  const needle = q.text.trim().toLowerCase();
  // Кошелёк не прочитался — «мои» и «можно взять» отвечать нечем, и пустая
  // полка под чипом, которого человек не видит, хуже полной.
  const scope: ShelfScope = me ? q.scope : 'all';
  const rank = new Map(cards.map((c, i) => [c.id, i]));
  const kept = cards.filter((c) => {
    if (scope === 'mine' && !held.has(c.id)) return false;
    if (scope === 'can' && (held.has(c.id) || !canAfford(c, me))) return false;
    if (q.raceId && c.raceId !== q.raceId) return false;
    if (q.tier !== null && c.tier !== q.tier) return false;
    if (q.kind && c.kind !== q.kind) return false;
    return needle === '' || matchesText(c, needle, lang);
  });
  const own = (c: BattleCard) => rank.get(c.id) ?? 0;
  const by: Record<ShelfSort, (a: BattleCard, b: BattleCard) => number> = {
    keeper: (a, b) => own(a) - own(b),
    tier: (a, b) => a.tier - b.tier || own(a) - own(b),
    price: (a, b) => {
      const [ac, ap] = priceKey(a);
      const [bc, bp] = priceKey(b);
      return ac - bc || ap - bp || own(a) - own(b);
    },
    name: (a, b) =>
      cardCopy(a, lang).title.localeCompare(cardCopy(b, lang).title, lang) || own(a) - own(b),
  };
  return kept.sort(by[q.sort]);
}

/** `all` — единственная группа без заголовка: гостю делить нечего. */
export type ShelfGroupId = 'all' | 'mine' | 'can' | 'later';

export interface ShelfGroup {
  id: ShelfGroupId;
  cards: BattleCard[];
}

/**
 * Свои, затем те, что можно взять, затем те, на которые пока не хватает.
 * Карта без цены — в последней: «не взять» верно и для неё.
 */
export function shelfGroups(
  cards: BattleCard[],
  me: Pick<BattleMe, 'dust' | 'feed'> | null,
  held: ReadonlySet<string>,
): ShelfGroup[] {
  if (!me) return cards.length ? [{ id: 'all', cards }] : [];
  const mine: BattleCard[] = [];
  const can: BattleCard[] = [];
  const later: BattleCard[] = [];
  for (const card of cards) {
    if (held.has(card.id)) mine.push(card);
    else if (canAfford(card, me)) can.push(card);
    else later.push(card);
  }
  const groups: ShelfGroup[] = [
    { id: 'mine', cards: mine },
    { id: 'can', cards: can },
    { id: 'later', cards: later },
  ];
  return groups.filter((g) => g.cards.length > 0);
}
