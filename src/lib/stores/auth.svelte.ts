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

function clearToken(): void {
  if (!browser) return;
  localStorage.removeItem(SESSION_KEY);
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

  get token(): string | null {
    return this.sessionToken;
  }
}

export const authStore = new AuthStore();
