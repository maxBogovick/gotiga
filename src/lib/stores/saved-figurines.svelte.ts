import { browser } from '$app/environment';
import { authStore } from './auth.svelte';
import { api } from '$lib/api';

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
    this.#pushToServer();
  }

  remove(id: string) {
    this.load();
    this.ids = this.ids.filter((savedId) => savedId !== id);
    this.#persist();
    this.#pushToServer();
  }

  // Merge the server-stored wishlist with the local one and converge both sides.
  // Called after a session is established (login / session restore). When logged
  // out or offline it's a no-op, so anonymous visitors keep using localStorage.
  async syncWithServer() {
    if (!browser) return;
    const token = authStore.token;
    if (!token) return;
    this.load();
    try {
      const server = await api.getWishlist(token);
      const merged = dedupe([...this.ids, ...server]);
      const changed =
        merged.length !== this.ids.length || merged.length !== server.length;
      this.ids = merged;
      this.#persist();
      // Only write back when the merged set actually differs from the server's.
      if (changed) await api.setWishlist(token, merged);
    } catch {
      // Not logged in / transient error — keep the local list untouched.
    }
  }

  #pushToServer() {
    if (!browser) return;
    const token = authStore.token;
    if (!token) return;
    // Best effort: localStorage already holds the source of truth for this device.
    api.setWishlist(token, this.ids).catch(() => {});
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
