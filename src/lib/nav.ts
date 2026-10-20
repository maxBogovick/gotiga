// Разделы сайта и то, как узнают активный. Читают шапка (компьютер) и нижняя
// панель разделов (телефон): два списка одного и того же разошлись бы на первой
// же правке.

/** Страница одной работы: `/figurines/<слаг>`. */
export const WORK_LEAF_RE = /^\/figurines\/[^/]+\/?$/;

/** Пункт «Игра» ведёт на этюды, но полка карт — та же игра. */
export const GAME_HREF = '/battles/etude';

export function isNavActive(pathname: string, href: string): boolean {
  if (href === GAME_HREF) return pathname === '/battles' || pathname.startsWith('/battles/');
  return pathname === href || pathname.startsWith(`${href}/`);
}
