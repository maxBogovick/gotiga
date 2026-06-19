import { browser } from '$app/environment';
import { authStore } from './auth.svelte';
import { api } from '$lib/api';

const CANONICAL_KEY = 'gotiga_saved_figurines';
const LEGACY_KEYS = ['gotiga_liked', 'gotiga_wishlist'] as const;
const ALL_KEYS = [CANONICAL_KEY, ...LEGACY_KEYS] as const;
type SyncOptions = { importLocal?: boolean };

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
  syncError = $state(false);

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

  localIds() {
    return dedupe(ALL_KEYS.flatMap(readIds));
  }

  replaceLocal(ids: string[]) {
    this.ids = dedupe(ids);
    this.#persist();
    this.#listen();
    this.#loaded = true;
  }

  // Reconcile local state with the account wishlist. By default this only adopts
  // the server copy; importing guest saves must be explicit from the login UI.
  async syncWithServer(options: SyncOptions = {}) {
    if (!browser) return;
    const token = authStore.token;
    if (!token) return;
    this.load();
    try {
      const server = await api.getWishlist(token);
      const importLocal = options.importLocal === true;
      const next = importLocal ? dedupe([...this.ids, ...server]) : dedupe(server);
      const changed = importLocal && (
        next.length !== this.ids.length || next.length !== server.length
      );
      this.ids = next;
      this.#persist();
      // Only write back when the merged set actually differs from the server's.
      if (changed) await api.setWishlist(token, next);
      this.syncError = false;
    } catch {
      // Not logged in / transient error — keep the local list untouched.
      this.syncError = true;
    }
  }

  async importLocalToServer(localIds = this.localIds()) {
    if (!browser) return this.ids;
    const token = authStore.token;
    if (!token) return this.ids;
    this.load();
    const server = await api.getWishlist(token);
    const merged = dedupe([...server, ...localIds]);
    const saved = await api.setWishlist(token, merged);
    this.replaceLocal(saved);
    this.syncError = false;
    return this.ids;
  }

  #pushToServer() {
    if (!browser) return;
    const token = authStore.token;
    if (!token) return;
    // Best effort: localStorage already holds the source of truth for this device.
    api.setWishlist(token, this.ids)
      .then(() => { this.syncError = false; })
      .catch(() => { this.syncError = true; });
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
