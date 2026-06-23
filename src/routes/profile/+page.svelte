<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { t, lang , brandName } from '$lib/i18n';
  import { api, resolveMediaUrl } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
  import type { UserBookingDto, UserOrderDto, MessageThreadDto, ThreadDetailDto, CommissionDto, AttachmentInput, FigurineListItem, LinkClaimResponse, LinkClaimKind, WaitlistEntryDto } from '$lib/types/api';
  import AppImage from '$lib/components/AppImage.svelte';
  import MessageAttachments from '$lib/components/MessageAttachments.svelte';
  import CommissionEditModal from '$lib/components/CommissionEditModal.svelte';

  // ── Cabinet navigation: hub → section → card ──
  type View = 'hub' | 'dealings' | 'card' | 'messages' | 'wishlist';
  let view = $state<View>('hub');
  type DealFilter = 'all' | 'booking' | 'order' | 'commission' | 'waitlist';
  let dealFilter = $state<DealFilter>('all');
  let openCardKey = $state<string | null>(null);

  let bookings = $state<UserBookingDto[]>([]);
  let orders = $state<UserOrderDto[]>([]);
  let commissions = $state<CommissionDto[]>([]);
  let waitlist = $state<WaitlistEntryDto[]>([]);
  let wishlistIds = $state<string[]>([]);
  let wishlistItems = $state<Map<string, FigurineListItem | null>>(new Map());
  let sourceFigurineItems = $state<Map<string, FigurineListItem | null>>(new Map());
  // Full figurine lookup (image/name) for any piece referenced by a dealing.
  let figurineById = $state<Map<string, FigurineListItem>>(new Map());
  let threads = $state<MessageThreadDto[]>([]);
  let unreadCount = $state(0);
  let loading = $state(true);
  let error = $state('');

  // ── Attach a guest request by its secret code ──
  let claimCode = $state('');
  let claimSubmitting = $state(false);
  let claimResult = $state<LinkClaimResponse | null>(null);
  let claimError = $state('');

  // After linking a guest request, drop the user into the dealings section,
  // filtered to the kind they just attached.
  function claimKindToFilter(kind: LinkClaimKind): DealFilter {
    return kind === 'booking' ? 'booking'
      : kind === 'notify' ? 'order'
      : kind === 'commission' ? 'commission'
      : 'waitlist';
  }

  function claimKindLabel(kind: LinkClaimKind): string {
    const map: Record<LinkClaimKind, string> = {
      booking: $t('profileLinkClaimKindBooking'),
      waitlist: $t('profileLinkClaimKindWaitlist'),
      notify: $t('profileLinkClaimKindNotify'),
      commission: $t('profileLinkClaimKindCommission'),
    };
    return map[kind];
  }

  async function submitClaim() {
    const code = claimCode.trim();
    if (!code || claimSubmitting) return;
    claimSubmitting = true;
    claimError = '';
    claimResult = null;
    try {
      const res = await api.linkClaimByToken(authStore.token!, code);
      claimResult = res;
      if (res.result === 'linked') {
        claimCode = '';
        await loadData();
        if (res.kind) {
          dealFilter = claimKindToFilter(res.kind);
          view = 'dealings';
        }
      }
    } catch {
      claimError = $t('profileActionError');
    } finally {
      claimSubmitting = false;
    }
  }

  const COMMISSION_STATUS_LABEL: Record<string, string> = {
    new: 'New', reviewing: 'Reviewing', accepted: 'Accepted',
    in_progress: 'In progress', completed: 'Completed', declined: 'Declined',
  };

  // Linear happy-path of a commission, for the client-facing progress stepper.
  // 'declined' is terminal and off-path — shown separately, not on the line.
  const COMMISSION_STAGES = ['new', 'reviewing', 'accepted', 'in_progress', 'completed'] as const;
  function commissionStageState(status: string, stage: string): 'done' | 'current' | 'todo' {
    if (status === 'declined') return 'todo';
    const stages: readonly string[] = COMMISSION_STAGES;
    const cur = stages.indexOf(status);
    const idx = stages.indexOf(stage);
    if (cur < 0) return 'todo';
    return idx < cur ? 'done' : idx === cur ? 'current' : 'todo';
  }

  // Petition edit / delete
  let editingCommission = $state<CommissionDto | null>(null);
  let deletingId = $state<string | null>(null);
  let confirmDeleteId = $state<string | null>(null);

  function onCommissionSaved(updated: CommissionDto) {
    commissions = commissions.map((x) => (x.id === updated.id ? updated : x));
    editingCommission = null;
  }

  async function removeCommission(c: CommissionDto) {
    deletingId = c.id;
    try {
      await api.deleteCommission(authStore.token!, c.id);
      commissions = commissions.filter((x) => x.id !== c.id);
      confirmDeleteId = null;
    } catch {
      // ignore
    } finally {
      deletingId = null;
    }
  }

  // Thread view
  type MsgFilter = 'all' | 'booking' | 'order' | 'waitlist' | 'general';
  let msgFilter = $state<MsgFilter>('all');
  let openThread = $state<ThreadDetailDto | null>(null);
  let threadLoading = $state(false);
  let messagesEl = $state<HTMLElement | null>(null);

  // Focus action — replaces the autofocus attribute (better a11y, same UX)
  function focusOnMount(node: HTMLElement) {
    node.focus();
  }

  // Reply
  let replyBody = $state('');
  let replySending = $state(false);
  let replySent = $state(false);
  let replyAttachments = $state<AttachmentInput[]>([]);
  let replyUploading = $state(false);

  async function handleReplyFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files || !authStore.token) return;
    for (const file of Array.from(input.files)) {
      if (replyAttachments.length >= 5) break;
      if (file.size > 8 * 1024 * 1024) continue;
      replyUploading = true;
      try {
        const att = await api.uploadUserMedia(authStore.token, file);
        replyAttachments = [...replyAttachments, att];
      } catch { /* ignore */ }
      finally { replyUploading = false; }
    }
    input.value = '';
  }

  // Compose new thread
  let showCompose = $state(false);
  let composeSubject = $state('');
  let composeBody = $state('');
  let composeSending = $state(false);
  let composeSent = $state(false);
  let composeError = $state('');

  // Per-action error feedback (previously these failures were swallowed silently)
  let replyError = $state('');
  let nameError = $state('');
  let avatarError = $state('');
  let deleteError = $state('');

  // Account editing
  let editingName = $state(false);
  let editNameValue = $state('');
  let savingName = $state(false);
  let nameSaved = $state(false);

  // Avatar upload
  let avatarInput: HTMLInputElement;
  let uploadingAvatar = $state(false);

  // Delete account
  let showDeleteConfirm = $state(false);
  let deleting = $state(false);

  // Claim-by-code is an edge tool, hidden behind a masthead toggle (progressive disclosure).
  let showClaim = $state(false);

  onMount(async () => {
    if (!authStore.isLoggedIn && !authStore.token) {
      goto('/login?from=/profile');
      return;
    }
    if (!authStore.isLoggedIn && authStore.token) {
      try {
        const user = await api.userMe(authStore.token!);
        authStore.user = user;
      } catch {
        authStore.clearSession();
        goto('/login?from=/profile');
        return;
      }
    }
    // Auto-claim a commission submitted earlier as a guest.
    if (authStore.token && typeof localStorage !== 'undefined') {
      const pending = localStorage.getItem('gotiga_pending_claim');
      if (pending) {
        try { await api.claimCommission(authStore.token, pending); } catch { /* ignore */ }
        localStorage.removeItem('gotiga_pending_claim');
      }
    }
    await loadData();
  });

  async function loadData() {
    loading = true;
    error = '';
    try {
      const token = authStore.token!;
      const [b, o, th, com, wl] = await Promise.all([
        api.userProfileBookings(token),
        api.userProfileOrders(token),
        api.getUserThreads(token),
        api.getUserCommissions(token),
        api.userProfileWaitlist(token).catch(() => [] as WaitlistEntryDto[]),
      ]);
      bookings = b;
      orders = o;
      commissions = com;
      waitlist = wl;
      threads = th.threads;
      unreadCount = th.unread;

      await savedFigurines.syncWithServer({ importLocal: false });
      wishlistIds = [...savedFigurines.ids];
      await loadFigurineReferenceDetails();
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : '';
      if (msg.includes('401')) {
        authStore.clearSession();
        goto('/login?from=/profile');
        return;
      }
      error = $t('authErrorServer');
    } finally {
      loading = false;
    }
  }

  async function loadFigurineReferenceDetails() {
    const ids = [...savedFigurines.ids];
    const sourceIds = [...new Set(commissions.map((c) => c.sourceFigurineId).filter((id): id is string => Boolean(id)))];
    // Every piece a dealing points at (so work-cards can show its photo).
    const dealingIds = [
      ...bookings.map((b) => b.figurineId),
      ...orders.map((o) => o.figurineId),
      ...waitlist.map((w) => w.figurineId),
    ].filter((id): id is string => Boolean(id));
    wishlistIds = ids;
    if (ids.length === 0 && sourceIds.length === 0 && dealingIds.length === 0) {
      wishlistItems = new Map();
      sourceFigurineItems = new Map();
      figurineById = new Map();
      return;
    }
    const all = await api.getAllFigurines().catch(() => [] as FigurineListItem[]);
    const byId = new Map(all.map((fig) => [fig.id, fig]));
    figurineById = byId;
    const map = new Map<string, FigurineListItem | null>();
    ids.forEach((id) => {
      map.set(id, byId.get(id) ?? null);
    });
    wishlistItems = map;

    const sourceMap = new Map<string, FigurineListItem | null>();
    sourceIds.forEach((id) => {
      sourceMap.set(id, byId.get(id) ?? null);
    });
    sourceFigurineItems = sourceMap;
  }

  function removeSavedFigurine(id: string) {
    savedFigurines.remove(id);
    wishlistIds = [...savedFigurines.ids];
    const next = new Map(wishlistItems);
    next.delete(id);
    wishlistItems = next;
  }

  async function logout() {
    const token = authStore.token;
    if (token) {
      try { await api.userLogout(token); } catch { /* ok */ }
    }
    authStore.logout();
    goto('/');
  }

  function startEditName() {
    editNameValue = authStore.user?.displayName ?? '';
    editingName = true;
    nameSaved = false;
  }

  async function saveName() {
    if (!editNameValue.trim() || savingName) return;
    savingName = true;
    nameError = '';
    try {
      const updated = await api.updateProfile(authStore.token!, editNameValue.trim());
      authStore.user = updated;
      editingName = false;
      nameSaved = true;
      setTimeout(() => nameSaved = false, 2000);
    } catch {
      nameError = $t('profileActionError'); // keep editing open
    } finally {
      savingName = false;
    }
  }

  function cancelEditName() {
    editingName = false;
  }

  async function handleAvatarChange(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    uploadingAvatar = true;
    avatarError = '';
    try {
      const updated = await api.uploadAvatar(authStore.token!, file);
      authStore.user = updated;
    } catch {
      avatarError = $t('profileAvatarError');
    } finally {
      uploadingAvatar = false;
      if (avatarInput) avatarInput.value = '';
    }
  }

  async function deleteAccount() {
    if (deleting) return;
    deleting = true;
    deleteError = '';
    try {
      await api.deleteAccount(authStore.token!);
      authStore.purgeAllLocalData();
      // Hard navigation so in-memory stores (wishlist, claims) reset too, not
      // just their localStorage backing.
      window.location.href = '/';
    } catch {
      deleteError = $t('profileActionError');
      deleting = false;
    }
  }

  function scrollToBottom() {
    if (messagesEl) messagesEl.scrollTop = messagesEl.scrollHeight;
  }

  async function openThreadDetail(id: string) {
    threadLoading = true;
    openThread = null;
    replyBody = '';
    replyAttachments = [];
    replySent = false;
    replyError = '';
    try {
      const detail = await api.getThread(authStore.token!, id);
      openThread = detail;
      const prev = threads.find(t => t.id === id);
      if (prev && prev.unread > 0) {
        unreadCount = Math.max(0, unreadCount - prev.unread);
        threads = threads.map(t => t.id === id ? { ...t, unread: 0 } : t);
      }
      await tick();
      scrollToBottom();
    } catch { /* silent */ } finally {
      threadLoading = false;
    }
  }

  function closeThread() {
    openThread = null;
    replyBody = '';
    replyAttachments = [];
    replySent = false;
  }

  async function sendReply() {
    if ((!replyBody.trim() && replyAttachments.length === 0) || replySending || !openThread) return;
    replySending = true;
    replyError = '';
    try {
      const msg = await api.replyToThread(authStore.token!, openThread.thread.id, replyBody.trim(), replyAttachments);
      openThread = { ...openThread, messages: [...openThread.messages, msg] };
      replyBody = '';
      replyAttachments = [];
      replySent = true;
      setTimeout(() => { replySent = false; }, 2000);
      await tick();
      scrollToBottom();
    } catch {
      replyError = $t('profileActionError');
    } finally {
      replySending = false;
    }
  }

  async function sendCompose() {
    if (!composeSubject.trim() || !composeBody.trim() || composeSending) return;
    composeSending = true;
    composeError = '';
    try {
      const detail = await api.createThread(authStore.token!, composeSubject.trim(), composeBody.trim(), 'general');
      threads = [detail.thread, ...threads];
      composeSubject = '';
      composeBody = '';
      composeSent = true;
      setTimeout(() => {
        composeSent = false;
        showCompose = false;
      }, 1800);
    } catch {
      composeError = $t('profileActionError');
    } finally {
      composeSending = false;
    }
  }

  let filteredThreads = $derived(
    msgFilter === 'all' ? threads : threads.filter(t => t.category === msgFilter)
  );

  function bookingStatusLabel(status: string): string {
    const map: Record<string, string> = {
      pending:   $t('profileBookingPending'),
      confirmed: $t('profileBookingConfirmed'),
      rejected:  $t('profileBookingRejected'),
      cancelled: $t('profileBookingCancelled'),
    };
    return map[status] ?? status;
  }

  function orderModeLabel(mode: string): string {
    const map: Record<string, string> = {
      request:  $t('profileOrderModeRequest'),
      question: $t('profileOrderModeQuestion'),
      notify:   $t('profileOrderModeNotify'),
      reserve:  $t('profileOrderModeReserve'),
    };
    return map[mode] ?? mode;
  }

  function reserveStatusLabel(status: string | null): string {
    const map: Record<string, string> = {
      requested:  $t('profileReserveRequested'),
      reviewing:  $t('profileReserveReviewing'),
      terms_sent: $t('profileReserveTermsSent'),
      confirmed:  $t('profileReserveConfirmed'),
      declined:   $t('profileReserveDeclined'),
      expired:    $t('profileReserveExpired'),
    };
    return status ? (map[status] ?? status) : $t('profileReserveRequested');
  }

  function orderStatusLabel(status: string): string {
    if (status === 'replied') return $t('profileOrderReplied');
    if (status === 'seen') return $t('profileOrderSeen');
    return $t('profileOrderNew');
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString($lang, { day: 'numeric', month: 'long', year: 'numeric' });
  }

  function formatDateRange(start: string, end: string): string {
    const s = new Date(start);
    const e = new Date(end);
    const opts: Intl.DateTimeFormatOptions = { day: 'numeric', month: 'short' };
    return `${s.toLocaleDateString($lang, opts)} — ${e.toLocaleDateString($lang, opts)}`;
  }

  function wishlistStatusLabel(status: string): string {
    const map: Record<string, string> = {
      available:   $t('profileWishAvailable'),
      sold:        $t('profileWishSold'),
      reserved:    $t('profileWishReserved'),
      in_progress: $t('profileWishInProgress'),
    };
    return map[status] ?? status;
  }

  // ── Overview: one quiet chronological feed across all request types ──
  type FeedTone = 'pending' | 'confirmed' | 'rejected' | 'neutral' | 'attention';
  interface FeedItem {
    kind: 'booking' | 'order' | 'commission' | 'waitlist';
    status: string;
    id: string;
    title: string;
    date: string;
    tone: FeedTone;
    figurineId?: string;
    threadId: string | null;
    unread: number;
  }

  // Labels are resolved in the template (where the $t store can be read), so the
  // derived feed below stays free of store subscriptions.
  function feedKindLabel(kind: FeedItem['kind']): string {
    return kind === 'booking' ? $t('profileOverviewKindBooking')
      : kind === 'order' ? $t('profileOverviewKindOrder')
      : kind === 'waitlist' ? $t('profileLinkClaimKindWaitlist')
      : $t('profileOverviewKindCommission');
  }
  function feedStatusLabel(item: FeedItem): string {
    return item.kind === 'booking' ? bookingStatusLabel(item.status)
      : item.kind === 'order' ? orderStatusLabel(item.status)
      : item.kind === 'waitlist' ? `№${item.status}`
      : (COMMISSION_STATUS_LABEL[item.status] ?? item.status);
  }

  // referenceId on a thread points back at the booking/order/commission it belongs to,
  // so a feed row can show "new reply" and jump straight into the conversation.
  let threadsByRef = $derived.by(() => {
    const m = new Map<string, { id: string; unread: number }>();
    for (const th of threads) {
      if (!th.referenceId) continue;
      const prev = m.get(th.referenceId);
      m.set(th.referenceId, { id: th.id, unread: (prev?.unread ?? 0) + th.unread });
    }
    return m;
  });

  function bookingTone(s: string): FeedTone {
    return s === 'confirmed' ? 'confirmed' : s === 'rejected' ? 'rejected' : s === 'cancelled' ? 'neutral' : 'pending';
  }
  function orderTone(s: string): FeedTone {
    return s === 'replied' ? 'confirmed' : s === 'seen' ? 'neutral' : 'pending';
  }
  function commissionTone(s: string): FeedTone {
    return s === 'completed' ? 'confirmed'
      : s === 'declined' ? 'rejected'
      : (s === 'accepted' || s === 'in_progress') ? 'attention'
      : 'pending';
  }

  let feed = $derived.by<FeedItem[]>(() => {
    const ref = threadsByRef;
    const items: FeedItem[] = [];
    for (const b of bookings) {
      const t = ref.get(b.id);
      items.push({
        kind: 'booking', status: b.status,
        id: b.id, title: b.figurineName, date: b.createdAt,
        tone: bookingTone(b.status),
        figurineId: b.figurineId, threadId: t?.id ?? null, unread: t?.unread ?? 0,
      });
    }
    for (const o of orders) {
      const t = ref.get(o.id);
      items.push({
        kind: 'order', status: o.status,
        id: o.id, title: o.figurineName, date: o.createdAt,
        tone: orderTone(o.status),
        figurineId: o.figurineId, threadId: t?.id ?? null, unread: t?.unread ?? 0,
      });
    }
    for (const c of commissions) {
      const t = ref.get(c.id);
      items.push({
        kind: 'commission', status: c.status,
        id: c.id, title: c.title, date: c.createdAt,
        tone: commissionTone(c.status),
        threadId: c.threadId ?? t?.id ?? null, unread: t?.unread ?? 0,
      });
    }
    for (const w of waitlist) {
      const t = ref.get(w.id);
      items.push({
        kind: 'waitlist', status: String(w.position),
        id: w.id, title: w.figurineName, date: w.createdAt,
        tone: 'neutral',
        figurineId: w.figurineId, threadId: t?.id ?? null, unread: t?.unread ?? 0,
      });
    }
    return items.sort((a, b) => +new Date(b.date) - +new Date(a.date));
  });

  function openFeedThread(item: FeedItem) {
    if (!item.threadId) return;
    view = 'messages';
    openThreadDetail(item.threadId);
  }

  // Each record shows its full detail inline (dates, mode, description, actions)
  // from these lookups — nothing hidden behind a click.
  let bookingById = $derived(new Map(bookings.map((b) => [b.id, b])));
  let orderById = $derived(new Map(orders.map((o) => [o.id, o])));
  let commissionById = $derived(new Map(commissions.map((c) => [c.id, c])));

  // ── Object-centric grouping: one card per work, aggregating every dealing the
  // user has on it. Commissions (a proposed new piece) stand as their own cards.
  interface WorkCard {
    key: string;
    figurineId?: string;
    title: string;
    items: FeedItem[];
    kinds: FeedItem['kind'][];
    unread: number;
    tone: FeedTone;
    date: string;
  }
  // Headline tone for a card = the most "live" tone among its dealings.
  const TONE_RANK: FeedTone[] = ['attention', 'pending', 'confirmed', 'rejected', 'neutral'];
  let workCards = $derived.by<WorkCard[]>(() => {
    const groups = new Map<string, FeedItem[]>();
    for (const it of feed) {
      const key = it.figurineId ?? `${it.kind}:${it.id}`;
      const arr = groups.get(key);
      if (arr) arr.push(it); else groups.set(key, [it]);
    }
    const cards: WorkCard[] = [];
    for (const [key, items] of groups) {
      items.sort((a, b) => +new Date(b.date) - +new Date(a.date));
      cards.push({
        key,
        figurineId: items[0].figurineId,
        title: items[0].title,
        items,
        kinds: [...new Set(items.map((i) => i.kind))],
        unread: items.reduce((s, i) => s + i.unread, 0),
        tone: TONE_RANK.find((t) => items.some((i) => i.tone === t)) ?? 'neutral',
        date: items[0].date,
      });
    }
    return cards.sort((a, b) => +new Date(b.date) - +new Date(a.date));
  });

  const DEAL_FILTERS: DealFilter[] = ['all', 'booking', 'order', 'commission', 'waitlist'];
  let visibleCards = $derived.by(() => {
    if (dealFilter === 'all') return workCards;
    const k = dealFilter;
    return workCards.filter((c) => c.kinds.includes(k));
  });
  let selectedCard = $derived(workCards.find((c) => c.key === openCardKey) ?? null);

  function openCard(key: string) { openCardKey = key; view = 'card'; }

  function dealFilterLabel(f: DealFilter): string {
    return f === 'all' ? $t('profileRailAll')
      : f === 'booking' ? $t('profileBookings')
      : f === 'order' ? $t('profileOrders')
      : f === 'commission' ? $t('profileCommissions')
      : $t('profileLinkClaimKindWaitlist');
  }
  function dealFilterCount(f: DealFilter): number {
    return f === 'all' ? workCards.length : workCards.filter((c) => c.kinds.includes(f)).length;
  }
</script>

<svelte:head>
  <title>{$t('profileTitle')} — {$brandName}</title>
</svelte:head>

<div class="page">
  <div class="frame">
    <div class="body">

      <!-- ── Masthead: identity + account actions ── -->
      <header class="masthead">
        <div class="masthead-id">
          <button
            class="ms-avatar"
            onclick={() => avatarInput?.click()}
            title={$t('profileUploadPhoto')}
            disabled={uploadingAvatar}
          >
            {#if resolveMediaUrl(authStore.user?.avatarUrl)}
              <img src={resolveMediaUrl(authStore.user?.avatarUrl)} alt="" class="ms-avatar-img" />
            {:else}
              <span class="ms-avatar-initials">{(authStore.user?.displayName ?? '?')[0].toUpperCase()}</span>
            {/if}
            <span class="ms-avatar-cap">{uploadingAvatar ? $t('profileUploadingPhoto') : $t('profileUploadPhoto')}</span>
          </button>
          <input
            bind:this={avatarInput}
            type="file"
            accept="image/jpeg,image/png,image/webp"
            class="sr-only"
            onchange={handleAvatarChange}
          />

          <div class="ms-identity">
            {#if editingName}
              <div class="ms-name-edit">
                <input
                  class="ms-name-input"
                  bind:value={editNameValue}
                  onkeydown={(e) => { if (e.key === 'Enter') saveName(); if (e.key === 'Escape') cancelEditName(); }}
                  use:focusOnMount
                />
                <button class="ms-name-save" onclick={saveName} disabled={savingName}>
                  {savingName ? '…' : $t('profileSaveName')}
                </button>
                <button class="ms-name-cancel" onclick={cancelEditName}>✕</button>
              </div>
              {#if nameError}<span class="field-error" role="alert">{nameError}</span>{/if}
            {:else}
              <div class="ms-name-row">
                <h1 class="ms-name">{authStore.user?.displayName ?? ''}</h1>
                <button class="ms-edit-btn" onclick={startEditName} title={$t('profileEditName')}>
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
                    <path d="M8.5 1.5L10.5 3.5L4 10H2V8L8.5 1.5Z"/>
                  </svg>
                </button>
                {#if nameSaved}<span class="ms-name-saved">{$t('profileNameSaved')}</span>{/if}
              </div>
            {/if}
            <p class="ms-sub">
              <span class="ms-email">{authStore.user?.email ?? ''}</span>
              {#if authStore.user?.createdAt}
                <span class="ms-since">· {$t('profileMemberSince')} {formatDate(authStore.user.createdAt)}</span>
              {/if}
            </p>
            {#if avatarError}<p class="field-error" role="alert">{avatarError}</p>{/if}
          </div>
        </div>

        <div class="masthead-actions">
          <button class="ms-link" class:active={showClaim} onclick={() => showClaim = !showClaim}>{$t('profileCodeToggle')}</button>
          <span class="ms-sep" aria-hidden="true">·</span>
          <button class="ms-link" onclick={logout}>{$t('profileLogout')}</button>
        </div>
      </header>

      <!-- ── Attach a guest request by code (progressive disclosure) ── -->
      {#if showClaim}
        <section class="claim" aria-labelledby="claim-title">
          <p id="claim-title" class="claim-hint">{$t('profileLinkClaimHint')}</p>
          <form class="claim-form" onsubmit={(e) => { e.preventDefault(); submitClaim(); }}>
            <input
              class="claim-input"
              bind:value={claimCode}
              placeholder={$t('profileLinkClaimPlaceholder')}
              autocomplete="off"
              autocapitalize="off"
              spellcheck="false"
              aria-label={$t('profileLinkClaimTitle')}
            />
            <button class="claim-btn" type="submit" disabled={claimSubmitting || !claimCode.trim()}>
              {claimSubmitting ? $t('profileLinkClaimLinking') : $t('profileLinkClaimButton')}
            </button>
          </form>
          {#if claimError}
            <p class="claim-msg claim-msg--err" role="alert">{claimError}</p>
          {:else if claimResult}
            {#if claimResult.result === 'linked'}
              <p class="claim-msg claim-msg--ok" role="status">
                {$t('profileLinkClaimLinked')}
                {#if claimResult.kind}<span class="claim-kind">{claimKindLabel(claimResult.kind)}</span>{/if}
                {#if claimResult.name}<span class="claim-name">{claimResult.name}</span>{/if}
              </p>
            {:else if claimResult.result === 'email_mismatch'}
              <p class="claim-msg claim-msg--warn" role="alert">{$t('profileLinkClaimMismatch')}</p>
            {:else if claimResult.result === 'already_linked'}
              <p class="claim-msg claim-msg--warn" role="status">{$t('profileLinkClaimAlready')}</p>
            {:else}
              <p class="claim-msg claim-msg--err" role="alert">{$t('profileLinkClaimNotFound')}</p>
            {/if}
          {/if}
        </section>
      {/if}

      <!-- Body: hub → section → card -->
      <div class="ledger-body">

        {#if loading}
          <p class="empty">…</p>
        {:else if error}
          <p class="error-msg">{error}</p>
        {:else if view === 'hub'}
          <!-- ── Hub: three large entrances ── -->
          <section class="hub">
            <button class="hub-tile" onclick={() => view = 'dealings'}>
              <span class="hub-kicker">{$t('profileHubDealingsHint')}</span>
              <span class="hub-name">{$t('profileHubDealings')}</span>
              <span class="hub-foot">
                {#if workCards.length > 0}<span class="hub-count">{workCards.length}</span> {$t('profileHubWorks')}{:else}{$t('profileEmpty')}{/if}
              </span>
            </button>
            <button class="hub-tile" onclick={() => { view = 'messages'; closeThread(); }}>
              <span class="hub-kicker">{$t('profileHubMessagesHint')}</span>
              <span class="hub-name">{$t('profileMessages')}</span>
              <span class="hub-foot">
                {#if unreadCount > 0}<span class="hub-count hub-count--unread">{unreadCount}</span> {$t('profileHubNew')}{:else if threads.length > 0}{threads.length} {$t('profileHubThreads')}{:else}{$t('profileEmpty')}{/if}
              </span>
            </button>
            <button class="hub-tile" onclick={() => view = 'wishlist'}>
              <span class="hub-kicker">{$t('profileHubWishlistHint')}</span>
              <span class="hub-name">{$t('profileWishlist')}</span>
              <span class="hub-foot">
                {#if wishlistIds.length > 0}<span class="hub-count">{wishlistIds.length}</span> {$t('profileHubPieces')}{:else}{$t('profileEmpty')}{/if}
              </span>
            </button>
          </section>
        {:else if view === 'wishlist'}
          <button class="crumb" onclick={() => view = 'hub'}>← {$t('profileTitle')}</button>
          {#if savedFigurines.syncError}
            <p class="wishlist-sync-error">{$t('profileWishlistSyncError')}</p>
          {/if}
          {#if wishlistIds.length === 0}
            <p class="empty">{$t('profileEmpty')}</p>
          {:else}
            <div class="cards-grid">
              {#each wishlistIds as id}
                {@const item = wishlistItems.get(id)}
                <div class="card wishlist-card">
                  {#if item?.faceImageUrl}
                    <a href="/figurines/{id}" class="wishlist-thumb" aria-label="{item.name}">
                      <AppImage src={item.faceImageUrl} thumbUrl={item.thumbUrl} alt={item.name} class="wishlist-img" loading="lazy" />
                    </a>
                  {:else}
                    <a href="/figurines/{id}" class="wishlist-thumb wishlist-thumb--empty" aria-label="{item?.name ?? id}">
                      <span>?</span>
                    </a>
                  {/if}

                  <div class="card-head wishlist-head">
                    {#if item}
                      <div class="wishlist-title">
                        <a href="/figurines/{id}" class="card-name card-name--link">{item.name}</a>
                        <div class="wishlist-meta">
                          {#if item.year}
                            <span>{item.year}</span>
                          {/if}
                          <span class="wishlist-status wishlist-status--{item.status}">{wishlistStatusLabel(item.status)}</span>
                        </div>
                      </div>
                    {:else if item === undefined}
                      <span class="card-name card-name--loading">…</span>
                    {:else}
                      <a href="/figurines/{id}" class="card-name card-name--missing">{id}</a>
                    {/if}
                    <button class="wishlist-remove" onclick={() => removeSavedFigurine(id)} aria-label={$t('profileWishRemove')} title={$t('profileWishRemove')}>
                      ×
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {:else if view === 'messages'}
          <button class="crumb" onclick={() => view = 'hub'}>← {$t('profileTitle')}</button>
          <div class="messages-wrap">
            {#if openThread}
              <!-- ── Thread detail ── -->
              <div class="thread-detail">
                <div class="thread-detail-header">
                  <button class="thread-back" onclick={closeThread}>← {$t('profileMessagesBack')}</button>
                  <span class="thread-detail-subject">{openThread.thread.subject}</span>
                  {#if openThread.thread.status === 'resolved'}
                    <span class="thread-resolved-badge">{$t('profileMessagesResolved')}</span>
                  {/if}
                </div>

                <div class="chat-messages" bind:this={messagesEl}>
                  {#if threadLoading}
                    <p class="empty">…</p>
                  {:else}
                    {#each openThread.messages as msg}
                      <div class="chat-row" class:chat-row--user={!msg.fromAdmin} class:chat-row--admin={msg.fromAdmin}>
                        <div class="chat-bubble" class:chat-bubble--user={!msg.fromAdmin} class:chat-bubble--admin={msg.fromAdmin}>
                          <p class="chat-sender">{msg.fromAdmin ? $t('profileMessagesFromAdmin') : $t('profileMessagesFromYou')}</p>
                          {#if msg.body}<p class="chat-body">{msg.body}</p>{/if}
                          {#if msg.attachments && msg.attachments.length > 0}
                            <MessageAttachments attachments={msg.attachments} />
                          {/if}
                        </div>
                        <p class="chat-time">{formatDate(msg.createdAt)}</p>
                      </div>
                    {/each}
                  {/if}
                </div>

                {#if openThread.thread.status !== 'resolved'}
                  <div class="chat-reply">
                    <textarea
                      class="chat-reply-input"
                      bind:value={replyBody}
                      rows="2"
                      placeholder={$t('profileMessageWriteBody')}
                      onkeydown={(e) => { if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) sendReply(); }}
                    ></textarea>
                    {#if replyAttachments.length > 0}
                      <div class="reply-atts">
                        {#each replyAttachments as att, i (att.url)}
                          <div class="reply-att">
                            <img src={resolveMediaUrl(att.thumbUrl ?? att.url)} alt="" />
                            <button type="button" onclick={() => replyAttachments = replyAttachments.filter((_, idx) => idx !== i)} aria-label="×">×</button>
                          </div>
                        {/each}
                      </div>
                    {/if}
                    <div class="chat-reply-footer">
                      <span class="chat-reply-hint">Ctrl+Enter</span>
                      <label class="chat-attach-btn" title={$t('profileAttachImage')}>
                        <input type="file" accept="image/*" multiple hidden onchange={handleReplyFiles} />
                        {replyUploading ? '…' : '📎'}
                      </label>
                      <button
                        class="chat-reply-btn"
                        onclick={sendReply}
                        disabled={replySending || (!replyBody.trim() && replyAttachments.length === 0)}
                      >
                        {replySending ? $t('profileMessagesReplying') : replySent ? $t('profileMessageWriteSent') : $t('profileMessagesReply')} →
                      </button>
                    </div>
                    {#if replyError}<p class="field-error" role="alert">{replyError}</p>{/if}
                  </div>
                {:else}
                  <p class="chat-resolved-note">{$t('profileMessagesResolved')}</p>
                {/if}
              </div>

            {:else}
              <!-- ── Thread list ── -->
              <div class="threads-header">
                <div class="thread-filters">
                  {#each (['all', 'booking', 'order', 'waitlist', 'general'] as MsgFilter[]) as f}
                    <button
                      class="thread-filter-btn"
                      class:active={msgFilter === f}
                      onclick={() => msgFilter = f}
                    >
                      {#if f === 'all'}{$t('profileMessagesAll')}
                      {:else if f === 'booking'}{$t('profileMessagesBooking')}
                      {:else if f === 'order'}{$t('profileMessagesOrder')}
                      {:else if f === 'waitlist'}{$t('profileMessagesWaitlist')}
                      {:else}{$t('profileMessagesGeneral')}
                      {/if}
                    </button>
                  {/each}
                </div>
                <button class="compose-toggle-btn" onclick={() => showCompose = !showCompose}>
                  {$t('profileMessagesCompose')}
                </button>
              </div>

              {#if showCompose}
                <div class="msg-compose">
                  <input
                    class="msg-subject"
                    bind:value={composeSubject}
                    placeholder={$t('profileMessageWriteSubject')}
                  />
                  <textarea
                    class="msg-body"
                    bind:value={composeBody}
                    rows="3"
                    placeholder={$t('profileMessageWriteBody')}
                  ></textarea>
                  <button
                    class="msg-send-btn"
                    onclick={sendCompose}
                    disabled={composeSending || !composeSubject.trim() || !composeBody.trim()}
                  >
                    {composeSending ? $t('profileMessagesSending') : composeSent ? $t('profileMessagesSent') : $t('profileMessagesSend')}
                  </button>
                  {#if composeError}<p class="field-error" role="alert">{composeError}</p>{/if}
                </div>
              {/if}

              {#if filteredThreads.length === 0}
                <p class="empty">{$t('profileMessagesEmpty')}</p>
              {:else}
                <ul class="list">
                  {#each filteredThreads as thread}
                    <li class="item msg-item" class:msg-unread={thread.unread > 0}>
                      <button class="item-hit" onclick={() => openThreadDetail(thread.id)} aria-label={thread.subject}></button>
                      <div class="item-main">
                        <span class="thread-subject">{thread.subject}</span>
                        <div class="thread-meta-right">
                          {#if thread.unread > 0}
                            <span class="msg-new-badge">{thread.unread}</span>
                          {/if}
                          {#if thread.status === 'resolved'}
                            <span class="thread-resolved-small">{$t('profileMessagesResolved')}</span>
                          {/if}
                        </div>
                      </div>
                      {#if thread.preview}
                        <p class="msg-body-preview">{thread.preview}</p>
                      {/if}
                      <p class="item-date">{formatDate(thread.lastMessageAt)}</p>
                    </li>
                  {/each}
                </ul>
              {/if}
            {/if}
          </div>
        {:else if view === 'card'}
          <!-- ── One work: every dealing the user has on it ── -->
          <button class="crumb" onclick={() => view = 'dealings'}>← {$t('profileHubDealings')}</button>
          {#if selectedCard}
            {@const card = selectedCard}
            {@const fig = card.figurineId ? figurineById.get(card.figurineId) : undefined}
            <header class="work-head">
              {#if card.figurineId}
                <a href="/figurines/{card.figurineId}" class="work-head-thumb" aria-label={card.title}>
                  {#if fig?.faceImageUrl}
                    <AppImage src={fig.faceImageUrl} thumbUrl={fig.thumbUrl} alt={card.title} class="work-head-img" loading="lazy" />
                  {:else}
                    <span class="work-head-ph">{(card.title || '?')[0]}</span>
                  {/if}
                </a>
              {/if}
              <div class="work-head-copy">
                <h2 class="work-head-name">{card.title || $t('commissionUntitled')}</h2>
                <p class="work-head-sub">{card.items.length} · {$t('profileHubActivities')}</p>
              </div>
            </header>
            <ul class="book">
              {#each card.items as item (item.kind + ':' + item.id)}
                <li class="entry" class:entry--unread={item.unread > 0}>
                  <div class="entry-head">
                    <span class="entry-date">{formatDate(item.date)}</span>
                    <span class="entry-kind">{feedKindLabel(item.kind)}</span>
                    {#if item.figurineId}
                      <a class="entry-title entry-title--link" href="/figurines/{item.figurineId}">{item.title || $t('commissionUntitled')}</a>
                    {:else}
                      <span class="entry-title">{item.title || $t('commissionUntitled')}</span>
                    {/if}
                    <span class="entry-status entry-status--{item.tone}">{feedStatusLabel(item)}</span>
                  </div>

                  {#if item.kind === 'booking'}
                    {@const b = bookingById.get(item.id)}
                    {#if b}
                      <div class="entry-detail">
                        <p class="d-range">{formatDateRange(b.startsAt, b.endsAt)}</p>
                        {#if b.curatorConditions}
                          <div class="d-curator">
                            <span class="d-curator-label">{$t('profileBookingCuratorConditions')}</span>
                            <p class="d-curator-text">{b.curatorConditions}</p>
                          </div>
                        {/if}
                        <div class="d-actions">
                          <a class="d-link" href="/cancel/{b.cancelToken}">{$t('profileBookingManage')}</a>
                        </div>
                      </div>
                    {/if}
                  {:else if item.kind === 'order'}
                    {@const o = orderById.get(item.id)}
                    {#if o}
                      <div class="entry-detail">
                        <p class="d-line"><span class="d-tag">{orderModeLabel(o.mode)}</span></p>
                        {#if o.mode === 'reserve'}
                          <div class="reserve-detail">
                            <span class="d-tag">{reserveStatusLabel(o.reserveStatus)}</span>
                            {#if o.reserveExpiresAt}
                              <p>{$t('profileReserveExpires')} {formatDate(o.reserveExpiresAt)}</p>
                            {/if}
                            {#if o.adminTermsNote}
                              <div class="d-curator">
                                <span class="d-curator-label">{$t('profileReserveTerms')}</span>
                                <p class="d-curator-text">{o.adminTermsNote}</p>
                              </div>
                            {/if}
                            {#if o.invoiceNote}
                              <div class="d-curator">
                                <span class="d-curator-label">{$t('profileReserveInvoice')}</span>
                                <p class="d-curator-text">{o.invoiceNote}</p>
                              </div>
                            {/if}
                            {#if o.adminNotes}
                              <p class="reserve-note">{o.adminNotes}</p>
                            {/if}
                            {#if o.certificate}
                              <div class="d-curator">
                                <span class="d-curator-label">{$t('profileCertificateTitle')}</span>
                                <p class="d-curator-text">
                                  {o.certificate.certificateNumber}
                                  {#if o.certificate.revokedAt}
                                    · {$t('profileCertificateRevoked')}
                                  {/if}
                                </p>
                                <div class="d-actions">
                                  <a class="d-link" href="/certificate/{o.certificate.token}">
                                    {$t('profileCertificateOpen')}
                                  </a>
                                </div>
                              </div>
                            {/if}
                          </div>
                        {/if}
                      </div>
                    {/if}
                  {:else if item.kind === 'commission'}
                    {@const c = commissionById.get(item.id)}
                    {#if c}
                      <div class="entry-detail">
                        {#if c.status === 'declined'}
                          <p class="ctl-declined">{COMMISSION_STATUS_LABEL.declined}</p>
                        {:else}
                          <ol class="ctl">
                            {#each COMMISSION_STAGES as stage (stage)}
                              {@const st = commissionStageState(c.status, stage)}
                              <li class="ctl-step ctl-step--{st}">
                                <span class="ctl-dot">{st === 'done' ? '✓' : ''}</span>
                                <span class="ctl-label">{COMMISSION_STATUS_LABEL[stage]}</span>
                              </li>
                            {/each}
                          </ol>
                        {/if}
                        {#if c.adminNotes}
                          <div class="d-curator">
                            <span class="d-curator-label">{$t('profileCommissionMasterNote')}</span>
                            <p class="d-curator-text">{c.adminNotes}</p>
                          </div>
                        {/if}
                        {#if c.certificate}
                          <div class="d-curator">
                            <span class="d-curator-label">{$t('profileCertificateTitle')}</span>
                            <p class="d-curator-text">
                              {c.certificate.certificateNumber}
                              {#if c.certificate.revokedAt}· {$t('profileCertificateRevoked')}{/if}
                            </p>
                            <div class="d-actions">
                              <a class="d-link" href="/certificate/{c.certificate.token}">{$t('profileCertificateOpen')}</a>
                            </div>
                          </div>
                        {/if}
                        {#if c.sourceFigurineId}
                          {@const source = sourceFigurineItems.get(c.sourceFigurineId)}
                          <div class="commission-source">
                            {#if source?.faceImageUrl}
                              <a href="/figurines/{c.sourceFigurineId}" class="commission-source-thumb" aria-label={source.name}>
                                <AppImage src={source.faceImageUrl} thumbUrl={source.thumbUrl} alt={source.name} class="commission-source-img" loading="lazy" />
                              </a>
                            {:else}
                              <a href="/figurines/{c.sourceFigurineId}" class="commission-source-thumb commission-source-thumb--empty" aria-label={source?.name ?? c.sourceFigurineId}>
                                <span>GT</span>
                              </a>
                            {/if}
                            <div class="commission-source-copy">
                              <span class="commission-source-label">{$t('profileCommissionSource')}</span>
                              {#if source}
                                <a href="/figurines/{c.sourceFigurineId}" class="commission-source-name">{source.name}</a>
                                <span class="commission-source-meta">{wishlistStatusLabel(source.status)}</span>
                              {:else}
                                <span class="commission-source-name commission-source-name--missing">{c.sourceFigurineId}</span>
                                <span class="commission-source-meta">{$t('profileCommissionSourceMissing')}</span>
                              {/if}
                            </div>
                          </div>
                        {/if}
                        {#if c.description}<p class="d-desc">{c.description}</p>{/if}
                        {#if c.similarKeepNote || c.similarChangeNote}
                          <div class="commission-similar-notes">
                            {#if c.similarKeepNote}
                              <p><span>{$t('profileCommissionKeep')}</span>{c.similarKeepNote}</p>
                            {/if}
                            {#if c.similarChangeNote}
                              <p><span>{$t('profileCommissionChange')}</span>{c.similarChangeNote}</p>
                            {/if}
                          </div>
                        {/if}
                        {#if c.attachments.length > 0}
                          <div class="commission-thumbs">
                            {#each c.attachments as att (att.id)}
                              <img src={resolveMediaUrl(att.thumbUrl ?? att.url)} alt="" />
                            {/each}
                          </div>
                        {/if}
                        <div class="d-actions">
                          {#if c.threadId}
                            <button class="d-link" onclick={() => { view = 'messages'; openThreadDetail(c.threadId!); }}>{$t('profileCommissionsOpenChat')} →</button>
                          {/if}
                          {#if c.started}
                            <span class="commission-locked">{$t('profileCommissionsLocked')}</span>
                          {:else if confirmDeleteId === c.id}
                            <span class="commission-confirm">{$t('profileCommissionsDeleteConfirm')}</span>
                            <button class="d-link danger" onclick={() => removeCommission(c)} disabled={deletingId === c.id}>{deletingId === c.id ? '…' : $t('profileCommissionsDeleteYes')}</button>
                            <button class="d-link" onclick={() => confirmDeleteId = null}>{$t('commissionBack')}</button>
                          {:else}
                            <button class="d-link" onclick={() => editingCommission = c}>{$t('profileCommissionsEdit')}</button>
                            <button class="d-link danger" onclick={() => confirmDeleteId = c.id}>{$t('profileCommissionsDelete')}</button>
                          {/if}
                        </div>
                      </div>
                    {/if}
                  {/if}

                  {#if item.unread > 0 && item.threadId}
                    <button class="entry-reply" onclick={() => openFeedThread(item)}>✦ {$t('profileOverviewNewReply')} →</button>
                  {/if}
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty">{$t('profileEmpty')}</p>
          {/if}
        {:else}
          <!-- ── Dealings: one card per work, with its dealing markers ── -->
          <button class="crumb" onclick={() => view = 'hub'}>← {$t('profileTitle')}</button>
          <nav class="subfilter">
            {#each DEAL_FILTERS as f (f)}
              {@const n = dealFilterCount(f)}
              <button class="subfilter-tab" class:active={dealFilter === f} onclick={() => dealFilter = f} aria-pressed={dealFilter === f}>
                {dealFilterLabel(f)}{#if n > 0}<span class="subfilter-count">{n}</span>{/if}
              </button>
            {/each}
          </nav>
          {#if dealFilter === 'commission' && commissions.length === 0}
            <div class="empty-with-cta">
              <p class="empty">{$t('profileCommissionsEmpty')}</p>
              <a href="/commission" class="commission-cta">{$t('profileCommissionsNew')}</a>
            </div>
          {:else if visibleCards.length === 0}
            <p class="empty">{$t('profileEmpty')}</p>
          {:else}
            <div class="works-grid">
              {#each visibleCards as card (card.key)}
                {@const fig = card.figurineId ? figurineById.get(card.figurineId) : undefined}
                <button class="work work--{card.tone}" onclick={() => openCard(card.key)}>
                  <span class="work-thumb">
                    {#if fig?.faceImageUrl}
                      <AppImage src={fig.faceImageUrl} thumbUrl={fig.thumbUrl} alt={card.title} class="work-img" loading="lazy" />
                    {:else}
                      <span class="work-thumb-ph">{(card.title || '?')[0]}</span>
                    {/if}
                    {#if card.unread > 0}<span class="work-flag" title={$t('profileOverviewNewReply')}>✦</span>{/if}
                  </span>
                  <span class="work-body">
                    <span class="work-name">{card.title || $t('commissionUntitled')}</span>
                    <span class="work-kinds">
                      {#each card.kinds as k}<span class="work-kind">{feedKindLabel(k)}</span>{/each}
                    </span>
                  </span>
                </button>
              {/each}
            </div>
          {/if}
        {/if}
      </div>

      <!-- ── Footer: account removal (quiet danger zone, hub only) ── -->
      {#if view === 'hub' && !loading}
      <footer class="ledger-foot">
        {#if !showDeleteConfirm}
          <button class="foot-delete" onclick={() => showDeleteConfirm = true}>{$t('profileDeleteAccount')}</button>
        {:else}
          <div class="foot-confirm">
            <p class="foot-warning">{$t('profileDeleteWarning')}</p>
            <div class="foot-confirm-actions">
              <button class="foot-yes" onclick={deleteAccount} disabled={deleting}>
                {deleting ? $t('profileDeleting') : $t('profileDeleteConfirm')}
              </button>
              <button class="foot-cancel" onclick={() => showDeleteConfirm = false}>
                {$t('profileDeleteCancel')}
              </button>
            </div>
            {#if deleteError}<p class="field-error" role="alert">{deleteError}</p>{/if}
          </div>
        {/if}
      </footer>
      {/if}

    </div>
  </div>
</div>

{#if editingCommission}
  <CommissionEditModal
    commission={editingCommission}
    onClose={() => (editingCommission = null)}
    onSaved={onCommissionSaved}
  />
{/if}

<style>
  /* ── Page shell ── */

  .page {
    min-height: 100vh;
    background: #f8f1e7;
    font-family: Georgia, serif;
    color: #34251c;
  }

  .frame { display: contents; }

  /* ── Masthead: identity + account actions ── */

  .masthead {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem 1.5rem;
    flex-wrap: wrap;
    padding-bottom: 1.1rem;
    border-bottom: 1px solid #d8c6b1;
  }

  .masthead-id {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    min-width: 0;
  }

  .ms-avatar {
    width: 52px;
    height: 52px;
    border-radius: 3px;
    border: 1px solid #cbb89c;
    background: #efe4d2;
    cursor: pointer;
    overflow: hidden;
    position: relative;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: border-color 0.2s;
  }
  .ms-avatar:hover { border-color: #c65f3c; }
  .ms-avatar:disabled { cursor: default; opacity: 0.7; }

  .ms-avatar-img { width: 100%; height: 100%; object-fit: cover; }

  .ms-avatar-initials {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.4rem;
    color: #8a7253;
    line-height: 1;
    pointer-events: none;
  }

  .ms-avatar-cap {
    position: absolute;
    inset: 0;
    background: rgba(42,26,16,0.72);
    color: #f8f1e7;
    font-family: Inter, sans-serif;
    font-size: 0.5rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 3px;
    opacity: 0;
    transition: opacity 0.18s;
    pointer-events: none;
  }
  .ms-avatar:hover .ms-avatar-cap { opacity: 1; }
  .ms-avatar:disabled .ms-avatar-cap { opacity: 1; }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }

  .ms-identity { min-width: 0; }

  .ms-name-row {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .ms-name {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.5rem;
    font-weight: 400;
    color: #34251c;
    margin: 0;
    line-height: 1.15;
  }

  .ms-edit-btn {
    background: transparent;
    border: none;
    color: #b5a090;
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
    transition: color 0.15s;
    flex-shrink: 0;
  }
  .ms-edit-btn:hover { color: #c65f3c; }

  .ms-name-saved {
    font-family: Inter, sans-serif;
    font-size: 0.6rem;
    color: #4a7a3a;
    letter-spacing: 0.07em;
    text-transform: uppercase;
  }

  .ms-name-edit {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .ms-name-input {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.25rem;
    background: transparent;
    border: none;
    border-bottom: 1.5px solid #c65f3c;
    color: #34251c;
    padding: 2px 0;
    outline: none;
    width: 220px;
    max-width: 60vw;
  }

  .ms-name-save {
    background: transparent;
    border: 1px solid rgba(198,95,60,0.55);
    color: #b9522f;
    font-family: Inter, sans-serif;
    font-size: 0.65rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 3px 9px;
    border-radius: 3px;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.15s;
  }
  .ms-name-save:hover:not(:disabled) { background: rgba(198,95,60,0.1); }
  .ms-name-save:disabled { opacity: 0.5; }

  .ms-name-cancel {
    background: transparent;
    border: none;
    color: #b5a090;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 2px 4px;
    line-height: 1;
    flex-shrink: 0;
    transition: color 0.15s;
  }
  .ms-name-cancel:hover { color: #34251c; }

  .ms-sub {
    font-family: Inter, sans-serif;
    font-size: 0.76rem;
    color: #9a7c5c;
    margin: 0.2rem 0 0;
    line-height: 1.4;
  }
  .ms-since { color: #b5a090; }

  /* account actions (code · logout) */
  .masthead-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
    padding-top: 0.3rem;
  }

  .ms-link {
    background: transparent;
    border: none;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    cursor: pointer;
    padding: 0.2rem 0;
    transition: color 0.15s;
  }
  .ms-link:hover { color: #34251c; }
  .ms-link.active { color: #c65f3c; }
  .ms-sep { color: #cbb89c; }

  /* ── Ledger column layout ── */

  .body {
    display: flex;
    flex-direction: column;
    max-width: 880px;
    margin: 0 auto;
    padding: 2.25rem 2rem 3rem;
    min-height: calc(100vh - 68px);
  }

  @media (max-width: 680px) {
    .body { padding: 1.5rem 1.1rem 2.5rem; min-height: calc(100vh - 58px); }
  }

  /* ── Hub: three large entrances ── */

  .hub {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
    margin-top: 1.6rem;
  }

  .hub-tile {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-height: 9.5rem;
    padding: 1.4rem 1.3rem;
    text-align: left;
    background: #fffaf3;
    border: 1px solid #e0d2bd;
    border-radius: 4px;
    cursor: pointer;
    transition: border-color 0.18s, background 0.18s;
  }
  .hub-tile:hover { border-color: #c65f3c; background: #fff6ec; }

  .hub-kicker {
    font-family: Inter, sans-serif;
    font-size: 0.62rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #a8916f;
  }

  .hub-name {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.85rem;
    line-height: 1.05;
    color: #34251c;
    margin-top: auto;
  }

  .hub-foot {
    font-family: Inter, sans-serif;
    font-size: 0.74rem;
    color: #9a7c5c;
  }
  .hub-count { font-weight: 700; color: #6f3b24; }
  .hub-count--unread {
    background: #c65f3c;
    color: #fff;
    border-radius: 999px;
    padding: 1px 7px;
  }

  /* ── Breadcrumb / back ── */

  .crumb {
    display: inline-block;
    margin: 0.2rem 0 1.1rem;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: #9a7c5c;
    transition: color 0.15s;
  }
  .crumb:hover { color: #c65f3c; }

  /* ── Sub-filter inside Dealings ── */

  .subfilter {
    display: flex;
    flex-wrap: wrap;
    gap: 0.15rem 0.3rem;
    align-items: baseline;
    margin-bottom: 1.1rem;
    padding-bottom: 0.6rem;
    border-bottom: 1px solid #e7dccb;
  }

  .subfilter-tab {
    display: inline-flex;
    align-items: baseline;
    gap: 0.3rem;
    background: transparent;
    border: none;
    border-bottom: 1.5px solid transparent;
    color: #9a7c5c;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 0.92rem;
    cursor: pointer;
    padding: 0.2rem 0.5rem 0.3rem;
    transition: color 0.15s, border-color 0.15s;
  }
  .subfilter-tab:hover { color: #34251c; }
  .subfilter-tab.active { color: #6f3b24; border-bottom-color: #c65f3c; }
  .subfilter-count {
    font-family: Inter, sans-serif;
    font-size: 0.6rem;
    color: #b5a090;
  }
  .subfilter-tab.active .subfilter-count { color: #c65f3c; }

  /* ── Dealings: cards grouped per work ── */

  .works-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 1rem;
  }

  .work {
    display: flex;
    flex-direction: column;
    text-align: left;
    padding: 0;
    overflow: hidden;
    background: #fffaf3;
    border: 1px solid #e4d8c8;
    border-left: 3px solid #d8c6b1;
    border-radius: 4px;
    cursor: pointer;
    transition: border-color 0.18s;
  }
  .work:hover { border-color: #c65f3c; border-left-color: #c65f3c; }
  .work--pending   { border-left-color: #c8a96a; }
  .work--confirmed { border-left-color: #5a9a5a; }
  .work--rejected  { border-left-color: #c06a6a; }
  .work--attention { border-left-color: #6f7bd0; }
  .work--neutral   { border-left-color: #d8c6b1; }

  .work-thumb {
    position: relative;
    display: block;
    aspect-ratio: 4 / 3;
    overflow: hidden;
    background: #f0e6d6;
    border-bottom: 1px solid #e4d8c8;
  }
  .work-thumb :global(.work-img),
  .work-thumb :global(.work-img .app-image-main),
  .work-thumb :global(.work-img .app-image-thumb) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: cover;
    object-position: center 42%;
  }
  .work-thumb :global(.work-img .app-image-main) {
    filter: grayscale(0.08) saturate(0.96);
    transition: transform 0.45s ease, filter 0.45s ease;
  }
  .work:hover .work-thumb :global(.work-img .app-image-main) {
    transform: scale(1.035);
    filter: grayscale(0) saturate(1.02);
  }

  .work-thumb-ph {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 2rem;
    color: #b5a090;
  }

  .work-flag {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
    display: grid;
    place-items: center;
    width: 1.4rem;
    height: 1.4rem;
    background: #c65f3c;
    color: #fff;
    border-radius: 999px;
    font-size: 0.7rem;
  }

  .work-body {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.8rem 0.85rem 0.9rem;
  }

  .work-name {
    font-family: Georgia, serif;
    font-size: 0.92rem;
    line-height: 1.3;
    color: #34251c;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .work-kinds {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .work-kind {
    font-family: Inter, sans-serif;
    font-size: 0.55rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #6f3b24;
    background: rgba(111,59,36,0.07);
    border: 1px solid rgba(111,59,36,0.16);
    border-radius: 2px;
    padding: 1px 6px;
  }

  /* ── Work detail header ── */

  .work-head {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.2rem;
    padding-bottom: 1.1rem;
    border-bottom: 1px solid #e7dccb;
  }
  .work-head-thumb {
    display: block;
    width: 84px;
    aspect-ratio: 4 / 3;
    flex-shrink: 0;
    overflow: hidden;
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    background: #f0e6d6;
  }
  .work-head-thumb :global(.work-head-img),
  .work-head-thumb :global(.work-head-img .app-image-main),
  .work-head-thumb :global(.work-head-img .app-image-thumb) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: cover;
    object-position: center 42%;
  }
  .work-head-ph {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.6rem;
    color: #b5a090;
  }
  .work-head-name {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.5rem;
    font-weight: 400;
    color: #34251c;
    margin: 0 0 0.2rem;
    line-height: 1.15;
  }
  .work-head-sub {
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.04em;
    color: #9a7c5c;
    margin: 0;
  }

  /* ── Body ── */

  .ledger-body {
    flex: 1;
    min-width: 0;
    padding-top: 0.3rem;
  }

  /* ── Footer: quiet danger zone ── */

  .ledger-foot {
    margin-top: 2.5rem;
    padding-top: 1.1rem;
    border-top: 1px solid #e4d8c8;
  }

  .foot-delete {
    background: transparent;
    border: none;
    color: #c8b89a;
    font-family: Inter, sans-serif;
    font-size: 0.66rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    cursor: pointer;
    padding: 0;
    transition: color 0.15s;
  }
  .foot-delete:hover { color: #a3361d; }

  .foot-confirm {
    max-width: 420px;
    padding: 0.75rem 0.85rem;
    background: #fdf3f3;
    border: 1px solid #f0d0c8;
    border-radius: 3px;
  }

  .foot-warning {
    font-family: Inter, sans-serif;
    font-size: 0.74rem;
    color: #7a3020;
    margin: 0 0 0.7rem;
    line-height: 1.45;
  }

  .foot-confirm-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .foot-yes {
    background: #9b2020;
    border: none;
    color: #fff;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.4rem 0.85rem;
    border-radius: 3px;
    cursor: pointer;
    transition: background 0.15s;
  }
  .foot-yes:hover:not(:disabled) { background: #7a1818; }
  .foot-yes:disabled { opacity: 0.6; cursor: not-allowed; }

  .foot-cancel {
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.4rem 0.85rem;
    border-radius: 3px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .foot-cancel:hover { border-color: #9a7c5c; color: #34251c; }

  .empty {
    color: #9a7c5c;
    font-style: italic;
    font-size: 0.9rem;
    padding: 2.5rem 0;
    text-align: center;
    font-family: Georgia, serif;
  }

  .error-msg {
    color: #c65f3c;
    font-size: 0.85rem;
    font-family: Inter, sans-serif;
    padding: 1rem 0;
  }

  /* ── Attach a request by code ── */
  .claim {
    margin: 0.9rem 0 0.2rem;
    padding: 0.9rem 1rem 1rem;
    border: 1px solid #e4d8c8;
    border-left: 2px solid #c65f3c;
    border-radius: 3px;
    background: rgba(255,252,246,0.7);
  }

  .claim-hint {
    font-family: Inter, sans-serif;
    font-size: 0.76rem;
    line-height: 1.5;
    color: #8a7253;
    margin: 0 0 0.7rem;
    max-width: 56ch;
  }

  .claim-form {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .claim-input {
    flex: 1;
    min-width: 180px;
    font-family: 'Courier New', monospace;
    font-size: 0.82rem;
    letter-spacing: 0.04em;
    background: #fffaf4;
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    color: #34251c;
    padding: 0.5rem 0.7rem;
    outline: none;
    transition: border-color 0.15s;
  }
  .claim-input::placeholder { color: #b5a090; font-family: Inter, sans-serif; letter-spacing: 0; }
  .claim-input:focus { border-color: #c65f3c; }

  .claim-btn {
    background: #6f3b24;
    border: none;
    color: #f8f1e7;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    padding: 0.5rem 1.1rem;
    border-radius: 3px;
    cursor: pointer;
    transition: background 0.15s;
    white-space: nowrap;
  }
  .claim-btn:hover:not(:disabled) { background: #c65f3c; }
  .claim-btn:disabled { opacity: 0.45; cursor: default; }

  .claim-msg {
    margin: 0.65rem 0 0;
    font-family: Inter, sans-serif;
    font-size: 0.78rem;
    line-height: 1.45;
  }
  .claim-msg--ok   { color: #2d6a3f; }
  .claim-msg--warn { color: #8a5a1a; }
  .claim-msg--err  { color: #a03020; }

  .claim-kind {
    display: inline-block;
    font-size: 0.6rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #6f3b24;
    background: rgba(198,95,60,0.1);
    border: 1px solid rgba(198,95,60,0.25);
    padding: 1px 6px;
    margin: 0 0.15rem;
    vertical-align: middle;
  }
  .claim-name { font-style: italic; }

  /* ── The ledger (гроссбух): one ruled account of every dealing ── */
  .book {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .entry {
    border-bottom: 1px solid #e7dccb;
    border-left: 2px solid transparent;
    padding: 0.95rem 0.5rem 1rem 0.9rem;
  }
  .entry:first-child { border-top: 1px solid #e7dccb; }
  .entry--unread { border-left-color: #c65f3c; background: rgba(198,95,60,0.025); }

  .entry-head {
    display: flex;
    align-items: baseline;
    gap: 0.7rem;
    color: #34251c;
    font-family: Georgia, serif;
  }

  .entry-date {
    font-family: Inter, sans-serif;
    font-size: 0.7rem;
    color: #a8916f;
    flex-shrink: 0;
    min-width: 8.5rem;
    font-variant-numeric: tabular-nums;
  }

  .entry-kind {
    font-family: Inter, sans-serif;
    font-size: 0.58rem;
    letter-spacing: 0.11em;
    text-transform: uppercase;
    color: #9a7c5c;
    flex-shrink: 0;
    min-width: 5.2rem;
  }

  .entry-title {
    font-size: 0.97rem;
    color: #34251c;
    text-decoration: none;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1 1 auto;
    min-width: 0;
  }
  .entry-title--link:hover { color: #c65f3c; }

  .entry-status {
    font-family: Inter, sans-serif;
    font-size: 0.62rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    padding: 2px 8px;
    border-radius: 2px;
    white-space: nowrap;
    flex-shrink: 0;
    margin-left: auto;
  }
  .entry-status--pending   { background: #f4ead8; color: #6f3b24; }
  .entry-status--confirmed { background: #e8f4e8; color: #2d6a3f; }
  .entry-status--rejected  { background: #fde8e8; color: #9b2020; }
  .entry-status--attention { background: #eef0fb; color: #3a4a8a; }
  .entry-status--neutral   { background: #ececec; color: #666; }

  /* always-visible record detail, indented under the title */
  .entry-detail {
    padding: 0.55rem 0 0 9.2rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    align-items: flex-start;
  }

  .d-range {
    font-family: Inter, sans-serif;
    font-style: italic;
    font-size: 0.82rem;
    color: #6f4e37;
    margin: 0;
  }

  .d-line { margin: 0; }
  .d-tag {
    font-family: Inter, sans-serif;
    font-size: 0.62rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: #9a7c5c;
  }

  .d-desc {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 1.02rem;
    line-height: 1.5;
    color: #5f4636;
    margin: 0;
    max-width: 60ch;
    white-space: pre-wrap;
  }

  /* Commission progress stepper (client-facing) */
  .ctl {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem 0;
    margin: 0 0 0.9rem;
    padding: 0;
  }
  .ctl-step {
    position: relative;
    flex: 1 1 0;
    min-width: 4.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.35rem;
    text-align: center;
  }
  /* connector line between dots */
  .ctl-step::before {
    content: '';
    position: absolute;
    top: 0.6rem;
    left: -50%;
    width: 100%;
    height: 2px;
    background: #d8c6b1;
    z-index: 0;
  }
  .ctl-step:first-child::before { display: none; }
  .ctl-step--done::before { background: #c65f3c; }
  .ctl-dot {
    position: relative;
    z-index: 1;
    display: grid;
    place-items: center;
    width: 1.2rem;
    height: 1.2rem;
    border-radius: 50%;
    border: 2px solid #d8c6b1;
    background: #f8f1e7;
    color: #fffaf2;
    font-size: 0.65rem;
    line-height: 1;
  }
  .ctl-step--done .ctl-dot { background: #c65f3c; border-color: #c65f3c; }
  .ctl-step--current .ctl-dot { border-color: #c65f3c; box-shadow: 0 0 0 3px rgba(198,95,60,0.2); background: #fffaf2; }
  .ctl-label {
    font-family: Inter, sans-serif;
    font-size: 0.66rem;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: #9a7c5c;
    line-height: 1.2;
  }
  .ctl-step--done .ctl-label,
  .ctl-step--current .ctl-label { color: #6f3b24; }
  .ctl-step--current .ctl-label { font-weight: 700; }
  .ctl-declined {
    margin: 0 0 0.9rem;
    display: inline-block;
    padding: 0.3rem 0.7rem;
    border: 1px solid #c0a08f;
    background: rgba(120,80,60,0.06);
    color: #8a5a44;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .reserve-detail {
    display: grid;
    gap: 0.55rem;
    width: min(100%, 34rem);
  }

  .reserve-detail > p,
  .reserve-note {
    margin: 0;
    color: #5f4636;
    font-family: Inter, sans-serif;
    font-size: 0.78rem;
    line-height: 1.45;
  }

  .reserve-note {
    font-style: italic;
  }

  .commission-source {
    display: grid;
    grid-template-columns: 54px minmax(0, 1fr);
    gap: 0.65rem;
    align-items: center;
    width: min(100%, 28rem);
    padding: 0.55rem;
    border: 1px solid rgba(52,37,28,0.12);
    background: rgba(255,250,242,0.62);
  }

  .commission-source-thumb {
    display: block;
    width: 54px;
    aspect-ratio: 1;
    overflow: hidden;
    border: 1px solid #d8c6b1;
    background: #f0e6d6;
    text-decoration: none;
  }

  .commission-source-thumb--empty {
    display: grid;
    place-items: center;
    color: #6f3b24;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 0.72rem;
    letter-spacing: 0.08em;
  }

  .commission-source-copy {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 0.08rem;
  }

  .commission-source-label {
    font-family: Inter, sans-serif;
    font-size: 0.55rem;
    font-weight: 700;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.62);
  }

  .commission-source-name {
    overflow: hidden;
    color: #34251c;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 0.96rem;
    text-decoration: none;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .commission-source-name:hover { color: #c65f3c; }
  .commission-source-name--missing {
    color: #9a7c5c;
    font-family: 'Courier New', monospace;
    font-size: 0.74rem;
  }
  .commission-source-meta {
    color: #8a6d55;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
  }

  .commission-similar-notes {
    display: grid;
    gap: 0.3rem;
    width: min(100%, 34rem);
  }

  .commission-similar-notes p {
    margin: 0;
    color: #5f4636;
    font-family: Inter, sans-serif;
    font-size: 0.78rem;
    line-height: 1.45;
  }

  .commission-similar-notes span {
    display: block;
    margin-bottom: 0.08rem;
    color: rgba(95,70,54,0.62);
    font-size: 0.56rem;
    font-weight: 700;
    letter-spacing: 0.09em;
    text-transform: uppercase;
  }

  .d-curator {
    padding: 0.45rem 0.7rem;
    background: rgba(34,139,34,0.05);
    border-left: 2px solid rgba(34,139,34,0.3);
  }
  .d-curator-label {
    display: block;
    font-family: Inter, sans-serif;
    font-size: 0.56rem;
    text-transform: uppercase;
    letter-spacing: 0.09em;
    color: rgba(95,70,54,0.55);
    font-weight: 700;
    margin-bottom: 0.2rem;
  }
  .d-curator-text {
    margin: 0;
    font-size: 0.84rem;
    color: #34251c;
    font-style: italic;
    line-height: 1.4;
  }

  .d-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.4rem 1rem;
  }

  .d-link {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: #6f3b24;
    text-decoration: none;
    transition: color 0.15s;
  }
  .d-link:hover { color: #c65f3c; }
  .d-link.danger { color: #a3361d; }
  .d-link.danger:hover { color: #7a1818; }
  .d-link:disabled { opacity: 0.5; cursor: default; }

  .entry-reply {
    margin: 0.55rem 0 0 9.2rem;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.04em;
    color: #c65f3c;
  }
  .entry-reply:hover { text-decoration: underline; }

  @media (max-width: 680px) {
    .entry-head { flex-wrap: wrap; gap: 0.3rem 0.6rem; }
    .entry-kind { min-width: 0; }
    .entry-title { white-space: normal; flex: 1 1 100%; order: 5; }
    .entry-status { margin-left: 0; }
    .entry-detail { padding-left: 0; }
    .entry-reply { margin-left: 0; }
  }

  /* ── Cards grid ── */

  .cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.85rem;
    max-width: 1100px;
  }

  .card {
    display: block;
    background: #fffaf3;
    border: 1px solid #e4d8c8;
    border-radius: 3px;
    padding: 1rem 1.1rem;
    text-decoration: none;
    color: #34251c;
    transition: border-color 0.18s;
  }
  .card:hover {
    border-color: #c65f3c;
  }

  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.4rem;
    margin-bottom: 0.4rem;
  }

  .card-name {
    font-family: Georgia, serif;
    font-size: 0.93rem;
    color: #34251c;
    line-height: 1.35;
    flex: 1;
    min-width: 0;
    text-decoration: none;
  }
  .card-name--link:hover { color: #c65f3c; }
  .card-name--loading { color: #b5a090; font-style: italic; }
  .card-name--missing { color: #b5a090; font-size: 0.78rem; font-family: 'Courier New', monospace; }

  .wishlist-card {
    padding: 0;
    overflow: hidden;
  }

  .wishlist-sync-error {
    margin: 0 0 0.8rem;
    padding: 0.55rem 0.7rem;
    border: 1px solid #f0d0c8;
    background: #fdf3f0;
    color: #8a3926;
    font-family: Inter, sans-serif;
    font-size: 0.76rem;
    line-height: 1.4;
  }

  .wishlist-thumb {
    display: block;
    aspect-ratio: 4 / 3;
    overflow: hidden;
    background: #f4ead8;
    border-bottom: 1px solid #e4d8c8;
    text-decoration: none;
  }

  .wishlist-thumb :global(.wishlist-img),
  .wishlist-thumb :global(.wishlist-img .app-image-main),
  .wishlist-thumb :global(.wishlist-img .app-image-thumb) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: cover;
    object-position: center 42%;
  }

  .wishlist-thumb :global(.wishlist-img .app-image-main) {
    filter: grayscale(0.08) saturate(0.96);
    transition: transform 0.45s ease, filter 0.45s ease;
  }

  .wishlist-card:hover .wishlist-thumb :global(.wishlist-img .app-image-main) {
    transform: scale(1.035);
    filter: grayscale(0) saturate(1.02);
  }

  .wishlist-thumb--empty {
    display: grid;
    place-items: center;
    font-family: Georgia, serif;
    font-size: 1.4rem;
    color: #b5a090;
  }

  .wishlist-head {
    align-items: flex-start;
    margin: 0;
    padding: 0.9rem 1rem 1rem;
  }

  .wishlist-title {
    min-width: 0;
  }

  .wishlist-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.35rem;
    margin-top: 0.28rem;
    font-size: 0.67rem;
    color: #b5a090;
    font-family: Inter, sans-serif;
  }

  .wishlist-remove {
    width: 1.55rem;
    height: 1.55rem;
    display: inline-grid;
    place-items: center;
    flex-shrink: 0;
    border: 1px solid #e4d8c8;
    border-radius: 999px;
    background: #fffaf4;
    color: #9a7c5c;
    cursor: pointer;
    line-height: 1;
    transition: color 0.18s, border-color 0.18s, background 0.18s;
  }

  .wishlist-remove:hover {
    color: #c65f3c;
    border-color: rgba(198,95,60,0.35);
    background: #fff3ec;
  }

  .card-range {
    font-size: 0.77rem;
    color: #9a7c5c;
    margin: 0 0 0.2rem;
    font-family: Inter, sans-serif;
    font-style: italic;
  }

  .card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    margin-top: 0.2rem;
  }

  .card-date {
    font-size: 0.68rem;
    color: #b5a090;
    margin: 0;
    font-family: Inter, sans-serif;
  }

  .card-manage {
    font-family: Inter, sans-serif;
    font-size: 0.66rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: #c65f3c;
    text-decoration: none;
    opacity: 0.75;
    flex-shrink: 0;
    transition: opacity 0.15s;
  }
  .card-manage:hover { opacity: 1; }

  .card-curator {
    margin: 0.4rem 0 0.25rem;
    padding: 0.4rem 0.65rem;
    background: rgba(34,139,34,0.05);
    border-left: 2px solid rgba(34,139,34,0.3);
  }
  .card-curator-label {
    display: block;
    font-family: Inter, sans-serif;
    font-size: 0.58rem;
    text-transform: uppercase;
    letter-spacing: 0.09em;
    color: rgba(95,70,54,0.5);
    font-weight: 700;
    margin-bottom: 0.2rem;
  }
  .card-curator-text {
    margin: 0;
    font-size: 0.8rem;
    color: #34251c;
    font-style: italic;
    line-height: 1.4;
  }

  .status {
    font-size: 0.62rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-family: Inter, sans-serif;
    padding: 2px 6px;
    border-radius: 2px;
    flex-shrink: 0;
    white-space: nowrap;
  }
  .status--pending   { background: #f4ead8; color: #6f3b24; }
  .status--confirmed { background: #e8f4e8; color: #2d6a3f; }
  .status--rejected  { background: #fde8e8; color: #9b2020; }
  .status--cancelled { background: #ebebeb; color: #666; }

  .order-badges {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex-shrink: 0;
  }

  .mode {
    font-size: 0.62rem;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .order-status {
    font-size: 0.6rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    font-family: Inter, sans-serif;
    padding: 2px 5px;
    border-radius: 2px;
    white-space: nowrap;
  }
  .order-status--new     { background: #f4ead8; color: #6f3b24; }
  .order-status--seen    { background: #e8eef4; color: #2d4a6a; }
  .order-status--replied { background: #e8f4e8; color: #2d6a3f; }

  .wishlist-status {
    font-size: 0.67rem;
    letter-spacing: 0.05em;
    font-family: Inter, sans-serif;
    color: #9a7c5c;
    font-style: italic;
    flex-shrink: 0;
  }
  .wishlist-status--sold      { color: #b5a090; }
  .wishlist-status--available { color: #4a7a3a; }

  .item-date {
    font-size: 0.7rem;
    color: #b5a090;
    margin: 0.1rem 0 0;
    font-family: Inter, sans-serif;
  }


  /* ── Messages ── */

  /* Commission attachments in reply + petitions tab */
  .reply-atts { display: flex; flex-wrap: wrap; gap: 0.4rem; padding: 0.4rem 0; }
  .reply-att { position: relative; width: 48px; height: 48px; border: 1px solid #d8c6b1; overflow: hidden; }
  .reply-att img { width: 100%; height: 100%; object-fit: cover; }
  .reply-att button { position: absolute; top: 0; right: 0; width: 16px; height: 16px; background: rgba(52,37,28,0.8); color: #fff; border: none; cursor: pointer; line-height: 1; font-size: 0.7rem; }
  .chat-attach-btn { display: inline-grid; place-items: center; width: 2rem; height: 2rem; border: 1px solid #d8c6b1; cursor: pointer; font-size: 0.9rem; margin-right: auto; }
  .chat-attach-btn:hover { border-color: #c65f3c; }
  .empty-with-cta { text-align: center; }
  .commission-cta { display: inline-block; margin-top: 1rem; background: #6f3b24; color: #f8f1e7; padding: 0.6rem 1.4rem; text-decoration: none; font-size: 0.8rem; letter-spacing: 0.06em; text-transform: uppercase; transition: background 0.2s; }
  .commission-cta:hover { background: #c65f3c; }
  .commission-cta--inline { margin-top: 1.5rem; }
  .commission-card { display: flex; flex-direction: column; gap: 0.5rem; }
  .card-desc { font-family: 'Cormorant Garamond', Georgia, serif; font-size: 1rem; color: #5f4636; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
  .commission-thumbs { display: flex; gap: 0.3rem; flex-wrap: wrap; }
  .commission-thumbs img { width: 40px; height: 40px; object-fit: cover; border: 1px solid #d8c6b1; }
  .card-foot { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; margin-top: auto; }
  .commission-open { background: none; border: none; color: #c65f3c; cursor: pointer; font-size: 0.78rem; letter-spacing: 0.04em; padding: 0; }
  .commission-open:hover { text-decoration: underline; }
  .commission-manage { display: flex; align-items: center; gap: 0.75rem; margin-top: 0.6rem; padding-top: 0.6rem; border-top: 1px solid #e8dcc9; flex-wrap: wrap; }
  .commission-link { background: none; border: none; padding: 0; cursor: pointer; font-size: 0.75rem; letter-spacing: 0.04em; color: #6f3b24; }
  .commission-link:hover { text-decoration: underline; }
  .commission-link.danger { color: #a3361d; }
  .commission-locked { font-size: 0.72rem; font-style: italic; color: #8a7a6a; }
  .commission-confirm { font-size: 0.75rem; color: #a3361d; }

  .messages-wrap {
    max-width: 720px;
  }

  .badge--unread {
    background: #c65f3c;
    color: #fff;
  }

  .threads-header {
    position: sticky;
    top: 68px;
    z-index: 2;
    background: #f8f1e7;
    padding-top: 0.75rem;
    padding-bottom: 0.85rem;
    margin-bottom: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
    border-bottom: 1px solid rgba(216,198,177,0.5);
    margin-left: -2rem;
    margin-right: -2rem;
    padding-left: 2rem;
    padding-right: 2rem;
  }

  @media (max-width: 680px) {
    .threads-header { top: 58px; }
  }

  .thread-filters {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
  }

  .thread-filter-btn {
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.2rem 0.55rem;
    cursor: pointer;
    transition: all 0.15s;
  }
  .thread-filter-btn:hover { border-color: #9a7c5c; color: #34251c; }
  .thread-filter-btn.active { border-color: #c65f3c; color: #c65f3c; background: rgba(198,95,60,0.06); }

  .compose-toggle-btn {
    background: transparent;
    border: 1px solid #c65f3c;
    color: #c65f3c;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.2rem 0.65rem;
    cursor: pointer;
    transition: background 0.15s;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .compose-toggle-btn:hover { background: rgba(198,95,60,0.08); }

  .msg-compose {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 1.25rem;
    padding: 0.85rem;
    border: 1px solid #d8c6b1;
    background: rgba(248,241,231,0.5);
  }

  .msg-subject {
    font-family: Georgia, serif;
    font-size: 0.88rem;
    background: transparent;
    border: none;
    border-bottom: 1px solid #d8c6b1;
    color: #34251c;
    padding: 4px 0;
    outline: none;
  }
  .msg-subject::placeholder { color: #b5a090; }

  .msg-body {
    font-family: Inter, sans-serif;
    font-size: 0.82rem;
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #34251c;
    padding: 0.5rem;
    outline: none;
    resize: vertical;
    line-height: 1.5;
  }
  .msg-body::placeholder { color: #b5a090; }
  .msg-body:focus { border-color: #c65f3c; }

  .msg-send-btn {
    align-self: flex-end;
    background: transparent;
    border: 1px solid #c65f3c;
    color: #c65f3c;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    padding: 0.35rem 0.85rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .msg-send-btn:hover:not(:disabled) { background: rgba(198,95,60,0.08); }
  .msg-send-btn:disabled { opacity: 0.45; cursor: default; }

  .field-error {
    margin: 0.4rem 0 0;
    font-size: 0.75rem;
    color: #a03020;
    font-style: italic;
  }

  .msg-item { cursor: pointer; transition: background 0.12s; position: relative; }
  .msg-item:hover { background: rgba(216,198,177,0.15); }
  /* Invisible full-area button: keyboard-accessible row without a role on the <li> */
  .item-hit {
    position: absolute;
    inset: 0;
    width: 100%;
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    cursor: pointer;
  }
  .msg-unread { border-left: 2px solid #c65f3c; padding-left: 0.6rem; }

  .thread-subject {
    font-family: Georgia, serif;
    font-size: 0.92rem;
    color: #34251c;
    line-height: 1.3;
    flex: 1;
    min-width: 0;
  }

  .thread-meta-right {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex-shrink: 0;
  }

  .msg-new-badge {
    font-family: Inter, sans-serif;
    font-size: 0.6rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    background: #c65f3c;
    color: #fff;
    padding: 1px 5px;
  }

  .thread-resolved-small {
    font-family: Inter, sans-serif;
    font-size: 0.6rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #b5a090;
    font-style: italic;
  }

  .msg-body-preview {
    font-family: Inter, sans-serif;
    font-size: 0.82rem;
    color: #6f4e37;
    margin: 0.15rem 0 0;
    line-height: 1.5;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ── Thread detail ── */

  .thread-detail {
    display: flex;
    flex-direction: column;
  }

  .thread-detail-header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid #d8c6b1;
    margin-bottom: 0.85rem;
    flex-wrap: wrap;
  }

  .thread-back {
    background: transparent;
    border: none;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    font-size: 0.75rem;
    letter-spacing: 0.04em;
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
    transition: color 0.15s;
  }
  .thread-back:hover { color: #c65f3c; }

  .thread-detail-subject {
    font-family: Georgia, serif;
    font-size: 0.95rem;
    color: #34251c;
    flex: 1;
    min-width: 0;
  }

  .thread-resolved-badge {
    font-family: Inter, sans-serif;
    font-size: 0.62rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #9a7c5c;
    border: 1px solid #d8c6b1;
    padding: 1px 6px;
    flex-shrink: 0;
  }

  /* ── Chat bubbles ── */

  .chat-messages {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    max-height: 360px;
    overflow-y: auto;
    padding: 0.25rem 0 0.75rem;
    scroll-behavior: smooth;
  }

  .chat-row {
    display: flex;
    flex-direction: column;
    max-width: 82%;
  }
  .chat-row--user {
    align-self: flex-end;
    align-items: flex-end;
  }
  .chat-row--admin {
    align-self: flex-start;
    align-items: flex-start;
  }

  .chat-bubble {
    padding: 0.55rem 0.8rem;
    line-height: 1.55;
  }
  .chat-bubble--user {
    background: #f4ead8;
    border: 1px solid #d8c6b1;
  }
  .chat-bubble--admin {
    background: #ece0ce;
    border: 1px solid #c8b89a;
  }

  .chat-sender {
    font-family: Inter, sans-serif;
    font-size: 0.6rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #9a7c5c;
    margin: 0 0 0.2rem;
  }

  .chat-body {
    font-family: Inter, sans-serif;
    font-size: 0.85rem;
    color: #34251c;
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .chat-time {
    font-family: Inter, sans-serif;
    font-size: 0.63rem;
    color: #b5a090;
    margin: 0.2rem 0 0;
  }

  /* ── Chat reply ── */

  .chat-reply {
    padding-top: 0.85rem;
    border-top: 1px solid #eee3d6;
    margin-top: 0.25rem;
  }

  .chat-reply-input {
    width: 100%;
    box-sizing: border-box;
    font-family: Inter, sans-serif;
    font-size: 0.85rem;
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #34251c;
    padding: 0.6rem 0.75rem;
    outline: none;
    resize: none;
    line-height: 1.55;
    min-height: 72px;
    transition: border-color 0.15s;
  }
  .chat-reply-input::placeholder { color: #b5a090; }
  .chat-reply-input:focus { border-color: #c65f3c; }

  .chat-reply-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 0.5rem;
  }

  .chat-reply-hint {
    font-family: Inter, sans-serif;
    font-size: 0.63rem;
    color: #c8b89a;
    letter-spacing: 0.03em;
  }

  .chat-reply-btn {
    background: #c65f3c;
    border: none;
    color: #fff;
    font-family: Inter, sans-serif;
    font-size: 0.73rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.4rem 1rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .chat-reply-btn:hover:not(:disabled) { background: #a84e30; }
  .chat-reply-btn:disabled { opacity: 0.45; cursor: default; }

  .chat-resolved-note {
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    color: #b5a090;
    font-style: italic;
    text-align: center;
    padding: 0.75rem 0 0;
    border-top: 1px solid #eee3d6;
    margin-top: 0.25rem;
  }
</style>
