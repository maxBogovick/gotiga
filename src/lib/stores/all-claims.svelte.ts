import { api } from '$lib/api';
import { get } from 'svelte/store';
import { t } from '$lib/i18n';

type ClaimStatus = 'pending' | 'confirmed' | 'rejected' | 'cancelled' | 'completed';

export type GlobalClaimData = {
  token: string;
  figurineId: string;
  figurineName: string;
  startsAt: string;
  endsAt: string;
  submittedAt: string;
  status?: ClaimStatus;
};

export type StatusNotification = {
  id: string;
  token: string;
  figurineName: string;
  figurineId: string;
  newStatus: ClaimStatus;
};

const POLL_MS = 30_000;
const PREFIX = 'gotiga_claims_';

class AllClaimsStore {
  claims = $state<GlobalClaimData[]>([]);
  cancellingToken = $state<string | null>(null);
  errors = $state<Record<string, string>>({});
  notifications = $state<StatusNotification[]>([]);

  #pollTimer: ReturnType<typeof setInterval> | null = null;
  #pollRefs = 0;
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
    // Single batched request for all tokens. Tokens absent from the map are treated
    // as "not found / unchanged" — same semantics as a rejected single-token lookup.
    let fresh: Record<string, { status: string }>;
    try {
      fresh = await api.getBookingsByTokens(this.claims.map(c => c.token));
    } catch {
      this.#syncTimer();
      return;
    }
    // Snapshot status before any changes to detect transitions
    const before = new Map(this.claims.map(c => [c.token, c.status]));
    let changed = false;
    const mapped = this.claims.map((c) => {
      const r = fresh[c.token];
      if (!r) return c;
      const s = r.status as ClaimStatus;
      if (s !== c.status) { changed = true; return { ...c, status: s }; }
      return c;
    });
    // Emit notifications for every status transition
    for (const c of mapped) {
      const oldStatus = before.get(c.token);
      if (c.status && c.status !== oldStatus) {
        this.#pushNotification(c as GlobalClaimData & { status: ClaimStatus });
      }
    }
    const updated = mapped.filter(
      c => c.status !== 'cancelled' && c.status !== 'rejected' && c.status !== 'completed'
    );
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

  startPolling() {
    this.#pollRefs++;
    this.#syncTimer();
  }

  stopPolling() {
    this.#pollRefs = Math.max(0, this.#pollRefs - 1);
    if (this.#pollRefs === 0 && this.#pollTimer) {
      clearInterval(this.#pollTimer);
      this.#pollTimer = null;
    }
  }

  reload() {
    this.#loaded = false;
    this.claims = [];
    this.load();
  }

  dismissNotification(id: string) {
    this.notifications = this.notifications.filter(n => n.id !== id);
  }

  #pushNotification(claim: GlobalClaimData & { status: ClaimStatus }) {
    const id = `${claim.token}-${Date.now()}`;
    this.notifications = [...this.notifications, {
      id,
      token: claim.token,
      figurineName: claim.figurineName,
      figurineId: claim.figurineId,
      newStatus: claim.status,
    }];
    setTimeout(() => { this.dismissNotification(id); }, 6000);
  }

  // Poll while: (a) future pending claims exist, OR (b) any confirmed claim exists.
  // Past-date pending claims are excluded — they can never change via admin action in time.
  #hasPollable() {
    const today = new Date().toISOString().split('T')[0];
    return this.claims.some(c =>
      ((c.status === 'pending' || c.status == null) && c.endsAt >= today) ||
      c.status === 'confirmed'
    );
  }

  #syncTimer() {
    if (this.#hasPollable() && this.#pollRefs > 0) {
      if (!this.#pollTimer) this.#pollTimer = setInterval(() => this.verify(), POLL_MS);
    } else if (!this.#hasPollable() && this.#pollTimer) {
      clearInterval(this.#pollTimer);
      this.#pollTimer = null;
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
