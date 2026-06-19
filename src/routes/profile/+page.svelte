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

  type Tab = 'overview' | 'bookings' | 'orders' | 'commissions' | 'wishlist' | 'messages';
  let activeTab = $state<Tab>('overview');

  let bookings = $state<UserBookingDto[]>([]);
  let orders = $state<UserOrderDto[]>([]);
  let commissions = $state<CommissionDto[]>([]);
  let waitlist = $state<WaitlistEntryDto[]>([]);
  let wishlistIds = $state<string[]>([]);
  let wishlistItems = $state<Map<string, FigurineListItem | null>>(new Map());
  let threads = $state<MessageThreadDto[]>([]);
  let unreadCount = $state(0);
  let loading = $state(true);
  let error = $state('');

  // ── Attach a guest request by its secret code ──
  let claimCode = $state('');
  let claimSubmitting = $state(false);
  let claimResult = $state<LinkClaimResponse | null>(null);
  let claimError = $state('');

  // Where to send the user after a successful link. Waitlist has no profile tab,
  // so it stays on the current one and relies on the inline confirmation.
  const CLAIM_KIND_TAB: Partial<Record<LinkClaimKind, Tab>> = {
    booking: 'bookings',
    notify: 'orders',
    commission: 'commissions',
  };

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
        const tab = res.kind ? CLAIM_KIND_TAB[res.kind] : undefined;
        if (tab) activeTab = tab;
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

      await savedFigurines.syncWithServer();
      wishlistIds = [...savedFigurines.ids];

      if (wishlistIds.length > 0) {
        loadWishlistDetails();
      }
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

  async function loadWishlistDetails() {
    const ids = [...savedFigurines.ids];
    wishlistIds = ids;
    const all = await api.getAllFigurines().catch(() => [] as FigurineListItem[]);
    const byId = new Map(all.map((fig) => [fig.id, fig]));
    const map = new Map<string, FigurineListItem | null>();
    ids.forEach((id) => {
      map.set(id, byId.get(id) ?? null);
    });
    wishlistItems = map;
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
    authStore.clearSession();
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
      authStore.clearSession();
      goto('/');
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
    };
    return map[mode] ?? mode;
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
    activeTab = 'messages';
    openThreadDetail(item.threadId);
  }
</script>

<svelte:head>
  <title>{$t('profileTitle')} — {$brandName}</title>
</svelte:head>

<div class="page">
  <div class="frame">
    <div class="body">

      <!-- ── Sidebar ── -->
      <aside class="sidebar">

        <!-- Profile (hero) -->
        <div class="sidebar-profile">
          <div class="hero-avatar-zone">
            <button
              class="hero-avatar-btn"
              onclick={() => avatarInput?.click()}
              title={$t('profileUploadPhoto')}
              disabled={uploadingAvatar}
            >
              {#if resolveMediaUrl(authStore.user?.avatarUrl)}
                <img src={resolveMediaUrl(authStore.user?.avatarUrl)} alt="" class="hero-avatar-img" />
              {:else}
                <span class="hero-avatar-initials">{(authStore.user?.displayName ?? '?')[0].toUpperCase()}</span>
              {/if}
              <span class="hero-avatar-overlay">{uploadingAvatar ? $t('profileUploadingPhoto') : $t('profileUploadPhoto')}</span>
            </button>
            <input
              bind:this={avatarInput}
              type="file"
              accept="image/jpeg,image/png,image/webp"
              class="sr-only"
              onchange={handleAvatarChange}
            />
            {#if avatarError}<p class="field-error" role="alert">{avatarError}</p>{/if}
          </div>

          <div class="hero-identity">
            {#if editingName}
              <div class="hero-name-edit">
                <input
                  class="hero-name-input"
                  bind:value={editNameValue}
                  onkeydown={(e) => { if (e.key === 'Enter') saveName(); if (e.key === 'Escape') cancelEditName(); }}
                  use:focusOnMount
                />
                <button class="hero-name-save" onclick={saveName} disabled={savingName}>
                  {savingName ? '…' : $t('profileSaveName')}
                </button>
                <button class="hero-name-cancel" onclick={cancelEditName}>✕</button>
              </div>
              {#if nameError}<span class="field-error" role="alert">{nameError}</span>{/if}
            {:else}
              <div class="hero-name-row">
                <h1 class="hero-name">{authStore.user?.displayName ?? ''}</h1>
                <button class="hero-edit-btn" onclick={startEditName} title={$t('profileEditName')}>
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
                    <path d="M8.5 1.5L10.5 3.5L4 10H2V8L8.5 1.5Z"/>
                  </svg>
                </button>
              </div>
              {#if nameSaved}<span class="hero-name-saved">{$t('profileNameSaved')}</span>{/if}
            {/if}
            <p class="hero-email">{authStore.user?.email ?? ''}</p>
            {#if authStore.user?.createdAt}
              <p class="hero-since">{$t('profileMemberSince')} {formatDate(authStore.user.createdAt)}</p>
            {/if}
          </div>
        </div>
        <nav class="sidebar-nav">
          <button class="nav-item" class:active={activeTab === 'overview'} onclick={() => activeTab = 'overview'}>
            <span>{$t('profileOverview')}</span>
            {#if feed.length > 0}<span class="nav-badge">{feed.length}</span>{/if}
          </button>
          <button class="nav-item" class:active={activeTab === 'bookings'} onclick={() => activeTab = 'bookings'}>
            <span>{$t('profileBookings')}</span>
            {#if bookings.length > 0}<span class="nav-badge">{bookings.length}</span>{/if}
          </button>
          <button class="nav-item" class:active={activeTab === 'orders'} onclick={() => activeTab = 'orders'}>
            <span>{$t('profileOrders')}</span>
            {#if orders.length > 0}<span class="nav-badge">{orders.length}</span>{/if}
          </button>
          <button class="nav-item" class:active={activeTab === 'commissions'} onclick={() => activeTab = 'commissions'}>
            <span>{$t('profileCommissions')}</span>
            {#if commissions.length > 0}<span class="nav-badge">{commissions.length}</span>{/if}
          </button>
          <button class="nav-item" class:active={activeTab === 'wishlist'} onclick={() => activeTab = 'wishlist'}>
            <span>{$t('profileWishlist')}</span>
            {#if wishlistIds.length > 0}<span class="nav-badge">{wishlistIds.length}</span>{/if}
          </button>
          <button class="nav-item" class:active={activeTab === 'messages'} onclick={() => { activeTab = 'messages'; closeThread(); }}>
            <span>{$t('profileMessages')}</span>
            {#if unreadCount > 0}<span class="nav-badge nav-badge--unread">{unreadCount}</span>{/if}
          </button>
        </nav>

        <div class="sidebar-footer">
          <button class="sidebar-logout" onclick={logout}>{$t('profileLogout')}</button>
          {#if !showDeleteConfirm}
            <button class="sidebar-delete-btn" onclick={() => showDeleteConfirm = true}>{$t('profileDeleteAccount')}</button>
          {:else}
            <div class="sidebar-delete-confirm">
              <p class="sidebar-delete-warning">{$t('profileDeleteWarning')}</p>
              <div class="sidebar-delete-actions">
                <button class="sidebar-delete-yes" onclick={deleteAccount} disabled={deleting}>
                  {deleting ? $t('profileDeleting') : $t('profileDeleteConfirm')}
                </button>
                <button class="sidebar-delete-cancel" onclick={() => showDeleteConfirm = false}>
                  {$t('profileDeleteCancel')}
                </button>
              </div>
              {#if deleteError}<p class="field-error" role="alert">{deleteError}</p>{/if}
            </div>
          {/if}
        </div>
      </aside>

      <!-- Content -->
      <div class="content">

        <!-- Attach a guest request by code (lost localStorage / changed device) -->
        <section class="claim-link" aria-labelledby="claim-link-title">
          <div class="claim-link-head">
            <h2 id="claim-link-title" class="claim-link-title">{$t('profileLinkClaimTitle')}</h2>
            <p class="claim-link-hint">{$t('profileLinkClaimHint')}</p>
          </div>
          <form class="claim-link-form" onsubmit={(e) => { e.preventDefault(); submitClaim(); }}>
            <input
              class="claim-link-input"
              bind:value={claimCode}
              placeholder={$t('profileLinkClaimPlaceholder')}
              autocomplete="off"
              autocapitalize="off"
              spellcheck="false"
              aria-label={$t('profileLinkClaimTitle')}
            />
            <button class="claim-link-btn" type="submit" disabled={claimSubmitting || !claimCode.trim()}>
              {claimSubmitting ? $t('profileLinkClaimLinking') : $t('profileLinkClaimButton')}
            </button>
          </form>
          {#if claimError}
            <p class="claim-link-msg claim-link-msg--err" role="alert">{claimError}</p>
          {:else if claimResult}
            {#if claimResult.result === 'linked'}
              <p class="claim-link-msg claim-link-msg--ok" role="status">
                {$t('profileLinkClaimLinked')}
                {#if claimResult.kind}<span class="claim-link-kind">{claimKindLabel(claimResult.kind)}</span>{/if}
                {#if claimResult.name}<span class="claim-link-name">{claimResult.name}</span>{/if}
              </p>
            {:else if claimResult.result === 'email_mismatch'}
              <p class="claim-link-msg claim-link-msg--warn" role="alert">{$t('profileLinkClaimMismatch')}</p>
            {:else if claimResult.result === 'already_linked'}
              <p class="claim-link-msg claim-link-msg--warn" role="status">{$t('profileLinkClaimAlready')}</p>
            {:else}
              <p class="claim-link-msg claim-link-msg--err" role="alert">{$t('profileLinkClaimNotFound')}</p>
            {/if}
          {/if}
        </section>

        {#if loading}
          <p class="empty">…</p>
        {:else if error}
          <p class="error-msg">{error}</p>
        {:else if activeTab === 'overview'}
          {#if feed.length === 0}
            <p class="empty">{$t('profileEmpty')}</p>
          {:else}
            <ul class="feed">
              {#each feed as item (item.kind + item.id)}
                <li class="feed-row feed-row--{item.tone}" class:feed-row--unread={item.unread > 0}>
                  <div class="feed-meta">
                    <span class="feed-kind">{feedKindLabel(item.kind)}</span>
                    <span class="feed-date">{formatDate(item.date)}</span>
                  </div>
                  <div class="feed-main">
                    {#if item.figurineId}
                      <a href="/figurines/{item.figurineId}" class="feed-title feed-title--link">{item.title || $t('commissionUntitled')}</a>
                    {:else}
                      <span class="feed-title">{item.title || $t('commissionUntitled')}</span>
                    {/if}
                  </div>
                  <div class="feed-side">
                    {#if item.unread > 0}
                      <button class="feed-reply" onclick={() => openFeedThread(item)}>{$t('profileOverviewNewReply')} →</button>
                    {/if}
                    <span class="feed-status feed-status--{item.tone}">{feedStatusLabel(item)}</span>
                  </div>
                </li>
              {/each}
            </ul>
          {/if}
        {:else if activeTab === 'bookings'}
          {#if bookings.length === 0}
            <p class="empty">{$t('profileEmpty')}</p>
          {:else}
            <div class="cards-grid">
              {#each bookings as b}
                <div class="card">
                  <div class="card-head">
                    <a href="/figurines/{b.figurineId}" class="card-name card-name--link">{b.figurineName}</a>
                    <span class="status status--{b.status}">{bookingStatusLabel(b.status)}</span>
                  </div>
                  <p class="card-range">{formatDateRange(b.startsAt, b.endsAt)}</p>
                  {#if b.curatorConditions}
                    <div class="card-curator">
                      <span class="card-curator-label">{$t('profileBookingCuratorConditions')}</span>
                      <p class="card-curator-text">{b.curatorConditions}</p>
                    </div>
                  {/if}
                  <div class="card-footer">
                    <p class="card-date">{formatDate(b.createdAt)}</p>
                    <a href="/cancel/{b.cancelToken}" class="card-manage">{$t('profileBookingManage')}</a>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {:else if activeTab === 'orders'}
          {#if orders.length === 0}
            <p class="empty">{$t('profileEmpty')}</p>
          {:else}
            <div class="cards-grid">
              {#each orders as o}
                <a href="/figurines/{o.figurineId}" class="card">
                  <div class="card-head">
                    <span class="card-name">{o.figurineName}</span>
                    <div class="order-badges">
                      <span class="mode">{orderModeLabel(o.mode)}</span>
                      <span class="order-status order-status--{o.status}">{orderStatusLabel(o.status)}</span>
                    </div>
                  </div>
                  <p class="card-date">{formatDate(o.createdAt)}</p>
                </a>
              {/each}
            </div>
          {/if}
        {:else if activeTab === 'commissions'}
          {#if commissions.length === 0}
            <div class="empty-with-cta">
              <p class="empty">{$t('profileCommissionsEmpty')}</p>
              <a href="/commission" class="commission-cta">{$t('profileCommissionsNew')}</a>
            </div>
          {:else}
            <div class="cards-grid">
              {#each commissions as c}
                <div class="card commission-card">
                  <div class="card-head">
                    <span class="card-name">{c.title || $t('commissionUntitled')}</span>
                    <span class="order-status order-status--{c.status === 'declined' ? 'rejected' : c.status === 'completed' ? 'confirmed' : 'pending'}">{COMMISSION_STATUS_LABEL[c.status] ?? c.status}</span>
                  </div>
                  <p class="card-desc">{c.description}</p>
                  {#if c.attachments.length > 0}
                    <div class="commission-thumbs">
                      {#each c.attachments as att (att.id)}
                        <img src={resolveMediaUrl(att.thumbUrl ?? att.url)} alt="" />
                      {/each}
                    </div>
                  {/if}
                  <div class="card-foot">
                    <span class="card-date">{formatDate(c.createdAt)}</span>
                    {#if c.threadId}
                      <button class="commission-open" onclick={() => { activeTab = 'messages'; openThreadDetail(c.threadId!); }}>{$t('profileCommissionsOpenChat')} →</button>
                    {/if}
                  </div>
                  <div class="commission-manage">
                    {#if c.started}
                      <span class="commission-locked">{$t('profileCommissionsLocked')}</span>
                    {:else if confirmDeleteId === c.id}
                      <span class="commission-confirm">{$t('profileCommissionsDeleteConfirm')}</span>
                      <button class="commission-link danger" onclick={() => removeCommission(c)} disabled={deletingId === c.id}>{deletingId === c.id ? '…' : $t('profileCommissionsDeleteYes')}</button>
                      <button class="commission-link" onclick={() => confirmDeleteId = null}>{$t('commissionBack')}</button>
                    {:else}
                      <button class="commission-link" onclick={() => editingCommission = c}>{$t('profileCommissionsEdit')}</button>
                      <button class="commission-link danger" onclick={() => confirmDeleteId = c.id}>{$t('profileCommissionsDelete')}</button>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
            <a href="/commission" class="commission-cta commission-cta--inline">{$t('profileCommissionsNew')}</a>
          {/if}
        {:else if activeTab === 'wishlist'}
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
        {:else if activeTab === 'messages'}
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
        {/if}
      </div>

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

  /* ── Body layout ── */

  /* ── Sidebar profile (hero) ── */

  .sidebar-profile {
    background: #2a1a10;
    padding: 1.75rem 1.25rem 1.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    text-align: center;
  }

  .hero-avatar-zone { position: relative; }

  .hero-avatar-btn {
    width: 70px;
    height: 70px;
    border-radius: 50%;
    border: 2px solid rgba(248,241,231,0.2);
    background: rgba(248,241,231,0.06);
    cursor: pointer;
    overflow: hidden;
    position: relative;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: border-color 0.2s;
  }
  .hero-avatar-btn:hover { border-color: rgba(198,95,60,0.6); }
  .hero-avatar-btn:disabled { cursor: default; opacity: 0.7; }

  .hero-avatar-img { width: 100%; height: 100%; object-fit: cover; }

  .hero-avatar-initials {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.6rem;
    color: rgba(248,241,231,0.55);
    line-height: 1;
    pointer-events: none;
  }

  .hero-avatar-overlay {
    position: absolute;
    inset: 0;
    background: rgba(42,26,16,0.72);
    color: #f8f1e7;
    font-family: Inter, sans-serif;
    font-size: 0.55rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 4px;
    opacity: 0;
    transition: opacity 0.18s;
    pointer-events: none;
  }
  .hero-avatar-btn:hover .hero-avatar-overlay { opacity: 1; }
  .hero-avatar-btn:disabled .hero-avatar-overlay { opacity: 1; }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }

  .hero-identity { text-align: center; }

  .hero-name-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    margin-bottom: 0.25rem;
  }

  .hero-name {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.35rem;
    font-weight: 400;
    color: #f8f1e7;
    margin: 0;
    line-height: 1.2;
  }

  .hero-edit-btn {
    background: transparent;
    border: none;
    color: rgba(248,241,231,0.3);
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
    transition: color 0.15s;
    flex-shrink: 0;
  }
  .hero-edit-btn:hover { color: #c65f3c; }

  .hero-name-saved {
    display: block;
    font-family: Inter, sans-serif;
    font-size: 0.62rem;
    color: #6a9e5a;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    text-align: center;
    margin-bottom: 0.2rem;
  }

  .hero-name-edit {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    margin-bottom: 0.25rem;
  }

  .hero-name-input {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.1rem;
    background: transparent;
    border: none;
    border-bottom: 1.5px solid rgba(198,95,60,0.65);
    color: #f8f1e7;
    padding: 2px 0;
    outline: none;
    text-align: center;
    width: 180px;
  }

  .hero-name-save {
    background: transparent;
    border: 1px solid rgba(198,95,60,0.55);
    color: rgba(198,95,60,0.9);
    font-family: Inter, sans-serif;
    font-size: 0.67rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 2px 8px;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.15s;
  }
  .hero-name-save:hover:not(:disabled) { background: rgba(198,95,60,0.12); }
  .hero-name-save:disabled { opacity: 0.5; }

  .hero-name-cancel {
    background: transparent;
    border: none;
    color: rgba(248,241,231,0.35);
    cursor: pointer;
    font-size: 0.85rem;
    padding: 2px 4px;
    line-height: 1;
    flex-shrink: 0;
    transition: color 0.15s;
  }
  .hero-name-cancel:hover { color: #f8f1e7; }

  .hero-email {
    font-family: Inter, sans-serif;
    font-size: 0.79rem;
    color: rgba(200,168,130,0.75);
    margin: 0 0 0.12rem;
  }

  .hero-since {
    font-family: Inter, sans-serif;
    font-size: 0.67rem;
    color: rgba(200,168,130,0.45);
    margin: 0;
    letter-spacing: 0.02em;
  }

  /* ── Body layout ── */

  .body {
    display: flex;
    align-items: flex-start;
    min-height: calc(100vh - 68px);
  }

  /* ── Sidebar ── */

  .sidebar {
    position: fixed;
    left: 0;
    top: 68px;
    height: calc(100vh - 68px);
    width: 220px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    border-right: 1px solid rgba(52,37,28,0.12);
    z-index: 5;
  }

  @media (max-width: 680px) {
    .sidebar {
      position: static;
      width: 100%;
      height: auto;
      z-index: auto;
    }
    .body {
      flex-direction: column;
      height: auto;
      min-height: calc(100vh - 58px);
    }
    .content {
      margin-left: 0;
    }
  }

  .sidebar-nav {
    flex: 1;
    padding: 0.85rem 0;
    display: flex;
    flex-direction: column;
    background: #f0e6d6;
  }

  .nav-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.6rem 1.25rem;
    background: transparent;
    border: none;
    border-left: 2px solid transparent;
    text-align: left;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #9a7c5c;
    cursor: pointer;
    transition: all 0.15s;
  }
  .nav-item:hover { background: rgba(52,37,28,0.05); color: #34251c; }
  .nav-item.active { color: #c65f3c; background: rgba(198,95,60,0.07); border-left-color: #c65f3c; }

  .nav-badge {
    background: #d8c6b1;
    color: #6f3b24;
    font-family: Inter, sans-serif;
    font-size: 0.6rem;
    border-radius: 10px;
    padding: 1px 6px;
    min-width: 18px;
    text-align: center;
    flex-shrink: 0;
  }
  .nav-badge--unread { background: #c65f3c; color: #fff; }

  .sidebar-footer {
    padding: 1rem 1.25rem;
    border-top: 1px solid rgba(52,37,28,0.1);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    background: #f0e6d6;
  }

  .sidebar-logout {
    background: transparent;
    border: none;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    font-size: 0.7rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    cursor: pointer;
    padding: 0.25rem 0;
    text-align: left;
    transition: color 0.15s;
  }
  .sidebar-logout:hover { color: #34251c; }

  .sidebar-delete-btn {
    background: transparent;
    border: none;
    color: #c8b89a;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    cursor: pointer;
    padding: 0;
    text-align: left;
    transition: color 0.15s;
  }
  .sidebar-delete-btn:hover { color: #c65f3c; }

  .sidebar-delete-confirm {
    padding: 0.65rem 0.75rem;
    background: #fdf3f3;
    border: 1px solid #f0d0c8;
  }

  .sidebar-delete-warning {
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    color: #7a3020;
    margin: 0 0 0.6rem;
    line-height: 1.45;
  }

  .sidebar-delete-actions {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .sidebar-delete-yes {
    background: #9b2020;
    border: none;
    color: #fff;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.35rem 0.75rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .sidebar-delete-yes:hover:not(:disabled) { background: #7a1818; }
  .sidebar-delete-yes:disabled { opacity: 0.6; cursor: not-allowed; }

  .sidebar-delete-cancel {
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.3rem 0.75rem;
    cursor: pointer;
    transition: all 0.15s;
    text-align: center;
  }
  .sidebar-delete-cancel:hover { border-color: #9a7c5c; color: #34251c; }

  /* ── Content ── */

  .content {
    flex: 1;
    margin-left: 220px;
    background: #f8f1e7;
    padding: 2rem 2.5rem;
    min-height: 400px;
  }

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
  .claim-link {
    margin-bottom: 1.75rem;
    padding: 1rem 1.1rem 1.1rem;
    border: 1px solid #e4d8c8;
    border-left: 2px solid rgba(198,95,60,0.55);
    background: rgba(255,252,246,0.6);
  }

  .claim-link-head { margin-bottom: 0.7rem; }

  .claim-link-title {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1rem;
    font-weight: 400;
    color: #34251c;
    margin: 0 0 0.2rem;
  }

  .claim-link-hint {
    font-family: Inter, sans-serif;
    font-size: 0.76rem;
    line-height: 1.5;
    color: #8a7253;
    margin: 0;
    max-width: 52ch;
  }

  .claim-link-form {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .claim-link-input {
    flex: 1;
    min-width: 180px;
    font-family: 'Courier New', monospace;
    font-size: 0.82rem;
    letter-spacing: 0.04em;
    background: #fffaf4;
    border: 1px solid #d8c6b1;
    color: #34251c;
    padding: 0.5rem 0.7rem;
    outline: none;
    transition: border-color 0.15s;
  }
  .claim-link-input::placeholder { color: #b5a090; font-family: Inter, sans-serif; letter-spacing: 0; }
  .claim-link-input:focus { border-color: #c65f3c; }

  .claim-link-btn {
    background: #6f3b24;
    border: none;
    color: #f8f1e7;
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    padding: 0.5rem 1.1rem;
    cursor: pointer;
    transition: background 0.15s;
    white-space: nowrap;
  }
  .claim-link-btn:hover:not(:disabled) { background: #c65f3c; }
  .claim-link-btn:disabled { opacity: 0.45; cursor: default; }

  .claim-link-msg {
    margin: 0.65rem 0 0;
    font-family: Inter, sans-serif;
    font-size: 0.78rem;
    line-height: 1.45;
  }
  .claim-link-msg--ok   { color: #2d6a3f; }
  .claim-link-msg--warn { color: #8a5a1a; }
  .claim-link-msg--err  { color: #a03020; }

  .claim-link-kind {
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

  .claim-link-name { font-style: italic; }

  /* ── Overview feed (quiet ledger across all request types) ── */
  .feed {
    list-style: none;
    margin: 0;
    padding: 0;
    max-width: 760px;
  }

  .feed-row {
    display: grid;
    grid-template-columns: 150px 1fr auto;
    align-items: center;
    gap: 1rem;
    padding: 0.8rem 0.25rem 0.8rem 0.9rem;
    border-bottom: 1px solid #eadfce;
    border-left: 2px solid transparent;
  }
  .feed-row:first-child { border-top: 1px solid #eadfce; }
  .feed-row--unread { border-left-color: #c65f3c; background: rgba(198,95,60,0.03); }

  .feed-meta {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .feed-kind {
    font-family: Inter, sans-serif;
    font-size: 0.6rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #9a7c5c;
  }
  .feed-date {
    font-family: Inter, sans-serif;
    font-size: 0.7rem;
    color: #b5a090;
  }

  .feed-main { min-width: 0; }
  .feed-title {
    font-family: Georgia, serif;
    font-size: 0.95rem;
    color: #34251c;
    text-decoration: none;
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .feed-title--link:hover { color: #c65f3c; }

  .feed-side {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    flex-shrink: 0;
  }

  .feed-reply {
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    font-family: Inter, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.04em;
    color: #c65f3c;
    white-space: nowrap;
  }
  .feed-reply:hover { text-decoration: underline; }

  .feed-status {
    font-family: Inter, sans-serif;
    font-size: 0.62rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    padding: 2px 7px;
    border-radius: 2px;
    white-space: nowrap;
  }
  .feed-status--pending   { background: #f4ead8; color: #6f3b24; }
  .feed-status--confirmed { background: #e8f4e8; color: #2d6a3f; }
  .feed-status--rejected  { background: #fde8e8; color: #9b2020; }
  .feed-status--attention { background: #eef0fb; color: #3a4a8a; }
  .feed-status--neutral   { background: #ececec; color: #666; }

  @media (max-width: 680px) {
    .feed-row {
      grid-template-columns: 1fr auto;
      grid-template-areas: "meta side" "main main";
      gap: 0.4rem 1rem;
    }
    .feed-meta { grid-area: meta; flex-direction: row; align-items: baseline; gap: 0.5rem; }
    .feed-main { grid-area: main; }
    .feed-side { grid-area: side; }
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
    background: #fff;
    border: 1px solid #e4d8c8;
    padding: 1rem 1.1rem;
    text-decoration: none;
    color: #34251c;
    transition: border-color 0.18s, box-shadow 0.18s, transform 0.15s;
  }
  .card:hover {
    border-color: #c65f3c;
    box-shadow: 0 2px 10px rgba(198,95,60,0.1);
    transform: translateY(-1px);
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
    margin-left: -2.5rem;
    margin-right: -2.5rem;
    padding-left: 2.5rem;
    padding-right: 2.5rem;
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
