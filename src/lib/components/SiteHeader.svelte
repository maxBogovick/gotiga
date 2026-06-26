<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import LangSwitcher from '$lib/components/LangSwitcher.svelte';
  import FontSwitcher from '$lib/components/FontSwitcher.svelte';
  import { allClaims } from '$lib/stores/all-claims.svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
  import { t, brandName } from '$lib/i18n';
  import { fade, fly } from 'svelte/transition';
  import { api, resolveMediaUrl } from '$lib/api';
  import RavenWatcher from '$lib/components/RavenWatcher.svelte';
  import HeaderBirdWalk from '$lib/components/HeaderBirdWalk.svelte';

  let leftLinks = $derived([
    { href: '/figurines', label: $t('navArchive') },
    { href: '/upcoming', label: $t('navUpcoming') },
  ]);

  let rightLinks = $derived([
    { href: '/workshop', label: $t('navWorkshop') },
    { href: '/author',   label: $t('navAuthor') },
  ]);

  let pathname = $derived(page.url.pathname);
  let panelOpen = $state(false);
  let panelRef = $state<HTMLElement | null>(null);
  let mobileNavOpen = $state(false);
  let isScrolled = $state(false);

  function isActive(href: string) {
    return pathname === href || pathname.startsWith(`${href}/`);
  }

  function toggleMobileNav() { mobileNavOpen = !mobileNavOpen; }
  function closeMobileNav() { mobileNavOpen = false; }
  function togglePanel() { panelOpen = !panelOpen; }
  function closePanel()  { panelOpen = false; }

  function handleScroll() {
    isScrolled = window.scrollY > 48;
  }

  // Keep --site-header-h in sync so sticky sub-headers on detail/archive pages
  // track the main header height without JS coupling between components.
  $effect(() => {
    if (typeof document === 'undefined') return;
    document.documentElement.style.setProperty(
      '--site-header-h',
      isScrolled ? '54px' : '78px'
    );
  });

  function fmtDate(iso: string) {
    return new Date(iso).toLocaleDateString(undefined, { day: '2-digit', month: 'short' });
  }

  function handleOutside(e: MouseEvent) {
    if (panelOpen && panelRef && !panelRef.contains(e.target as Node)) {
      panelOpen = false;
    }
  }

  let count = $derived(allClaims.pendingCount);

  let userMenuOpen = $state(false);
  let userMenuRef = $state<HTMLElement | null>(null);

  function toggleUserMenu() { userMenuOpen = !userMenuOpen; }

  function handleUserOutside(e: MouseEvent) {
    if (userMenuOpen && userMenuRef && !userMenuRef.contains(e.target as Node)) {
      userMenuOpen = false;
    }
  }

  async function handleLogout() {
    userMenuOpen = false;
    mobileNavOpen = false;
    const token = authStore.token;
    if (token) {
      try { await api.userLogout(token); } catch { /* ok */ }
    }
    authStore.logout();
    goto('/');
  }

  let avatarUrl = $derived(resolveMediaUrl(authStore.user?.avatarUrl));

  onMount(async () => {
    allClaims.load();
    allClaims.verify();
    allClaims.startPolling();
    document.addEventListener('click', handleOutside, { capture: true });
    document.addEventListener('click', handleUserOutside, { capture: true });
    window.addEventListener('scroll', handleScroll, { passive: true });
    handleScroll();

    if (!authStore.isLoggedIn && authStore.token) {
      try {
        const user = await api.userMe(authStore.token);
        authStore.user = user;
      } catch {
        authStore.clearSession();
      }
    }

    if (authStore.token) {
      savedFigurines.load();
      savedFigurines.syncWithServer({ importLocal: false });
    }
  });

  onDestroy(() => {
    allClaims.stopPolling();
    if (typeof document !== 'undefined') {
      document.removeEventListener('click', handleOutside, { capture: true });
      document.removeEventListener('click', handleUserOutside, { capture: true });
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('scroll', handleScroll);
    }
  });

  $effect(() => {
    void pathname;
    mobileNavOpen = false;
  });
</script>

<header class="site-header" class:is-scrolled={isScrolled}>
  <HeaderBirdWalk />
  <div class="header-inner">

    <!-- ① Ghost-left: muted utilities pinned to left edge -->
    <div class="ghost-left">
      <LangSwitcher variant="light" />
      <FontSwitcher variant="header" />
    </div>

    <!-- ② Left nav -->
    <nav class="nav-side nav-left" aria-label="Primary left">
      {#each leftLinks as link}
        <a
          href={link.href}
          class="nav-link"
          class:is-active={isActive(link.href)}
          aria-current={isActive(link.href) ? 'page' : undefined}
        >{link.label}</a>
      {/each}
    </nav>

    <!-- ③ Brand: proscenium centerpiece -->
    <a href="/" class="brand" aria-label={$brandName}>
      <span class="brand-inner">
        <RavenWatcher />
        <span class="brand-name">{$brandName}</span>
      </span>
      <span class="brand-sub">Cabinet of Gothic Miniatures</span>
    </a>

    <!-- ④ Right nav -->
    <nav class="nav-side nav-right" aria-label="Primary right">
      {#each rightLinks as link}
        <a
          href={link.href}
          class="nav-link"
          class:is-active={isActive(link.href)}
          aria-current={isActive(link.href) ? 'page' : undefined}
        >{link.label}</a>
      {/each}
    </nav>

    <!-- ⑤ Ghost-right: muted utilities pinned to right edge -->
    <div class="ghost-right">
      {#if allClaims.claims.length > 0}
      <div class="bookings-anchor" bind:this={panelRef}>
        <button
          class="bookings-btn"
          class:has-claims={count > 0}
          class:is-open={panelOpen}
          onclick={togglePanel}
          aria-label={$t('bookingsHeaderTitle')}
          title={$t('bookingsHeaderTitle')}
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <rect x="1" y="1" width="12" height="12" rx="1" stroke="currentColor" stroke-width="1"/>
            <path d="M4 5h6M4 7.5h6M4 10h4" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
          </svg>
          {#if count > 0}
            <span class="badge">{count}</span>
          {/if}
        </button>

        {#if panelOpen}
          <div class="bookings-panel" transition:fade={{ duration: 150 }}>
            <div class="panel-head">
              <span class="panel-title">{$t('bookingsHeaderTitle')}</span>
              <button class="panel-close" onclick={closePanel} aria-label="Close">✕</button>
            </div>

            {#if allClaims.claims.length === 0}
              <div class="panel-empty">
                <p>{$t('bookingsEmpty')}</p>
              </div>
            {:else}
              <ul class="panel-list">
                {#each allClaims.claims as c (c.token)}
                  <li class="panel-item">
                    <div class="panel-item-top">
                      <a
                        href="/figurines/{c.figurineId}"
                        class="panel-figurine-name"
                        onclick={closePanel}
                      >{c.figurineName}</a>
                      <span class="panel-status panel-status--{c.status ?? 'pending'}">
                        {c.status === 'confirmed' ? $t('bookingsConfirmed')
                        : c.status === 'rejected'  ? $t('bookingsRejected')
                        : c.status === 'completed' ? $t('bookingsCompleted')
                        : $t('bookingsPending')}
                      </span>
                    </div>
                    <p class="panel-dates">{fmtDate(c.startsAt)} — {fmtDate(c.endsAt)}</p>
                    {#if !c.status || c.status === 'pending'}
                      <button
                        class="panel-cancel-btn"
                        onclick={() => allClaims.cancel(c)}
                        disabled={allClaims.cancellingToken === c.token}
                      >
                        {allClaims.cancellingToken === c.token ? $t('claimCancelling') : $t('claimCancelBtn')}
                      </button>
                      {#if allClaims.errors[c.token]}
                        <p class="panel-err">{allClaims.errors[c.token]}</p>
                      {/if}
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}

            <div class="panel-footer">
              <a href={authStore.isLoggedIn ? '/profile' : '/bookings'} class="panel-view-all" onclick={closePanel}>
                {$t('bookingsViewAll')} →
              </a>
            </div>
          </div>
        {/if}
      </div>
      {/if}

      <div class="user-anchor" bind:this={userMenuRef}>
        <button
          class="user-btn"
          class:logged-in={authStore.isLoggedIn}
          class:is-open={userMenuOpen}
          onclick={authStore.isLoggedIn ? toggleUserMenu : () => goto('/login')}
          aria-label={authStore.isLoggedIn ? authStore.user?.displayName : $t('authLogin')}
          title={authStore.isLoggedIn ? authStore.user?.displayName : $t('authLogin')}
        >
          {#if authStore.isLoggedIn}
            {#if avatarUrl}
              <img src={avatarUrl} alt="" class="user-avatar" />
            {:else}
              <span class="user-initial">{(authStore.user?.displayName ?? '?')[0].toUpperCase()}</span>
            {/if}
          {:else}
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
              <circle cx="7" cy="4.5" r="2.5" stroke="currentColor" stroke-width="1"/>
              <path d="M1.5 13c0-3 2.5-4.5 5.5-4.5S12 10 12 13" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
            </svg>
          {/if}
        </button>

        {#if userMenuOpen && authStore.isLoggedIn}
          <div class="user-panel" transition:fade={{ duration: 150 }}>
            <div class="user-panel-head">
              {#if avatarUrl}
                <img src={avatarUrl} alt="" class="user-panel-avatar" />
              {:else}
                <span class="user-panel-initial">{(authStore.user?.displayName ?? '?')[0].toUpperCase()}</span>
              {/if}
              <div class="user-panel-info">
                <span class="user-panel-name">{authStore.user?.displayName}</span>
                <span class="user-panel-email">{authStore.user?.email}</span>
              </div>
            </div>
            <a href="/profile" class="user-panel-link" onclick={() => userMenuOpen = false}>
              {$t('profileTitle')} →
            </a>
            <button class="user-panel-logout" onclick={handleLogout}>
              {$t('profileLogout')}
            </button>
          </div>
        {/if}
      </div>
    </div>

    <!-- ⑥ Mobile-only controls -->
    <div class="mobile-controls">
      <button
        class="mobile-user-btn"
        onclick={authStore.isLoggedIn ? () => { closeMobileNav(); goto('/profile'); } : () => goto('/login')}
        aria-label={authStore.isLoggedIn ? authStore.user?.displayName : $t('authLogin')}
      >
        {#if authStore.isLoggedIn}
          {#if avatarUrl}
            <img src={avatarUrl} alt="" class="user-avatar" />
          {:else}
            <span class="user-initial">{(authStore.user?.displayName ?? '?')[0].toUpperCase()}</span>
          {/if}
        {:else}
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <circle cx="7" cy="4.5" r="2.5" stroke="currentColor" stroke-width="1"/>
            <path d="M1.5 13c0-3 2.5-4.5 5.5-4.5S12 10 12 13" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
          </svg>
        {/if}
      </button>
      <button
        class="mobile-menu-btn"
        class:is-open={mobileNavOpen}
        type="button"
        onclick={toggleMobileNav}
        aria-label={$t('navMenu')}
        aria-expanded={mobileNavOpen}
        aria-controls="site-mobile-nav"
      >
        <span></span>
        <span></span>
        <span></span>
      </button>
    </div>

  </div>

  <!-- Mobile nav dropdown -->
  <nav id="site-mobile-nav" class="mobile-nav" class:is-open={mobileNavOpen} aria-label="Primary">
    {#each [...leftLinks, ...rightLinks] as link}
      <a
        href={link.href}
        class="mobile-nav-link"
        class:is-active={isActive(link.href)}
        aria-current={isActive(link.href) ? 'page' : undefined}
        onclick={closeMobileNav}
      >{link.label}</a>
    {/each}
    <div class="mobile-nav-footer">
      <LangSwitcher variant="light" />
      <FontSwitcher variant="header" />
    </div>
  </nav>
</header>

<!-- Status change notifications -->
<div class="notif-stack" aria-live="polite">
  {#each allClaims.notifications as notif (notif.id)}
    <div
      class="notif notif--{notif.newStatus}"
      role="status"
      in:fly={{ x: 60, duration: 350 }}
      out:fly={{ x: 60, duration: 250 }}
    >
      <div class="notif-body">
        <span class="notif-name">{notif.figurineName}</span>
        <span class="notif-text">
          {notif.newStatus === 'confirmed' ? $t('notifConfirmed')
          : notif.newStatus === 'rejected'  ? $t('notifRejected')
          : notif.newStatus === 'cancelled' ? $t('notifCancelled')
          : $t('notifCompleted')}
        </span>
      </div>
      <div class="notif-actions">
        {#if notif.newStatus === 'rejected'}
          <a href="/figurines/{notif.figurineId}" class="notif-link">{$t('notifTryAgain')}</a>
        {:else if notif.newStatus === 'completed'}
          <a href="/figurines/{notif.figurineId}" class="notif-link">{$t('notifViewFigurine')}</a>
        {/if}
        <button class="notif-dismiss" onclick={() => allClaims.dismissNotification(notif.id)} aria-label="Dismiss">✕</button>
      </div>
    </div>
  {/each}
</div>

<style>
  /* ── Tokens ─────────────────────────────────────────────────── */
  .site-header {
    --ink:    var(--color-ink-primary);
    --mid:    var(--color-ember-deep);
    --copper: var(--color-ember);
    --border: color-mix(in srgb, var(--color-ink-primary) 10%, transparent);
    --ease:   cubic-bezier(0.16, 1, 0.3, 1);

    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    /* tallest state: room for brand name row + subtitle */
    height: 78px;
    background: rgba(248, 241, 231, 0.97);
    border-bottom: 1px solid var(--border);
    z-index: 200;
    transition: height 0.4s var(--ease), background 0.3s ease;
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
  }

  .site-header.is-scrolled {
    height: 54px;
    background: rgba(248, 241, 231, 0.99);
  }

  /* ── 5-column proscenium grid ────────────────────────────────
     ghost-l | nav-l | brand | nav-r | ghost-r
     The two outer minmax(0,1fr) columns absorb remaining space
     symmetrically so the brand stays perfectly centered even on
     ultrawide monitors. max-width caps at 1380px so nav links
     never fly too far from the raven.
  ──────────────────────────────────────────────────────────── */
  .header-inner {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto auto minmax(0, 1fr);
    grid-template-rows: 1fr;
    /* stretch so nav-links can fill the full header height */
    align-items: stretch;
    width: 100%;
    max-width: 1380px;
    margin: 0 auto;
    padding: 0 clamp(20px, 4vw, 52px);
    height: 100%;
  }

  /* ── Ghost zones ──────────────────────────────────────────── */
  .ghost-left,
  .ghost-right {
    display: flex;
    align-items: center;
    gap: 4px;
    /* muted until hovered — they shouldn't compete with nav */
    opacity: 0.42;
    transition: opacity 0.3s ease;
  }

  .ghost-left:hover,
  .ghost-right:hover,
  .ghost-left:focus-within,
  .ghost-right:focus-within {
    opacity: 0.78;
  }

  .ghost-left  { grid-column: 1; justify-content: flex-start; }
  .ghost-right { grid-column: 5; justify-content: flex-end; }

  /* bookings & user sit at normal opacity inside ghost-right
     when they have active state */
  .ghost-right:has(.bookings-btn.has-claims),
  .ghost-right:has(.user-btn.logged-in) {
    opacity: 0.68;
  }

  /* ── Nav sides ────────────────────────────────────────────── */
  .nav-side {
    display: flex;
    align-items: stretch;
  }

  .nav-left  { grid-column: 2; justify-content: flex-end;   padding-right: 4px; }
  .nav-right { grid-column: 4; justify-content: flex-start; padding-left: 4px; }

  .nav-link {
    position: relative;
    display: flex;
    align-items: center;
    padding: 0 17px;
    font-family: var(--font-body);
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.10em;
    text-transform: uppercase;
    color: var(--color-ink-secondary);
    text-decoration: none;
    transition: color 0.25s;
    overflow: hidden;
  }

  /* copper underline slides in from left */
  .nav-link::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 19px;
    right: 19px;
    height: 1px;
    background: var(--copper);
    transform: scaleX(0);
    transform-origin: left;
    transition: transform 0.35s var(--ease);
  }

  .nav-link:hover,
  .nav-link.is-active {
    color: var(--ink);
  }

  .nav-link:hover::after,
  .nav-link.is-active::after {
    transform: scaleX(1);
  }

  /* ── Brand: theatrical centerpiece ───────────────────────── */
  .brand {
    grid-column: 3;
    align-self: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    padding: 0 clamp(14px, 2.2vw, 34px);
    text-decoration: none;
    color: inherit;
    transition: opacity 0.3s;
  }

  .brand:hover { opacity: 0.82; }

  /* inner row: emblem + wordmark side by side */
  .brand-inner {
    display: flex;
    align-items: center;
    gap: 11px;
  }


  .brand-name {
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 400;
    letter-spacing: 0.3em;
    text-transform: uppercase;
    color: var(--ink);
    line-height: 1;
  }

  /* subtitle: fades out on scroll */
  .brand-sub {
    font-family: var(--font-body);
    font-size: 9px;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
    line-height: 1;
    white-space: nowrap;
    /* scroll transition */
    opacity: 1;
    max-height: 14px;
    transform: translateY(0);
    transition:
      opacity 0.38s var(--ease),
      transform 0.38s var(--ease),
      max-height 0.38s var(--ease);
  }

  .site-header.is-scrolled .brand-sub {
    opacity: 0;
    max-height: 0;
    transform: translateY(-5px);
    pointer-events: none;
  }

  /* ── Mobile controls + nav (hidden on desktop) ───────────── */
  .mobile-controls { display: none; }
  .mobile-nav { display: none; }

  /* ── Bookings button ─────────────────────────────────────── */
  .bookings-anchor { position: relative; }

  .bookings-btn {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    color: var(--color-ink-tertiary);
    transition: color 0.25s;
  }

  .bookings-btn:hover,
  .bookings-btn.is-open { color: var(--mid); }

  .bookings-btn.has-claims { color: var(--copper); }

  .badge {
    position: absolute;
    top: -2px;
    right: -2px;
    min-width: 13px;
    height: 13px;
    padding: 0 3px;
    background: var(--copper);
    color: #f8f1e7;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 7.5px;
    font-weight: 600;
    border-radius: 7px;
    display: flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
  }

  /* ── Bookings dropdown panel ─────────────────────────────── */
  .bookings-panel {
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    width: 300px;
    background: #f2e8d9;
    border: 1px solid #d8c6b1;
    box-shadow: 0 8px 32px rgba(52, 37, 28, 0.12);
    z-index: 300;
    font-family: Georgia, serif;
    color: #34251c;
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px 10px;
    border-bottom: 1px solid rgba(52, 37, 28, 0.08);
  }

  .panel-title {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 9px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: rgba(95, 70, 54, 0.6);
  }

  .panel-close {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 11px;
    color: rgba(95, 70, 54, 0.4);
    padding: 2px 4px;
    line-height: 1;
    transition: color 0.2s;
  }
  .panel-close:hover { color: #c65f3c; }

  .panel-empty {
    padding: 20px 16px;
    text-align: center;
    font-size: 0.8rem;
    color: rgba(95, 70, 54, 0.5);
    font-style: italic;
  }

  .panel-list {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 320px;
    overflow-y: auto;
  }

  .panel-item {
    padding: 10px 16px;
    border-bottom: 1px solid rgba(52, 37, 28, 0.06);
  }
  .panel-item:last-child { border-bottom: none; }

  .panel-item-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 3px;
  }

  .panel-figurine-name {
    font-size: 0.82rem;
    font-weight: 500;
    color: #34251c;
    text-decoration: none;
    line-height: 1.3;
    flex: 1;
    transition: color 0.2s;
  }
  .panel-figurine-name:hover { color: #c65f3c; }

  .panel-status {
    flex-shrink: 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 7.5px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 2px 6px;
    border-radius: 2px;
  }
  .panel-status--pending   { background: #f5e6c8; color: #7a5520; }
  .panel-status--confirmed { background: #d4e8c8; color: #3a6020; }

  .panel-dates {
    margin: 0 0 6px;
    font-size: 0.72rem;
    color: rgba(95, 70, 54, 0.6);
    font-style: italic;
  }

  .panel-cancel-btn {
    background: none;
    border: 1px solid rgba(198, 95, 60, 0.4);
    color: #c65f3c;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 8px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 3px 8px;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }
  .panel-cancel-btn:hover:not(:disabled) { background: rgba(198, 95, 60, 0.08); border-color: #c65f3c; }
  .panel-cancel-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .panel-err {
    margin: 4px 0 0;
    font-size: 0.7rem;
    color: #a03020;
    font-style: italic;
  }

  .panel-footer {
    padding: 10px 16px;
    border-top: 1px solid rgba(52, 37, 28, 0.08);
    text-align: right;
  }

  .panel-view-all {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 8.5px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: rgba(95, 70, 54, 0.55);
    text-decoration: none;
    transition: color 0.2s;
  }
  .panel-view-all:hover { color: #c65f3c; }

  /* ── User button ─────────────────────────────────────────── */
  .user-anchor { position: relative; }

  .user-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: none;
    border: 1px solid transparent;
    border-radius: 50%;
    padding: 0;
    cursor: pointer;
    color: var(--color-ink-tertiary);
    transition: color 0.25s, border-color 0.25s;
  }

  .user-btn:hover,
  .user-btn.is-open { color: var(--mid); }

  .user-btn.logged-in {
    border-color: var(--border);
    color: var(--mid);
  }
  .user-btn.logged-in:hover,
  .user-btn.logged-in.is-open {
    border-color: var(--copper);
    color: var(--copper);
  }

  .user-avatar {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 50%;
    display: block;
  }

  .user-initial {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 12px;
    font-weight: 400;
    line-height: 1;
  }

  .user-panel {
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    width: 220px;
    background: #f2e8d9;
    border: 1px solid #d8c6b1;
    box-shadow: 0 8px 32px rgba(52, 37, 28, 0.12);
    z-index: 300;
    font-family: Georgia, serif;
    color: #34251c;
  }

  .user-panel-head {
    padding: 12px 14px 10px;
    border-bottom: 1px solid rgba(52, 37, 28, 0.08);
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .user-panel-avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
    border: 1px solid rgba(52, 37, 28, 0.10);
  }

  .user-panel-initial {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: #efe6d6;
    border: 1px solid rgba(52, 37, 28, 0.10);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1rem;
    color: #9a7c5c;
  }

  .user-panel-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .user-panel-name {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 0.85rem;
    color: #34251c;
    line-height: 1.2;
  }

  .user-panel-email {
    font-family: Inter, sans-serif;
    font-size: 0.7rem;
    color: rgba(95, 70, 54, 0.55);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .user-panel-link {
    display: block;
    padding: 10px 14px;
    font-family: Inter, sans-serif;
    font-size: 0.78rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: rgba(95, 70, 54, 0.7);
    text-decoration: none;
    border-bottom: 1px solid rgba(52, 37, 28, 0.06);
    transition: color 0.2s;
  }
  .user-panel-link:hover { color: #c65f3c; }

  .user-panel-logout {
    display: block;
    width: 100%;
    padding: 10px 14px;
    background: none;
    border: none;
    text-align: left;
    font-family: Inter, sans-serif;
    font-size: 0.78rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: rgba(95, 70, 54, 0.5);
    cursor: pointer;
    transition: color 0.2s;
  }
  .user-panel-logout:hover { color: #c65f3c; }

  /* ── Mobile breakpoint ───────────────────────────────────── */
  @media (max-width: 720px) {
    .site-header {
      height: 58px;
    }

    /* On mobile, override the 5-col grid with a simple flex row */
    .header-inner {
      display: flex;
      align-items: center;
      padding: 0 16px;
    }

    /* Hide desktop-only zones */
    .ghost-left,
    .ghost-right,
    .nav-side { display: none; }

    /* Brand: move to flex-start, horizontal layout */
    .brand {
      flex-direction: row;
      align-items: center;
      gap: 9px;
      padding: 0;
      align-self: auto;
    }

    .brand-inner { gap: 9px; }


    .brand-name { font-size: 18px; }

    /* subtitle always hidden on mobile */
    .brand-sub { display: none; }

    /* Show mobile controls */
    .mobile-controls {
      display: flex;
      align-items: center;
      gap: 6px;
      margin-left: auto;
    }

    .mobile-user-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 36px;
      height: 36px;
      background: none;
      border: 1px solid transparent;
      border-radius: 50%;
      padding: 0;
      cursor: pointer;
      color: var(--color-ink-tertiary);
      transition: color 0.25s;
    }

    .mobile-user-btn:has(.user-avatar) {
      border-color: var(--border);
    }

    .mobile-menu-btn {
      display: flex;
      width: 44px;
      height: 44px;
      align-items: center;
      justify-content: center;
      flex-direction: column;
      gap: 4px;
      border: none;
      background: none;
      cursor: pointer;
      color: var(--color-ink-secondary);
      padding: 0;
    }

    .mobile-menu-btn span {
      width: 16px;
      height: 1px;
      background: currentColor;
      transform-origin: center;
      transition: transform 0.22s ease, opacity 0.22s ease;
    }

    .mobile-menu-btn.is-open span:nth-child(1) { transform: translateY(5px) rotate(45deg); }
    .mobile-menu-btn.is-open span:nth-child(2) { opacity: 0; }
    .mobile-menu-btn.is-open span:nth-child(3) { transform: translateY(-5px) rotate(-45deg); }

    .mobile-menu-btn:focus-visible {
      outline: 2px solid rgba(198, 95, 60, 0.52);
      outline-offset: 3px;
    }

    /* Mobile nav dropdown */
    .mobile-nav {
      display: grid;
      position: fixed;
      top: 58px;
      left: 0;
      right: 0;
      padding: 6px 16px 16px;
      background: rgba(248, 241, 231, 0.99);
      border-bottom: 1px solid var(--border);
      box-shadow: 0 18px 34px rgba(52, 37, 28, 0.10);
      transform: translateY(-6px);
      opacity: 0;
      pointer-events: none;
      transition: opacity 0.22s ease, transform 0.22s ease;
    }

    .mobile-nav.is-open {
      transform: translateY(0);
      opacity: 1;
      pointer-events: auto;
    }

    .mobile-nav-link {
      display: flex;
      align-items: center;
      height: 44px;
      font-family: var(--font-body);
      font-size: 12px;
      font-weight: 500;
      letter-spacing: 0.10em;
      text-transform: uppercase;
      color: var(--color-ink-secondary);
      text-decoration: none;
      border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 8%, transparent);
      transition: color 0.2s;
    }

    .mobile-nav-link.is-active,
    .mobile-nav-link:hover { color: var(--ink); }

    .mobile-nav-footer {
      display: flex;
      align-items: center;
      gap: 10px;
      padding-top: 12px;
      opacity: 0.55;
    }
  }

  /* ── Notifications ───────────────────────────────────────── */
  .notif-stack {
    position: fixed;
    bottom: 20px;
    right: 20px;
    z-index: 500;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }

  .notif {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    width: 280px;
    padding: 12px 14px;
    background: #f2e8d9;
    border: 1px solid #d8c6b1;
    border-left-width: 3px;
    box-shadow: 0 4px 18px rgba(52, 37, 28, 0.13);
    font-family: Georgia, serif;
    color: #34251c;
    pointer-events: all;
  }

  .notif--confirmed { border-left-color: #3a7a40; }
  .notif--rejected  { border-left-color: #c65f3c; }
  .notif--cancelled { border-left-color: #9a8070; }
  .notif--completed { border-left-color: #3a7060; }

  .notif-body { flex: 1; min-width: 0; }

  .notif-name {
    display: block;
    font-size: 0.8rem;
    font-weight: 500;
    color: #34251c;
    margin-bottom: 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .notif-text {
    display: block;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: rgba(95, 70, 54, 0.7);
  }

  .notif-actions {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
    flex-shrink: 0;
  }

  .notif-link {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 0.62rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #c65f3c;
    text-decoration: none;
    white-space: nowrap;
    transition: opacity 0.15s;
  }
  .notif-link:hover { opacity: 0.75; }

  .notif-dismiss {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 10px;
    color: rgba(95, 70, 54, 0.35);
    padding: 8px;
    margin: -8px;
    line-height: 1;
    transition: color 0.15s;
  }
  .notif-dismiss:hover { color: #c65f3c; }

  @media (max-width: 720px) {
    .notif-stack { right: 12px; bottom: 12px; }
    .notif { width: calc(100vw - 24px); max-width: 280px; }
  }
</style>
