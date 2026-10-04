import { building } from '$app/environment';
import { redirect } from '@sveltejs/kit';
import { api } from '$lib/api';
import { isGazetteReservedSlug } from '$lib/gazette';
import { isTale } from '$lib/tales';
import { resolveSiteRefs, siteRefsIn, type SiteRefInfo } from '$lib/siteLinks';
import type { GazetteLeaf } from '$lib/types/api';

export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

// Deliberately without a try/catch: an unreachable shelf must fail the build,
// not quietly produce zero entries. `handleUnseenRoutes: 'ignore'` in
// svelte.config.js only forgives a route with genuinely nothing to prerender —
// the figurine archive relies on the same "entries() itself throws" contract.
export const entries = async () => {
  if (import.meta.env.VITE_BUILD_TARGET !== 'web') return [];
  return (await api.getTales()).map((tale) => ({ slug: tale.slug }));
};

export const load = async ({
  params,
  fetch,
}: {
  params: { slug: string };
  fetch: typeof globalThis.fetch;
}) => {
  if (isGazetteReservedSlug(params.slug)) {
    return { leaf: null, loadError: false, links: {} };
  }

  let leaf: GazetteLeaf | null = null;
  let loadError = false;
  try {
    leaf = await api.getGazetteLeaf(params.slug, fetch);
  } catch (e) {
    leaf = null;
    loadError = !(e instanceof Error && /API 404|API 410/.test(e.message));
    // Every slug reaching this point at build time came out of entries(), i.e.
    // the shelf itself listed it a moment ago. A failure now is the house being
    // unreachable, and prerendering "the house could not fetch this tale" to a
    // 200 page is how a dead page gets indexed as the tale.
    if (building && loadError) throw e;
  }

  // Slugs are unique across the whole gazette, so this address can be reached
  // with the name of a leaf that never belonged on the shelf. Send it home
  // rather than dressing an announcement up as a tale.
  if (leaf && !isTale(leaf)) redirect(308, `/gazette/${leaf.slug}`);

  // Names for the links to this house written into the prose — both languages
  // at once, because the reader switches language without a new load.
  let links: Record<string, SiteRefInfo> = {};
  if (leaf) {
    const refs = siteRefsIn(`${leaf.bodyEn ?? ''}\n${leaf.bodyRu ?? ''}`);
    if (refs.length) {
      links = await resolveSiteRefs(refs, {
        getFigurine: (handle) => api.getFigurine(handle, fetch),
        getGazetteLeaf: (slug) => api.getGazetteLeaf(slug, fetch),
      });
    }
  }

  return { leaf, loadError, links };
};
