import { api } from '$lib/api';
import type { GazetteLeaf } from '$lib/types/api';

export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const entries = async () => {
  if (import.meta.env.VITE_BUILD_TARGET !== 'web') return [];
  try {
    const page = await api.getGazettePage(1, 200);
    return page.items.map((leaf) => ({ slug: leaf.slug }));
  } catch {
    return [];
  }
};

export const load = async ({ params, fetch }: { params: { slug: string }; fetch: typeof globalThis.fetch }) => {
  let leaf: GazetteLeaf | null = null;
  let loadError = false;
  try {
    leaf = await api.getGazetteLeaf(params.slug, fetch);
  } catch (e) {
    loadError = true;
    leaf = null;
  }
  return { leaf, loadError };
};
