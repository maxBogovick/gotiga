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
  #inflight: Promise<void> | null = null;
  /** Bumped by refresh(); an in-flight load from an older generation must not write. */
  #generation = 0;

  /**
   * Fetch rooms once. Idempotent — many tiles call this on mount — and, importantly,
   * every caller AWAITS THE SAME FETCH.
   *
   * This used to guard with a plain `#loaded` flag set before the request went out, so a
   * second caller arriving while the first was still in flight was told "already loaded"
   * and returned immediately — with `list` still empty. That is not a theoretical race:
   * the home page awaits this to decide which work the vitrine may spotlight (a sealed
   * piece must never be picked), and HouseNoticeBoard, mounted on the same page, calls
   * load() too. Whoever lost got an empty room list and gated against nothing.
   */
  async load() {
    if (!browser || this.#loaded) return;
    this.#inflight ??= (async () => {
      // A refresh() that lands while this request is in the air supersedes it. Without the
      // generation check the older, slower response would still write its (now stale) rooms
      // over the newer ones, and its `finally` would clear the NEW request's registration.
      const generation = this.#generation;
      try {
        const rooms = await api.getShowingRooms();
        if (generation !== this.#generation) return;
        this.list = rooms;
        this.#loaded = true;
      } catch {
        // Leave #loaded false so a later caller can retry.
      } finally {
        if (generation === this.#generation) this.#inflight = null;
      }
    })();
    return this.#inflight;
  }

  /** Force a refresh (admin saved/deleted a room). */
  async refresh() {
    this.#generation += 1;
    this.#loaded = false;
    this.#inflight = null;
    await this.load();
  }
}

export const showingRooms = new ShowingRoomsStore();
