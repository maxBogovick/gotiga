<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { fade } from 'svelte/transition';
  import { invalidateAll } from '$app/navigation';
  import DustParticles from '$lib/components/DustParticles.svelte';
  import FigurineDetailView from '$lib/components/FigurineDetailView.svelte';
  import NotFound from '$lib/components/NotFound.svelte';
  import { t , brandName } from '$lib/i18n';

  let { data } = $props();
  let figurine = $derived(data.figurine);

  onMount(() => {
    const fid = data.figurine?.id;
    if (!fid) return;
    try {
      const VIEWED_KEY = 'gotiga_viewed';
      const viewed: string[] = JSON.parse(localStorage.getItem(VIEWED_KEY) ?? '[]');
      const next = [fid, ...viewed.filter((id) => id !== fid)].slice(0, 50);
      localStorage.setItem(VIEWED_KEY, JSON.stringify(next));
    } catch {}
  });
  let id = $derived(page.params.id ?? '');

  // Foxing-on-stillness: the longer a reader rests on a leaf without moving, the
  // more the parchment slowly develops faint age-stains — as if attention itself
  // ages the paper. Blooms in slowly, clears quickly on any activity, and resets
  // whenever a new work is opened. Rewards exactly the stillness the place is for.
  let aged = $state(false);
  let idleTimer: ReturnType<typeof setTimeout> | null = null;

  function bumpStillness() {
    aged = false;
    if (idleTimer) { clearTimeout(idleTimer); idleTimer = null; }
    if (typeof window === 'undefined') return;
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    idleTimer = setTimeout(() => (aged = true), 3500);
  }

  // Restart the dwell timer on mount and on every navigation to a different work.
  $effect(() => {
    id;
    bumpStillness();
  });

  onMount(() => {
    const events = ['scroll', 'pointermove', 'pointerdown', 'keydown', 'wheel', 'touchstart'] as const;
    const opts: AddEventListenerOptions = { passive: true };
    events.forEach((e) => window.addEventListener(e, bumpStillness, opts));
    return () => {
      events.forEach((e) => window.removeEventListener(e, bumpStillness, opts));
      if (idleTimer) clearTimeout(idleTimer);
    };
  });

  // Absolute, public-origin URL for OG image. Crawlers fetch this from the static
  // HTML, so it must point at the live site — never at the build-time API host
  // (the backend returns absolute URLs bound to whatever host built the page, e.g.
  // localhost). We keep only the asset path and re-root it on page.url.origin, which
  // is the public origin at prerender time (kit.prerender.origin) and the real site
  // origin in the browser. nginx serves /static + /api at the same origin.
  let ogImage = $derived(() => {
    const img = figurine?.images?.find((i: { imageType: string }) => i.imageType === 'face')?.url
      ?? figurine?.images?.[0]?.url ?? '';
    if (!img) return '';
    if (img.startsWith('http')) {
      try {
        const p = new URL(img);
        return `${page.url.origin}${p.pathname}${p.search}`;
      } catch {
        return img;
      }
    }
    return `${page.url.origin}${img}`;
  });

  // Inject per-figurine view-transition timing via DOM (can't use {@html} with </style> in svelte:head)
  $effect(() => {
    const s = document.createElement('style');
    s.dataset.vtId = id;
    s.textContent =
      `::view-transition-image-pair(figurine-${id}){isolation:isolate}` +
      `::view-transition-old(figurine-${id}),::view-transition-new(figurine-${id})` +
      `{animation-duration:500ms;animation-timing-function:cubic-bezier(0.16,1,0.3,1)}` +
      `@media(prefers-reduced-motion:reduce){::view-transition-old(figurine-${id}),` +
      `::view-transition-new(figurine-${id}){animation:none!important}}`;
    document.head.appendChild(s);
    return () => s.remove();
  });

  function propertyValue(name: string, value: string | null | undefined) {
    const text = value?.trim();
    return text ? { '@type': 'PropertyValue', name, value: text } : null;
  }

  // VisualArtwork (not Product/Offer) — this is a showcase, not a shop.
  let jsonLd = $derived(() => {
    const additionalProperty = [
      propertyValue('Passport number', figurine?.passportNumber),
      propertyValue('Edition', figurine?.edition),
      propertyValue('Created period', figurine?.createdPeriod),
      propertyValue('Care instructions', figurine?.careInstructions),
      propertyValue('Authenticity note', figurine?.authenticityNote),
      propertyValue('Included items', figurine?.includedItems),
    ].filter(Boolean);

    return JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'VisualArtwork',
      name: figurine?.name ?? '',
      description: figurine?.shortText ?? figurine?.fullDescription ?? '',
      image: ogImage(),
      url: page.url.href,
      creator: { '@type': 'Organization', name: $brandName, url: page.url.origin },
      ...(figurine?.material ? { artMedium: figurine.material } : {}),
      ...(figurine?.technique ? { artform: figurine.technique } : {}),
      ...(figurine?.year ? { dateCreated: String(figurine.year) } : {}),
      ...(figurine?.passportNumber ? { identifier: figurine.passportNumber } : {}),
      ...(figurine?.provenanceNote ? { provenance: figurine.provenanceNote } : {}),
      ...(additionalProperty.length ? { additionalProperty } : {}),
    });
  });

  // Breadcrumb trail — Google renders these in the result snippet (Home › Archive › name).
  let breadcrumbJsonLd = $derived(() => JSON.stringify({
    '@context': 'https://schema.org',
    '@type': 'BreadcrumbList',
    itemListElement: [
      { '@type': 'ListItem', position: 1, name: $brandName, item: page.url.origin },
      { '@type': 'ListItem', position: 2, name: 'Archive', item: `${page.url.origin}/figurines` },
      { '@type': 'ListItem', position: 3, name: figurine?.name ?? 'Miniature', item: page.url.href },
    ],
  }));
</script>

<svelte:head>
  <title>{figurine?.name ?? $t('zoneShowcase')} — {$brandName}</title>
  <meta name="description" content={figurine?.shortText ?? figurine?.fullDescription ?? 'Gothic handcrafted miniature'} />

  <!-- Open Graph -->
  <meta property="og:type"        content="article" />
  <meta property="og:site_name"   content={$brandName} />
  <meta property="og:locale"      content="en_US" />
  <meta property="og:title"       content="{figurine?.name ?? 'Gothic Miniature'} — {$brandName}" />
  <meta property="og:description" content={figurine?.shortText ?? figurine?.fullDescription ?? ''} />
  {#if ogImage()}
    <meta property="og:image" content={ogImage()} />
    <meta property="og:image:width"  content="1800" />
    <meta property="og:image:height" content="1800" />
  {/if}
  <meta property="og:url" content={page.url.href} />
  <!-- canonical is emitted globally in +layout.svelte; no per-page duplicate -->


  <!-- Twitter / X -->
  <meta name="twitter:card"        content="summary_large_image" />
  <meta name="twitter:title"       content="{figurine?.name ?? 'Gothic Miniature'} — {$brandName}" />
  <meta name="twitter:description" content={figurine?.shortText ?? ''} />
  {#if ogImage()}
    <meta name="twitter:image" content={ogImage()} />
  {/if}

  <!-- view-transition timing injected via $effect in script -->

  <!-- JSON-LD: VisualArtwork + breadcrumb trail -->
  {#if figurine}
    {@html `<script type="application/ld+json">${jsonLd()}<\/script>`}
    {@html `<script type="application/ld+json">${breadcrumbJsonLd()}<\/script>`}
  {/if}

  <!-- Fonts loaded once globally in app.html -->
</svelte:head>

<div class="fixed inset-0 bg-cabinet-bg -z-50"></div>
<div class="fixed inset-0 pointer-events-none z-0 bg-noise opacity-[0.08] mix-blend-overlay"></div>
<div class="fixed inset-0 pointer-events-none z-0 detail-backdrop"></div>
<div
  class="fixed inset-0 pointer-events-none z-0 foxing"
  style:opacity={aged ? 0.85 : 0}
  style:transition-duration={aged ? '7000ms' : '650ms'}
></div>

<DustParticles />

{#if data.loadError}
  <!-- Backend unreachable — distinct from a genuine 404 (see +page.ts) -->
  <div class="min-h-screen flex flex-col items-center justify-center p-8 text-center max-w-md mx-auto" in:fade>
    <h1 class="font-['Fraunces'] text-4xl text-[#6f3b24] mb-4 opacity-85">{$t('loadErrorTitle')}</h1>
    <div class="w-24 h-px bg-gradient-to-r from-transparent via-[#34251c]/30 to-transparent mx-auto mb-6" aria-hidden="true"></div>
    <p class="text-[#34251c] text-sm tracking-wide leading-relaxed mb-8">{$t('loadErrorHint')}</p>
    <div class="flex items-center gap-4">
      <button
        onclick={() => invalidateAll()}
        class="px-8 py-3 border border-[#34251c]/25 hover:border-[#34251c]/55 text-[#5f4636] hover:text-[#34251c] text-xs tracking-[0.08em] uppercase transition-all duration-500"
      >
        {$t('loadErrorRetry')}
      </button>
      <a href="/figurines" class="text-xs tracking-[0.08em] uppercase text-[#7c6554] hover:text-[#34251c] underline underline-offset-4 transition-colors">
        {$t('figurineBackToArchive')}
      </a>
    </div>
  </div>
{:else if !figurine}
  <!-- No such work (404) — load() resolves before render, so a null figurine here
       means the backend returned not-found, not a loading state. -->
  <NotFound backHref="/figurines" backLabel={$t('figurineBackToArchive')} />
{:else}
  {#key id}
    <FigurineDetailView {figurine} {id} prev={data.prev ?? null} next={data.next ?? null} />
  {/key}
{/if}

<style>
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }

  .detail-backdrop {
    background:
      radial-gradient(ellipse 70% 55% at 72% 38%, color-mix(in srgb, var(--color-ember) 7%, transparent) 0%, transparent 65%),
      radial-gradient(ellipse 50% 70% at 18% 72%, color-mix(in srgb, var(--color-ochre) 6%, transparent) 0%, transparent 60%),
      var(--color-canvas-base);
  }

  /* Foxing: scattered, soft sepia blotches multiplied onto the parchment. Sits
     behind all content (z-0), so it tints only the paper — never the text. The
     slow/fast bloom is driven by transition-duration toggled from the script. */
  .foxing {
    background-image:
      radial-gradient(circle at 12% 22%, rgba(120, 70, 30, 0.11), transparent 8%),
      radial-gradient(circle at 79% 14%, rgba(118, 66, 28, 0.09), transparent 6%),
      radial-gradient(circle at 64% 73%, rgba(110, 60, 28, 0.10), transparent 7%),
      radial-gradient(circle at 28% 85%, rgba(120, 70, 30, 0.08), transparent 6%),
      radial-gradient(circle at 90% 58%, rgba(110, 60, 28, 0.07), transparent 6%),
      radial-gradient(circle at 45% 40%, rgba(120, 70, 30, 0.06), transparent 9%),
      radial-gradient(circle at 7% 62%, rgba(112, 62, 26, 0.07), transparent 5%),
      radial-gradient(circle at 53% 9%, rgba(118, 66, 28, 0.06), transparent 5%);
    mix-blend-mode: multiply;
    transition-property: opacity;
    transition-timing-function: ease;
  }

  @media (prefers-reduced-motion: reduce) {
    .foxing {
      display: none;
    }
  }
</style>
