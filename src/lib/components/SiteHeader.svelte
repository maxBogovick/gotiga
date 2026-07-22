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
  import ContactMessageForm from '$lib/components/ContactMessageForm.svelte';
  import type { AuthorProfile } from '$lib/types/api';

  // Header-left maker avatar — admin-editable photo + frame styling.
  let authorProfile = $state<AuthorProfile | null>(null);
  let headerAvatarUrl = $derived(authorProfile?.heroPhotoUrl?.trim() || '/images/avatar.jpg');
  let headerAuthorName = $derived(authorProfile?.name?.trim() || $brandName);
  let avatarShape = $derived(authorProfile?.avatarShape ?? 'circle');
  let avatarRadiusCss = $derived(avatarShape === 'square' ? `${authorProfile?.avatarRadius ?? 12}px` : '50%');
  let avatarBorderWidthCss = $derived(`${authorProfile?.avatarBorderWidth ?? 3.5}px`);
  let avatarBorderColor = $derived(authorProfile?.avatarBorderColor?.trim() || 'var(--copper)');
  let avatarBg = $derived(authorProfile?.avatarBg?.trim() || 'transparent');

  let leftLinks = $derived([
    { href: '/figurines', label: $t('navArchive') },
    { href: '/upcoming', label: $t('navUpcoming') },
  ]);

  let rightLinks = $derived([
    { href: '/workshop', label: $t('navWorkshop') },
    { href: '/author',   label: $t('navAuthor') },
  ]);

  let pathname = $derived(page.url.pathname);

  function jumpToWorkshopStory(event: MouseEvent) {
    if (pathname === '/') {
      event.preventDefault();
      document.getElementById('in-the-workshop')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
    // Elsewhere: let the link navigate to "/#in-the-workshop" normally —
    // the browser scrolls there once the home page has mounted.
  }

  let panelOpen = $state(false);
  let panelRef = $state<HTMLElement | null>(null);
  let mobileNavOpen = $state(false);
  let isScrolled = $state(false);
  // Reserved for pages that open on a full-bleed dark plate. The home page is
  // parchment again, so nothing turns this on today.
  let isOverPlate = $state(false);

  // On the home page, the maker avatar stays out of sight until the visitor
  // has scrolled past the hero photograph — it shouldn't compete with that
  // first image. Everywhere else it's just always there. Seeded from the
  // pathname (not just `true`) so a home-page load doesn't flash the avatar
  // in before the first scroll check hides it again.
  let avatarVisible = $state(pathname !== '/');

  function updateAvatarVisibility() {
    if (pathname !== '/') {
      avatarVisible = true;
      return;
    }
    const heroFrame = document.querySelector('.cine-frame');
    avatarVisible = heroFrame ? heroFrame.getBoundingClientRect().bottom <= 0 : true;
  }

  function isActive(href: string) {
    return pathname === href || pathname.startsWith(`${href}/`);
  }

  function toggleMobileNav() { mobileNavOpen = !mobileNavOpen; }
  function closeMobileNav() { mobileNavOpen = false; }
  function togglePanel() { panelOpen = !panelOpen; }
  function closePanel()  { panelOpen = false; }

  function handleScroll() {
    isScrolled = window.scrollY > 48;
    updateAvatarVisibility();
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

  // "Write to the author" panel — a quill icon always in reach, so the
  // lightweight contact form (see ContactMessageForm) is one click away
  // from any page, not just the home correspondence card.
  let contactPanelOpen = $state(false);
  let contactPanelRef = $state<HTMLElement | null>(null);

  function toggleContactPanel() { contactPanelOpen = !contactPanelOpen; }
  function closeContactPanel() { contactPanelOpen = false; }

  function handleContactOutside(e: MouseEvent) {
    if (contactPanelOpen && contactPanelRef && !contactPanelRef.contains(e.target as Node)) {
      contactPanelOpen = false;
    }
  }

  // Mobile mirror of the "write to the author" quill. The desktop contact button
  // lives in .ghost-right-group (hidden on mobile), so the compact mobile bar gets
  // its own button + panel next to the account icon. Separate open-state and ref so
  // its outside-click handler doesn't fight the desktop one.
  let mobileContactOpen = $state(false);
  let mobileContactRef = $state<HTMLElement | null>(null);

  function toggleMobileContact() {
    mobileContactOpen = !mobileContactOpen;
    if (mobileContactOpen) mobileNavOpen = false;
  }
  function closeMobileContact() { mobileContactOpen = false; }

  function handleMobileContactOutside(e: MouseEvent) {
    if (mobileContactOpen && mobileContactRef && !mobileContactRef.contains(e.target as Node)) {
      mobileContactOpen = false;
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
    document.addEventListener('click', handleContactOutside, { capture: true });
    document.addEventListener('click', handleMobileContactOutside, { capture: true });
    window.addEventListener('scroll', handleScroll, { passive: true });
    handleScroll();

    api.getAuthorProfile().then((p) => { authorProfile = p; }).catch(() => {});

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
      document.removeEventListener('click', handleContactOutside, { capture: true });
      document.removeEventListener('click', handleMobileContactOutside, { capture: true });
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('scroll', handleScroll);
    }
  });

  $effect(() => {
    void pathname;
    mobileNavOpen = false;
    mobileContactOpen = false;
    // Wait a tick so the new page's markup (and its .cine-frame, if any)
    // has mounted before we measure it.
    requestAnimationFrame(updateAvatarVisibility);
  });
</script>

<header class="site-header" class:is-scrolled={isScrolled} class:over-plate={isOverPlate}>
  <HeaderBirdWalk />
  <div class="header-inner">

    <!-- ① Ghost-left: muted utilities pinned to left edge -->
    <div class="ghost-left">
      <!-- Maker avatar: large, full colour, fills the header's height —
           a real face, not a muted icon. -->
      <a
        class="header-avatar"
        class:is-hidden={!avatarVisible}
        href="/#in-the-workshop"
        onclick={jumpToWorkshopStory}
        aria-label={headerAuthorName}
        aria-hidden={!avatarVisible}
        tabindex={avatarVisible ? 0 : -1}
        style="
          --avatar-radius: {avatarRadiusCss};
          --avatar-border-w: {avatarBorderWidthCss};
          --avatar-border-color: {avatarBorderColor};
          --avatar-bg: {avatarBg};
        "
      >
        <img src={headerAvatarUrl} alt="" class="header-avatar-img" loading="eager" decoding="async" />
        <span class="header-avatar-tip">{headerAuthorName}</span>
      </a>
      <span class="ghost-left-utils">
        <LangSwitcher variant="light" />
        <FontSwitcher variant="header" />
      </span>
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

    <!-- ⑤ Ghost-right: muted utilities pinned to right edge. The "Write"
         button lives OUTSIDE .ghost-right on purpose: that container (and
         its :has()/:hover states) dims itself down to as little as 0.42
         opacity for the bookings/user icons, and CSS opacity on a parent
         composites its whole subtree as one layer — nesting the accented
         button (and its dropdown) in there made both render translucent no
         matter how opaque their own backgrounds were. -->
    <div class="ghost-right-group">
      <div class="contact-anchor" bind:this={contactPanelRef}>
        <button
          class="contact-btn"
          class:is-open={contactPanelOpen}
          onclick={toggleContactPanel}
          aria-label={$t('headerContactLabel')}
          title={$t('headerContactLabel')}
        >
          <svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <path d="M12.2 1.8c-3.6 0-8.4 3.4-9.9 7-.3.7.4 1.4 1.1 1.1l1.9-.8" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M12.2 1.8c0 3.6-3.4 8.4-7 9.9-.7.3-1.4-.4-1.1-1.1l.8-1.9" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M5.4 8.6L1.8 12.2" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
          </svg>
          <span class="contact-btn-label">{$t('headerContactShortLabel')}</span>
        </button>

        {#if contactPanelOpen}
          <div class="contact-panel" transition:fade={{ duration: 150 }}>
            <div class="panel-head">
              <span class="panel-title">{$t('headerContactLabel')}</span>
              <button class="panel-close" onclick={closeContactPanel} aria-label="Close">✕</button>
            </div>
            <p class="contact-panel-note">{$t('headerContactNote')}</p>
            <ContactMessageForm source="header" compact />
          </div>
        {/if}
      </div>

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
    </div>

    <!-- ⑥ Mobile-only controls -->
    <div class="mobile-controls">
      <!-- "Write to the author" — mobile mirror of the desktop quill (which lives in
           the hidden .ghost-right-group). Sits before the account icon. -->
      <div class="mobile-contact-anchor" bind:this={mobileContactRef}>
        <button
          class="mobile-contact-btn"
          class:is-open={mobileContactOpen}
          onclick={toggleMobileContact}
          aria-label={$t('headerContactLabel')}
          title={$t('headerContactLabel')}
        >
          <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <path d="M12.2 1.8c-3.6 0-8.4 3.4-9.9 7-.3.7.4 1.4 1.1 1.1l1.9-.8" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M12.2 1.8c0 3.6-3.4 8.4-7 9.9-.7.3-1.4-.4-1.1-1.1l.8-1.9" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M5.4 8.6L1.8 12.2" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
          </svg>
        </button>

        {#if mobileContactOpen}
          <div class="contact-panel mobile-contact-panel" transition:fade={{ duration: 150 }}>
            <div class="panel-head">
              <span class="panel-title">{$t('headerContactLabel')}</span>
              <button class="panel-close" onclick={closeMobileContact} aria-label="Close">✕</button>
            </div>
            <p class="contact-panel-note">{$t('headerContactNote')}</p>
            <ContactMessageForm source="header" compact />
          </div>
        {/if}
      </div>
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

  /* ── Floating over the home room photograph ──────────────────
     No bar, no rule: just type on the picture. The whole palette inverts —
     parchment where ink used to be — and the header leans on a soft gradient
     rather than a fill, so the photograph runs right up under it.
  ──────────────────────────────────────────────────────────── */
  .site-header.over-plate {
    /* The reel theme decides these: on a pale backdrop (the daylight themes) the
       header's ink flips to dark and its scrim to light, or the nav vanishes. */
    --plate-ink: var(--reel-header-ink, 250 246 238);
    --plate-scrim: var(--reel-header-scrim, 20 11 7);

    --ink: rgb(var(--plate-ink));
    --mid: rgb(var(--plate-ink) / 0.72);
    --copper: rgb(var(--plate-ink));
    --border: transparent;
    background: linear-gradient(
      180deg,
      rgb(var(--plate-scrim) / 0.72) 0%,
      rgb(var(--plate-scrim) / 0.38) 55%,
      transparent 100%
    );
    border-bottom-color: transparent;
    color: rgb(var(--plate-ink));
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }

  /* The nav links, the subtitle and the avatar tip pin their colour straight to
     the ink tokens rather than to --ink, so swapping --ink above never reached
     them — over a dark photograph they stayed brown-on-brown. State them here. */
  .site-header.over-plate .nav-link,
  .site-header.over-plate .brand-name,
  .site-header.over-plate .brand-sub {
    color: rgb(var(--plate-ink) / 0.78);
    text-shadow: 0 1px 14px rgb(var(--plate-scrim) / 0.75);
  }

  .site-header.over-plate .nav-link:hover,
  .site-header.over-plate .nav-link.is-active,
  .site-header.over-plate .brand-name {
    color: rgb(var(--plate-ink));
  }

  .site-header.over-plate .brand-sub {
    color: rgb(var(--plate-ink) / 0.55);
  }

  /* 0.42 opacity is a discreet grey on parchment; on a photograph it is simply
     gone. The utilities need real presence here — still quiet, but legible.
     Scoped to the icon BUTTONS only (not `.ghost-right` as a whole): opacity
     on an ancestor composites its entire subtree as one layer, and the
     bookings/user/contact buttons each open a solid dropdown panel — nesting
     those panels under a dimmed ancestor made them render half see-through
     no matter how opaque their own background was. The always-accented
     contact button is excluded — see its own rule below. */
  .site-header.over-plate .ghost-left-utils,
  .site-header.over-plate .ghost-right .bookings-btn,
  .site-header.over-plate .ghost-right .user-btn {
    opacity: 0.72;
  }

  .site-header.over-plate .ghost-left-utils:hover,
  .site-header.over-plate .ghost-left-utils:focus-within,
  .site-header.over-plate .ghost-right .bookings-anchor:hover .bookings-btn,
  .site-header.over-plate .ghost-right .bookings-anchor:focus-within .bookings-btn,
  .site-header.over-plate .ghost-right .user-anchor:hover .user-btn,
  .site-header.over-plate .ghost-right .user-anchor:focus-within .user-btn {
    opacity: 1;
  }

  /* The lang and font switchers are separate components that hard-code their ink
     as Tailwind literals (text-[#34251c] …), so no variable of ours can reach
     them. Over the photograph they have to be repainted from the outside. */
  .site-header.over-plate .ghost-left-utils :global(button),
  .site-header.over-plate .ghost-left-utils :global(span),
  .site-header.over-plate .ghost-left-utils :global(a) {
    color: rgb(var(--plate-ink) / 0.9);
    text-shadow: 0 1px 10px rgb(var(--plate-scrim) / 0.8);
  }

  /* Same for the bookings/account icons on the right — they draw with
     currentColor, so one colour here repaints the SVGs too. */
  .site-header.over-plate .ghost-right :global(button),
  .site-header.over-plate .ghost-right :global(a) {
    color: rgb(var(--plate-ink) / 0.9);
  }

  /* The avatar is the one thing already bright enough; it only needs separating
     from whatever happens to be behind it. */
  .site-header.over-plate .header-avatar {
    box-shadow: 0 4px 20px rgba(20, 11, 7, 0.55);
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
    /* POINT 1 — overlap made structurally impossible.
       The two outer tracks hold the "ghost" clusters (avatar + language/font
       switchers on the left; Write button + bookings/account icons on the
       right). With minmax(0,1fr) those tracks could shrink to ZERO while their
       flex content stayed full-width, so the content spilled OUT of its track
       and painted over the nav — the switchers on ARCHIVE/IN PROGRESS, the
       Write button on AUTHOR. minmax(auto,1fr) floors each track at its own
       min-content, so a ghost cluster can never be narrower than what it holds
       and therefore never overflows onto a neighbour. (We can't just clip the
       clusters: they host absolutely-positioned popovers — the Write form, the
       bookings/account menus, the font dropdown — that must escape the box.) */
    grid-template-columns: minmax(auto, 1fr) auto auto auto minmax(auto, 1fr);
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
  }

  /* Holds the always-accented "Write" button alongside the muted
     bookings/user cluster without being subject to the latter's opacity
     rules — see the markup comment by .ghost-right-group. */
  .ghost-right-group {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  /* .ghost-left itself stays at full opacity now that it carries the maker
     avatar — that must never look grey/dimmed. Only the lang/font switchers
     (wrapped below in .ghost-left-utils) keep the old muted-until-hovered
     behaviour; .ghost-right (bookings/account icons) is unaffected. */
  .ghost-right {
    opacity: 0.42;
    transition: opacity 0.3s ease;
  }
  .ghost-right:hover,
  .ghost-right:focus-within {
    opacity: 0.78;
  }

  .ghost-left-utils {
    display: flex;
    align-items: center;
    gap: 4px;
    opacity: 0.42;
    transition: opacity 0.3s ease;
  }
  .ghost-left-utils:hover,
  .ghost-left-utils:focus-within {
    opacity: 0.78;
  }

  .ghost-left  { grid-column: 1; justify-content: flex-start; }
  .ghost-right-group { grid-column: 5; justify-content: flex-end; }

  /* ── Maker avatar — large, full colour, fills the header's height.
     No sepia/grayscale filter: the face must read as a real photo, not a
     muted icon. Sized off the header's own height so it scales down
     automatically with the .is-scrolled 78px→54px shrink. ── */
  .header-avatar {
    /* Admin-editable, overridden inline via style="--avatar-*"; these are
       just the defaults if the profile hasn't set anything yet. */
    --avatar-radius: 50%;
    --avatar-border-w: 3.5px;
    --avatar-border-color: var(--copper);
    --avatar-bg: transparent;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 62px;
    height: 62px;
    box-sizing: border-box;
    padding: 3px;
    flex-shrink: 0;
    border-radius: var(--avatar-radius);
    background: var(--avatar-bg);
    text-decoration: none;
    opacity: 1;
    transform: scale(1);
    transition: width 0.25s ease, height 0.25s ease, opacity 0.35s var(--ease), transform 0.35s var(--ease);
  }

  /* Shrink with the header's own 78px→54px scroll transition. */
  .site-header.is-scrolled .header-avatar {
    width: 44px;
    height: 44px;
  }

  /* Held out of sight on the home page until the hero photograph has
     scrolled past — see updateAvatarVisibility() in the script. */
  .header-avatar.is-hidden {
    opacity: 0;
    transform: scale(0.7);
    pointer-events: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .header-avatar {
      transition: width 0.25s ease, height 0.25s ease;
    }
  }

  .header-avatar-img {
    width: 100%;
    height: 100%;
    border-radius: var(--avatar-radius);
    object-fit: cover;
    object-position: center 20%;
    display: block;
    box-shadow:
      0 0 0 var(--avatar-border-w) var(--avatar-border-color),
      0 4px 14px rgba(52, 37, 28, 0.3);
    transition: box-shadow 0.22s ease, transform 0.22s ease;
  }

  .header-avatar:hover .header-avatar-img,
  .header-avatar:focus-visible .header-avatar-img {
    box-shadow:
      0 0 0 var(--avatar-border-w) var(--avatar-border-color),
      0 6px 20px rgba(52, 37, 28, 0.38);
    transform: scale(1.06);
  }

  .header-avatar:focus-visible {
    outline: 2px solid var(--copper);
    outline-offset: 3px;
  }

  .header-avatar-tip {
    position: absolute;
    top: calc(100% + 10px);
    left: 50%;
    transform: translateX(-50%) translateY(-4px);
    z-index: 20;
    white-space: nowrap;
    padding: 4px 10px;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 12px;
    font-style: italic;
    color: #f8f1e7;
    background: rgba(28, 16, 10, 0.85);
    backdrop-filter: blur(6px);
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.18s ease, transform 0.18s ease;
  }

  .header-avatar:hover .header-avatar-tip,
  .header-avatar:focus-visible .header-avatar-tip {
    opacity: 1;
    transform: translateX(-50%) translateY(0);
  }

  @media (prefers-reduced-motion: reduce) {
    .header-avatar:hover .header-avatar-img,
    .header-avatar:focus-visible .header-avatar-img {
      transform: none;
    }
  }

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
    white-space: nowrap;
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

  /* ── Contact ("write to the author") button ─────────────────
     Deliberately NOT muted like the bookings/user icons — this is the
     answer to "how do I reach the maker," so it carries a permanent
     copper outline instead of blending in until hovered. Literal colour
     (not the --copper token, which the .over-plate state repaints) so it
     reads the same accent on parchment and over a photograph alike. */
  .contact-anchor { position: relative; }

  .contact-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 30px;
    padding: 0 13px 0 11px;
    background: transparent;
    border: 1px solid #c65f3c;
    border-radius: 2px;
    color: #c65f3c;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    white-space: nowrap;
    cursor: pointer;
    transition: background 0.25s ease, color 0.25s ease, border-color 0.25s ease;
  }
  .contact-btn svg { flex-shrink: 0; }
  .contact-btn:hover,
  .contact-btn.is-open {
    background: #c65f3c;
    color: #fff9f0;
  }

  .contact-panel {
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    width: 300px;
    padding: 4px 16px 18px;
    background: #f2e8d9;
    border: 1px solid #d8c6b1;
    box-shadow: 0 8px 32px rgba(52, 37, 28, 0.12);
    z-index: 300;
    font-family: Georgia, serif;
    color: #34251c;
  }
  .contact-panel .panel-head { margin: 0 -16px 14px; }
  .contact-panel-note {
    margin: 0 0 14px;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: 14px;
    line-height: 1.5;
    color: var(--color-ink-secondary, #5f4636);
  }

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

  /* ── Mobile breakpoint ───────────────────────────────────────
     The desktop layout is a wide 5-column "proscenium" (avatar + 2 switchers |
     nav | brand + subtitle | nav | Write + icons). Thanks to the minmax(auto,1fr)
     tracks above it no longer OVERLAPS when squeezed — but the content still
     genuinely runs out of room, so below this width we hand over to the
     hamburger layout. This is now a cosmetic threshold, not a correctness gate:
     if a longer brand name or locale pushes the fit-point around, the header
     degrades gracefully instead of breaking. 1024px clears the tightest locale
     (RU: МАСТЕРСКАЯ / НАПИСАТЬ), which stops fitting around ~1000px. */
  @media (max-width: 1024px) {
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
    .ghost-right-group,
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

    /* Mobile "write" button — copper-outlined like the desktop CTA, sized to sit
       inline with the account icon. */
    .mobile-contact-anchor {
      position: relative;
      display: flex;
      align-items: center;
    }

    .mobile-contact-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 36px;
      height: 36px;
      background: transparent;
      border: 1px solid #c65f3c;
      border-radius: 2px;
      padding: 0;
      cursor: pointer;
      color: #c65f3c;
      transition: background 0.2s ease, color 0.2s ease;
    }
    .mobile-contact-btn:hover,
    .mobile-contact-btn.is-open {
      background: #c65f3c;
      color: #fff9f0;
    }

    /* The shared .contact-panel is right-anchored to its button; on the mobile bar
       that button isn't the rightmost item, so pin the panel to the header edge
       instead and cap its width to the viewport. */
    .mobile-contact-panel {
      position: fixed;
      top: 58px;
      right: 12px;
      left: auto;
      width: min(320px, calc(100vw - 24px));
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
