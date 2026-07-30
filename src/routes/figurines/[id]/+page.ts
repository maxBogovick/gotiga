import { api } from '$lib/api';
import type { FigurineListItem } from '$lib/types/api';

// Web build: prerender one static HTML page per figurine. SvelteKit can't know the
// ids of a dynamic [id] route on its own, so entries() enumerates them from the API
// at build time. Any other build (dev): prerender off → SPA as before.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const entries = async () => {
  if (import.meta.env.VITE_BUILD_TARGET !== 'web') return [];
  // Deliberately NOT wrapped in try/catch: if the API is unreachable at build time we
  // want the web build to FAIL loudly rather than silently ship a site with no works.
  // A genuinely empty catalog returns [] and is allowed via prerender.handleUnseenRoutes.
  const all = await api.getAllFigurines();
  // Emit a static page for BOTH handles: the pretty slug (canonical) and the raw
  // UUID (so old share-links / bookmarks still resolve on the prerendered host).
  // Dedupe when a work has no slug yet.
  return all.flatMap((f) =>
    f.slug && f.slug !== f.id ? [{ id: f.slug }, { id: f.id }] : [{ id: f.id }]
  );
};

export const load = async ({ params, fetch }: { params: { id: string }; fetch: typeof globalThis.fetch }) => {
  // The work itself is the primary fetch. getFigurine returns null only on 404
  // (api.ts), so a null here means "no such work" — distinct from loadError, which
  // means the backend was unreachable. The page renders NotFound vs an error/retry
  // screen accordingly, instead of an endless skeleton.
  // Both requests fire in parallel; the work is required, neighbours are best-effort
  // (their failure resolves to null and must never block the work or flag loadError).
  const figurineReq = api.getFigurine(params.id, fetch);
  const allReq = api.getAllFigurines(undefined, fetch).catch(() => null);

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
    // params.id is a handle — match on either the UUID or the slug, since the
    // page can be entered by either.
    let idx = sorted.findIndex(f => f.id === params.id || f.slug === params.id);
    // A gated work (showing window currently shut) is absent from the visible list,
    // so findIndex misses it. Splice it into its rightful sort slot using the work's
    // own sortOrder/name so prev/next still resolve — the sealed door must stay
    // pageable, not become a dead end.
    if (idx === -1 && figurine) {
      const f = figurine;
      const at = sorted.findIndex(
        s => (s.sortOrder ?? 0) > (f.sortOrder ?? 0) ||
          ((s.sortOrder ?? 0) === (f.sortOrder ?? 0) && s.name.localeCompare(f.name) > 0)
      );
      idx = at === -1 ? sorted.length : at;
      sorted.splice(idx, 0, f as unknown as FigurineListItem);
    }
    prev = idx > 0 ? sorted[idx - 1] : null;
    next = idx >= 0 && idx < sorted.length - 1 ? sorted[idx + 1] : null;
  }

  // Canonical path uses the slug when the work has one, so a UUID URL points
  // search engines at the pretty URL. Falls back to whatever handle was used.
  const canonicalPath = `/figurines/${figurine?.slug ?? params.id}`;

  return { figurine, prev, next, loadError, canonicalPath };
};
