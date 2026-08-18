import { api } from '$lib/api';
import type { GazetteRoom } from '$lib/types/api';

export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const load = async ({ fetch }: { fetch: typeof globalThis.fetch }) => {
  const room: GazetteRoom = await api.getGazetteRoom(undefined, fetch);
  return { room };
};
