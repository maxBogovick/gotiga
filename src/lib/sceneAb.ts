// Сцена байки: показывать ли её читателю.
//
// Был опыт «со сценой / без»: половина читателей видела сцену, другая нет, а
// замер посещений (§ 18) записывал, кто что видел. Опыт закончен — сцена
// показывается всем. Модуль остался единственным местом, где решается
// «показывать ли», и замер спрашивает здесь же: показанное и записанное не
// должны расходиться.
import { browser } from '$app/environment';
import { taleSceneFor } from '$lib/taleScenes';

export type SceneArm = 'on' | 'off';

export interface SceneAssignment {
  arm: SceneArm;
  /**
   * Попадает ли этот просмотр в счёт. Сейчас нет никогда: делить читателей
   * больше не на что, и в замер `scene:on` / `scene:off` не пишется.
   */
  measured: boolean;
}

/**
 * Какой вариант видит читатель. Всем — со сценой; ярлык `?scene=off` (или
 * `0`) остаётся автору, чтобы сравнить страницу без неё. До монтирования
 * (пререндер) сцены нет: она и так ниже того места, где читатель стоит при
 * загрузке, и её появление ничего не сдвигает.
 */
export function sceneAssignment(_slug: string): SceneAssignment {
  if (!browser) return { arm: 'off', measured: false };
  const forced = new URLSearchParams(location.search).get('scene');
  if (forced === 'off' || forced === '0') return { arm: 'off', measured: false };
  return { arm: 'on', measured: false };
}

/**
 * Байки, у которых сцена лежит в базе, а не в коде: страница узнаёт о ней при
 * загрузке (`+page.ts`) и записывает сюда, а замер спрашивает здесь же. Модуль,
 * а не хранилище: это знание одной вкладки, и оно не должно переживать её.
 */
const remote = new Set<string>();

export function registerRemoteScene(slug: string, has: boolean): void {
  if (has) remote.add(slug);
  else remote.delete(slug);
}

/** Есть ли у байки сцена, из кода или из базы. */
export function hasScene(slug: string): boolean {
  return !!taleSceneFor(slug) || remote.has(slug);
}

/**
 * Что записать в замер для этого адреса: `scene:on` / `scene:off` на странице
 * байки, у которой есть сцена, и ничего на всех остальных (и на просмотре с
 * ярлыком). Принимает путь, а не слаг, потому что замер знает только адрес.
 */
export function sceneVariantForPath(pathname: string): string | undefined {
  const m = /^\/tales\/([^/]+)\/?$/.exec(pathname);
  if (!m) return undefined;
  const slug = decodeURIComponent(m[1]);
  if (!hasScene(slug)) return undefined;
  const { arm, measured } = sceneAssignment(slug);
  return measured ? `scene:${arm}` : undefined;
}
