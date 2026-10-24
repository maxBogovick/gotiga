import { get } from 'svelte/store';
import { lang as langStore, quote as quoteIn } from '$lib/i18n';
// Общее для «Спросить сайт»: виды ответа и ссылки, склонения, ссылки на
// вещи сайта, слова-пустышки вопроса. Отдельно от `site-ask.ts`, потому что
// ответы про страницу, переходы и подбор живут в своих файлах и все они берут
// отсюда одно и то же: общая часть в одном месте, а не копия в каждом.
import { workBase } from '$lib/workText';
import { localized, pick, type Lang } from '$lib/i18n';
import { readingMinutes } from '$lib/tales';
import { leafCopy, leafHref } from '$lib/gazette';
import { figurineHref } from '$lib/figurineHref';
import { cardHref, etudeHref } from '$lib/world';
import { cardRace, cardTitle, shortWorkName } from '$lib/site-search';
import type { AskIndex } from '$lib/site-index';
export type { Lang };
import type { BattleCard, BattleChallenge, FigurineListItem, GazetteLeaf } from '$lib/types/api';

export type AskIntent = 'summary' | 'start' | 'new' | 'tale' | 'play' | 'search' | 'count' | 'about' | 'page' | 'go' | 'match' | 'chat' | 'catalog' | 'none';

export type AskLinkKind = 'work' | 'tale' | 'leaf' | 'card' | 'etude' | 'section' | 'question' | 'choice';

export interface AskLink {
  kind: AskLinkKind;
  label: string;
  hint?: string;
  /** У `question` и `choice` пусто: нажатие задаёт `label` как новый вопрос. */
  href: string;
  /** Ответы подбора до этого выбора включительно (только у `choice`). */
  quiz?: string[];
  /** Адрес миниатюры (только у работ); страница разрешает его сама. */
  image?: string | null;
}

/** Что окно делает само после ответа: переход или смена языка. */
export type AskAction = { kind: 'go'; href: string } | { kind: 'lang'; lang: Lang };

export interface AskReply {
  intent: AskIntent;
  /** Ответа нет: вопрос не разобран или по теме ничего не нашлось. Такие вопросы
   *  записываются (без имени и почты), чтобы научить сайт отвечать. */
  missed?: boolean;
  /** Команда: окно говорит, что сделает, и через мгновение делает. */
  action?: AskAction;
  /** Абзацы ответа. Печатаются по словам, поэтому без разметки. */
  paragraphs: string[];
  links: AskLink[];
  /** «Возможно, вы спрашивали»: вопросы-предложения от каталога, когда ответ
   *  собран по словам и его могло быть недостаточно. Нажатие задаёт вопрос. */
  suggest?: AskLink[];
}

/** Сколько ссылок под ответом: больше шести не читают. */
export const LINKS_MAX = 6;
export const SRC = 'site_ask';

// ── Слова ──────────────────────────────────────────────────────────────────

/** Одна фраза на шести языках: русский, украинский, английский, испанский,
 *  немецкий, итальянский. Недостающий язык говорит по-английски. */
export const say = (lang: Lang, ru: string, uk: string, en: string, es?: string, de?: string, it?: string) =>
  pick(lang, { en, ru, uk, es, de, it });

/** Те же шесть слов кортежем — для подписей, которые лежат в таблицах. */
export type Said = [ru: string, uk: string, en: string, es?: string, de?: string, it?: string];

export type Forms = {
  en: [string, string];
  ru: [string, string, string];
  uk: [string, string, string];
  es?: [string, string];
  de?: [string, string];
  it?: [string, string];
};

/** «63 работы»: число и слово в нужной форме. */
export function amount(lang: Lang, n: number, f: Forms): string {
  // Три формы слова — у русского и украинского; у остальных языков две, как у английского.
  if (lang !== 'ru' && lang !== 'uk') {
    const two = (lang === 'en' ? f.en : f[lang]) ?? f.en;
    return `${n} ${n === 1 ? two[0] : two[1]}`;
  }
  const forms = f[lang];
  const m10 = n % 10;
  const m100 = n % 100;
  const i = m10 === 1 && m100 !== 11 ? 0 : m10 >= 2 && m10 <= 4 && (m100 < 12 || m100 > 14) ? 1 : 2;
  return `${n} ${forms[i]}`;
}

export const WORKS: Forms = { en: ['work', 'works'], ru: ['работа', 'работы', 'работ'], uk: ['робота', 'роботи', 'робіт'], es: ['obra', 'obras'], de: ['Werk', 'Werke'], it: ['opera', 'opere'] };
export const TALES: Forms = { en: ['tall tale', 'tall tales'], ru: ['небылица', 'небылицы', 'небылиц'], uk: ['небилиця', 'небилиці', 'небилиць'], es: ['cuento', 'cuentos'], de: ['Geschichte', 'Geschichten'], it: ['racconto', 'racconti'] };
export const LEAVES: Forms = { en: ['gazette leaf', 'gazette leaves'], ru: ['лист вестника', 'листа вестника', 'листов вестника'], uk: ['аркуш вісника', 'аркуші вісника', 'аркушів вісника'], es: ['hoja de la Gaceta', 'hojas de la Gaceta'], de: ['Blatt der Zeitung', 'Blätter der Zeitung'], it: ['foglio della Gazzetta', 'fogli della Gazzetta'] };
export const CARDS: Forms = { en: ['card', 'cards'], ru: ['карта', 'карты', 'карт'], uk: ['карта', 'карти', 'карт'], es: ['carta', 'cartas'], de: ['Karte', 'Karten'], it: ['carta', 'carte'] };
export const ETUDES: Forms = { en: ['study', 'studies'], ru: ['этюд', 'этюда', 'этюдов'], uk: ['етюд', 'етюди', 'етюдів'], es: ['étude', 'études'], de: ['Etüde', 'Etüden'], it: ['étude', 'étude'] };
export const MINUTES: Forms = { en: ['minute', 'minutes'], ru: ['минута', 'минуты', 'минут'], uk: ['хвилина', 'хвилини', 'хвилин'], es: ['minuto', 'minutos'], de: ['Minute', 'Minuten'], it: ['minuto', 'minuti'] };

export const n_works = (lang: Lang, n: number) => amount(lang, n, WORKS);
export const n_tales = (lang: Lang, n: number) => amount(lang, n, TALES);
export const n_leaves = (lang: Lang, n: number) => amount(lang, n, LEAVES);
export const n_cards = (lang: Lang, n: number) => amount(lang, n, CARDS);
export const n_etudes = (lang: Lang, n: number) => amount(lang, n, ETUDES);
export const n_minutes = (lang: Lang, n: number) => amount(lang, n, MINUTES);

export const short_min = (lang: Lang, n: number) => `${n} ${say(lang, 'мин', 'хв', 'min', 'min', 'Min.', 'min')}`;

export function joinList(lang: Lang, items: string[]): string {
  if (items.length <= 1) return items[0] ?? '';
  const and = say(lang, ' и ', ' і ', ' and ', ' y ', ' und ', ' e ');
  return `${items.slice(0, -1).join(', ')}${and}${items[items.length - 1]}`;
}

/** Кавычки языка читателя: ответ собирается на его языке. */
export const quote = (s: string) => quoteIn(get(langStore), s);

// ── Данные ─────────────────────────────────────────────────────────────────

export const workName = (w: FigurineListItem) => shortWorkName(w.name);
export const leafTitle = (l: GazetteLeaf, lang: Lang) => leafCopy(l, lang).title;

export function etudeTitle(e: BattleChallenge, lang: Lang): string {
  if (lang === 'ru') return e.titleRu || e.titleEn;
  if (lang === 'uk') return e.titleUk || e.titleRu || e.titleEn;
  if (lang !== 'en') return localized(e, 'title', lang);
  return e.titleEn || e.titleRu;
}

export const newest = <T>(items: T[], at: (x: T) => string | null | undefined): T[] =>
  [...items].sort((a, b) => (at(b) ?? '').localeCompare(at(a) ?? ''));

export const heroCards = (index: AskIndex): BattleCard[] => index.cards.filter((c) => c.kind !== 'structure');

export const etudesInOrder = (index: AskIndex): BattleChallenge[] =>
  [...index.etudes].sort((a, b) => (a.sortOrder ?? 1e9) - (b.sortOrder ?? 1e9));

export const talesNewest = (index: AskIndex) => newest(index.tales, (l) => l.publishedAt ?? l.createdAt);

export function workLink(w: FigurineListItem, lang: Lang, hint?: string): AskLink {
  return {
    kind: 'work',
    label: workName(w),
    hint: hint ?? say(lang, 'работа', 'робота', 'work', 'obra', 'Werk', 'opera'),
    href: figurineHref(w, SRC),
    image: w.thumbUrl ?? w.faceImageUrl,
  };
}

export function taleLink(l: GazetteLeaf, lang: Lang): AskLink {
  const own = leafCopy(l, lang);
  const min = readingMinutes(own.body);
  const word = say(lang, 'небылица', 'небилиця', 'tall tale', 'cuento', 'Geschichte', 'racconto');
  return {
    kind: 'tale',
    label: own.title,
    hint: min ? `${word} · ${short_min(lang, min)}` : word,
    href: leafHref(l, SRC),
  };
}

export function leafLink(l: GazetteLeaf, lang: Lang): AskLink {
  return { kind: 'leaf', label: leafTitle(l, lang), hint: say(lang, 'вестник', 'вісник', 'gazette', 'Gaceta', 'Zeitung', 'Gazzetta'), href: leafHref(l, SRC) };
}

export function cardLink(c: BattleCard, lang: Lang): AskLink {
  const race = cardRace(c, lang);
  const word = say(lang, 'карта', 'карта', 'card', 'carta', 'Karte', 'carta');
  return { kind: 'card', label: cardTitle(c, lang), hint: race ? `${word} · ${race}` : word, href: cardHref(c) };
}

export function etudeLink(e: BattleChallenge, lang: Lang): AskLink {
  return { kind: 'etude', label: etudeTitle(e, lang), hint: say(lang, 'этюд', 'етюд', 'study', 'étude', 'Etüde', 'étude'), href: etudeHref(e) };
}


/** Слова, которые ничего не ищут: вопрос вокруг темы, а не сама тема. */
export const FILLER = new Set([
  'что', 'нибудь', 'что-нибудь', 'какой', 'какую', 'какие', 'какая', 'покажи', 'показать', 'покажите', 'найди', 'найти',
  'хочу', 'мне', 'есть', 'ли', 'про', 'о', 'об', 'обо', 'для', 'за', 'на', 'у', 'я', 'можно', 'нужно', 'расскажи',
  'подскажи', 'где', 'как', 'мы', 'вы', 'ты', 'это', 'эта', 'этот', 'и', 'в', 'во', 'с', 'со', 'по', 'к', 'из', 'или', 'бы', 'а',
  'що', 'яка', 'які', 'знайди', 'мені', 'є', 'чи', 'можна', 'розкажи', 'як', 'це', 'та', 'й', 'із', 'з', 'до',
  'show', 'me', 'find', 'what', 'is', 'are', 'there', 'any', 'a', 'an', 'the', 'some', 'about', 'for', 'i', 'want', 'to', 'can',
  'please', 'tell', 'of', 'in', 'on', 'with', 'and', 'do', 'you', 'have', 'how', 'where',
  'qué', 'que', 'algo', 'algún', 'alguna', 'muéstrame', 'muestrame', 'enséñame', 'ensename', 'busca', 'buscar', 'quiero', 'me',
  'hay', 'sobre', 'de', 'del', 'para', 'con', 'y', 'el', 'la', 'los', 'las', 'un', 'una', 'unos', 'unas', 'en', 'por', 'cómo',
  'como', 'dónde', 'donde', 'puedo', 'dime', 'tienes', 'es', 'por favor',
  'was', 'zeig', 'zeige', 'mir', 'finde', 'such', 'suche', 'gibt', 'etwas', 'über', 'für', 'mit', 'und', 'der', 'die', 'das',
  'ein', 'eine', 'einen', 'ich', 'möchte', 'will', 'kann', 'wie', 'wo', 'bitte', 'von', 'im', 'zu', 'hast', 'du',
  'cosa', 'che', 'qualcosa', 'mostrami', 'fammi', 'vedere', 'trova', 'cerca', 'voglio', 'mi', 'c’è', 'sono', 'su', 'di',
  'della', 'per', 'e', 'il', 'lo', 'gli', 'le', 'uno', 'dove', 'posso', 'dimmi', 'hai', 'ci', 'da', 'qual', 'quale', 'quali',
]);

export function tokensOf(q: string): string[] {
  return q.split(/[\s,.;:!?¿¡«»"“”„()]+/).filter(Boolean);
}

export const startsWithAny = (token: string, stems: string[]) => stems.some((s) => token.startsWith(s));

// Названия материалов в данных английские; читателю называем по-своему.
export const MATERIALS: Record<string, [string, string, string, string, string, string]> = {
  'polymer clay': ['полимерная глина', 'полімерна глина', 'polymer clay', 'arcilla polimérica', 'Polymerton', 'argilla polimerica'],
  'acrylic paint': ['акриловые краски', 'акрилові фарби', 'acrylic paint', 'pintura acrílica', 'Acrylfarbe', 'colori acrilici'],
  fabric: ['ткань', 'тканина', 'fabric', 'tela', 'Stoff', 'tessuto'],
  wool: ['шерсть', 'вовна', 'wool', 'lana', 'Wolle', 'lana'],
  varnish: ['лак', 'лак', 'varnish', 'barniz', 'Lack', 'vernice'],
  wire: ['проволока', 'дріт', 'wire', 'alambre', 'Draht', 'filo metallico'],
  lace: ['кружево', 'мереживо', 'lace', 'encaje', 'Spitze', 'pizzo'],
  wood: ['дерево', 'дерево', 'wood', 'madera', 'Holz', 'legno'],
  metal: ['металл', 'метал', 'metal', 'metal', 'Metall', 'metallo'],
};

export function materialsOf(index: AskIndex, lang: Lang): string[] {
  const counts = new Map<string, number>();
  for (const w of index.works) {
    for (const raw of workBase(w, 'material').split(',')) {
      const key = raw.trim().toLowerCase();
      if (MATERIALS[key]) counts.set(key, (counts.get(key) ?? 0) + 1);
    }
  }
  return [...counts.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5)
    .map(([k]) => say(lang, MATERIALS[k][0], MATERIALS[k][1], MATERIALS[k][2], MATERIALS[k][3], MATERIALS[k][4], MATERIALS[k][5]));
}

/** Рост фигурок из строк «Approx. 10-10.5 cm tall»: только если примеров хватает. */
export function heightsOf(index: AskIndex): [number, number] | null {
  const values: number[] = [];
  for (const w of index.works) {
    const m = workBase(w, 'dimensions').match(/(\d+(?:[.,]\d+)?)(?:\s*[-–]\s*(\d+(?:[.,]\d+)?))?\s*(?:cm|см)/i);
    if (!m) continue;
    values.push(Number(m[1].replace(',', '.')));
    if (m[2]) values.push(Number(m[2].replace(',', '.')));
  }
  if (values.length < 3) return null;
  return [Math.min(...values), Math.max(...values)];
}

