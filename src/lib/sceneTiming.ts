// Когда в байке менять кадр сцены.
//
// Автор загружает кадры и подписывает, что на них, а алгоритм сам решает, где в
// тексте каждому кадру проступить. «Из контекста» здесь значит три вещи, и все
// три читаются из самого текста, без внешних сервисов и без отправки текста
// куда-либо:
//
//  1. **Подпись кадра.** Слова подписи ищутся в абзацах (по основе слова, пять
//     букв: «ведьма», «ведьме», «ведьмой» — одно). Кадр проступает перед тем
//     абзацем, где о его содержимом начинают говорить.
//  2. **Устройство текста.** Новый кадр уместен там, где в тексте и так поворот:
//     перед знаком ✦ или после него, перед заголовком, после короткой реплики
//     отдельной строкой («И тогда начался настоящий кошмар.»), там, где словарь
//     соседних абзацев резко меняется.
//  3. **Длина.** Если ни подписи, ни устройства не хватает, кадры расходятся
//     поровну по длине текста, потому что чтение идёт по длине.
//
// Решает всё это одна и та же задача: из всех границ между абзацами выбрать
// по одной на переход так, чтобы они шли по возрастанию, не слипались и
// каждая была и ближе к желаемому месту, и осмысленнее соседних. Ручные
// переходы (`manual`) стоят неподвижно, остальные раскладываются между ними.
//
// Мера «где» — доля текста по числу знаков, а не номер абзаца: у версий байки
// на двух языках абзацы не обязаны совпадать, а доля текста — совпадает.
import type { TaleBlock } from '$lib/tales';

export type TimingReasonCode = 'manual' | 'caption' | 'ornament' | 'heading' | 'punch' | 'topic' | 'even';

export interface TimingReason {
  code: TimingReasonCode;
  /** Для `caption` — слово подписи, которое нашлось в тексте. */
  word?: string;
}

export interface TimingInput {
  blocks: TaleBlock[];
  /** Подписи кадров по порядку (кадров на один больше, чем переходов). */
  captions: string[];
  /** Текущие переходы: у ручных `at` не меняется. */
  transitions: { at: number; manual?: boolean }[];
}

export interface TimingResult {
  at: number[];
  reasons: TimingReason[];
}

/** Не ближе друг к другу, чем эта доля текста: иначе два перехода сливаются. */
const MIN_GAP = 0.06;
const STEM = 5;

const stemsOf = (text: string): string[] =>
  (text.toLowerCase().match(/[a-zа-яё]{4,}/g) ?? []).map((w) => w.slice(0, STEM));

function textOf(b: TaleBlock): string {
  return b.kind === 'p' || b.kind === 'heading' || b.kind === 'quote' ? b.text : '';
}

interface Boundary {
  /** Граница стоит после блока с этим номером. */
  i: number;
  /** Доля текста до границы. */
  frac: number;
  score: number;
  why: TimingReasonCode;
}

/** Границы между блоками и то, насколько каждая похожа на поворот. */
export function boundariesOf(blocks: TaleBlock[]): Boundary[] {
  const n = blocks.length;
  if (n < 2) return [];
  const texts = blocks.map(textOf);
  const total = Math.max(1, texts.reduce((a, t) => a + t.length, 0));
  let run = 0;
  const frac = texts.map((t) => (run += t.length) / total);
  const stemSets = texts.map((t) => new Set(stemsOf(t)));
  const union = (from: number, to: number) => {
    const s = new Set<string>();
    for (let k = Math.max(0, from); k <= Math.min(n - 1, to); k++) stemSets[k].forEach((x) => s.add(x));
    return s;
  };

  const out: Boundary[] = [];
  for (let i = 0; i < n - 1; i++) {
    const here = blocks[i];
    const next = blocks[i + 1];
    let score = 0;
    let why: TimingReasonCode = 'even';
    const add = (v: number, code: TimingReasonCode) => {
      score += v;
      if (v >= 1.5 && (why === 'even' || v > 2)) why = code;
    };

    if (next.kind === 'ornament') add(4, 'ornament');
    if (here.kind === 'ornament') add(3, 'ornament');
    if (next.kind === 'heading') add(3.5, 'heading');
    if (here.kind === 'heading') score -= 2;
    // Короткая строка отдельным абзацем — реплика-поворот: после неё дышат.
    if (here.kind === 'p' && texts[i].length < 70 && texts[i].length > 0) add(1.8, 'punch');
    if (here.kind === 'p' && /[!?…]$/.test(texts[i].trim())) score += 0.4;

    const before = union(i - 1, i);
    const after = union(i + 1, i + 2);
    if (before.size >= 6 && after.size >= 6) {
      let common = 0;
      before.forEach((w) => after.has(w) && common++);
      const distance = 1 - common / (before.size + after.size - common);
      add(2.5 * distance, 'topic');
    }
    out.push({ i, frac: frac[i], score, why });
  }
  return out;
}

/** Где в тексте начинают говорить о том, что на кадре: номер блока и слово. */
function captionBlock(blocks: TaleBlock[], caption: string): { block: number; word: string } | null {
  const keys = [...new Set(stemsOf(caption))];
  if (keys.length === 0) return null;
  const sets = blocks.map((b) => new Set(stemsOf(textOf(b))));
  const hits = sets.map((s) => keys.filter((k) => s.has(k)));
  let best = -1;
  let bestScore = 0;
  for (let b = 0; b < blocks.length; b++) {
    const w = hits[b].length + 0.5 * (hits[b + 1]?.length ?? 0) + 0.25 * (hits[b + 2]?.length ?? 0);
    if (w > bestScore) {
      bestScore = w;
      best = b;
    }
  }
  if (best < 0 || bestScore < 1) return null;
  const first = hits[best][0] ?? hits[best + 1]?.[0] ?? hits[best + 2]?.[0] ?? '';
  return { block: best, word: first };
}

interface Candidate {
  frac: number;
  score: number;
  why: TimingReasonCode;
  fixed: boolean;
  /** Номер блока, после которого граница; −1 у ручной. */
  i: number;
}

export function placeTransitions(input: TimingInput): TimingResult {
  const m = input.transitions.length;
  const bounds = boundariesOf(input.blocks);
  const reasons: TimingReason[] = [];
  if (m === 0) return { at: [], reasons };

  // Желаемое место каждого перехода.
  const target: { frac: number; locked: boolean; block: number; word: string }[] = [];
  const total = Math.max(1, input.blocks.reduce((a, b) => a + textOf(b).length, 0));
  const cum: number[] = [];
  let run = 0;
  for (const b of input.blocks) cum.push((run += textOf(b).length) / total);
  for (let j = 0; j < m; j++) {
    const hit = captionBlock(input.blocks, input.captions[j + 1] ?? '');
    if (hit) {
      const b = Math.max(0, hit.block - 1);
      target.push({ frac: hit.block === 0 ? 0.02 : cum[b], locked: true, block: b, word: hit.word });
    } else {
      target.push({ frac: (j + 1) / (m + 1), locked: false, block: -1, word: '' });
    }
  }

  const options = (j: number): Candidate[] => {
    const t = input.transitions[j];
    if (t.manual) return [{ frac: t.at, score: 0, why: 'manual', fixed: true, i: -1 }];
    if (bounds.length === 0) return [{ frac: target[j].frac, score: 0, why: 'even', fixed: false, i: -1 }];
    return bounds.map((b) => ({ frac: b.frac, score: b.score, why: b.why, fixed: false, i: b.i }));
  };

  const cost = (j: number, c: Candidate) => {
    if (c.fixed) return 0;
    const tg = target[j];
    const alpha = tg.locked ? 12 : 6;
    let v = alpha * Math.abs(c.frac - tg.frac) - 0.35 * c.score;
    if (tg.locked && Math.abs(c.i - tg.block) > 1) v += 50;
    return v;
  };

  // Ищем самый дешёвый набор границ по возрастанию. Если зазор между
  // переходами невыполним (текст слишком короток), его ослабляем.
  const solve = (gap: number): Candidate[] | null => {
    const layers = Array.from({ length: m }, (_, j) => options(j));
    const best: number[][] = [];
    const from: number[][] = [];
    for (let j = 0; j < m; j++) {
      best.push(layers[j].map(() => Infinity));
      from.push(layers[j].map(() => -1));
      layers[j].forEach((c, k) => {
        if (j === 0) {
          best[0][k] = cost(0, c);
          return;
        }
        layers[j - 1].forEach((p, q) => {
          if (!Number.isFinite(best[j - 1][q])) return;
          const need = p.fixed && c.fixed ? 0 : gap;
          if (c.frac < p.frac + need) return;
          const v = best[j - 1][q] + cost(j, c);
          if (v < best[j][k]) {
            best[j][k] = v;
            from[j][k] = q;
          }
        });
      });
    }
    let k = best[m - 1].reduce((bi, v, idx, arr) => (v < arr[bi] ? idx : bi), 0);
    if (!Number.isFinite(best[m - 1][k])) return null;
    const out: Candidate[] = [];
    for (let j = m - 1; j >= 0; j--) {
      out[j] = layers[j][k];
      k = from[j][k];
    }
    return out;
  };

  const chosen = solve(MIN_GAP) ?? solve(0.02) ?? solve(0);
  if (!chosen) {
    return {
      at: input.transitions.map((t, j) => (t.manual ? t.at : target[j].frac)),
      reasons: input.transitions.map((t) => ({ code: t.manual ? 'manual' : 'even' })),
    };
  }
  chosen.forEach((c, j) => {
    if (c.fixed) reasons.push({ code: 'manual' });
    else if (target[j].locked && Math.abs(c.i - target[j].block) <= 1) reasons.push({ code: 'caption', word: target[j].word });
    else reasons.push({ code: c.why });
  });
  return { at: chosen.map((c) => c.frac), reasons };
}

/**
 * Длительность каждого перехода: стандартная, но не больше трёх четвертей
 * расстояния до соседнего перехода, чтобы два не наезжали друг на друга.
 */
export function spansFor(at: number[], base = 0.14): number[] {
  return at.map((a, j) => {
    const left = j === 0 ? a + 0.05 : a - at[j - 1];
    const right = j === at.length - 1 ? 1.05 - a : at[j + 1] - a;
    return Math.max(0.04, Math.min(base, 0.75 * Math.min(left, right)));
  });
}
