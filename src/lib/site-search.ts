// Поиск в шапке: работы, небылицы, вестник и карты одним запросом.
//
// Здесь только сопоставление — без сети и без хранилищ, чтобы его можно было
// проверить отдельно. Загружает данные и спрашивает смысловой поиск сервера
// (`/search`, тот же, что у поля на главной) компонент `SiteSearch`.
//
// Работы на сайте названы по-английски, а спрашивают их по-русски и
// по-украински. Поэтому основы слов переводятся словарём `WORD_STEMS` в те
// английские слова, которые стоят в названиях. Смысловой поиск сервера тоже
// понимает русский, но отвечает только про работы и не всегда сразу; словарь
// даёт мгновенный ответ, а сервер дополняет его тем, что близко по смыслу.
import { leafCopy } from '$lib/gazette';
import { normalizeQuery } from '$lib/keeper-search';
import type { Lang } from '$lib/i18n';
import type { BattleCard, FigurineListItem, GazetteLeaf } from '$lib/types/api';

export interface SiteIndex {
  works: FigurineListItem[];
  tales: GazetteLeaf[];
  gazette: GazetteLeaf[];
  cards: BattleCard[];
}

export type SearchScope = 'all' | 'works' | 'tales' | 'gazette' | 'cards';

/** Почему вещь найдена — печатается под ней словами. */
export type FoundWhy =
  | { kind: 'name' }
  | { kind: 'card'; title: string }
  | { kind: 'race'; race: string }
  | { kind: 'craft' }
  | { kind: 'text' }
  | { kind: 'meaning' }
  | { kind: 'work' };

export interface Found<T> {
  item: T;
  why: FoundWhy;
}

export interface SiteFound {
  works: Found<FigurineListItem>[];
  tales: Found<GazetteLeaf>[];
  gazette: Found<GazetteLeaf>[];
  cards: Found<BattleCard>[];
  /** Слова, по которым подчёркивать совпадение (после перевода). */
  words: string[];
}

/** Сколько работ из ответа сервера брать сверх найденного словарём: дальше его
 *  список уходит в слабые совпадения. */
export const MEANING_EXTRA = 6;

// Основа слова → английские слова из названий работ и материалов.
// Русские и украинские основы вместе: запрос не обязан совпадать с языком сайта.
const WORD_STEMS: [string, string][] = [
  ['ведьм', 'witch'], ['відьм', 'witch'], ['яга', 'yaga'], ['ягу', 'yaga'], ['баба', 'baba'],
  ['бабушк', 'granny grandma'], ['бабк', 'granny grandma'], ['бабус', 'granny grandma'],
  ['старушк', 'granny grandma'], ['кукл', 'doll'], ['ляльк', 'doll'],
  ['лесник', 'forester'], ['лесов', 'forester'], ['лісник', 'forester'], ['лісов', 'forester'],
  ['кур', 'hen chicken'], ['цыпл', 'chicken'], ['птиц', 'bird birds'], ['птах', 'bird birds'],
  ['франкенш', 'frankenstein'], ['чудовищ', 'frankenstein'], ['корзин', 'basket'], ['кошик', 'basket'],
  ['гном', 'gnome'], ['вампир', 'vampire'], ['вампір', 'vampire'],
  ['маг', 'wizard'], ['волшебн', 'wizard'], ['чарівник', 'wizard'], ['чародей', 'wizard'],
  ['шаман', 'shaman'], ['русалк', 'mermaid'], ['кот', 'cats'], ['кош', 'cats'], ['кіт', 'cats'],
  ['мыш', 'mice'], ['миш', 'mice'], ['воробь', 'sparrow sparrows'], ['горобц', 'sparrow sparrows'], ['горобец', 'sparrow sparrows'],
  ['змей', 'snakes'], ['змея', 'snakes'], ['змі', 'snakes'], ['метл', 'broom'], ['мітл', 'broom'], ['ступ', 'mortar'],
  ['книг', 'book books'], ['книж', 'book books'], ['сказк', 'tales fairy'], ['казк', 'tales fairy'],
  ['улитк', 'snail'], ['равлик', 'snail'], ['хлеб', 'bread'], ['хліб', 'bread'],
  ['портн', 'tailor'], ['кравец', 'tailor'], ['кравц', 'tailor'], ['повар', 'cooking'], ['кашевар', 'cooking'], ['кухар', 'cooking'],
  ['собак', 'dog'], ['пес', 'dog'], ['пёс', 'dog'], ['сокол', 'falconer'], ['сокіл', 'falconer'],
  ['горбун', 'quazimodo notre'], ['горбань', 'quazimodo notre'], ['квазимод', 'quazimodo'], ['квазімод', 'quazimodo'],
  ['эсмеральд', 'esmeralda'], ['есмеральд', 'esmeralda'], ['агат', 'agatha'],
  ['музыкант', 'musician'], ['музикант', 'musician'], ['дудочн', 'piper'], ['дудар', 'piper'], ['крысолов', 'pied piper'], ['щуролов', 'pied piper'],
  ['волынщ', 'piper'], ['волинщ', 'piper'], ['гамлет', 'hamlet'], ['пещерн', 'caveman'], ['печерн', 'caveman'],
  ['натурщ', 'model'], ['модел', 'model'], ['секир', 'axe'], ['сокир', 'axe'], ['топор', 'axe'],
  ['балерин', 'ballerina'], ['девочк', 'girl'], ['девушк', 'girl'], ['дівчин', 'girl'],
  ['поросен', 'piglet'], ['порос', 'piglet'], ['свин', 'piglet'], ['мушкет', 'musketeer porthos'], ['портос', 'porthos'],
  ['призрак', 'ghost'], ['привиден', 'ghost'], ['привид', 'ghost'], ['индеец', 'indian'], ['індіан', 'indian'],
  ['шапочк', 'riding hood'], ['обезьян', 'monkey'], ['мавп', 'monkey'], ['рыж', 'red-haired'], ['руд', 'red-haired'],
  ['стимпанк', 'steampunk'], ['собор', 'notre-dame'], ['черн', 'black'], ['чорн', 'black'], ['стар', 'old'],
  ['кухн', 'kitchen'], ['леди', 'lady'], ['леді', 'lady'], ['дам', 'lady'], ['мужчин', 'man'], ['чолов', 'man'],
  ['поттер', 'potter'], ['эдгар', 'edgar poe'], ['едгар', 'edgar poe'], ['кролик', 'beatrix potter'],
  ['глин', 'polymer clay'], ['шерст', 'wool'], ['вовн', 'wool'], ['ткан', 'fabric'], ['тканин', 'fabric'],
  ['проволок', 'wire'], ['дріт', 'wire'], ['акрил', 'acrylic'], ['дерев', 'wood'], ['металл', 'metal'], ['метал', 'metal'],
  ['кружев', 'lace'], ['мереж', 'lace'],
];

/** Служебные слова запроса: «бабушка с котами» ищет бабушку и котов, а не «с». */
const STOP = new Set(['с', 'со', 'и', 'в', 'во', 'на', 'з', 'із', 'та', 'й', 'with', 'and', 'the', 'a', 'an', 'of']);

function expand(token: string): string[] {
  const out = new Set([token]);
  for (const [stem, en] of WORD_STEMS) {
    if (token.startsWith(stem) || (token.length >= 3 && stem.startsWith(token))) {
      for (const w of en.split(' ')) out.add(w);
    }
  }
  return [...out];
}

function has(text: string | null | undefined, words: string[]): boolean {
  if (!text) return false;
  const t = normalizeQuery(text);
  return words.some((w) => t.includes(w));
}

function cardTitle(card: BattleCard, lang: Lang): string {
  if (lang === 'ru') return card.titleRu || card.titleEn;
  if (lang === 'uk') return card.titleUk || card.titleRu || card.titleEn;
  return card.titleEn || card.titleRu;
}

function cardRace(card: BattleCard, lang: Lang): string {
  if (lang === 'ru') return card.raceNameRu ?? card.raceNameEn ?? '';
  if (lang === 'uk') return card.raceNameUk ?? card.raceNameRu ?? card.raceNameEn ?? '';
  return card.raceNameEn ?? card.raceNameRu ?? '';
}

export { cardTitle, cardRace };

/** Карта работы: у карты одна работа. Строения — не героини. */
function cardsByWork(cards: BattleCard[]): Map<string, BattleCard> {
  const map = new Map<string, BattleCard>();
  for (const c of cards) if (c.figurineId && c.kind !== 'structure') map.set(c.figurineId, c);
  return map;
}

/**
 * Найти по запросу. Каждое слово запроса должно найтись хоть в одном поле вещи
 * («бабушка с котами» — и бабушка, и коты). `meaningIds` — порядок ответа
 * смыслового поиска сервера (только работы), `null`, пока он не ответил.
 */
export function searchSite(
  index: SiteIndex,
  query: string,
  lang: Lang,
  meaningIds: string[] | null,
): SiteFound | null {
  const tokens = normalizeQuery(query)
    .split(/[\s,.;:!?«»"“”()]+/)
    .filter((t) => t.length >= 2 && !STOP.has(t));
  if (tokens.length === 0) return null;
  const sets = tokens.map(expand);
  const words = [...new Set(sets.flat())];
  const every = (fields: (string | null | undefined)[]) => sets.every((ws) => fields.some((f) => has(f, ws)));

  const cardOf = cardsByWork(index.cards);

  const scored: { item: FigurineListItem; why: FoundWhy; score: number }[] = [];
  for (const w of index.works) {
    const card = cardOf.get(w.id) ?? null;
    const titles = card ? [card.titleRu, card.titleEn, card.titleUk] : [];
    const races = card ? [card.raceNameRu, card.raceNameEn, card.raceNameUk] : [];
    const craft = [w.material, w.technique, w.series];
    if (!every([w.name, ...titles, ...races, ...craft, w.shortText])) continue;
    let score = 0;
    let why: FoundWhy | null = null;
    if (has(w.name, words)) { score += 10; why = { kind: 'name' }; }
    if (card && titles.some((x) => has(x, words))) { score += 8; why ??= { kind: 'card', title: cardTitle(card, lang) }; }
    if (card && races.some((x) => has(x, words))) { score += 5; why ??= { kind: 'race', race: cardRace(card, lang) }; }
    if (craft.some((x) => has(x, words))) { score += 2; why ??= { kind: 'craft' }; }
    if (!why) why = { kind: 'text' };
    if (w.status === 'available') score += 0.5;
    scored.push({ item: w, why, score });
  }
  scored.sort((a, b) => b.score - a.score);
  const works: Found<FigurineListItem>[] = scored.map(({ item, why }) => ({ item, why }));

  // Сервер дополняет найденное словарём тем, что близко по смыслу.
  if (meaningIds) {
    const seen = new Set(works.map((f) => f.item.id));
    const byId = new Map(index.works.map((w) => [w.id, w]));
    let extra = 0;
    for (const id of meaningIds) {
      if (extra >= MEANING_EXTRA) break;
      const w = byId.get(id);
      if (!w || seen.has(id)) continue;
      works.push({ item: w, why: { kind: 'meaning' } });
      seen.add(id);
      extra += 1;
    }
  }

  const tales: Found<GazetteLeaf>[] = [];
  for (const leaf of index.tales) {
    const own = leafCopy(leaf, lang);
    const other = [leaf.titleRu, leaf.titleEn, leaf.titleUk, leaf.dekRu, leaf.dekEn];
    if (!every([own.title, own.dek, own.body, leaf.figurineName, ...other])) continue;
    const why: FoundWhy = has(own.title, words) || has(own.dek, words)
      ? { kind: 'name' }
      : has(leaf.figurineName, words)
        ? { kind: 'work' }
        : { kind: 'text' };
    tales.push({ item: leaf, why });
  }

  const gazette: Found<GazetteLeaf>[] = [];
  for (const leaf of index.gazette) {
    const own = leafCopy(leaf, lang);
    if (!every([own.title, own.dek, own.body, leaf.figurineName, leaf.titleRu, leaf.titleEn])) continue;
    gazette.push({ item: leaf, why: has(own.title, words) ? { kind: 'name' } : { kind: 'text' } });
  }

  const cards: Found<BattleCard>[] = [];
  for (const c of index.cards) {
    if (c.kind === 'structure') continue;
    const titles = [c.titleRu, c.titleEn, c.titleUk];
    const races = [c.raceNameRu, c.raceNameEn, c.raceNameUk];
    if (!every([...titles, ...races, c.figurineName])) continue;
    const why: FoundWhy = titles.some((x) => has(x, words))
      ? { kind: 'name' }
      : races.some((x) => has(x, words))
        ? { kind: 'race', race: cardRace(c, lang) }
        : { kind: 'work' };
    cards.push({ item: c, why });
  }

  return { works, tales, gazette, cards, words };
}

export function foundTotal(f: SiteFound, scope: SearchScope = 'all'): number {
  if (scope !== 'all') return f[scope].length;
  return f.works.length + f.tales.length + f.gazette.length + f.cards.length;
}

/** Куски строки для подчёркивания совпадения: первое найденное слово. Разметки
 *  не возвращает — страница печатает куски сама, без `{@html}`. */
export function markParts(text: string, words: string[]): { text: string; hit: boolean }[] {
  if (!text) return [];
  const low = text.toLocaleLowerCase().replace(/ё/g, 'е');
  let best: [number, number] | null = null;
  for (const w of words) {
    const i = low.indexOf(w);
    if (i >= 0 && (!best || i < best[0] || (i === best[0] && w.length > best[1]))) best = [i, w.length];
  }
  if (!best) return [{ text, hit: false }];
  const [i, n] = best;
  return [
    { text: text.slice(0, i), hit: false },
    { text: text.slice(i, i + n), hit: true },
    { text: text.slice(i + n), hit: false },
  ].filter((p) => p.text);
}

/** Название работы без общих слов («Small art doll», «Tiny OOAK»): в листе
 *  поиска они стоят у каждой второй и не отличают одну от другой. */
export function shortWorkName(name: string): string {
  const s = name
    .replace(/\b(small|tiny|ooak|art|authors|doll|miniature)\b/gi, ' ')
    .replace(/\s+/g, ' ')
    .trim();
  if (!s) return name.trim();
  return s.charAt(0).toUpperCase() + s.slice(1);
}
