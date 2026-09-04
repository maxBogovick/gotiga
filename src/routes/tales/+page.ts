import { building } from '$app/environment';
import { api } from '$lib/api';
import type { GazetteLeaf } from '$lib/types/api';

// Same switch as every other public room: the root layout turns SSR off for the
// SPA/Tauri profile, and a child cannot prerender under a parent without SSR.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const load = async ({ fetch }: { fetch: typeof globalThis.fetch }) => {
  try {
    const tales: GazetteLeaf[] = await api.getTales(fetch);
    return { tales, loadError: false };
  } catch (e) {
    // At build time this must be loud. A shelf prerendered from a failed fetch
    // reads exactly like a shelf that is genuinely bare, and it stays that way
    // in the served HTML until someone redeploys — while the sitemap, which the
    // backend generates live, goes on advertising tale addresses that were never
    // prerendered with it. An empty shelf is a fine page; an empty shelf that is
    // a lie is a room Google indexes as vacant.
    if (building) throw e;
    // In the running app the room degrades instead: loadError separates "the
    // shelf is bare" from "the house could not reach the shelf", the same
    // distinction the archive draws.
    return { tales: [] as GazetteLeaf[], loadError: true };
  }
};
