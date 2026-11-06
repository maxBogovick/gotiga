// Расстановка карт перед волной забега: чистые правила без страницы.
//
// Сервер называет открытые клетки (`openCells`) в том порядке, которым он ставит
// карты сам, и проверяет итог (`lay_out`). Здесь — то, что нужно странице, чтобы
// предложить расстановку и править её по нажатиям.

export type Cell = { x: number; y: number };
export type Placement = Record<string, Cell>;

export const cellKey = (c: Cell) => `${c.x},${c.y}`;

/**
 * Клетки выбранным картам. Своя прежняя клетка остаётся, если она открыта и не
 * занята выше по списку; иначе берётся запомненная с прошлой волны; иначе первая
 * свободная в порядке сервера. Карте, которой клеток не хватило, места нет.
 */
export function reconcile(
  chosen: string[],
  current: Placement,
  remembered: Record<string, Cell | null | undefined>,
  open: Cell[],
): Placement {
  const openKeys = new Set(open.map(cellKey));
  const taken = new Set<string>();
  const out: Placement = {};
  for (const id of chosen) {
    const wish = current[id] ?? remembered[id];
    if (wish && openKeys.has(cellKey(wish)) && !taken.has(cellKey(wish))) {
      out[id] = { x: wish.x, y: wish.y };
      taken.add(cellKey(wish));
    }
  }
  for (const id of chosen) {
    if (out[id]) continue;
    const free = open.find((c) => !taken.has(cellKey(c)));
    if (!free) continue;
    out[id] = { x: free.x, y: free.y };
    taken.add(cellKey(free));
  }
  return out;
}

/** Расстановка сервера: как будто никто нигде не стоял. */
export const autoLayout = (chosen: string[], open: Cell[]): Placement => reconcile(chosen, {}, {}, open);

/**
 * Расстановка «подальше от ударов»: клетки вне зон врага идут первыми (внутри
 * группы — в порядке сервера), и самые раненые карты получают самые безопасные.
 * Равное здоровье — порядок выбора.
 */
export function safeLayout(
  chosen: string[],
  open: Cell[],
  danger: Set<string>,
  health: Record<string, number>,
): Placement {
  const cells = [...open.filter((c) => !danger.has(cellKey(c))), ...open.filter((c) => danger.has(cellKey(c)))];
  const order = chosen
    .map((id, i) => ({ id, i }))
    .sort((a, b) => (health[a.id] ?? 1) - (health[b.id] ?? 1) || a.i - b.i);
  const out: Placement = {};
  order.forEach(({ id }, k) => {
    const c = cells[k];
    if (c) out[id] = { x: c.x, y: c.y };
  });
  return out;
}

/**
 * Поставить карту на клетку. Клетка закрыта — ничего. Занята другой выбранной
 * картой — они меняются местами: человек двигает карту, а не выбирает порядок.
 */
export function moveTo(cells: Placement, id: string, target: Cell, open: Cell[]): Placement {
  if (!open.some((c) => c.x === target.x && c.y === target.y)) return cells;
  const here = cells[id];
  const occupant = Object.keys(cells).find(
    (other) => other !== id && cells[other].x === target.x && cells[other].y === target.y,
  );
  const next: Placement = { ...cells, [id]: { x: target.x, y: target.y } };
  if (occupant) {
    if (!here) delete next[occupant];
    else next[occupant] = { x: here.x, y: here.y };
  }
  return next;
}

/** Клетки, до которых достаёт хоть один враг: для подсветки опасных мест. */
export function threatened(zones: { cells: Cell[] }[]): Set<string> {
  const out = new Set<string>();
  for (const zone of zones) for (const c of zone.cells) out.add(cellKey(c));
  return out;
}

/** Что уходит на сервер при начале волны: каждая карта и её клетка. */
export function toRequest(chosen: string[], cells: Placement): { cardId: string; x?: number; y?: number }[] {
  return chosen.map((cardId) => {
    const at = cells[cardId];
    return at ? { cardId, x: at.x, y: at.y } : { cardId };
  });
}
