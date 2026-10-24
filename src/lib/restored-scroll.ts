// Куда страница будет прокручена при возврате («назад», обновление): SvelteKit
// хранит позиции в sessionStorage, а прокрутку применяет уже после гидрации.
//
// Нужно шапке: HTML из пререндера рисует развёрнутую шапку, и при возврате
// на середину страницы она схлопывалась бы уже на глазах (сдвиг макета и
// переход ≈240 мс). Тот же расчёт, что в скрипте перед закрывающим `</body>`
// в `app.html`, — править вместе с ним.

/** Прокрутка, с которой шапка сворачивается. В `app.html` то же число (там нет модулей). */
export const COLLAPSE_AT = 48;

const SCROLL_KEY = 'sveltekit:scroll';
const HISTORY_INDEX = 'sveltekit:history';

/** Шапка должна быть свёрнута уже в первом кадре: страница вернётся ниже порога. */
export function restoredCollapsed(): boolean {
  return restoredScrollY() > COLLAPSE_AT;
}

export function restoredScrollY(): number {
  if (typeof window === 'undefined') return 0;
  try {
    const index = history.state?.[HISTORY_INDEX];
    const all = JSON.parse(sessionStorage.getItem(SCROLL_KEY) ?? '{}') as Record<string, { y?: number }>;
    const y = all[index]?.y;
    return typeof y === 'number' && y > 0 ? y : 0;
  } catch {
    return 0;
  }
}
