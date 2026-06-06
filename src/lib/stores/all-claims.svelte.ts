import { api } from '$lib/api';
import { get } from 'svelte/store';
import { t } from '$lib/i18n';

type ClaimStatus = 'pending' | 'confirmed' | 'rejected' | 'cancelled';

export type GlobalClaimData = {
  token: string;
  figurineId: string;
  figurineName: string;
  startsAt: string;
  endsAt: string;
  submittedAt: string;
  status?: ClaimStatus;
};

const POLL_MS = 30_000;
const PREFIX = 'gotiga_claims_';

class AllClaimsStore {
  claims = $state<GlobalClaimData[]>([]);
  cancellingToken = $state<string | null>(null);
  errors = $state<Record<string, string>>({});

  #pollTimer: ReturnType<typeof setInterval> | null = null;
  #loaded = false;

  get activeCount() {
    return this.claims.filter(c => c.status !== 'cancelled' && c.status !== 'rejected').length;
  }

  get pendingCount() {
    return this.claims.filter(c => c.status === 'pending' || c.status == null).length;
  }

  load() {
    if (this.#loaded) return;
    this.#loaded = true;
    const result: GlobalClaimData[] = [];
    try {
      for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i);
        if (!key?.startsWith(PREFIX)) continue;
        const figurineId = key.slice(PREFIX.length);
        const raw = localStorage.getItem(key);
        if (!raw) continue;
        const items = JSON.parse(raw) as Array<Omit<GlobalClaimData, 'figurineId'>>;
        for (const item of items) result.push({ ...item, figurineId });
      }
    } catch { /* ignore */ }
    this.claims = result;
  }

  async verify() {
    if (this.claims.length === 0) { this.#syncTimer(); return; }
    const results = await Promise.allSettled(
      this.claims.map(c => api.getBookingByToken(c.token))
    );
    let changed = false;
    const updated = this.claims
      .map((c, i) => {
        const r = results[i];
        if (r.status !== 'fulfilled') return c;
        const s = r.value.status as ClaimStatus;
        if (s !== c.status) { changed = true; return { ...c, status: s }; }
        return c;
      })
      .filter(c => c.status !== 'cancelled' && c.status !== 'rejected');
    if (changed || updated.length !== this.claims.length) {
      this.claims = updated;
      this.#persistAll();
    }
    this.#syncTimer();
  }

  async cancel(claim: GlobalClaimData) {
    this.cancellingToken = claim.token;
    this.errors = { ...this.errors, [claim.token]: '' };
    try {
      await api.cancelBookingByToken(claim.token);
      this.claims = this.claims.filter(c => c.token !== claim.token);
      this.#persistFigurine(claim.figurineId);
      this.#syncTimer();
    } catch {
      this.errors = { ...this.errors, [claim.token]: get(t)('claimCancelError') };
    } finally {
      this.cancellingToken = null;
    }
  }

  startPolling() { this.#syncTimer(); }

  stopPolling() {
    if (this.#pollTimer) { clearInterval(this.#pollTimer); this.#pollTimer = null; }
  }

  reload() {
    this.#loaded = false;
    this.claims = [];
    this.load();
  }

  #hasPending() {
    return this.claims.some(c => c.status === 'pending' || c.status == null);
  }

  #syncTimer() {
    if (this.#hasPending()) {
      if (!this.#pollTimer) this.#pollTimer = setInterval(() => this.verify(), POLL_MS);
    } else {
      this.stopPolling();
    }
  }

  #persistFigurine(figurineId: string) {
    try {
      const items = this.claims.filter(c => c.figurineId === figurineId);
      const key = `${PREFIX}${figurineId}`;
      if (items.length === 0) localStorage.removeItem(key);
      else localStorage.setItem(key, JSON.stringify(items));
    } catch { /* ignore */ }
  }

  #persistAll() {
    const byFig = new Map<string, GlobalClaimData[]>();
    for (const c of this.claims) {
      const arr = byFig.get(c.figurineId) ?? [];
      arr.push(c);
      byFig.set(c.figurineId, arr);
    }
    try {
      const toRemove: string[] = [];
      for (let i = 0; i < localStorage.length; i++) {
        const k = localStorage.key(i);
        if (k?.startsWith(PREFIX)) toRemove.push(k);
      }
      for (const k of toRemove) localStorage.removeItem(k);
      for (const [fid, items] of byFig) {
        localStorage.setItem(`${PREFIX}${fid}`, JSON.stringify(items));
      }
    } catch { /* ignore */ }
  }
}

export const allClaims = new AllClaimsStore();
