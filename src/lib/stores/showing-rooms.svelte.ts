import { browser } from '$app/environment';
import { api } from '$lib/api';
import type { ShowingRoom } from '$lib/types/api';

/**
 * Showing rooms cache — the named shared windows a figurine can point at.
 *
 * The door gating (HomeFigurineTile / archive / detail) resolves each work's
 * effective window against this list (see `resolveWindow`). Loaded once and held
 * reactively: when it arrives the doors recompute, so a card that belongs to a
 * room seals/opens correctly without a reload. Failure is silent — an empty list
 * means rooms simply don't gate (works fall back to their own window / open).
 */
class ShowingRoomsStore {
  list = $state<ShowingRoom[]>([]);
  #loaded = false;

  /** Fetch rooms once. Idempotent — many tiles call this on mount. */
  async load() {
    if (!browser || this.#loaded) return;
    this.#loaded = true;
    try {
      this.list = await api.getShowingRooms();
    } catch {
      this.#loaded = false; // allow a later retry
    }
  }

  /** Force a refresh (admin saved/deleted a room). */
  async refresh() {
    this.#loaded = false;
    await this.load();
  }
}

export const showingRooms = new ShowingRoomsStore();
