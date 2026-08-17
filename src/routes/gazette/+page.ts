import { api } from '$lib/api';
import type { GazetteCutting, GazetteLeavesPage } from '$lib/types/api';

export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const load = async ({ fetch }: { fetch: typeof globalThis.fetch }) => {
  const emptyPage: GazetteLeavesPage = { items: [], total: 0, page: 1, perPage: 24 };
  const [page, home] = await Promise.all([
    api.getGazettePage(1, 24, fetch).catch(() => emptyPage),
    api.getGazetteHome(fetch),
  ]);
  const cuttings: GazetteCutting[] = home.cuttings ?? [];
  return { page, cuttings };
};
