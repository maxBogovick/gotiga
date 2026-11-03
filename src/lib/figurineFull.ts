import { api } from '$lib/api';
import type { Figurine } from '$lib/types/api';

// Список работ несёт два снимка и одну строку текста; полная запись нужна ради
// описания и всех фото. Запись берётся один раз на работу, для всех, кто её просит.
const cache = new Map<string, Figurine | null>();
const pending = new Map<string, Promise<Figurine | null>>();

export function cachedFigurine(id: string): Figurine | null | undefined {
  return cache.get(id);
}

export function loadFigurine(id: string): Promise<Figurine | null> {
  if (cache.has(id)) return Promise.resolve(cache.get(id) ?? null);
  let p = pending.get(id);
  if (!p) {
    p = api
      .getFigurine(id)
      .then((f) => {
        cache.set(id, f);
        return f;
      })
      .catch(() => null)
      .finally(() => pending.delete(id));
    pending.set(id, p);
  }
  return p;
}
