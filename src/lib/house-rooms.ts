/**
 * Rooms of the house for view-transition gestures.
 *
 * Only two crossings have a character — hall↔archive is a cabinet drawer,
 * hall↔workshop is a portiere. Figurine detail keeps the plate morph / page-turn.
 * Every other door uses the default leaf fade. One system, not a new motion per link.
 */

export type HouseRoom = 'hall' | 'archive' | 'workshop' | 'work' | 'other';

export type RoomGesture = 'drawer-in' | 'drawer-out' | 'curtain-in' | 'curtain-out';

export function houseRoom(pathname: string): HouseRoom {
  const path = pathname.replace(/\/+$/, '') || '/';
  if (path === '/') return 'hall';
  if (path === '/workshop') return 'workshop';
  if (path === '/figurines') return 'archive';
  if (path.startsWith('/figurines/')) return 'work';
  return 'other';
}

export function roomGesture(fromPath: string, toPath: string): RoomGesture | null {
  const from = houseRoom(fromPath);
  const to = houseRoom(toPath);
  if (from === to) return null;
  if (from === 'work' || to === 'work') return null;
  if (from === 'hall' && to === 'archive') return 'drawer-in';
  if (from === 'archive' && to === 'hall') return 'drawer-out';
  if (from === 'hall' && to === 'workshop') return 'curtain-in';
  if (from === 'workshop' && to === 'hall') return 'curtain-out';
  return null;
}
