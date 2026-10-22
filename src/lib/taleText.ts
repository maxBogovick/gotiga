// Текст небылицы как простые абзацы и краткое содержание из него.
//
// Краткое содержание — не пересказ нейросетью, а выбор: из текста берутся
// предложения, в которых больше всего слов, повторяющихся по всему тексту
// (так находят о чём он), по одному из начала, середины и конца, чтобы было
// видно завязку и итог. Предложения остаются слово в слово: ничего не
// придумано, и сказанное в кратком содержании есть в самой небылице.
//
// Без сети и без хранилищ, чтобы проверять отдельно. Нужны двум местам:
// ответу «о чём эта страница» и чтению вслух.
import { renderTale, type TaleBlock, type TaleRun } from '$lib/tales';

export interface TalePlain {
  /** Абзацы прозы по порядку: без заголовков, реплик и ссылок на дом. */
  paragraphs: string[];
  /** Реплики, которые автор выделил особняком. */
  quotes: string[];
  /** Всё, что читается вслух, по порядку: заголовки, абзацы и реплики. */
  spoken: string[];
}

function runsText(runs: TaleRun[]): string {
  return runs
    .filter((r): r is Extract<TaleRun, { kind: 'text' }> => r.kind === 'text')
    .map((r) => r.text)
    .join('')
    .replace(/\s+/g, ' ')
    .trim();
}

function blockText(b: TaleBlock): string {
  if (b.kind === 'p' || b.kind === 'heading' || b.kind === 'quote') return runsText(b.runs);
  return '';
}

export function talePlain(body: string | null | undefined): TalePlain {
  const blocks = renderTale(body);
  const out: TalePlain = { paragraphs: [], quotes: [], spoken: [] };
  for (const b of blocks) {
    const text = blockText(b);
    if (!text) continue;
    out.spoken.push(text);
    if (b.kind === 'p') out.paragraphs.push(text);
    else if (b.kind === 'quote') out.quotes.push(text);
  }
  return out;
}

// ── Краткое содержание ────────────────────────────────────────────────────

const STOP = new Set([
  'это', 'как', 'что', 'так', 'его', 'она', 'они', 'оно', 'был', 'была', 'были', 'было', 'будет', 'для', 'при', 'про', 'или', 'тоже', 'только',
  'когда', 'если', 'потому', 'чтобы', 'этот', 'этой', 'этом', 'всё', 'все', 'всех', 'ещё', 'уже', 'даже', 'очень', 'можно', 'нужно', 'после',
  'який', 'яка', 'яке', 'були', 'було', 'буде', 'його', 'вона', 'вони', 'для', 'при', 'про', 'чи', 'також', 'тільки', 'коли', 'якщо', 'щоб',
  'this', 'that', 'with', 'from', 'have', 'were', 'was', 'will', 'they', 'them', 'their', 'there', 'then', 'than', 'when', 'what', 'which',
  // es
  'para', 'pero', 'como', 'cuando', 'porque', 'este', 'esta', 'esto', 'estos', 'estas', 'ella', 'ellos', 'ellas', 'eran', 'fueron', 'había',
  'tenía', 'sobre', 'entre', 'también', 'solo', 'sólo', 'muy', 'todo', 'todos', 'todas', 'desde', 'hasta', 'donde', 'aunque', 'después', 'luego',
  // de
  'dass', 'nicht', 'eine', 'einer', 'einem', 'einen', 'dieser', 'diese', 'dieses', 'aber', 'auch', 'noch', 'schon', 'wenn', 'weil', 'oder',
  'sein', 'seine', 'ihre', 'sind', 'waren', 'wurde', 'wird', 'haben', 'hatte', 'über', 'nach', 'dann', 'denn', 'sehr', 'alle', 'immer',
  // it
  'come', 'quando', 'perché', 'questo', 'questa', 'questi', 'queste', 'quello', 'quella', 'anche', 'ancora', 'già', 'sono', 'erano', 'aveva',
  'avevano', 'della', 'delle', 'degli', 'dello', 'nella', 'nelle', 'sulla', 'tutto', 'tutti', 'tutte', 'molto', 'dopo', 'poi', 'mentre',
]);

function sentencesOf(paragraph: string): string[] {
  return paragraph
    .split(/(?<=[.!?…])["»”’)]*\s+(?=["«“(]?[\p{Lu}\d])/u)
    .map((s) => s.trim())
    .filter(Boolean);
}

function wordsOf(sentence: string): string[] {
  return (sentence.toLocaleLowerCase().replace(/ё/g, 'е').match(/[\p{L}\p{N}]{4,}/gu) ?? []).filter((w) => !STOP.has(w));
}

/** Основа слова: первые пять букв — «ведьма», «ведьмы», «ведьму» считаются одним. */
const stem = (w: string) => w.slice(0, 5);

export interface TaleSummary {
  /** Предложения из текста, в том порядке, в каком они в нём стоят. */
  sentences: string[];
  /** Выделенная автором реплика, если она есть. */
  quote: string | null;
}

const MAX_SENTENCE = 230;

/**
 * Три предложения: лучшее из начала, середины и конца текста. В короткой
 * небылице (до пяти предложений) — первые два: выбирать там не из чего.
 */
export function summarizeTale(paragraphs: string[], quotes: string[], sentencesWanted = 3): TaleSummary {
  const all: { text: string; at: number }[] = [];
  paragraphs.forEach((p, pi) => {
    sentencesOf(p).forEach((text, si) => {
      const length = [...text].length;
      if (length >= 28 && length <= MAX_SENTENCE) all.push({ text, at: pi === 0 && si === 0 ? 0 : all.length });
    });
  });
  const quote = quotes.find((q) => [...q].length >= 12 && [...q].length <= 200) ?? null;
  if (all.length === 0) return { sentences: [], quote };
  if (all.length <= 5) return { sentences: all.slice(0, 2).map((s) => s.text), quote };

  const freq = new Map<string, number>();
  for (const s of all) for (const w of new Set(wordsOf(s.text).map(stem))) freq.set(w, (freq.get(w) ?? 0) + 1);

  const score = (text: string) => {
    const words = [...new Set(wordsOf(text).map(stem))];
    if (!words.length) return 0;
    // Слова, которые встречаются хотя бы в двух предложениях, говорят о теме;
    // единичные — о подробностях. Длинное предложение не выигрывает за длину.
    const weight = words.reduce((n, w) => n + ((freq.get(w) ?? 1) > 1 ? (freq.get(w) as number) : 0), 0);
    return weight / Math.sqrt(words.length);
  };

  const parts = Math.max(1, sentencesWanted);
  const chosen: number[] = [];
  for (let part = 0; part < parts; part += 1) {
    const from = Math.floor((all.length * part) / parts);
    const to = Math.floor((all.length * (part + 1)) / parts);
    let best = -1;
    let bestScore = -1;
    for (let i = from; i < to; i += 1) {
      // Начало текста всегда берётся первым предложением: это завязка.
      const sc = part === 0 && i === 0 ? Number.MAX_SAFE_INTEGER : score(all[i].text);
      if (sc > bestScore) {
        best = i;
        bestScore = sc;
      }
    }
    if (best >= 0) chosen.push(best);
  }
  return { sentences: [...new Set(chosen)].sort((a, b) => a - b).map((i) => all[i].text), quote };
}
