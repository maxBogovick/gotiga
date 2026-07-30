import { api } from '$lib/api';
import type { FigurineListItem } from '$lib/types/api';

// Web build: prerender to static HTML so in-progress pieces are crawlable. Rebuilt on
// each deploy, like the archive. Dev stays SPA.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const load = async ({ fetch }: { fetch: typeof globalThis.fetch }) => {
    const items = await api.getInProgressFigurines(fetch).catch(() => [] as FigurineListItem[]);
    return { items };
};
