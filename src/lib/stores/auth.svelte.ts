import { browser } from '$app/environment';
import type { UserDto } from '$lib/types/api';

const SESSION_KEY = 'gotiga_session';

function loadToken(): string | null {
  if (!browser) return null;
  return localStorage.getItem(SESSION_KEY);
}

function saveToken(token: string): void {
  if (!browser) return;
  localStorage.setItem(SESSION_KEY, token);
}

const CLAIMS_PREFIX = 'gotiga_claims_';

// Identity-sensitive receipts/tokens that must not outlive a deliberate logout:
// commission claim tokens, a pending claim awaiting account-link, and the
// plaintext visual-password reminder written at registration.
const SENSITIVE_KEYS = [
  'gotiga_commissions',
  'gotiga_pending_claim',
  'gotiga_visual_reminder',
] as const;

// Anonymous browsing data that works without an account (wishlist + its legacy
// aliases, and viewed-history). Kept across logout/expiry — wiped only when the
// account itself is deleted.
const BROWSING_KEYS = [
  'gotiga_saved_figurines',
  'gotiga_liked',
  'gotiga_wishlist',
  'gotiga_viewed',
] as const;

function clearToken(): void {
  if (!browser) return;
  localStorage.removeItem(SESSION_KEY);
}

function removeKeys(keys: readonly string[]): void {
  if (!browser) return;
  for (const k of keys) localStorage.removeItem(k);
}

// Anonymous booking/claim cancel tokens live under CLAIMS_PREFIX and work
// without an account. Only purge them on a deliberate logout — never on an
// automatic session-expiry, which would silently destroy the user's tokens.
function clearClaims(): void {
  if (!browser) return;
  Object.keys(localStorage)
    .filter(k => k.startsWith(CLAIMS_PREFIX))
    .forEach(k => localStorage.removeItem(k));
}

class AuthStore {
  user = $state<UserDto | null>(null);
  sessionToken = $state<string | null>(loadToken());
  loading = $state(false);

  get isLoggedIn(): boolean {
    return this.user !== null;
  }

  setSession(token: string, user: UserDto): void {
    this.sessionToken = token;
    this.user = user;
    saveToken(token);
  }

  clearSession(): void {
    this.sessionToken = null;
    this.user = null;
    clearToken();
  }

  // Deliberate logout: drop the session, anonymous claim tokens, and any
  // identity-sensitive receipts. Anonymous browsing data (wishlist, viewed)
  // survives — a guest keeps using it.
  logout(): void {
    this.clearSession();
    clearClaims();
    removeKeys(SENSITIVE_KEYS);
  }

  // Account deletion: wipe every trace of this visitor from the device —
  // session, claim/commission tokens, sensitive receipts, and anonymous
  // browsing data. App/device preferences (language, font, server URL,
  // contact email, home cache, admin keys) are intentionally left untouched.
  purgeAllLocalData(): void {
    this.clearSession();
    clearClaims();
    removeKeys(SENSITIVE_KEYS);
    removeKeys(BROWSING_KEYS);
  }

  get token(): string | null {
    return this.sessionToken;
  }
}

export const authStore = new AuthStore();
