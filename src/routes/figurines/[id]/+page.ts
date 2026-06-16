import { api } from '$lib/api';
import type { FigurineListItem } from '$lib/types/api';

// Web build: prerender one static HTML page per figurine. SvelteKit can't know the
// ids of a dynamic [id] route on its own, so entries() enumerates them from the API
// at build time. Tauri/default build: prerender off → SPA as before.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const entries = async () => {
  if (import.meta.env.VITE_BUILD_TARGET !== 'web') return [];
  // Deliberately NOT wrapped in try/catch: if the API is unreachable at build time we
  // want the web build to FAIL loudly rather than silently ship a site with no works.
  // A genuinely empty catalog returns [] and is allowed via prerender.handleUnseenRoutes.
  const all = await api.getAllFigurines();
  return all.map((f) => ({ id: f.id }));
};

export const load = async ({ params }: { params: { id: string } }) => {
  // The work itself is the primary fetch. getFigurine returns null only on 404
  // (api.ts), so a null here means "no such work" — distinct from loadError, which
  // means the backend was unreachable. The page renders NotFound vs an error/retry
  // screen accordingly, instead of an endless skeleton.
  // Both requests fire in parallel; the work is required, neighbours are best-effort
  // (their failure resolves to null and must never block the work or flag loadError).
  const figurineReq = api.getFigurine(params.id);
  const allReq = api.getAllFigurines().catch(() => null);

  let figurine: import('$lib/types/api').Figurine | null = null;
  let loadError = false;
  try {
    figurine = await figurineReq;
  } catch {
    loadError = true;
  }

  let prev: FigurineListItem | null = null;
  let next: FigurineListItem | null = null;
  const all = await allReq;
  if (all) {
    const sorted = [...all].sort(
      (a, b) => (a.sortOrder ?? 0) - (b.sortOrder ?? 0) || a.name.localeCompare(b.name)
    );
    const idx = sorted.findIndex(f => f.id === params.id);
    prev = idx > 0 ? sorted[idx - 1] : null;
    next = idx >= 0 && idx < sorted.length - 1 ? sorted[idx + 1] : null;
  }

  return { figurine, prev, next, loadError };
};
