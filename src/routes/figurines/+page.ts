import { api } from '$lib/api';

export const load = async () => {
  try {
    const figurines = await api.getAllFigurines();
    return { figurines };
  } catch {
    return { figurines: [] as import('$lib/types/api').FigurineListItem[] };
  }
};
