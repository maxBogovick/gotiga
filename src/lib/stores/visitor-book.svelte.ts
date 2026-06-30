import { browser } from '$app/environment';

/**
 * visitorBook — the reader's standing in the house guest book, kept on this
 * device. Single opt-in: the signature lives in localStorage (no account), so
 * the home page can recognise a returning subscriber and unlock the book-holder
 * surfaces ("first look") without a login. Shared as a singleton so the book
 * form, the recognition spread, and the first-look band all react together.
 */

const KEY = 'gotiga_visitor_book';

interface Saved {
  email?: string;
  name?: string;
  token?: string;
  signedAt?: string; // ISO
}

function read(): Saved | null {
  if (!browser) return null;
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return null;
    const data = JSON.parse(raw) as Saved;
    return data && typeof data === 'object' ? data : null;
  } catch {
    return null;
  }
}

class VisitorBookStore {
  signed = $state(false);
  email = $state('');
  name = $state('');
  token = $state<string | null>(null);
  signedAt = $state<string | null>(null);

  #loaded = false;
  #listening = false;

  load() {
    if (!browser || this.#loaded) return;
    this.#loaded = true;
    this.#hydrate(read());
    this.#listen();
  }

  /** Number of whole days since the signature (0 if unknown / today). */
  get daysSince(): number {
    if (!this.signedAt) return 0;
    const then = new Date(this.signedAt).getTime();
    if (Number.isNaN(then)) return 0;
    return Math.max(0, Math.floor((Date.now() - then) / 86_400_000));
  }

  sign(token: string, email: string, name: string) {
    this.token = token;
    this.email = email;
    this.name = name.trim();
    this.signedAt = new Date().toISOString();
    this.signed = true;
    this.#persist();
  }

  leave() {
    this.token = null;
    this.email = '';
    this.name = '';
    this.signedAt = null;
    this.signed = false;
    if (browser) {
      try { localStorage.removeItem(KEY); } catch { /* ignore */ }
    }
  }

  #hydrate(data: Saved | null) {
    if (data?.token) {
      this.token = data.token;
      this.email = data.email ?? '';
      this.name = data.name ?? '';
      this.signedAt = data.signedAt ?? null;
      this.signed = true;
    } else {
      this.signed = false;
      this.token = null;
      this.email = '';
      this.name = '';
      this.signedAt = null;
    }
  }

  #persist() {
    if (!browser) return;
    try {
      localStorage.setItem(
        KEY,
        JSON.stringify({
          email: this.email,
          name: this.name,
          token: this.token ?? undefined,
          signedAt: this.signedAt ?? undefined,
        } satisfies Saved)
      );
    } catch {
      /* ignore quota / disabled storage */
    }
  }

  #listen() {
    if (!browser || this.#listening) return;
    this.#listening = true;
    window.addEventListener('storage', (event) => {
      if (event.key !== KEY) return;
      this.#hydrate(read());
    });
  }
}

export const visitorBook = new VisitorBookStore();
