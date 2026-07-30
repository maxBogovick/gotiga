import { browser } from '$app/environment';
import type { FigurineListItem } from './types/api';

/**
 * The visit ledger — a private, client-only memory of how the collection stood
 * the *last* time this visitor was here, so the home page can answer "what has
 * changed since you were last in the museum?" without any server-side tracking.
 *
 * It is deliberately the same shape of anonymity as `gotiga_viewed` /
 * `gotiga_revealed`: a single localStorage snapshot, never sent anywhere. We
 * snapshot, per work, its *status* and its *last-edited* timestamp, plus the set
 * of showing rooms, and diff against them on the next visit to surface four
 * honest signals: works that have **arrived** (a new `id`), works that have
 * **found a home** (`available` → reserved/sold), works that have been
 * **refreshed** (a changed `updatedAt` — a new photo, edited text), and halls
 * beginning to **wake** (a new showing room).
 *
 * A missing `updatedAt` on either side keeps the "refreshed" signal quiet rather
 * than misfiring.
 */

const KEY = 'gotiga_ledger';
const SCHEMA = 2;

/** Per-work snapshot: `s` = status, `u` = last-edited timestamp (may be null). */
interface WorkMark {
  s: string;
  u?: string | null;
}

interface Snapshot {
  v: number;
  /** Epoch ms of the visit this snapshot was written on. */
  seenAt: number;
  /** id → { status, updatedAt }, for every work on the home payload. */
  works: Record<string, WorkMark>;
  /** Showing-room ids known to the museum at that visit. */
  roomIds: string[];
}

export interface RoomRef {
  id: string;
  name: string;
}

export interface VisitChanges {
  /** No prior snapshot — we have no baseline to compare against. */
  firstVisit: boolean;
  /** Whole days since the previous visit (null on first visit). */
  daysSince: number | null;
  /** Works present now that were absent last time. */
  arrivals: FigurineListItem[];
  /** Works that were `available` last time and have since found a home. */
  homed: FigurineListItem[];
  /** Works edited since the last visit (new photo / text), excluding arrivals
   *  and works already surfaced as "found a home". */
  updated: FigurineListItem[];
  /** Halls announced since the last visit. */
  newRooms: RoomRef[];
  /** Any concrete change worth surfacing. */
  hasAny: boolean;
}

function readSnapshot(): Snapshot | null {
  if (!browser) return null;
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Snapshot;
    if (!parsed || parsed.v !== SCHEMA || typeof parsed.works !== 'object') return null;
    return parsed;
  } catch {
    return null;
  }
}

function buildSnapshot(figurines: FigurineListItem[], rooms: RoomRef[]): Snapshot {
  const works: Record<string, WorkMark> = {};
  for (const f of figurines) works[f.id] = { s: f.status, u: f.updatedAt ?? null };
  return {
    v: SCHEMA,
    seenAt: Date.now(),
    works,
    roomIds: rooms.map((r) => r.id),
  };
}

const HOMED_FROM = 'available';
const HOMED_TO = new Set(['reserved', 'sold']);

/**
 * Diff the current collection against the stored snapshot. Pure — does not
 * touch localStorage. Returns the empty/first-visit shape when there is no
 * usable baseline.
 */
export function diffVisit(figurines: FigurineListItem[], rooms: RoomRef[]): VisitChanges {
  const prev = readSnapshot();
  if (!prev) {
    return {
      firstVisit: true,
      daysSince: null,
      arrivals: [],
      homed: [],
      updated: [],
      newRooms: [],
      hasAny: false,
    };
  }

  const arrivals = figurines.filter((f) => !(f.id in prev.works));
  const homed = figurines.filter(
    (f) => prev.works[f.id]?.s === HOMED_FROM && HOMED_TO.has(f.status),
  );
  const homedIds = new Set(homed.map((f) => f.id));
  // "Refreshed": an existing work whose last-edited stamp moved. Skip arrivals
  // (no prior mark), works already shown as homed (a status change also bumps
  // the stamp — don't double-count), and any case where a timestamp is missing
  // on either side (a pre-v2 snapshot) so we never misfire.
  const updated = figurines.filter((f) => {
    const mark = prev.works[f.id];
    if (!mark || homedIds.has(f.id)) return false;
    return mark.u != null && f.updatedAt != null && mark.u !== f.updatedAt;
  });
  const prevRooms = new Set(prev.roomIds);
  const newRooms = rooms.filter((r) => !prevRooms.has(r.id));

  const daysSince = Math.max(0, Math.floor((Date.now() - prev.seenAt) / 86_400_000));
  const hasAny =
    arrivals.length > 0 || homed.length > 0 || updated.length > 0 || newRooms.length > 0;

  return { firstVisit: false, daysSince, arrivals, homed, updated, newRooms, hasAny };
}

/**
 * Record the current state as the new baseline for the *next* visit. Guarded
 * against wiping a real baseline with an empty payload (a failed/slow load).
 */
export function commitVisit(figurines: FigurineListItem[], rooms: RoomRef[]): void {
  if (!browser || figurines.length === 0) return;
  try {
    localStorage.setItem(KEY, JSON.stringify(buildSnapshot(figurines, rooms)));
  } catch {
    /* private mode / quota — the ledger is best-effort */
  }
}
