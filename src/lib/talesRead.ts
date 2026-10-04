import { browser } from '$app/environment';

/**
 * Прочитанные небылицы — сургучная печать на арке полки.
 *
 * Лежит у самого читателя, а не на сервере: печать — отметка для него, а не
 * счёт для дома, и сводить её не с чем. Засчитывается дочитанная байка, а не
 * открытая: тот же знак дна текста (`lastLine`), по которому идёт пыль за
 * чтение, — последняя строка попала на экран.
 *
 * Отметка, а не награда: без очков, без числа «3 из 8», без пыли. Награда за
 * завершение отбивает охоту к самому делу; видимый след прочитанного — нет.
 */
const KEY = 'gotiga_tales_read';
/** Полка отдаётся целиком до пятисот байк (`SHELF_TALES`); больше помнить нечего. */
const MAX = 500;

export function readTales(): Set<string> {
  if (!browser) return new Set();
  try {
    const raw = localStorage.getItem(KEY);
    const list = raw ? (JSON.parse(raw) as unknown) : [];
    return new Set(Array.isArray(list) ? list.filter((id): id is string => typeof id === 'string') : []);
  } catch {
    return new Set();
  }
}

/** Отметить байку прочитанной. `true` — отметка новая, печать ставится впервые. */
export function markTaleRead(id: string): boolean {
  if (!browser || !id) return false;
  const read = readTales();
  if (read.has(id)) return false;
  read.add(id);
  try {
    localStorage.setItem(KEY, JSON.stringify([...read].slice(-MAX)));
  } catch {
    // Приватное окно или полное хранилище: печать стоит до закрытия вкладки.
  }
  return true;
}
