<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { onNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import SiteHeader from '$lib/components/SiteHeader.svelte';
  import SiteFooter from '$lib/components/SiteFooter.svelte';
  import DustParticles from '$lib/components/DustParticles.svelte';
  import { themeConfig, themeCSS, startListeningForPreview, applyPreviewPayload } from '$lib/stores/theme.svelte';
  import { injectStyle } from '$lib/inject-style';
  import { setCopyOverrides, lang } from '$lib/i18n';
  import '$lib/stores/reading-font.svelte'; // initialises --font-reading from saved preference
  import { pageTurn } from '$lib/stores/page-turn.svelte';
  import { roomGesture } from '$lib/house-rooms';
  import { api } from '$lib/api';
  import type { Lang } from '$lib/i18n';

  type ViewTransition = { finished: Promise<void>; ready: Promise<void> };
  type VTDocument = Document & {
    startViewTransition(cb: () => void | Promise<void>): ViewTransition;
  };

  // A page may override the canonical path via its load data (e.g. a figurine
  // reached by UUID canonicalises to its slug URL); otherwise use the live path.
  let canonicalUrl = $derived(
    `${page.url.origin}${page.data?.canonicalPath ?? page.url.pathname}`
  );
  let { children } = $props();
  let showSiteHeader = $derived(!page.url.pathname.startsWith('/admin'));
  let hasHeaderOffset = $derived(showSiteHeader && page.url.pathname !== '/');
  // Detail page has its own DustParticles at higher intensity — skip in layout to avoid double canvas
  let showDust = $derived(showSiteHeader && !page.url.pathname.startsWith('/figurines/'));
  // House-descent scroll dimmer: every public page EXCEPT the figurine detail /
  // passport routes, which run their own candle vignette (stacking a second
  // scroll-linked dimmer there would over-darken the specimen).
  let showDescent = $derived(showSiteHeader && !page.url.pathname.startsWith('/figurines/'));

  // Keep <html lang> in sync with the active language. app.html hard-codes lang="ru",
  // but the default content language is English (i18n getInitialLang) and the reader
  // can switch — a stale lang attribute mispronounces in screen readers and misleads
  // search engines. SPA mode (ssr=false) has no handle hook, so we set it client-side.
  $effect(() => {
    if (typeof document !== 'undefined') document.documentElement.lang = $lang;
  });

  // Admin theme override CSS. Injected via injectStyle (sets style.textContent, which
  // does NOT parse HTML) rather than `{@html <style>…</style>}` in <svelte:head>: the
  // theme values are admin-authored and served to every visitor, and a `</style>` in any
  // of them would break out of the tag under {@html} — a stored-XSS primitive. textContent
  // makes that impossible. themeConfig is fetched client-side (see onMount), so nothing was
  // ever baked into the prerendered HTML anyway; this matches the reel/home-layout CSS,
  // which already inject this way.
  $effect(() => injectStyle('theme-override', $themeCSS));

  let stopPreviewListener: (() => void) | null = null;
  let removeMessageListener: (() => void) | null = null;

  function applyThemeHighlight(css: string | null) {
    const id = 'gotiga-theme-highlight';
    const existing = document.getElementById(id);
    if (!css) {
      existing?.remove();
      return;
    }
    const style = existing instanceof HTMLStyleElement ? existing : document.createElement('style');
    style.id = id;
    style.textContent = css;
    if (!style.parentNode) document.head.appendChild(style);
  }

  onMount(() => {
    if ('serviceWorker' in navigator && import.meta.env.VITE_BUILD_TARGET === 'web') {
      import('virtual:pwa-register').then(({ registerSW }) => {
        registerSW({ immediate: false });
      }).catch(() => {});
    }

    // Load theme and copy overrides
    Promise.all([
      api.getThemeConfig().catch(() => null),
      api.getCopyOverrides().catch(() => null),
    ]).then(([themeData, copyData]) => {
      if (themeData) themeConfig.set(themeData);
      if (copyData) setCopyOverrides(copyData as Record<Lang, Record<string, string>>);
    });

    if (!page.url.pathname.startsWith('/admin')) {
      // BroadcastChannel — receives updates from other tabs AND from the parent admin frame
      stopPreviewListener = startListeningForPreview();

      // postMessage — parent frame sends the initial draft when this iframe first loads
      function onParentMessage(e: MessageEvent) {
        if (e.data?.type === 'gotiga-preview' && e.data.config) {
          applyPreviewPayload(e.data.config, e.data.bridgeCSS);
        } else if (e.data?.type === 'gotiga-font' && e.data.href) {
          if (!document.querySelector(`link[href="${e.data.href}"]`)) {
            const link = document.createElement('link');
            link.rel = 'stylesheet';
            link.href = e.data.href;
            document.head.appendChild(link);
          }
        } else if (e.data?.type === 'gotiga-highlight' && typeof e.data.css === 'string') {
          applyThemeHighlight(e.data.css);
        } else if (e.data?.type === 'gotiga-highlight-clear') {
          applyThemeHighlight(null);
        }
      }
      window.addEventListener('message', onParentMessage);
      removeMessageListener = () => window.removeEventListener('message', onParentMessage);
    }
  });

  onDestroy(() => {
    stopPreviewListener?.();
    removeMessageListener?.();
    if (typeof document !== 'undefined') applyThemeHighlight(null);
  });

  onNavigate((navigation) => {
    if (!('startViewTransition' in document)) {
      pageTurn.disarm();
      return;
    }
    const vtDocument = document as VTDocument;

    // Page-turn (prev/next figurine) wins when armed. Hall↔archive and
    // hall↔workshop use the house room gestures. Everything else — and reduced
    // motion — keeps the leaf fade in app.css.
    const direction = pageTurn.direction;
    const reduceMotion =
      typeof window !== 'undefined' &&
      window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    const turning = Boolean(direction) && !reduceMotion;

    if (turning) {
      const root = document.documentElement;
      root.classList.add('gt-page-turn', `gt-${direction}`);
      // Drop the figurine name from the OUTGOING plate so the whole leaf is captured
      // in the root snapshot and turns as one piece (the incoming plate omits it via
      // the pageTurn store in FigurineDetailView). Snapshots are taken synchronously
      // when startViewTransition() is called, so this must happen first.
      document.querySelectorAll<HTMLElement>('[data-figurine-plate]').forEach((el) => {
        el.style.viewTransitionName = 'none';
      });

      return new Promise<void>((resolve) => {
        const transition = vtDocument.startViewTransition(async () => {
          resolve();
          await navigation.complete;
        });
        transition.finished.finally(() => {
          root.classList.remove('gt-page-turn', 'gt-forward', 'gt-backward');
          pageTurn.disarm();
          // The WebGL plate (LivingDaguerreotype) parks its render loop after one
          // frame; a view transition can leave that frame blank until the next
          // draw. Nudge it to repaint now instead of waiting for a pointermove.
          requestAnimationFrame(() => window.dispatchEvent(new Event('gotiga:redraw')));
        });
      });
    }

    // Direction may be armed but suppressed (reduced motion) — clear it so the
    // incoming plate keeps its figurine-{id} name and morphs as usual.
    pageTurn.disarm();

    const fromPath = navigation.from?.url.pathname ?? '';
    const toPath = navigation.to?.url.pathname ?? '';
    const gesture = reduceMotion ? null : roomGesture(fromPath, toPath);

    if (!gesture) {
      return new Promise<void>((resolve) => {
        vtDocument.startViewTransition(async () => {
          resolve();
          await navigation.complete;
        });
      });
    }

    const root = document.documentElement;
    root.classList.add('gt-room', `gt-${gesture}`);
    // Home tiles and archive cards share figurine-{id} names. Morphing that
    // swarm during a room crossing aborts the drawer; drop the names on the
    // outgoing snapshot so the leaf turns as one piece.
    document
      .querySelectorAll<HTMLElement>('[data-figurine-plate], [style*="view-transition-name"]')
      .forEach((el) => {
        const name = el.style.viewTransitionName;
        if (el.hasAttribute('data-figurine-plate') || name.startsWith('figurine-')) {
          el.style.viewTransitionName = 'none';
        }
      });

    return new Promise<void>((resolve) => {
      const transition = vtDocument.startViewTransition(async () => {
        resolve();
        await navigation.complete;
      });
      transition.finished.finally(() => {
        root.classList.remove(
          'gt-room',
          'gt-drawer-in',
          'gt-drawer-out',
          'gt-curtain-in',
          'gt-curtain-out',
        );
        requestAnimationFrame(() => window.dispatchEvent(new Event('gotiga:redraw')));
      });
    });
  });
</script>

<svelte:head>
  <link rel="canonical" href={canonicalUrl} />
  <!-- Theme override CSS is applied via injectStyle in the script above (safe textContent),
       NOT emitted here as {@html <style>} — see the effect's comment. -->
</svelte:head>

<div class="min-h-screen bg-[#f8f1e7]" class:reading-typeset={showSiteHeader}>
  {#if showSiteHeader}
    <SiteHeader />
  {/if}

  {#if showDust}
    <DustParticles opacity={0.2} />
  {/if}

  {#if showDescent}
    <!-- Scroll-driven "descent into the house" dimmer (app.css .house-descent) -->
    <div class="house-descent" aria-hidden="true"></div>
  {/if}

  <main class="min-h-screen" class:with-site-header={hasHeaderOffset}>
    {@render children()}
  </main>

  {#if showSiteHeader}
    <SiteFooter />
  {/if}
</div>

<style>
  /* The reader's chosen typeface governs the ENTIRE public site — header,
     headings, body and prose on every page. Scoped to the wrapper (not :root)
     so it overrides any admin font set inline on <html>, and excluded on /admin
     so the design-system editor's own font preview keeps working.
     --font-serif is left alone: it feeds --font-reading (would be a var cycle). */
  .reading-typeset {
    --font-display: var(--font-reading);
    --font-body: var(--font-reading);
  }

  .with-site-header {
    padding-top: 68px;
  }

  /* Matches SiteHeader's own desktop→mobile switch: below it the header shrinks
     to its 58px mobile bar, so the content offset must follow at the same width
     or a gap opens under the header on non-home pages. */
  @media (max-width: 1024px) {
    .with-site-header {
      padding-top: 58px;
    }
  }
</style>
