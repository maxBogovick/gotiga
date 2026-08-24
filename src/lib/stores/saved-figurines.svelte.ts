import { browser } from '$app/environment';
import { authStore } from './auth.svelte';
import { api } from '$lib/api';

const CANONICAL_KEY = 'gotiga_saved_figurines';
const LEGACY_KEYS = ['gotiga_liked', 'gotiga_wishlist'] as const;
const ALL_KEYS = [CANONICAL_KEY, ...LEGACY_KEYS] as const;
const VISITOR_TOKEN_KEY = 'gotiga_visitor_token';
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
  #pending = new Map<string, Promise<boolean>>();
  #desired = new Map<string, boolean>();
  #pushChain: Promise<unknown> = Promise.resolve();
  #gen = 0;

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

  hasAny(ids: string[]) {
    return ids.some((id) => this.ids.includes(id));
  }

  toggle(id: string, aliases: string[] = []) {
    this.load();
    void this.set(id, !this.hasAny([id, ...aliases]), aliases);
  }

  remove(id: string, aliases: string[] = []) {
    void this.set(id, false, aliases);
  }

  // Explicit target state — a retry or a doubled click cannot flip twice.
  // `aliases` are extra localStorage keys for the same work (URL slug vs UUID)
  // so an unlike cannot leave a leftover handle that reappears on reload.
  set(id: string, liked: boolean, aliases: string[] = []): Promise<boolean> {
    this.load();
    this.#gen += 1;
    this.#desired.set(id, liked);
    this.#setLocal(id, liked, aliases);

    const ahead = this.#pending.get(id) ?? Promise.resolve(liked);
    const request = ahead.catch(() => liked).then(async () => {
      const target = this.#desired.get(id) ?? liked;
      try {
        await api.setFigurineLike(
          id,
          this.#token(),
          target,
          authStore.token,
        );
        const wanted = this.#desired.get(id) ?? target;
        // Trust the last tap, not the server echo. A liked:true response
        // after unlike used to write the id back into localStorage, so the
        // heart came back on reload. The like endpoint already updates the
        // account wishlist for this one id — a full PUT here raced and
        // resurrected it.
        this.#setLocal(id, wanted, aliases);
        return wanted;
      } catch {
        const wanted = this.#desired.get(id) ?? target;
        this.#setLocal(id, wanted, aliases);
        this.syncError = true;
        this.#pushToServer();
        return wanted;
      } finally {
        if (this.#pending.get(id) === request) this.#pending.delete(id);
      }
    });

    this.#pending.set(id, request);
    return request;
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
    const gen = this.#gen;
    try {
      await this.#pushChain.catch(() => undefined);
      const server = await api.getWishlist(token);
      if (gen !== this.#gen) return;
      const importLocal = options.importLocal === true;
      let next = importLocal ? dedupe([...this.ids, ...server]) : dedupe(server);
      for (const [id, liked] of this.#desired) {
        if (liked) {
          if (!next.includes(id)) next = [...next, id];
        } else {
          next = next.filter((savedId) => savedId !== id);
        }
      }
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
    const visitor = this.#token();
    const toSync = localIds.filter((id) => !server.includes(id));
    await Promise.allSettled(
      toSync.map((id) => api.setFigurineLike(id, visitor, true, token)),
    );
    this.syncError = false;
    return this.ids;
  }

  #setLocal(id: string, liked: boolean, aliases: string[] = []) {
    const keys = dedupe([id, ...aliases]);
    if (liked) {
      this.ids = dedupe([...this.ids.filter((savedId) => !keys.includes(savedId)), id]);
    } else {
      this.ids = this.ids.filter((savedId) => !keys.includes(savedId));
    }
    this.#persist();
  }

  #token(): string {
    let token = browser ? localStorage.getItem(VISITOR_TOKEN_KEY) : null;
    if (!token) {
      token = crypto.randomUUID();
      if (browser) localStorage.setItem(VISITOR_TOKEN_KEY, token);
    }
    return token;
  }

  #pushToServer() {
    if (!browser) return;
    const token = authStore.token;
    if (!token) return;
    // Read `ids` when this write actually runs, not when it was scheduled, so
    // an in-flight like cannot overwrite a later unlike on the account wishlist.
    this.#pushChain = this.#pushChain.catch(() => undefined).then(async () => {
      try {
        await api.setWishlist(token, [...this.ids]);
        this.syncError = false;
      } catch {
        this.syncError = true;
      }
    });
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
