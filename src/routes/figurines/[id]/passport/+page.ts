import { api } from '$lib/api';

export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const entries = async () => {
  if (import.meta.env.VITE_BUILD_TARGET !== 'web') return [];
  const all = await api.getAllFigurines();
  return all.map((f) => ({ id: f.id }));
};

export const load = async ({ params }: { params: { id: string } }) => {
  let figurine: import('$lib/types/api').Figurine | null = null;
  let loadError = false;
  try {
    figurine = await api.getFigurine(params.id);
  } catch {
    loadError = true;
  }

  return { figurine, loadError };
};
