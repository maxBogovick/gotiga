import { redirect } from '@sveltejs/kit';
import { api } from '$lib/api';
import { isGazetteReservedSlug, isGazetteYearSlug } from '$lib/gazette';
import { isTale } from '$lib/tales';
import type { GazetteLeaf, GazetteRoom } from '$lib/types/api';

export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const entries = async () => {
  if (import.meta.env.VITE_BUILD_TARGET !== 'web') return [];
  try {
    const [page, room] = await Promise.all([
      api.getGazettePage(1, 200),
      api.getGazetteRoom(),
    ]);
    const slugs = new Set<string>();
    for (const leaf of page.items) {
      if (!isGazetteYearSlug(leaf.slug)) slugs.add(leaf.slug);
    }
    for (const year of room.years) slugs.add(String(year));
    return [...slugs].map((slug) => ({ slug }));
  } catch {
    return [];
  }
};

export const load = async ({ params, fetch }: { params: { slug: string }; fetch: typeof globalThis.fetch }) => {
  if (isGazetteReservedSlug(params.slug)) {
    return { mode: 'leaf' as const, room: null, leaf: null, loadError: false };
  }

  if (isGazetteYearSlug(params.slug)) {
    const room: GazetteRoom = await api.getGazetteRoom(Number(params.slug), fetch);
    return { mode: 'year' as const, room, leaf: null, loadError: false };
  }

  let leaf: GazetteLeaf | null = null;
  let loadError = false;
  try {
    leaf = await api.getGazetteLeaf(params.slug, fetch);
  } catch (e) {
    leaf = null;
    loadError = !(e instanceof Error && /API 404|API 410/.test(e.message));
  }
  // A tale lives on the shelf now. Old links, feeds and bookmarks still work;
  // they just arrive by way of a permanent redirect.
  if (leaf && isTale(leaf)) redirect(308, `/tales/${leaf.slug}`);

  return { mode: 'leaf' as const, room: null, leaf, loadError };
};
