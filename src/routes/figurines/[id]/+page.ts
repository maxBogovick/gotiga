import { api } from '$lib/api';
import type { FigurineListItem } from '$lib/types/api';

export const load = async ({ params }: { params: { id: string } }) => {
  try {
    const [figurine, all] = await Promise.all([
      api.getFigurine(params.id),
      api.getAllFigurines(),
    ]);

    const sorted = [...all].sort(
      (a, b) => (a.sortOrder ?? 0) - (b.sortOrder ?? 0) || a.name.localeCompare(b.name)
    );
    const idx = sorted.findIndex(f => f.id === params.id);
    const prev: FigurineListItem | null = idx > 0 ? sorted[idx - 1] : null;
    const next: FigurineListItem | null = idx < sorted.length - 1 ? sorted[idx + 1] : null;

    return { figurine: figurine ?? null, prev, next };
  } catch {
    return {
      figurine: null as import('$lib/types/api').Figurine | null,
      prev:     null as FigurineListItem | null,
      next:     null as FigurineListItem | null,
    };
  }
};
