import { api } from '$lib/api';

export const load = async ({ params }: { params: { id: string } }) => {
  try {
    const figurine = await api.getFigurine(params.id);
    return { figurine: figurine ?? null };
  } catch {
    return { figurine: null as import('$lib/types/api').Figurine | null };
  }
};
