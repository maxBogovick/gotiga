import { api } from '$lib/api';
import type { FigurineListItem, ShowingRoom } from '$lib/types/api';

// Web build: prerender one static page per room (entries() enumerates room ids from
// the API at build time, like the figurine [id] route). Tauri/default build: SPA.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const entries = async () => {
  if (import.meta.env.VITE_BUILD_TARGET !== 'web') return [];
  // Not wrapped: if the API is unreachable at build time, fail loudly rather than
  // ship halls with no rooms. An empty list is allowed via handleUnseenRoutes.
  const rooms = await api.getShowingRooms();
  return rooms.map((r) => ({ id: r.id }));
};

export const load = async ({ params }: { params: { id: string } }) => {
  // Both fire in parallel. A missing room (room === null) renders NotFound; a
  // backend we can't reach (loadError) renders an error screen — kept distinct.
  const roomsReq = api.getShowingRooms();
  const figsReq = api.getAllFigurines().catch(() => null);

  let rooms: ShowingRoom[] | null = null;
  let loadError = false;
  try {
    rooms = await roomsReq;
  } catch {
    loadError = true;
  }

  const room = rooms?.find((r) => r.id === params.id) ?? null;
  const all = await figsReq;
  const works: FigurineListItem[] = all
    ? all.filter((f) => f.showingRoomId === params.id && f.status !== 'in_progress')
    : [];

  return { room, works, loadError };
};
