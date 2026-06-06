<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/state';
  import LangSwitcher from '$lib/components/LangSwitcher.svelte';
  import { allClaims } from '$lib/stores/all-claims.svelte';
  import { t } from '$lib/i18n';
  import { fade } from 'svelte/transition';

  const links = [
    { href: '/figurines', label: 'Archive' },
    { href: '/upcoming', label: 'In Progress' },
    { href: '/workshop', label: 'Workshop' },
    { href: '/author', label: 'Author' },
  ];

  let pathname = $derived(page.url.pathname);
  let panelOpen = $state(false);
  let panelRef = $state<HTMLElement | null>(null);

  function isActive(href: string) {
    return pathname === href || pathname.startsWith(`${href}/`);
  }

  function togglePanel() { panelOpen = !panelOpen; }
  function closePanel()  { panelOpen = false; }

  function fmtDate(iso: string) {
    return new Date(iso).toLocaleDateString(undefined, { day: '2-digit', month: 'short' });
  }

  function handleOutside(e: MouseEvent) {
    if (panelOpen && panelRef && !panelRef.contains(e.target as Node)) {
      panelOpen = false;
    }
  }

  onMount(() => {
    allClaims.load();
    allClaims.verify();
    allClaims.startPolling();
    document.addEventListener('click', handleOutside, { capture: true });
  });

  onDestroy(() => {
    allClaims.stopPolling();
    document.removeEventListener('click', handleOutside, { capture: true });
  });

  let count = $derived(allClaims.activeCount);
</script>

<header class="site-header">
  <a href="/" class="brand" aria-label="Gotiga">
    <span class="brand-name">Gotiga</span>
    <span class="brand-sub">Cabinet of Gothic Miniatures</span>
  </a>

  <nav class="nav" aria-label="Primary">
    {#each links as link}
      <a
        href={link.href}
        class="nav-link"
        class:is-active={isActive(link.href)}
        aria-current={isActive(link.href) ? 'page' : undefined}
      >
        {link.label}
      </a>
    {/each}
  </nav>

  <div class="header-end">
    <LangSwitcher variant="dark" />

    <!-- Bookings indicator -->
    <div class="bookings-anchor" bind:this={panelRef}>
      <button
        class="bookings-btn"
        class:has-claims={count > 0}
        class:is-open={panelOpen}
        onclick={togglePanel}
        aria-label={$t('bookingsHeaderTitle')}
        title={$t('bookingsHeaderTitle')}
      >
        <!-- Scroll/bookmark icon -->
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
                      {c.status === 'confirmed' ? $t('bookingsConfirmed') : $t('bookingsPending')}
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
            <a href="/bookings" class="panel-view-all" onclick={closePanel}>
              {$t('bookingsViewAll')} →
            </a>
          </div>
        </div>
      {/if}
    </div>

    <a href="/admin" class="key-link" aria-label="Admin">
      <svg width="13" height="13" viewBox="0 0 13 13" fill="none" aria-hidden="true">
        <circle cx="4.5" cy="4.5" r="3" stroke="currentColor" stroke-width="1"/>
        <path d="M7 7L11.5 11.5" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
        <path d="M9.5 9L11 7.5" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
      </svg>
    </a>
  </div>
</header>

<style>
  .site-header {
    --cream: #f8f1e7;
    --ink: #2c1710;
    --mid: #6f3b24;
    --copper: #c65f3c;
    --muted: rgba(95,70,54,0.68);
    --muted2: rgba(95,70,54,0.40);
    --border: rgba(52,37,28,0.10);
    --ease: cubic-bezier(0.16,1,0.3,1);

    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 68px;
    display: flex;
    align-items: center;
    padding: 0 clamp(20px, 4.5vw, 72px);
    background: rgba(248,241,231,0.85);
    backdrop-filter: blur(20px) saturate(1.3);
    -webkit-backdrop-filter: blur(20px) saturate(1.3);
    border-bottom: 1px solid var(--border);
    z-index: 200;
  }

  .brand {
    display: flex;
    flex-direction: column;
    gap: 3px;
    text-decoration: none;
    color: inherit;
    flex-shrink: 0;
  }

  .brand-name {
    font-family: 'Cormorant Garamond', 'Fraunces', Georgia, serif;
    font-size: 20px;
    font-weight: 400;
    letter-spacing: 0.3em;
    text-transform: uppercase;
    color: var(--ink);
    line-height: 1;
  }

  .brand-sub {
    font-family: 'Instrument Sans', var(--font-body), system-ui, sans-serif;
    font-size: 8.5px;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: var(--muted2);
    line-height: 1;
  }

  .nav {
    display: flex;
    align-items: center;
    margin-left: auto;
  }

  .nav-link {
    position: relative;
    display: flex;
    align-items: center;
    height: 68px;
    padding: 0 22px;
    font-family: 'Instrument Sans', var(--font-body), system-ui, sans-serif;
    font-size: 9.5px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--muted);
    text-decoration: none;
    transition: color 0.25s;
    overflow: hidden;
  }

  .nav-link::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 22px;
    right: 22px;
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

  .header-end {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-left: 20px;
    padding-left: 20px;
    border-left: 1px solid var(--border);
  }

  /* ── Bookings button ── */
  .bookings-anchor {
    position: relative;
  }

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
    color: var(--muted2);
    transition: color 0.25s;
  }

  .bookings-btn:hover,
  .bookings-btn.is-open {
    color: var(--mid);
  }

  .bookings-btn.has-claims {
    color: var(--copper);
  }

  .badge {
    position: absolute;
    top: 0;
    right: 0;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    background: var(--copper);
    color: #f8f1e7;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 8px;
    font-weight: 600;
    letter-spacing: 0;
    border-radius: 7px;
    display: flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
  }

  /* ── Dropdown panel ── */
  .bookings-panel {
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    width: 300px;
    background: #f2e8d9;
    border: 1px solid #d8c6b1;
    box-shadow: 0 8px 32px rgba(52,37,28,0.12);
    z-index: 300;
    font-family: Georgia, serif;
    color: #34251c;
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px 10px;
    border-bottom: 1px solid rgba(52,37,28,0.08);
  }

  .panel-title {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 9px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.6);
  }

  .panel-close {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 11px;
    color: rgba(95,70,54,0.4);
    padding: 2px 4px;
    line-height: 1;
    transition: color 0.2s;
  }
  .panel-close:hover { color: #c65f3c; }

  .panel-empty {
    padding: 20px 16px;
    text-align: center;
    font-size: 0.8rem;
    color: rgba(95,70,54,0.5);
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
    border-bottom: 1px solid rgba(52,37,28,0.06);
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
    color: rgba(95,70,54,0.6);
    font-style: italic;
  }

  .panel-cancel-btn {
    background: none;
    border: 1px solid rgba(198,95,60,0.4);
    color: #c65f3c;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 8px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 3px 8px;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }
  .panel-cancel-btn:hover:not(:disabled) { background: rgba(198,95,60,0.08); border-color: #c65f3c; }
  .panel-cancel-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .panel-err {
    margin: 4px 0 0;
    font-size: 0.7rem;
    color: #a03020;
    font-style: italic;
  }

  .panel-footer {
    padding: 10px 16px;
    border-top: 1px solid rgba(52,37,28,0.08);
    text-align: right;
  }

  .panel-view-all {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 8.5px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.55);
    text-decoration: none;
    transition: color 0.2s;
  }
  .panel-view-all:hover { color: #c65f3c; }

  /* ── Admin key ── */
  .key-link {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    color: var(--muted2);
    text-decoration: none;
    transition: color 0.25s;
  }
  .key-link:hover { color: var(--mid); }

  @media (max-width: 680px) {
    .site-header {
      height: 58px;
      padding: 0 16px;
    }

    .brand-name { font-size: 17px; }

    .brand-sub,
    .nav { display: none; }

    .header-end {
      margin-left: auto;
      padding-left: 14px;
      gap: 10px;
    }

    .bookings-panel {
      right: -16px;
      width: calc(100vw - 32px);
      max-width: 300px;
    }
  }
</style>
