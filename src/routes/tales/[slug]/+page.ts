import { redirect } from '@sveltejs/kit';
import { api } from '$lib/api';
import { isGazetteReservedSlug } from '$lib/gazette';
import { isTale } from '$lib/tales';
import type { GazetteLeaf } from '$lib/types/api';

export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const entries = async () => {
  if (import.meta.env.VITE_BUILD_TARGET !== 'web') return [];
  try {
    return (await api.getTales()).map((tale) => ({ slug: tale.slug }));
  } catch {
    return [];
  }
};

export const load = async ({
  params,
  fetch,
}: {
  params: { slug: string };
  fetch: typeof globalThis.fetch;
}) => {
  if (isGazetteReservedSlug(params.slug)) {
    return { leaf: null, loadError: false };
  }

  let leaf: GazetteLeaf | null = null;
  let loadError = false;
  try {
    leaf = await api.getGazetteLeaf(params.slug, fetch);
  } catch (e) {
    leaf = null;
    loadError = !(e instanceof Error && /API 404|API 410/.test(e.message));
  }

  // Slugs are unique across the whole gazette, so this address can be reached
  // with the name of a leaf that never belonged on the shelf. Send it home
  // rather than dressing an announcement up as a tale.
  if (leaf && !isTale(leaf)) redirect(308, `/gazette/${leaf.slug}`);

  return { leaf, loadError };
};
