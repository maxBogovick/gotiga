// Сцены в байках: слоёная картинка из вырезанной фигурки работы, которая
// оживает по мере чтения (`TaleStage.svelte`).
//
// Сцены из кадров правятся на столе рассказов и лежат в базе (`tale_scenes`);
// здесь остались сцены, у которых кадров нет, — движение поверх вырезанной
// фигуры, — и четыре сцены из кадров, с которых всё началось. База главнее:
// если у байки есть своя сцена, показывается она.
//
// Пока это список в коде, по слагу байки: сцена опытная, и стол рассказов
// ничего о ней не знает. Если сцены приживутся, место им — поле листа, как у
// порядка полки, а не ещё одна таблица в этом файле.

/**
 * Движение плиты на полях (`TaleScenePlate.svelte`): у каждой байки своё и про
 * её смысл, а не про её сюжет.
 */
import type { Lang } from '$lib/i18n';
import { firstTextLang } from '$lib/i18n';
import type { SceneTransitionType, TaleSceneData } from '$lib/types/api';

export type PlateEffect =
  | 'frames'
  | 'dusk'
  | 'print'
  | 'ruler'
  | 'rings'
  | 'fog';

/** Какая сцена и на какой слой она опирается (`glow`: жилы или трещины). */
export interface TaleScene {
  kind: 'awakening' | 'heat' | 'plate';
  /** Для `frames`: кадры по порядку (целые фотографии, не вырезки). */
  frames?: { url: string; position?: string }[];
  /** Для `frames`: переход `i` ведёт из кадра `i` в кадр `i + 1`. */
  transitions?: { type: SceneTransitionType; at: number; span: number }[];
  /** Для `plate`: пропорции плиты `ширина / высота`, если не 3 / 4. */
  ratio?: string;
  /** Для кадров: `object-position`, если фигура стоит не по центру. */
  position?: string;
  /** Для `plate`: какое движение. */
  effect?: PlateEffect;
  /** Для `plate`: фигура обрезана по краям кадра и уходит за край плиты. */
  bleed?: boolean;
  /**
   * Где стоит сцена. `margin` — в колонке справа от текста вместо фото
   * работы, портретная; `inline` — широкая плита между абзацами. Вторая
   * прерывает чтение, и для новых сцен выбирается первая.
   */
  place: 'margin' | 'inline';
  /**
   * Номер блока текста, после которого стоит сцена (как `after` у
   * `TaleProse`), — свой на каждый язык: версии байки не обязаны совпадать
   * абзац в абзац, и номер одной уронил бы сцену в середину другой.
   * `uk` необязателен: пока украинского перевода нет или он идёт абзац в абзац
   * за русским, берётся русский номер. Переведённой иначе байке номер
   * называют здесь. То же у `es`, `de`, `it`: их переводят с русского.
   */
  after?: { ru: number; en: number; uk?: number; es?: number; de?: number; it?: number };
  figure: string;
  /** Слой жара или жил; плитам не нужен. */
  glow?: string;
}

function plate(effect: PlateEffect, slug: string, bleed = false): TaleScene {
  return { kind: 'plate', effect, bleed, place: 'margin', figure: `/tales/${slug}/figure.webp` };
}

export const TALE_SCENES: Record<string, TaleScene> = {
  // «Именно этот коктейль из чистой любви и концентрированного яда сработал
  // как разряд…» — после этого абзаца трещины загораются и сердце бьётся.
  // В английской версии (поле написано по-русски, но это другой текст) то же
  // место — «Он стал настоящим».
  frankenshteyneyro: {
    kind: 'awakening',
    place: 'inline',
    after: { ru: 8, en: 12 },
    figure: '/tales/frankenshteyneyro/figure.webp',
    glow: '/tales/frankenshteyneyro/veins.webp',
  },

  // «Исполню все желания»: сцена стоит на полях и нагревается вместе с
  // чтением. `after` здесь не читается — оно нужно только сценам `inline`.
  'ispolnyu-vse-zhelaniya': {
    kind: 'heat',
    place: 'margin',
    after: { ru: 7, en: 7 },
    figure: '/tales/ispolnyu-vse-zhelaniya/figure.webp',
    glow: '/tales/ispolnyu-vse-zhelaniya/cracks.webp',
  },

  // Девять остальных работ, по одной плите на байку. Номер блока здесь не
  // нужен: плита стоит на полях и идёт за чтением всей байки.
  // Два снимка одной работы, женщина и старуха, сделанные на одном столе:
  // первый медленно становится вторым по ходу чтения.
  'molodost-bystree-tvoih': {
    kind: 'plate',
    effect: 'frames',
    place: 'margin',
    figure: '/tales/molodost-bystree-tvoih/stage-1.webp',
    frames: [1,2].map((n) => ({ url: `/tales/molodost-bystree-tvoih/stage-${n}.webp` })),
    transitions: [{ type: 'fade', at: 0.5, span: 0.9 }],
  },
  // Пять кадров истории, и между каждой парой свой переход: девочка в
  // библиотеке, книга, проповедь, камень в окно, небо.
  'ya-veryu': {
    kind: 'plate',
    effect: 'frames',
    place: 'margin',
    figure: '/tales/ya-veryu/stage-1.webp',
    frames: [
      { url: '/tales/ya-veryu/stage-1.webp' },
      { url: '/tales/ya-veryu/stage-2.webp' },
      { url: '/tales/ya-veryu/stage-3.webp' },
      { url: '/tales/ya-veryu/stage-4.webp', position: '22% 50%' },
      { url: '/tales/ya-veryu/stage-5.webp', position: '48% 50%' },
    ],
    transitions: [
      { type: 'push', at: 0.16, span: 0.2 },
      { type: 'bloom', at: 0.4, span: 0.2 },
      { type: 'wipe', at: 0.64, span: 0.2 },
      { type: 'rise', at: 0.87, span: 0.18 },
    ],
  },
  // Четыре кадра: привидение на верёвке, распад, сборка, и оно уже в чёрном
  // плаще посреди цифрового дождя. Переходы — медленный одинаковый наплыв.
  'mir-ne-takoy-kak-tebe': {
    kind: 'plate',
    effect: 'frames',
    place: 'margin',
    position: '50% 40%',
    figure: '/tales/mir-ne-takoy-kak-tebe/stage-1.webp',
    frames: [1,2,3,4].map((n) => ({ url: `/tales/mir-ne-takoy-kak-tebe/stage-${n}.webp` })),
    transitions: [
      { type: 'fade', at: 0.2, span: 0.3 },
      { type: 'fade', at: 0.5, span: 0.3 },
      { type: 'fade', at: 0.8, span: 0.3 },
    ],
  },
  // Пять кадров гнева. Снимки высокие, поэтому плита вытянута до 1 : 2.
  'ubit-a-potom-razorvat': {
    kind: 'plate',
    effect: 'frames',
    place: 'margin',
    ratio: '1 / 2',
    position: '50% 76%',
    figure: '/tales/ubit-a-potom-razorvat/stage-1.webp',
    frames: [1,2,3,4,5].map((n) => ({ url: `/tales/ubit-a-potom-razorvat/stage-${n}.webp` })),
    transitions: [
      { type: 'flush', at: 0.16, span: 0.2 },
      { type: 'flush', at: 0.4, span: 0.2 },
      { type: 'flush', at: 0.64, span: 0.2 },
      { type: 'surge', at: 0.87, span: 0.18 },
    ],
  },
  'ne-odinoko-grustno': plate('dusk', 'ne-odinoko-grustno'),
  gb: plate('print', 'gb'),
  'otmeryalschik-ot-boga': plate('ruler', 'otmeryalschik-ot-boga'),
  lechu: plate('rings', 'lechu'),
  'the-forester-and-his-unforgettable-memory': plate('fog', 'the-forester-and-his-unforgettable-memory', true),
};

export function taleSceneFor(slug: string | undefined | null): TaleScene | null {
  return (slug && TALE_SCENES[slug]) || null;
}

/**
 * Сцена из базы в той же форме, что и сцена из кода: страница и плита про
 * источник не знают, и у них одна дорога на оба.
 */
export function sceneFromData(d: TaleSceneData): TaleScene {
  return {
    kind: 'plate',
    effect: 'frames',
    place: 'margin',
    ratio: d.ratio,
    position: d.position,
    figure: d.frames[0]?.url ?? '',
    frames: d.frames.map((f) => ({ url: f.url, position: f.position || undefined })),
    transitions: d.transitions.map((t) => ({ type: t.type, at: t.at, span: t.span })),
  };
}


/** Номер блока, после которого стоит встроенная сцена. Считается по тому
 *  тексту, который читатель действительно видит: пустое тело на его языке
 *  заменяется по той же цепочке, что и у текста (`firstText`: для uk, es, de,
 *  it английский раньше русского). `bodies` — тела небылицы по языкам. */
export function sceneAfter(
  after: NonNullable<TaleScene['after']>,
  lang: Lang,
  bodies?: Partial<Record<Lang, string | null | undefined>>,
): number {
  const shown = (bodies && firstTextLang(lang, bodies)) || lang;
  if (shown === 'en') return after.en;
  if (shown === 'ru') return after.ru;
  return after[shown] ?? after.en ?? after.ru;
}
