import { api } from '$lib/api';
import type { WorkshopItem } from '$lib/types/api';

// Web build: prerender to static HTML so the workshop notes are visible to crawlers
// and LLMs (previously fetched in onMount). Dev stays SPA.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const load = async ({ fetch }: { fetch: typeof globalThis.fetch }) => {
    const items = await api.getWorkshopContent(fetch).catch(() => [] as WorkshopItem[]);
    return { items };
};
