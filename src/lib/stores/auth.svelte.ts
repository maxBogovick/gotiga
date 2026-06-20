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

function clearToken(): void {
  if (!browser) return;
  localStorage.removeItem(SESSION_KEY);
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

  // Deliberate logout: drop the session and also purge anonymous claim tokens.
  logout(): void {
    this.clearSession();
    clearClaims();
  }

  get token(): string | null {
    return this.sessionToken;
  }
}

export const authStore = new AuthStore();
