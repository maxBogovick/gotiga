<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { onNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import SiteHeader from '$lib/components/SiteHeader.svelte';
  import SiteFooter from '$lib/components/SiteFooter.svelte';
  import { themeConfig, themeCSS, startListeningForPreview, applyPreviewPayload } from '$lib/stores/theme.svelte';
  import { injectStyle } from '$lib/inject-style';
  import { setCopyOverrides, lang } from '$lib/i18n';
  import { pageTurn } from '$lib/stores/page-turn.svelte';
  import { roomGesture } from '$lib/house-rooms';
  import { loadSiteFonts } from '$lib/load-fonts';
  import { api } from '$lib/api';
  import { matchChrome } from '$lib/stores/match-chrome.svelte';
  import { enterRoom, leaveRoom } from '$lib/analytics';
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
  // Стол студии — рабочая поверхность, а не страница дома: он занимает весь
  // экран и носит свою шапку с дверью назад. С шапкой и подвалом сайта поверх
  // него стол не помещался бы никогда, и человек резал бы раму, прокручивая
  // страницу. Вход в студию (`/studio`) — обычная страница дома, шапка на ней
  // остаётся: это витрина, с неё уходят в другие комнаты.
  let atDesk = $derived(/^\/studio\/frames\//.test(page.url.pathname));
  let showSiteHeader = $derived(
    !page.url.pathname.startsWith('/admin') && !atDesk && !matchChrome.covering,
  );
  let hasHeaderOffset = $derived(showSiteHeader && page.url.pathname !== '/');
  // House-descent scroll dimmer: every public page EXCEPT the figurine detail /
  // passport routes, which run their own candle vignette (stacking a second
  // scroll-linked dimmer there would over-darken the specimen).
  let showDescent = $derived(showSiteHeader && !page.url.pathname.startsWith('/figurines/'));

  // Visit tracking lives here and only here. A page that has to remember to
  // call analytics is a page that will one day be added without calling it,
  // and the hole shows up a month later as an empty row in the report — which
  // is exactly how /gazette, /tales, /battles, /upcoming, /hall and the rest
  // went unrecorded. The layout survives every client-side navigation, so one
  // effect keyed on the route covers every room at once, now and later.
  //
  // `enterRoom` ignores a repeated path itself (a filter rewriting the query
  // string is not a new visit) and steps aside on routes that report
  // themselves — the figurine page sends `figurine_view`.
  // No cleanup returned on purpose: this effect re-runs on every `page.url`
  // change, query string included, and a cleanup would close the room before
  // `enterRoom` could see that the path is the same one. The room is closed by
  // `onDestroy` below and by the instance's own `pagehide` listener.
  $effect(() => {
    enterRoom(page.url.pathname, page.route.id);
  });

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

  let stopFontLoader: (() => void) | null = null;

  onMount(() => {
    stopFontLoader = loadSiteFonts();

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
    leaveRoom();
    stopFontLoader?.();
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
    const nudgePlateRedraw = () => {
      requestAnimationFrame(() => {
        requestAnimationFrame(() => window.dispatchEvent(new Event('gotiga:redraw')));
      });
    };

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
          nudgePlateRedraw();
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
        const transition = vtDocument.startViewTransition(async () => {
          resolve();
          await navigation.complete;
        });
        // Card→detail morph can leave the WebGL plate holding a discarded buffer.
        transition.finished.finally(nudgePlateRedraw);
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
        nudgePlateRedraw();
      });
    });
  });
</script>

<svelte:head>
  <link rel="canonical" href={canonicalUrl} />
  <!-- Theme override CSS is applied via injectStyle in the script above (safe textContent),
       NOT emitted here as {@html <style>} — see the effect's comment. -->
</svelte:head>

<div class="min-h-screen bg-[#f8f1e7]">
  {#if showSiteHeader}
    <SiteHeader />
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
