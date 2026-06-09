import { api } from '$lib/api';
import { get } from 'svelte/store';
import { t } from '$lib/i18n';

type ClaimStatus = 'pending' | 'confirmed' | 'rejected' | 'cancelled' | 'completed';
export type ClaimData = {
  token: string;
  figurineName: string;
  startsAt: string;
  endsAt: string;
  submittedAt: string;
  status?: ClaimStatus;
};

type TokenLookupInfo = { figurineName: string; startsAt: string; endsAt: string; status: string };

const POLL_INTERVAL_MS = 30_000;

export class FigurineClaimsStore {
  claims = $state<ClaimData[]>([]);
  cancellingToken = $state<string | null>(null);
  claimErrors = $state<Record<string, string>>({});
  cancelledTokens = $state<Set<string>>(new Set());
  showTokenForm = $state(false);
  tokenInput = $state('');
  tokenLookupInfo = $state<TokenLookupInfo | null>(null);
  tokenLookupErr = $state('');
  tokenLooking = $state(false);
  lookupCancelling = $state(false);

  #figurineId: string;
  #refreshSchedule: () => void;
  #pollTimer: ReturnType<typeof setInterval> | null = null;

  get #key() { return `gotiga_claims_${this.#figurineId}`; }

  constructor(figurineId: string, refreshSchedule: () => void) {
    this.#figurineId = figurineId;
    this.#refreshSchedule = refreshSchedule;
  }

  load() {
    try {
      const raw = localStorage.getItem(this.#key);
      if (raw) this.claims = JSON.parse(raw);
      // Migrate old single-item format
      const old = localStorage.getItem(`gotiga_claim_${this.#figurineId}`);
      if (old) {
        const parsed = JSON.parse(old) as ClaimData;
        if (!this.claims.some(c => c.token === parsed.token)) this.claims = [parsed, ...this.claims];
        localStorage.removeItem(`gotiga_claim_${this.#figurineId}`);
        this.#save();
      }
    } catch { /* ignore */ }
  }

  #save() {
    try { localStorage.setItem(this.#key, JSON.stringify(this.claims)); } catch { /* ignore */ }
  }

  async verify() {
    if (this.claims.length === 0) return;
    const results = await Promise.allSettled(this.claims.map(c => api.getBookingByToken(c.token)));
    let changed = false;
    const updated = this.claims
      .map((c, i) => {
        const r = results[i];
        if (r.status !== 'fulfilled') return c;
        const serverStatus = r.value.status as ClaimStatus;
        if (serverStatus !== c.status) { changed = true; return { ...c, status: serverStatus }; }
        return c;
      })
      .filter(c => c.status !== 'cancelled' && c.status !== 'rejected' && c.status !== 'completed');
    if (changed || updated.length !== this.claims.length) {
      this.claims = updated;
      this.#save();
      if (changed) this.#refreshSchedule();
    }
    this.#syncPollTimer();
  }

  #hasPendingClaims() {
    return this.claims.some(c => c.status === 'pending' || c.status == null);
  }

  #syncPollTimer() {
    if (this.#hasPendingClaims()) {
      if (!this.#pollTimer) {
        this.#pollTimer = setInterval(() => this.verify(), POLL_INTERVAL_MS);
      }
    } else {
      this.stopPolling();
    }
  }

  startPolling() {
    this.#syncPollTimer();
  }

  stopPolling() {
    if (this.#pollTimer) {
      clearInterval(this.#pollTimer);
      this.#pollTimer = null;
    }
  }

  onBookingCreated(claim: ClaimData) {
    this.claims = [claim, ...this.claims];
    this.#save();
    this.#refreshSchedule();
    this.#syncPollTimer();
  }

  async cancel(claim: ClaimData) {
    this.cancellingToken = claim.token;
    this.claimErrors = { ...this.claimErrors, [claim.token]: '' };
    try {
      await api.cancelBookingByToken(claim.token);
      this.cancelledTokens = new Set([...this.cancelledTokens, claim.token]);
      this.claims = this.claims.filter(c => c.token !== claim.token);
      this.#save();
      this.#refreshSchedule();
      this.#syncPollTimer();
      setTimeout(() => {
        this.cancelledTokens = new Set([...this.cancelledTokens].filter(tok => tok !== claim.token));
      }, 4000);
    } catch {
      this.claimErrors = { ...this.claimErrors, [claim.token]: get(t)('claimCancelError') };
    } finally {
      this.cancellingToken = null;
    }
  }

  async lookupToken() {
    const tok = this.tokenInput.trim().toUpperCase();
    if (!tok) return;
    this.tokenLooking = true;
    this.tokenLookupErr = '';
    this.tokenLookupInfo = null;
    try {
      this.tokenLookupInfo = await api.getBookingByToken(tok);
    } catch {
      this.tokenLookupErr = get(t)('claimTokenNotFound');
    } finally {
      this.tokenLooking = false;
    }
  }

  async cancelFromLookup() {
    if (!this.tokenLookupInfo) return;
    this.lookupCancelling = true;
    try {
      await api.cancelBookingByToken(this.tokenInput.trim().toUpperCase());
      this.tokenLookupInfo = { ...this.tokenLookupInfo, status: 'cancelled' };
      this.#refreshSchedule();
    } catch {
      this.tokenLookupErr = get(t)('claimCancelError');
    } finally {
      this.lookupCancelling = false;
    }
  }
}
