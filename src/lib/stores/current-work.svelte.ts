import { browser } from '$app/environment';
import type { FigurineListItem } from '$lib/types/api';

// Работа, которая сейчас на экране. Страница с лентой работ сообщает её сюда
// (`watchCurrentWork`), а док работы (`WorkDock`) читает. Хранилище общее, а не
// кусок главной: ту же ленту завтра покажет архив, и док не должен знать, какая
// страница ему кормит.

/** Доля высоты окна, на которой лежит «линия чтения». Работа, которую пересекает
 *  линия, и есть текущая. Чуть выше середины: взгляд лежит выше центра. */
const LINE_FROM_TOP = 0.42;

let works = $state.raw<FigurineListItem[]>([]);
// Всё собрание, а не только лента: похожие работы (`similarIds`) лежат по всему
// собранию, и в ленту из шестнадцати их попала бы пара.
let pool = $state.raw<FigurineListItem[]>([]);
let index = $state(-1);
let remaining = $state(0);
let reel = $state.raw<HTMLElement | null>(null);

export const currentWork = {
  get work(): FigurineListItem | null {
    return index >= 0 ? (works[index] ?? null) : null;
  },
  /** Номер текущей работы в ленте с нуля; -1, когда лента не на экране. */
  get index(): number {
    return index;
  },
  get works(): FigurineListItem[] {
    return works;
  },
  get pool(): FigurineListItem[] {
    return pool;
  },
  get total(): number {
    return works.length;
  },
  /** Сколько работ не вошло в ленту и осталось только в архиве. */
  get remaining(): number {
    return remaining;
  },
  get reel(): HTMLElement | null {
    return reel;
  },
};

/**
 * Следит за лентой и держит `currentWork` в согласии с тем, что на экране.
 *
 * Каждая работа в ленте — элемент `[data-reel-slot="<номер>"]`. Два наблюдателя
 * вместо обработчика прокрутки: одному нужна тонкая полоса на линии чтения
 * (какая работа под ней), другому — весь экран (есть ли в ленте хоть одна
 * работа на виду). Промежутки между работами линию не пересекает никто, и тогда
 * остаётся прежняя: мигать пустотой на каждом зазоре нельзя.
 *
 * Возвращает функцию, снимающую слежение и очищающую хранилище.
 */
export function watchCurrentWork(
  el: HTMLElement,
  list: FigurineListItem[],
  rest: number,
  everything: FigurineListItem[] = list,
): () => void {
  if (!browser || list.length === 0) return () => {};

  works = list;
  pool = everything;
  remaining = rest;
  reel = el;
  index = -1;

  const onLine = new Set<number>();
  const onScreen = new Set<number>();
  const numberOf = (target: Element) => Number((target as HTMLElement).dataset.reelSlot);

  const line = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        const i = numberOf(entry.target);
        if (entry.isIntersecting) onLine.add(i);
        else onLine.delete(i);
      }
      if (onLine.size > 0) index = Math.min(...onLine);
    },
    {
      // Полоса в один процент высоты окна: отступ сверху и снизу оставляет только её.
      rootMargin: `-${LINE_FROM_TOP * 100}% 0px -${(1 - LINE_FROM_TOP) * 100 - 1}% 0px`,
      threshold: 0,
    },
  );

  const screen = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        const i = numberOf(entry.target);
        if (entry.isIntersecting) onScreen.add(i);
        else onScreen.delete(i);
      }
      if (onScreen.size === 0) index = -1;
      // Лента только что въехала: линия ещё никого не пересекла, а док уже нужен.
      else if (index < 0 || !onScreen.has(index)) index = Math.min(...onScreen);
    },
    { threshold: 0 },
  );

  for (const slot of el.querySelectorAll<HTMLElement>('[data-reel-slot]')) {
    line.observe(slot);
    screen.observe(slot);
  }

  return () => {
    line.disconnect();
    screen.disconnect();
    works = [];
    pool = [];
    remaining = 0;
    reel = null;
    index = -1;
  };
}
