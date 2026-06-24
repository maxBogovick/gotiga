/**
 * "Дом просыпается" — the showing window.
 *
 * A work may be enterable only while the visitor's LOCAL clock sits inside a
 * daily window [openFromMin, openUntilMin), both expressed as minutes from
 * midnight (0..1439). Outside it, the card shows a carved sealed door (see
 * SealedDoor.svelte) and cannot be entered.
 *
 * This is a ritual, not a FOMO drop: the window repeats every day and nothing
 * ever vanishes — miss the night and the door simply opens again tomorrow.
 * Because there is no prize behind the gate, we never validate the clock on the
 * server: a guest who winds their clock to peek has merely performed the ritual
 * by hand. Read against the guest's own device time, it works offline (Tauri)
 * with no server round-trip.
 *
 * Pure + side-effect free so it can be unit-reasoned and reused on both card and
 * detail. The live "tick" that re-opens a door while the page is watched lives
 * in the house-clock store, not here.
 */

export interface ShowingWindow {
  openFromMin?: number | null;
  openUntilMin?: number | null;
  /** Allowed weekdays bitmask (bit0=Mon … bit6=Sun). null/0/127 → every day. */
  daysMask?: number | null;
  /** "MM-DD" — open every year on that date. */
  monthDay?: string | null;
  /** One-off inclusive date range, "YYYY-MM-DD". */
  dateFrom?: string | null;
  dateUntil?: string | null;
}

/** A named, shared showing window several works point at. */
export interface ShowingRoomLike {
  id: string;
  openFromMin: number;
  openUntilMin: number;
  openDaysMask?: number | null;
  openMonthDay?: string | null;
  openDateFrom?: string | null;
  openDateUntil?: string | null;
}

/** A figurine's window inputs: either a room reference OR its own hours. */
export interface WindowSource extends ShowingWindow {
  showingRoomId?: string | null;
}

/**
 * Resolve the effective window for a work. A work belongs to a room OR carries
 * its own hours — never both (the admin form enforces mutual exclusivity):
 *   showingRoomId set + room found → the room's window
 *   showingRoomId set but room missing (deleted) → ungated (door never seals)
 *   else → the work's own open_from/until (or ungated if unset)
 * Pure, so card/detail and server-agnostic gating share one source of truth.
 */
export function resolveWindow(fig: WindowSource, rooms: ShowingRoomLike[]): ShowingWindow {
  if (fig.showingRoomId) {
    const room = rooms.find((r) => r.id === fig.showingRoomId);
    return room
      ? {
          openFromMin: room.openFromMin,
          openUntilMin: room.openUntilMin,
          daysMask: room.openDaysMask,
          monthDay: room.openMonthDay,
          dateFrom: room.openDateFrom,
          dateUntil: room.openDateUntil,
        }
      : {};
  }
  // Figurine "own hours" mode carries no day/date restriction (rooms only).
  return { openFromMin: fig.openFromMin, openUntilMin: fig.openUntilMin };
}

export const MINUTES_IN_DAY = 24 * 60;

/** Minutes since local midnight for a moment (default: now). */
export function minutesOfDay(d: Date = new Date()): number {
  return d.getHours() * 60 + d.getMinutes();
}

/** True when the window restricts the time of day (both ends set). */
export function hasTimeWindow(w: ShowingWindow): boolean {
  return w.openFromMin != null && w.openUntilMin != null;
}

/** True when the weekday mask is a real restriction (not all/none → every day). */
export function hasDayRestriction(w: ShowingWindow): boolean {
  const m = w.daysMask;
  if (m == null) return false;
  const days = m & 0b1111111;
  return days !== 0 && days !== 0b1111111;
}

/** True when a calendar-date restriction (annual day or one-off range) is set. */
export function hasDateRestriction(w: ShowingWindow): boolean {
  return !!w.monthDay || !!w.dateFrom || !!w.dateUntil;
}

/**
 * A work is "gated" when any restriction is configured — time of day, weekday,
 * or calendar date. A half-set time window (one end null) alone is treated as not
 * configured, so a partial admin entry never accidentally seals a piece.
 */
export function isGated(w: ShowingWindow): boolean {
  return hasTimeWindow(w) || hasDayRestriction(w) || hasDateRestriction(w);
}

const pad2 = (n: number) => String(n).padStart(2, '0');
const localMonthDay = (d: Date) => `${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
const localIsoDate = (d: Date) => `${d.getFullYear()}-${localMonthDay(d)}`;
/** Weekday index with Monday = 0 … Sunday = 6 (matches the bitmask). */
const mondayIndex = (d: Date) => (d.getDay() + 6) % 7;

/**
 * Is the work showing right now (`now`, the visitor's local moment)? Combines all
 * three restrictions: calendar date → weekday → time of day. Ungated → always true.
 *
 * For a window that wraps past midnight (until < from), the small-hours tail
 * belongs to the *previous* day — so a "Friday 23:00→02:00" room is still open at
 * Saturday 01:00. The date and weekday checks therefore run against that start day.
 */
export function isShowingOpen(w: ShowingWindow, now: Date = new Date()): boolean {
  if (!isGated(w)) return true;

  const nowMin = minutesOfDay(now);
  // The day this showing instance belongs to (yesterday while in a wrap tail).
  let day = now;
  if (
    hasTimeWindow(w) &&
    (w.openUntilMin as number) < (w.openFromMin as number) &&
    nowMin < (w.openFromMin as number)
  ) {
    day = new Date(now.getTime() - 24 * 60 * 60 * 1000);
  }

  if (hasDateRestriction(w)) {
    if (w.monthDay) {
      if (localMonthDay(day) !== w.monthDay) return false;
    } else {
      const iso = localIsoDate(day);
      if (w.dateFrom && iso < w.dateFrom) return false;
      if (w.dateUntil && iso > w.dateUntil) return false;
    }
  }

  if (hasDayRestriction(w)) {
    if (((w.daysMask as number) >> mondayIndex(day)) & 1) {
      /* allowed weekday */
    } else {
      return false;
    }
  }

  if (hasTimeWindow(w) && !isWindowOpen(w, nowMin)) return false;

  return true;
}

/**
 * Minutes until this (currently closed) window next opens, or null if it isn't
 * opening within `horizon` minutes — or has no time-of-day edge to count toward
 * (days/date-only rooms), or is already open. Used by the door to warm a glow as
 * opening nears. Confirms the upcoming opening is real (right weekday/date), so a
 * weekend room doesn't glow on a Wednesday night.
 */
export function minutesUntilOpen(
  w: ShowingWindow,
  now: Date = new Date(),
  horizon = 60
): number | null {
  if (!hasTimeWindow(w) || isShowingOpen(w, now)) return null;
  const candidate = new Date(now);
  candidate.setHours(0, 0, 0, 0);
  candidate.setMinutes(w.openFromMin as number);
  if (candidate.getTime() <= now.getTime()) candidate.setDate(candidate.getDate() + 1);
  const mins = (candidate.getTime() - now.getTime()) / 60000;
  if (mins > horizon) return null;
  // A minute into the candidate window: confirms day/date actually allow it.
  if (!isShowingOpen(w, new Date(candidate.getTime() + 60000))) return null;
  return mins;
}

/**
 * Is the work enterable at `nowMin` (minutes from local midnight)?
 * Ungated → always true. Supports windows that wrap past midnight
 * (until < from), e.g. a night room open 23:00→04:00.
 */
export function isWindowOpen(w: ShowingWindow, nowMin: number = minutesOfDay()): boolean {
  if (!isGated(w)) return true;
  const from = w.openFromMin as number;
  const until = w.openUntilMin as number;
  if (from === until) return true; // degenerate window → never seal
  return from < until
    ? nowMin >= from && nowMin < until // same-day window
    : nowMin >= from || nowMin < until; // wraps past midnight
}

/** Format minutes-from-midnight as a 24h HH:MM clock label. */
export function formatMinutes(min: number): string {
  const m = ((Math.round(min) % MINUTES_IN_DAY) + MINUTES_IN_DAY) % MINUTES_IN_DAY;
  const hh = Math.floor(m / 60);
  const mm = m % 60;
  return `${String(hh).padStart(2, '0')}:${String(mm).padStart(2, '0')}`;
}

/**
 * Classify the closed window so the door can choose its phrase. "Night" when the
 * opening hour falls in the small hours (the house wakes after dark); otherwise
 * "day" (open while there is daylight, shut at night).
 */
export function windowKind(w: ShowingWindow): 'night' | 'day' {
  const from = w.openFromMin ?? 0;
  // Opens between 20:00 and 06:00 → a night room.
  return from >= 20 * 60 || from < 6 * 60 ? 'night' : 'day';
}
