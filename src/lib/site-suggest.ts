// Подсказки при вводе в поиске шапки: «хвост» слова серым после курсора и ряд
// готовых вариантов под полем.
//
// Подсказывает то, что на сайте есть: названия небылиц и карт, расы, работы и
// несколько слов-тем, по которым люди ищут («ведьма», «гном», «шерсть»).
// Слова берутся из тех же данных, что и сам поиск, поэтому подсказанное слово
// всегда что-то находит.
//
// Без сети и без хранилищ.
import { leafCopy } from '$lib/gazette';
import { cardRace, cardTitle, shortWorkName, type SiteIndex } from '$lib/site-search';
import type { Lang } from '$lib/i18n';

export interface Vocab {
  text: string;
  low: string;
  weight: number;
}

/** Слова-темы, по которым ищут чаще всего; на каждом языке свои. */
const THEMES: { en: string[] } & Partial<Record<Lang, string[]>> = {
  ru: ['ведьма', 'гном', 'вампир', 'русалка', 'бабушка', 'кот', 'птица', 'шерсть', 'метла', 'лесник', 'призрак', 'маг', 'шаман', 'книга', 'курица', 'собака', 'портной', 'кукла', 'Франкенштейн', 'Баба Яга'],
  uk: ['відьма', 'гном', 'вампір', 'русалка', 'бабуся', 'кіт', 'птах', 'вовна', 'мітла', 'лісник', 'привид', 'маг', 'шаман', 'книга', 'курка', 'собака', 'кравець', 'лялька', 'Франкенштейн', 'Баба Яга'],
  en: ['witch', 'gnome', 'vampire', 'mermaid', 'granny', 'cat', 'bird', 'wool', 'broom', 'forester', 'ghost', 'wizard', 'shaman', 'book', 'hen', 'dog', 'tailor', 'doll', 'Frankenstein', 'Baba Yaga'],
  es: ['bruja', 'gnomo', 'vampiro', 'sirena', 'abuela', 'gato', 'pájaro', 'lana', 'escoba', 'guardabosques', 'fantasma', 'mago', 'chamán', 'libro', 'gallina', 'perro', 'sastre', 'muñeca', 'Frankenstein', 'Baba Yaga'],
  de: ['Hexe', 'Gnom', 'Vampir', 'Meerjungfrau', 'Oma', 'Katze', 'Vogel', 'Wolle', 'Besen', 'Förster', 'Gespenst', 'Zauberer', 'Schamane', 'Buch', 'Huhn', 'Hund', 'Schneider', 'Puppe', 'Frankenstein', 'Baba Jaga'],
  it: ['strega', 'gnomo', 'vampiro', 'sirena', 'nonna', 'gatto', 'uccello', 'lana', 'scopa', 'guardaboschi', 'fantasma', 'mago', 'sciamano', 'libro', 'gallina', 'cane', 'sarto', 'bambola', 'Frankenstein', 'Baba Jaga'],
};

const THEME_WEIGHT = 5;

export function buildVocabulary(index: SiteIndex, lang: Lang): Vocab[] {
  const seen = new Map<string, Vocab>();
  const add = (raw: string | null | undefined, weight: number) => {
    const text = (raw ?? '').replace(/\s+/g, ' ').trim();
    if (text.length < 3 || text.length > 48) return;
    const low = text.toLocaleLowerCase();
    // Строчные буквы разной длины («İ») сдвинули бы хвост подсказки: такие пропускаем.
    if (low.length !== text.length) return;
    const have = seen.get(low);
    if (have) have.weight = Math.max(have.weight, weight);
    else seen.set(low, { text, low, weight });
  };

  // Темы подсказываются на тех языках, на которых по ним ищется (словарь поиска — русский, украинский, испанский, немецкий, итальянский); остальным — английские.
  for (const t of THEMES[lang] ?? THEMES.en) add(t, THEME_WEIGHT);
  for (const c of index.cards) {
    if (c.kind === 'structure') continue;
    add(cardRace(c, lang), 4);
    add(cardTitle(c, lang), 3);
  }
  for (const leaf of index.tales) add(leafCopy(leaf, lang).title, 3);
  for (const w of index.works) add(shortWorkName(w.name), 2);
  for (const leaf of index.gazette) add(leafCopy(leaf, lang).title, 1);
  return [...seen.values()];
}

const wordsOf = (low: string) => low.split(/[\s—–\-,.;:!?«»"“”()]+/).filter(Boolean);

export interface Suggestion {
  /** Что дописать к набранному; пусто, если нечего. */
  ghost: string;
  /** До пяти вариантов целиком. */
  chips: string[];
}

const NONE: Suggestion = { ghost: '', chips: [] };
const CHIPS_MAX = 5;

export function suggest(vocab: Vocab[], query: string): Suggestion {
  const typed = query.replace(/^\s+/, '');
  const qLow = typed.toLocaleLowerCase();
  if (qLow.trim().length < 2 || vocab.length === 0) return NONE;
  const tokens = wordsOf(qLow);
  if (tokens.length === 0) return NONE;
  const endsWithSpace = /\s$/.test(typed);
  const last = endsWithSpace ? '' : tokens[tokens.length - 1];

  // Целиком: вариант начинается с набранного.
  const whole = vocab
    .filter((e) => e.low.startsWith(qLow) && e.low !== qLow)
    .sort((a, b) => b.weight - a.weight || a.text.length - b.text.length);

  // По словам: каждое набранное слово — начало какого-то слова варианта.
  const byWords = vocab
    .filter((e) => {
      if (e.low === qLow || e.low.startsWith(qLow)) return false;
      const ws = wordsOf(e.low);
      return tokens.every((t) => ws.some((w) => w.startsWith(t)));
    })
    .sort((a, b) => b.weight - a.weight || a.text.length - b.text.length);

  const chips = [...whole, ...byWords].slice(0, CHIPS_MAX).map((e) => e.text);

  let ghost = '';
  if (whole[0]) {
    ghost = whole[0].text.slice(typed.length);
  } else if (last.length >= 2) {
    // Дописываем последнее слово по самому весомому слову из вариантов.
    const counts = new Map<string, number>();
    for (const e of vocab) {
      for (const w of wordsOf(e.low)) {
        if (w.length > last.length && w.startsWith(last)) counts.set(w, (counts.get(w) ?? 0) + e.weight);
      }
    }
    const best = [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].length - b[0].length)[0];
    if (best) ghost = best[0].slice(last.length);
  }
  return { ghost, chips };
}
