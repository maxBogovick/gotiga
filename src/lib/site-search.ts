// Поиск в шапке: работы, небылицы, вестник и карты одним запросом.
//
// Здесь только сопоставление — без сети и без хранилищ, чтобы его можно было
// проверить отдельно. Загружает данные и спрашивает смысловой поиск сервера
// (`/search`, тот же, что у поля на главной) компонент `SiteSearch`.
//
// Работы на сайте названы по-английски, а спрашивают их по-русски,
// по-украински, по-испански, по-немецки и по-итальянски. Поэтому основы слов переводятся словарём `WORD_STEMS` в те
// английские слова, которые стоят в названиях. Смысловой поиск сервера тоже
// понимает русский, но отвечает только про работы и не всегда сразу; словарь
// даёт мгновенный ответ, а сервер дополняет его тем, что близко по смыслу.
import { leafCopy } from '$lib/gazette';
import { normalizeQuery } from '$lib/keeper-search';
import { localized, type Lang } from '$lib/i18n';
import type { BattleCard, FigurineListItem, FigurineText, GazetteLeaf, SiteHit } from '$lib/types/api';

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
  // Испанский, немецкий, итальянский. Основы записаны без знаков над буквами:
  // `expand` сверяет и слово как есть, и слово без них (`plainLatin`).
  ['bruj', 'witch'], ['hexe', 'witch'], ['streg', 'witch'], ['jaga', 'yaga'], ['cane', 'dog'], ['cani', 'dog'],
  ['abuel', 'granny grandma'], ['oma', 'granny grandma'], ['omas', 'granny grandma'], ['grossmutter', 'granny grandma'], ['nonn', 'granny grandma'],
  ['muneca', 'doll'], ['munec', 'doll'], ['puppe', 'doll'], ['bambol', 'doll'],
  ['guardabos', 'forester'], ['forster', 'forester'], ['foerster', 'forester'],
  ['gallin', 'hen chicken'], ['huhn', 'hen chicken'], ['huhner', 'hen chicken'], ['henne', 'hen chicken'], ['pollo', 'chicken'], ['polli', 'chicken'], ['pulcin', 'chicken'], ['kuken', 'chicken'],
  ['pajar', 'bird birds'], ['vogel', 'bird birds'], ['uccell', 'bird birds'],
  ['monstru', 'frankenstein'], ['mostr', 'frankenstein'], ['ungeheuer', 'frankenstein'],
  ['cesta', 'basket'], ['cesto', 'basket'], ['cestin', 'basket'], ['korb', 'basket'], ['korbe', 'basket'],
  ['gnom', 'gnome'], ['zwerg', 'gnome'], ['vampir', 'vampire'],
  ['mago', 'wizard'], ['magos', 'wizard'], ['maghi', 'wizard'], ['zauberer', 'wizard'], ['hechicer', 'wizard'], ['stregon', 'wizard'],
  ['chaman', 'shaman'], ['schaman', 'shaman'], ['sciaman', 'shaman'],
  ['siren', 'mermaid'], ['meerjungfrau', 'mermaid'], ['nixe', 'mermaid'],
  ['gato', 'cats'], ['gata', 'cats'], ['gatos', 'cats'], ['gatt', 'cats'], ['katze', 'cats'], ['katz', 'cats'], ['kater', 'cats'],
  ['raton', 'mice'], ['maus', 'mice'], ['mause', 'mice'], ['topo', 'mice'], ['topi', 'mice'], ['topolin', 'mice'],
  ['gorrion', 'sparrow sparrows'], ['spatz', 'sparrow sparrows'], ['passer', 'sparrow sparrows'],
  ['serpient', 'snakes'], ['serpent', 'snakes'], ['schlang', 'snakes'],
  ['escob', 'broom'], ['besen', 'broom'], ['scopa', 'broom'],
  ['morter', 'mortar'], ['morser', 'mortar'], ['mortai', 'mortar'],
  ['libro', 'book books'], ['libri', 'book books'], ['buch', 'book books'], ['bucher', 'book books'],
  ['cuent', 'tales fairy'], ['marchen', 'tales fairy'], ['fiab', 'tales fairy'], ['favol', 'tales fairy'],
  ['caracol', 'snail'], ['schneck', 'snail'], ['lumac', 'snail'], ['brot', 'bread'],
  ['sastr', 'tailor'], ['schneider', 'tailor'], ['sarto', 'tailor'], ['sarta', 'tailor'],
  ['cociner', 'cooking'], ['koch', 'cooking'], ['cuoc', 'cooking'],
  ['perro', 'dog'], ['perrit', 'dog'], ['hund', 'dog'], ['cagnol', 'dog'],
  ['halcon', 'falconer'], ['falkn', 'falconer'], ['falcon', 'falconer'],
  ['jorobad', 'quazimodo notre'], ['glockner', 'quazimodo notre'], ['buckl', 'quazimodo notre'], ['gobb', 'quazimodo notre'], ['quasimod', 'quazimodo'],
  ['music', 'musician'], ['musik', 'musician'], ['flautist', 'piper'], ['pfeifer', 'piper'], ['gaiter', 'piper'], ['dudelsack', 'piper'], ['zampogn', 'piper'],
  ['rattenfanger', 'pied piper'], ['pifferaio', 'pied piper'],
  ['cavernicol', 'caveman'], ['hohlenmensch', 'caveman'], ['modelo', 'model'], ['modell', 'model'],
  ['hacha', 'axe'], ['axt', 'axe'], ['ascia', 'axe'],
  ['bailarin', 'ballerina'], ['balleri', 'ballerina'],
  ['nina', 'girl'], ['ninas', 'girl'], ['chica', 'girl'], ['madchen', 'girl'], ['ragazz', 'girl'],
  ['cerdit', 'piglet'], ['cerdo', 'piglet'], ['ferkel', 'piglet'], ['schwein', 'piglet'], ['maial', 'piglet'], ['porcell', 'piglet'],
  ['mosquet', 'musketeer porthos'], ['musket', 'musketeer porthos'], ['moschett', 'musketeer porthos'],
  ['fantasm', 'ghost'], ['geist', 'ghost'], ['gespenst', 'ghost'], ['indio', 'indian'], ['indianer', 'indian'], ['indiano', 'indian'],
  ['caperucit', 'riding hood'], ['rotkappchen', 'riding hood'], ['cappuccett', 'riding hood'],
  ['mono', 'monkey'], ['monos', 'monkey'], ['affe', 'monkey'], ['scimmi', 'monkey'],
  ['pelirroj', 'red-haired'], ['rothaarig', 'red-haired'], ['rossa', 'red-haired'], ['rosso', 'red-haired'],
  ['catedral', 'notre-dame'], ['kathedral', 'notre-dame'], ['cattedral', 'notre-dame'],
  ['negr', 'black'], ['schwarz', 'black'], ['nero', 'black'], ['nera', 'black'],
  ['viej', 'old'], ['vecch', 'old'],
  ['cocina', 'kitchen'], ['kuche', 'kitchen'], ['cucin', 'kitchen'],
  ['dama', 'lady'], ['senora', 'lady'], ['signora', 'lady'], ['hombr', 'man'], ['mann', 'man'], ['uomo', 'man'],
  ['arcill', 'polymer clay'], ['argill', 'polymer clay'], ['polimer', 'polymer clay'], ['modelliermasse', 'polymer clay'],
  ['lana', 'wool'], ['wolle', 'wool'], ['tela', 'fabric'], ['stoff', 'fabric'], ['tessut', 'fabric'],
  ['alambr', 'wire'], ['draht', 'wire'], ['acril', 'acrylic'], ['acryl', 'acrylic'],
  ['mader', 'wood'], ['holz', 'wood'], ['legno', 'wood'], ['encaj', 'lace'], ['spitze', 'lace'], ['pizzo', 'lace'],
];

/** Слово латиницей без знаков над буквами и с «ss» вместо «ß»: «muñeca» →
 *  «muneca», «Mädchen» → «madchen». Кириллицу не трогает: там «й» и «ё» —
 *  отдельные буквы, а не «и» и «е» со знаком. */
function plainLatin(token: string): string {
  if (!/^[\p{Script=Latin}\d'’-]+$/u.test(token)) return token;
  return token.normalize('NFD').replace(/\p{M}/gu, '').replace(/ß/g, 'ss');
}

/** Служебные слова запроса: «бабушка с котами» ищет бабушку и котов, а не «с». */
const STOP = new Set([
  'с', 'со', 'и', 'в', 'во', 'на', 'з', 'із', 'та', 'й', 'with', 'and', 'the', 'a', 'an', 'of',
  'con', 'y', 'e', 'el', 'la', 'los', 'las', 'de', 'del', 'un', 'una', 'il', 'lo', 'gli', 'le', 'di', 'da', 'col',
  'mit', 'und', 'der', 'die', 'das', 'den', 'dem', 'ein', 'eine', 'einen', 'von',
]);

function expand(token: string): string[] {
  const out = new Set([token]);
  const plain = plainLatin(token);
  // Недописанное слово тоже ищет («ведь» → ведьма). У латинских основ — с
  // четырёх букв: иначе английское «cat» дописывалось бы до «catedral».
  const hit = (word: string, stem: string) =>
    word.startsWith(stem) || (word.length >= (/^[a-z]/.test(stem) ? 4 : 3) && stem.startsWith(word));
  for (const [stem, en] of WORD_STEMS) {
    if (hit(token, stem) || (plain !== token && hit(plain, stem))) {
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
  if (lang !== 'en') return localized(card, 'title', lang);
  return card.titleEn || card.titleRu;
}

function cardRace(card: BattleCard, lang: Lang): string {
  if (lang === 'ru') return card.raceNameRu ?? card.raceNameEn ?? '';
  if (lang === 'uk') return card.raceNameUk ?? card.raceNameRu ?? card.raceNameEn ?? '';
  if (lang !== 'en') return localized(card, 'raceName', lang);
  return card.raceNameEn ?? card.raceNameRu ?? '';
}

export { cardTitle, cardRace };

/** Название работы рядом с листом или картой — на всех языках. */
function namesOfWork(rec: { figurineName?: string | null; figurineNameTranslations?: Partial<Record<string, string>> }): (string | null | undefined)[] {
  return [rec.figurineName, ...Object.values(rec.figurineNameTranslations ?? {})];
}

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
    .split(/[\s,.;:!?¿¡«»"“”„()]+/)
    .filter((t) => t.length >= 2 && !STOP.has(t));
  if (tokens.length === 0) return null;
  const sets = tokens.map(expand);
  const words = [...new Set(sets.flat())];
  const every = (fields: (string | null | undefined)[]) => sets.every((ws) => fields.some((f) => has(f, ws)));

  const cardOf = cardsByWork(index.cards);

  const scored: { item: FigurineListItem; why: FoundWhy; score: number }[] = [];
  for (const w of index.works) {
    const card = cardOf.get(w.id) ?? null;
    const titles = card ? [card.titleRu, card.titleEn, card.titleUk, card.titleEs, card.titleDe, card.titleIt] : [];
    const races = card ? [card.raceNameRu, card.raceNameEn, card.raceNameUk, card.raceNameEs, card.raceNameDe, card.raceNameIt] : [];
    // Тексты работы на всех языках сайта: работу находят по названию на любом.
    const texts = [w, ...Object.values(w.translations ?? {})].filter(Boolean) as FigurineText[];
    const names = texts.map((x) => x.name);
    const craft = texts.flatMap((x) => [x.material, x.technique, x.series]);
    const blurbs = texts.map((x) => x.shortText);
    if (!every([...names, ...titles, ...races, ...craft, ...blurbs])) continue;
    let score = 0;
    let why: FoundWhy | null = null;
    if (names.some((x) => has(x, words))) { score += 10; why = { kind: 'name' }; }
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
    const other = [leaf.titleRu, leaf.titleEn, leaf.titleUk, leaf.titleEs, leaf.titleDe, leaf.titleIt, leaf.dekRu, leaf.dekEn, leaf.dekUk, leaf.dekEs, leaf.dekDe, leaf.dekIt];
    const workNames = namesOfWork(leaf);
    if (!every([own.title, own.dek, own.body, ...workNames, ...other])) continue;
    const why: FoundWhy = has(own.title, words) || has(own.dek, words)
      ? { kind: 'name' }
      : workNames.some((x) => has(x, words))
        ? { kind: 'work' }
        : { kind: 'text' };
    tales.push({ item: leaf, why });
  }

  const gazette: Found<GazetteLeaf>[] = [];
  for (const leaf of index.gazette) {
    const own = leafCopy(leaf, lang);
    if (!every([own.title, own.dek, own.body, ...namesOfWork(leaf), leaf.titleRu, leaf.titleEn])) continue;
    gazette.push({ item: leaf, why: has(own.title, words) ? { kind: 'name' } : { kind: 'text' } });
  }

  const cards: Found<BattleCard>[] = [];
  for (const c of index.cards) {
    if (c.kind === 'structure') continue;
    const titles = [c.titleRu, c.titleEn, c.titleUk, c.titleEs, c.titleDe, c.titleIt];
    const races = [c.raceNameRu, c.raceNameEn, c.raceNameUk, c.raceNameEs, c.raceNameDe, c.raceNameIt];
    if (!every([...titles, ...races, ...namesOfWork(c)])) continue;
    const why: FoundWhy = titles.some((x) => has(x, words))
      ? { kind: 'name' }
      : races.some((x) => has(x, words))
        ? { kind: 'race', race: cardRace(c, lang) }
        : { kind: 'work' };
    cards.push({ item: c, why });
  }

  return { works, tales, gazette, cards, words };
}

/**
 * Результат по оценке сервера (`GET /search/site`): все виды вещей стоят в том
 * порядке, который назвал сервер, и разложены по своим разделам. Слова,
 * найденные в самом браузере (`local`), подсказывают, **почему** вещь найдена и
 * что подчёркнуть; вещь, которую нашёл только смысл, помечена словами «близко по
 * смыслу». Вещи, которой нет в указателе (скрыли после загрузки), пропускаются.
 */
export function rankFound(index: SiteIndex, hits: SiteHit[], local: SiteFound | null): SiteFound {
  const why = new Map<string, FoundWhy>();
  for (const list of [local?.works, local?.tales, local?.gazette, local?.cards]) {
    for (const f of (list ?? []) as Found<{ id: string }>[]) why.set(f.item.id, f.why);
  }
  const byId = {
    work: new Map(index.works.map((w) => [w.id, w])),
    tale: new Map(index.tales.map((l) => [l.id, l])),
    leaf: new Map(index.gazette.map((l) => [l.id, l])),
    card: new Map(index.cards.map((c) => [c.id, c])),
  };
  const out: SiteFound = { works: [], tales: [], gazette: [], cards: [], words: local?.words ?? [] };
  for (const h of hits) {
    const w: FoundWhy = why.get(h.id) ?? { kind: 'meaning' };
    if (h.kind === 'work') {
      const item = byId.work.get(h.id);
      if (item) out.works.push({ item, why: w });
    } else if (h.kind === 'tale') {
      const item = byId.tale.get(h.id);
      if (item) out.tales.push({ item, why: w });
    } else if (h.kind === 'leaf') {
      const item = byId.leaf.get(h.id);
      if (item) out.gazette.push({ item, why: w });
    } else {
      const item = byId.card.get(h.id);
      if (item) out.cards.push({ item, why: w });
    }
  }
  return out;
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
