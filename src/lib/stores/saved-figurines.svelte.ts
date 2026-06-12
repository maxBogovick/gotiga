import { browser } from '$app/environment';

const CANONICAL_KEY = 'gotiga_saved_figurines';
const LEGACY_KEYS = ['gotiga_liked', 'gotiga_wishlist'] as const;
const ALL_KEYS = [CANONICAL_KEY, ...LEGACY_KEYS] as const;

function readIds(key: string): string[] {
  if (!browser) return [];
  try {
    const parsed = JSON.parse(localStorage.getItem(key) ?? '[]');
    return Array.isArray(parsed) ? parsed.filter((id): id is string => typeof id === 'string') : [];
  } catch {
    return [];
  }
}

function dedupe(ids: string[]): string[] {
  return [...new Set(ids.filter(Boolean))];
}

class SavedFigurinesStore {
  ids = $state<string[]>([]);

  #loaded = false;
  #listening = false;

  get count() {
    return this.ids.length;
  }

  load() {
    if (!browser || this.#loaded) return;
    this.#loaded = true;
    this.ids = dedupe(ALL_KEYS.flatMap(readIds));
    this.#persist();
    this.#listen();
  }

  has(id: string) {
    return this.ids.includes(id);
  }

  toggle(id: string) {
    this.load();
    const set = new Set(this.ids);
    if (set.has(id)) set.delete(id);
    else set.add(id);
    this.ids = [...set];
    this.#persist();
  }

  remove(id: string) {
    this.load();
    this.ids = this.ids.filter((savedId) => savedId !== id);
    this.#persist();
  }

  #persist() {
    if (!browser) return;
    const payload = JSON.stringify(this.ids);
    for (const key of ALL_KEYS) localStorage.setItem(key, payload);
  }

  #listen() {
    if (!browser || this.#listening) return;
    this.#listening = true;
    window.addEventListener('storage', (event) => {
      if (!event.key || !ALL_KEYS.includes(event.key as (typeof ALL_KEYS)[number])) return;
      this.ids = dedupe(ALL_KEYS.flatMap(readIds));
      this.#persist();
    });
  }
}

export const savedFigurines = new SavedFigurinesStore();
