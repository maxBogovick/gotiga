import { api } from '$lib/api';

export const load = async () => {
  try {
    const all = await api.getAllFigurines();
    const figurines = all.filter(f => f.status !== 'in_progress');
    return { figurines };
  } catch {
    return { figurines: [] as import('$lib/types/api').FigurineListItem[] };
  }
};
