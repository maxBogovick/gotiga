import { browser } from '$app/environment';
import { minutesOfDay } from '$lib/showing-window';

/**
 * The house clock — the visitor's local minute-of-day, refreshed once a minute.
 *
 * Sealed doors read this so a door dissolves into the work *in the moment* the
 * showing window opens, while the visitor is still on the page — without it the
 * door would only lift on the next navigation. Ticking by the minute (not the
 * second) is enough: the window edges are whole minutes, and a per-minute beat
 * keeps paint near-idle across a gallery of doors.
 *
 * Re-syncs on focus / visibility, like revealed-figurines: a backgrounded tab's
 * timers throttle, so when the visitor returns the clock catches up at once
 * rather than waiting out a stale interval.
 */
class HouseClockStore {
  /** Minutes since local midnight (0..1439). */
  now = $state<number>(browser ? minutesOfDay() : 0);
  /**
   * The visitor's local moment, refreshed each minute. Day/date-scheduled doors
   * (weekday masks, annual dates) read this so they re-evaluate across midnight
   * and date boundaries, not just within a day. Updated together with `now`.
   */
  nowDate = $state<Date>(browser ? new Date() : new Date(0));

  #timer: ReturnType<typeof setInterval> | undefined;
  #started = false;

  /** Begin ticking. Idempotent — many tiles share one clock. */
  start() {
    if (!browser || this.#started) return;
    this.#started = true;
    this.#sync();
    // Align the first tick to the next minute boundary, then settle into a
    // steady per-minute beat — so doors open close to the real clock minute.
    const msToNextMinute = 60_000 - (Date.now() % 60_000);
    setTimeout(() => {
      this.#sync();
      this.#timer = setInterval(() => this.#sync(), 60_000);
    }, msToNextMinute);
    window.addEventListener('focus', this.#sync);
    document.addEventListener('visibilitychange', this.#sync);
  }

  #sync = () => {
    if (!browser) return;
    const next = minutesOfDay();
    if (next !== this.now) {
      this.now = next;
      this.nowDate = new Date();
    }
  };
}

export const houseClock = new HouseClockStore();
