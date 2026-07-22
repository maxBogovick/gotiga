import { api } from '$lib/api';

// In the web build this archive page is prerendered to static HTML (data comes from
// load() below, so it ends up in the markup). In the Tauri/default build it stays SPA.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const load = async ({ fetch }: { fetch: typeof globalThis.fetch }) => {
  try {
    const all = await api.getAllFigurines(undefined, fetch);
    const figurines = all.filter(f => f.status !== 'in_progress');
    // loadError distinguishes "the archive is genuinely empty" from "we couldn't
    // reach the backend" — the two looked identical before and showed "Emptiness…".
    return { figurines, loadError: false };
  } catch {
    return { figurines: [] as import('$lib/types/api').FigurineListItem[], loadError: true };
  }
};
