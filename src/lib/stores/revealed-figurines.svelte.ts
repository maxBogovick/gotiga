import { browser } from '$app/environment';

/**
 * "Keyhole" memory — which works are currently unsealed on the cards.
 *
 * Distinct from `gotiga_viewed` (the permanent "I have seen this" ledger that
 * drives the archive's filter). Revealing is deliberately *forgetful*: only the
 * last few opened works stay lit; opening more lets the older ones fall back
 * into shadow, so the archive never gives itself away wholesale — the dust
 * settles back over the rooms you have walked away from.
 *
 * Rolling window by count (not by time): predictable, never re-seals a card
 * while the visitor is looking at it, and reproduces the intended sequence
 * (open A, B → open C re-seals A). Change REVEAL_WINDOW to widen/narrow it.
 */
const KEY = 'gotiga_revealed';

/** How many most-recently-opened works stay unsealed. */
export const REVEAL_WINDOW = 2;

function read(): string[] {
  if (!browser) return [];
  try {
    const parsed = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return Array.isArray(parsed)
      ? parsed.filter((id): id is string => typeof id === 'string')
      : [];
  } catch {
    return [];
  }
}

class RevealedFigurinesStore {
  ids = $state<string[]>([]);

  #loaded = false;
  #listening = false;

  load() {
    if (!browser || this.#loaded) return;
    this.#loaded = true;
    this.ids = read().slice(-REVEAL_WINDOW);
    this.#listen();
  }

  refresh() {
    if (!browser) return;
    const next = read().slice(-REVEAL_WINDOW);
    // Skip the write when nothing changed — every assignment invalidates the
    // `sealed` derivation of every tile, and `focus`/`visibilitychange` fire on
    // ordinary tab switches with no cross-tab write behind them.
    if (next.length === this.ids.length && next.every((v, i) => v === this.ids[i])) return;
    this.ids = next;
  }

  has(id: string) {
    return this.ids.includes(id);
  }

  /**
   * Open a work: it becomes the most-recently revealed. Only the last
   * REVEAL_WINDOW stay lit — anything older falls back into shadow.
   */
  reveal(id: string) {
    if (!browser || !id) return;
    this.load();
    this.ids = [...this.ids.filter((x) => x !== id), id].slice(-REVEAL_WINDOW);
    this.#persist();
  }

  #persist() {
    if (!browser) return;
    try {
      localStorage.setItem(KEY, JSON.stringify(this.ids));
    } catch {}
  }

  #listen() {
    if (!browser || this.#listening) return;
    this.#listening = true;
    // Other tabs writing the key, and returning focus to this one, both
    // re-sync the in-memory window.
    window.addEventListener('storage', (event) => {
      if (event.key && event.key !== KEY) return;
      this.refresh();
    });
    window.addEventListener('focus', () => this.refresh());
    document.addEventListener('visibilitychange', () => this.refresh());
  }
}

export const revealedFigurines = new RevealedFigurinesStore();
