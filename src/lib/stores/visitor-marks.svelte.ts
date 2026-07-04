import { browser } from '$app/environment';
import { api } from '$lib/api';
import type { MarkTone } from '$lib/types/api';

// "Marks of attention" — a single quiet wax-seal gesture a visitor can leave on a
// figurine, in one of 3 private tones (see TONE_CYCLE). Deliberately not a rating:
// no number or tone is ever shown on the public site, only whether *this* visitor
// has marked *this* piece (see project decision to avoid vanity-metric /
// negative-social-proof effects). The visitor token is opaque client state, not a
// login — it exists purely so the server can dedupe repeat marks.
const TOKEN_KEY = 'gotiga_visitor_token';
const MARKS_KEY = 'gotiga_marks';

// Canonical display order for the 3 tones — the picker row on the image shows
// them in this order.
export const MARK_TONE_ORDER: MarkTone[] = ['touched', 'mesmerized', 'desired'];

function readMarks(): Record<string, MarkTone> {
  if (!browser) return {};
  try {
    const raw = localStorage.getItem(MARKS_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    // Legacy schema (checkpoint 1: plain id array, single implicit tone) — migrate.
    if (Array.isArray(parsed)) {
      const migrated: Record<string, MarkTone> = {};
      for (const id of parsed) if (typeof id === 'string') migrated[id] = 'touched';
      return migrated;
    }
    if (parsed && typeof parsed === 'object') {
      const result: Record<string, MarkTone> = {};
      for (const [id, tone] of Object.entries(parsed as Record<string, unknown>)) {
        if (typeof id === 'string' && MARK_TONE_ORDER.includes(tone as MarkTone)) result[id] = tone as MarkTone;
      }
      return result;
    }
  } catch {
    // fall through
  }
  return {};
}

class VisitorMarksStore {
  marks = $state<Record<string, MarkTone>>({});

  #loaded = false;
  #listening = false;
  // One request in flight per figurine at a time — a second call for the same
  // figurine (e.g. the visitor changes their tone pick before the first
  // request lands) is queued after the current one instead of firing
  // concurrently, so responses can't resolve out of order and desync the
  // local optimistic state. Unlike a plain de-dupe, a differing tone is never
  // silently dropped — it always gets its own request once the prior settles.
  #pending = new Map<string, Promise<MarkTone | null>>();

  load() {
    if (!browser || this.#loaded) return;
    this.#loaded = true;
    this.marks = readMarks();
    this.#listen();
  }

  // Never calls load() — this is read inside `$derived(...)` in components
  // (e.g. the layout gallery overlay), and mutating $state during a derived's
  // computation is forbidden by Svelte. Callers must invoke load() once from
  // onMount, same as the sibling savedFigurines store.
  toneOf(figurineId: string): MarkTone | null {
    return this.marks[figurineId] ?? null;
  }

  has(figurineId: string) {
    return figurineId in this.marks;
  }

  // Sets (or clears, when tone is null) the mark directly — the visitor picks
  // the exact tone from the on-image row, no cycling/guessing. Confirms with
  // the server; reverts silently if the request fails.
  set(figurineId: string, tone: MarkTone | null): Promise<MarkTone | null> {
    this.load();
    const previous = this.marks[figurineId] ?? null;
    this.#setLocal(figurineId, tone);

    const ahead = this.#pending.get(figurineId) ?? Promise.resolve(null);
    const request = ahead.catch(() => null).then(async () => {
      try {
        const { marked, tone: resultTone } = await api.toggleFigurineMark(figurineId, this.#token(), tone);
        const resolved = marked ? resultTone : null;
        this.#setLocal(figurineId, resolved);
        return resolved;
      } catch {
        this.#setLocal(figurineId, previous);
        return previous;
      } finally {
        if (this.#pending.get(figurineId) === request) this.#pending.delete(figurineId);
      }
    });

    this.#pending.set(figurineId, request);
    return request;
  }

  #setLocal(figurineId: string, tone: MarkTone | null) {
    const next = { ...this.marks };
    if (tone) next[figurineId] = tone;
    else delete next[figurineId];
    this.marks = next;
    this.#persist();
  }

  #token(): string {
    let token = browser ? localStorage.getItem(TOKEN_KEY) : null;
    if (!token) {
      token = crypto.randomUUID();
      if (browser) localStorage.setItem(TOKEN_KEY, token);
    }
    return token;
  }

  #persist() {
    if (!browser) return;
    localStorage.setItem(MARKS_KEY, JSON.stringify(this.marks));
  }

  #listen() {
    if (!browser || this.#listening) return;
    this.#listening = true;
    window.addEventListener('storage', (event) => {
      if (event.key !== MARKS_KEY) return;
      this.marks = readMarks();
    });
  }
}

export const visitorMarks = new VisitorMarksStore();
