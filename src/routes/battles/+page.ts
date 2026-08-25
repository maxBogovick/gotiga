import { api } from '$lib/api';
import type { BattleCard, BattleFrames } from '$lib/types/api';

// Same switch as every other public room: the root layout turns SSR off for the
// SPA/Tauri profile, and a child cannot prerender under a parent without SSR.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const load = async ({ fetch }: { fetch: typeof globalThis.fetch }) => {
  // Both are the same for every visitor, so both are cached at the edge and
  // both are safe to bake into a prerendered page.
  const [cards, frames]: [BattleCard[], BattleFrames] = await Promise.all([
    api.getBattleCards(fetch),
    api.getBattleFrames(fetch),
  ]);
  return { cards, frames: frames.frames };
};
