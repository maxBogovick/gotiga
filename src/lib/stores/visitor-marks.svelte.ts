import { browser } from '$app/environment';
import { api } from '$lib/api';

// "Marks of attention" — a single quiet wax-seal gesture a visitor can leave on a
// figurine. Deliberately not a rating: no number is ever shown on the public site,
// only whether *this* visitor has marked *this* piece (see project decision to
// avoid vanity-metric / negative-social-proof effects). The visitor token is opaque
// client state, not a login — it exists purely so the server can dedupe repeat marks.
const TOKEN_KEY = 'gotiga_visitor_token';
const MARKS_KEY = 'gotiga_marks';

function readIds(): string[] {
  if (!browser) return [];
  try {
    const parsed = JSON.parse(localStorage.getItem(MARKS_KEY) ?? '[]');
    return Array.isArray(parsed) ? parsed.filter((id): id is string => typeof id === 'string') : [];
  } catch {
    return [];
  }
}

function dedupe(ids: string[]): string[] {
  return [...new Set(ids.filter(Boolean))];
}

class VisitorMarksStore {
  ids = $state<string[]>([]);

  #loaded = false;
  #listening = false;
  // In-flight toggle per figurine, keyed so a double-click (or a click landing
  // before the previous response returns) joins the same request instead of
  // firing a second one — two overlapping requests can resolve out of order
  // and leave the local optimistic state disagreeing with the server.
  #pending = new Map<string, Promise<boolean>>();

  load() {
    if (!browser || this.#loaded) return;
    this.#loaded = true;
    this.ids = dedupe(readIds());
    this.#listen();
  }

  // Never calls load() — this is read inside `$derived(...)` in components
  // (e.g. FigurineDetailView), and mutating $state during a derived's
  // computation is forbidden by Svelte. Callers must invoke load() once from
  // onMount, same as the sibling savedFigurines store.
  has(figurineId: string) {
    return this.ids.includes(figurineId);
  }

  // Optimistic toggle: flips local state immediately (the seal presses instantly),
  // then confirms with the server; reverts silently if the request fails.
  toggle(figurineId: string): Promise<boolean> {
    this.load();
    const inFlight = this.#pending.get(figurineId);
    if (inFlight) return inFlight;

    const wasMarked = this.ids.includes(figurineId);
    this.#setLocal(figurineId, !wasMarked);

    const request = (async () => {
      try {
        const { marked } = await api.toggleFigurineMark(figurineId, this.#token());
        this.#setLocal(figurineId, marked);
        return marked;
      } catch {
        this.#setLocal(figurineId, wasMarked);
        return wasMarked;
      } finally {
        this.#pending.delete(figurineId);
      }
    })();

    this.#pending.set(figurineId, request);
    return request;
  }

  #setLocal(figurineId: string, marked: boolean) {
    const set = new Set(this.ids);
    if (marked) set.add(figurineId);
    else set.delete(figurineId);
    this.ids = [...set];
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
    localStorage.setItem(MARKS_KEY, JSON.stringify(this.ids));
  }

  #listen() {
    if (!browser || this.#listening) return;
    this.#listening = true;
    window.addEventListener('storage', (event) => {
      if (event.key !== MARKS_KEY) return;
      this.ids = dedupe(readIds());
    });
  }
}

export const visitorMarks = new VisitorMarksStore();
