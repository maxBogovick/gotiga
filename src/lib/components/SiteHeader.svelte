<script lang="ts">
  import { page } from '$app/state';
  import LangSwitcher from '$lib/components/LangSwitcher.svelte';

  const links = [
    { href: '/figurines', label: 'Archive' },
    { href: '/workshop', label: 'Workshop' },
    { href: '/author', label: 'Author' },
  ];

  let pathname = $derived(page.url.pathname);

  function isActive(href: string) {
    return pathname === href || pathname.startsWith(`${href}/`);
  }
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

  .key-link:hover {
    color: var(--mid);
  }

  @media (max-width: 680px) {
    .site-header {
      height: 58px;
      padding: 0 16px;
    }

    .brand-name {
      font-size: 17px;
    }

    .brand-sub,
    .nav {
      display: none;
    }

    .header-end {
      margin-left: auto;
      padding-left: 14px;
      gap: 10px;
    }
  }
</style>
