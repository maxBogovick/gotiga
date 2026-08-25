import { api } from '$lib/api';
import type { GazetteLeaf } from '$lib/types/api';

// Same switch as every other public room: the root layout turns SSR off for the
// SPA/Tauri profile, and a child cannot prerender under a parent without SSR.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const load = async ({ fetch }: { fetch: typeof globalThis.fetch }) => {
  const tales: GazetteLeaf[] = await api.getTales(fetch);
  return { tales };
};
