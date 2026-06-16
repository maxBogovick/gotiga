import { api } from '$lib/api';

export const load = async () => {
  try {
    const all = await api.getAllFigurines();
    const figurines = all.filter(f => f.status !== 'in_progress');
    // loadError distinguishes "the archive is genuinely empty" from "we couldn't
    // reach the backend" — the two looked identical before and showed "Emptiness…".
    return { figurines, loadError: false };
  } catch {
    return { figurines: [] as import('$lib/types/api').FigurineListItem[], loadError: true };
  }
};
