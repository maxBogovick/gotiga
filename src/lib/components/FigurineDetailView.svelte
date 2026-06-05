<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';
  import type { Figurine, FigurineSchedule } from '$lib/types/api';
  import OrderModal from '$lib/components/OrderModal.svelte';
  import BookingModal from '$lib/components/BookingModal.svelte';
  import BrassLens from '$lib/components/BrassLens.svelte';
  import CandleReveal from '$lib/components/CandleReveal.svelte';
  import MemoryMirror from '$lib/components/MemoryMirror.svelte';
  import SecretText from '$lib/components/SecretText.svelte';
  import Lightbox from '$lib/components/Lightbox.svelte';
  import { get } from 'svelte/store';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import ShowingsTimeline from '$lib/components/ShowingsTimeline.svelte';

  import type { FigurineListItem } from '$lib/types/api';

  let {
    figurine,
    id,
    prev = null,
    next = null,
  }: {
    figurine: Figurine;
    id: string;
    prev?: FigurineListItem | null;
    next?: FigurineListItem | null;
  } = $props();

  let selectedImageIndex = $state(0);
  let isGrimoireOpen = $state(false);
  let showOrderModal = $state(false);
  let showBookingModal = $state(false);
  let orderMode = $state<'request' | 'question' | 'notify'>('request');
  let figurineSchedule = $state<FigurineSchedule>({ entries: [] });
  let isAudioPlaying = $state(false);
  let isCandleLit = $state(false);
  let showLightbox = $state(false);
  let lightboxStartIndex = $state(0);
  let audioRef = $state<HTMLAudioElement | null>(null);
  let videoRef = $state<HTMLVideoElement | null>(null);

  let upcomingShowings = $derived(figurineSchedule.entries.filter(e => e.entryType === 'showing'));

  // Nearest date when figurine is fully free (after all showings + confirmed bookings)
  let nextAvailableDate = $derived.by(() => {
    const today = new Date().toISOString().split('T')[0];
    const blocking = figurineSchedule.entries.filter(e => e.entryType === 'showing' || e.entryType === 'booking');
    if (blocking.length === 0) return null;
    const latestEnd = blocking.reduce((max, e) => e.endsAt > max ? e.endsAt : max, today);
    if (latestEnd < today) return null;
    const d = new Date(latestEnd + 'T00:00:00');
    d.setDate(d.getDate() + 1);
    return d;
  });

  // Showing that is happening TODAY (started but not yet ended)
  let todayStr = new Date().toISOString().split('T')[0];
  let hasActiveShowing = $derived(upcomingShowings.some(s => s.startsAt <= todayStr));

  // Confirmed bookings visible in schedule (entryType === 'booking')
  let upcomingBookings = $derived(figurineSchedule.entries.filter(e => e.entryType === 'booking'));

  // === CLAIM TOKEN (self-cancellation) ===
  type ClaimStatus = 'pending' | 'confirmed' | 'rejected' | 'cancelled';
  type ClaimData = { token: string; figurineName: string; startsAt: string; endsAt: string; submittedAt: string; status?: ClaimStatus };

  const CLAIMS_KEY = () => `gotiga_claims_${figurine.id}`;

  let claims          = $state<ClaimData[]>([]);
  let cancellingToken = $state<string | null>(null);
  let claimErrors     = $state<Record<string, string>>({});
  let cancelledTokens = $state<Set<string>>(new Set());

  // Manual token lookup form
  let showTokenForm   = $state(false);
  let tokenInput      = $state('');
  let tokenLookupInfo = $state<{ figurineName: string; startsAt: string; endsAt: string; status: string } | null>(null);
  let tokenLookupErr  = $state('');
  let tokenLooking    = $state(false);
  let lookupCancelling = $state(false);

  function loadClaims() {
    try {
      const raw = localStorage.getItem(CLAIMS_KEY());
      if (raw) claims = JSON.parse(raw);
      // Migrate old single-item format
      const old = localStorage.getItem(`gotiga_claim_${figurine.id}`);
      if (old) {
        const parsed = JSON.parse(old) as ClaimData;
        if (!claims.some(c => c.token === parsed.token)) claims = [parsed, ...claims];
        localStorage.removeItem(`gotiga_claim_${figurine.id}`);
        saveClaims();
      }
    } catch { /* ignore */ }
  }

  function saveClaims() {
    try { localStorage.setItem(CLAIMS_KEY(), JSON.stringify(claims)); } catch { /* ignore */ }
  }

  // Sync claim statuses with server — show confirmed, drop cancelled/rejected
  async function verifyClaims() {
    if (claims.length === 0) return;
    const results = await Promise.allSettled(claims.map(c => api.getBookingByToken(c.token)));
    let changed = false;
    const updated = claims
      .map((c, i) => {
        const r = results[i];
        if (r.status !== 'fulfilled') return c;
        const serverStatus = r.value.status as ClaimStatus;
        if (serverStatus !== c.status) { changed = true; return { ...c, status: serverStatus }; }
        return c;
      })
      .filter(c => c.status !== 'cancelled' && c.status !== 'rejected');
    if (changed || updated.length !== claims.length) {
      claims = updated;
      saveClaims();
    }
  }

  // Called by BookingModal immediately after submission — no reload needed
  function onBookingCreated(claim: ClaimData) {
    claims = [claim, ...claims];
    saveClaims();
    api.getFigurineSchedule(figurine.id).then(s => { figurineSchedule = s; });
  }

  async function cancelClaim(claim: ClaimData) {
    cancellingToken = claim.token;
    claimErrors = { ...claimErrors, [claim.token]: '' };
    try {
      await api.cancelBookingByToken(claim.token);
      cancelledTokens = new Set([...cancelledTokens, claim.token]);
      claims = claims.filter(c => c.token !== claim.token);
      saveClaims();
      api.getFigurineSchedule(figurine.id).then(s => { figurineSchedule = s; });
      setTimeout(() => { cancelledTokens = new Set([...cancelledTokens].filter(t => t !== claim.token)); }, 4000);
    } catch {
      claimErrors = { ...claimErrors, [claim.token]: get(t)('claimCancelError') };
    } finally {
      cancellingToken = null;
    }
  }

  async function lookupToken() {
    const tok = tokenInput.trim().toUpperCase();
    if (!tok) return;
    tokenLooking = true;
    tokenLookupErr = '';
    tokenLookupInfo = null;
    try {
      const info = await api.getBookingByToken(tok);
      tokenLookupInfo = info;
    } catch {
      tokenLookupErr = get(t)('claimTokenNotFound');
    } finally {
      tokenLooking = false;
    }
  }

  async function cancelFromLookup() {
    if (!tokenLookupInfo) return;
    lookupCancelling = true;
    try {
      await api.cancelBookingByToken(tokenInput.trim().toUpperCase());
      tokenLookupInfo = { ...tokenLookupInfo, status: 'cancelled' };
      api.getFigurineSchedule(figurine.id).then(s => { figurineSchedule = s; });
    } catch {
      tokenLookupErr = get(t)('claimCancelError');
    } finally {
      lookupCancelling = false;
    }
  }

  function fmtDate(ds: string) {
    return new Date(ds + 'T00:00:00').toLocaleDateString(undefined, { day: 'numeric', month: 'long', year: 'numeric' });
  }

  let sortedImages = $derived(
    figurine.images.slice().sort((a, b) => {
      if (a.imageType === 'face') return -1;
      if (b.imageType === 'face') return 1;
      return 0;
    })
  );

  let currentImage = $derived(sortedImages[selectedImageIndex]);
  let lightboxImages = $derived(
    sortedImages.map((img) => ({ url: resolveUrl(img.originalUrl ?? img.url), alt: img.altText ?? '' }))
  );

  function resolveUrl(path: string | undefined | null) { return path ?? ''; }
  function selectImage(index: number) { if (index !== selectedImageIndex) selectedImageIndex = index; }
  function openLightbox(index: number) { lightboxStartIndex = index; showLightbox = true; }
  function toggleGrimoire() { isGrimoireOpen = !isGrimoireOpen; }
  function toggleCandle() { isCandleLit = !isCandleLit; }

  function toggleFullscreen() {
    if (!videoRef) return;
    document.fullscreenElement ? document.exitFullscreen() : videoRef.requestFullscreen().catch(() => {});
  }

  function toggleAudio() {
    if (!audioRef || !figurine.ambiencePath) return;
    isAudioPlaying ? fadeOutAudio() : (audioRef.volume = 0, audioRef.play().catch(console.error), isAudioPlaying = true, fadeInAudio());
  }

  function fadeInAudio() {
    if (!audioRef) return;
    let vol = 0;
    const iv = setInterval(() => { vol < 0.5 ? (vol += 0.05, audioRef!.volume = vol) : clearInterval(iv); }, 100);
  }

  function fadeOutAudio() {
    if (!audioRef) return;
    let vol = audioRef.volume;
    const iv = setInterval(() => {
      vol > 0.05 ? (vol -= 0.05, audioRef!.volume = vol) : (clearInterval(iv), audioRef!.pause(), isAudioPlaying = false);
    }, 100);
  }

  // ── Share ────────────────────────────────────────────────────────────────
  let copied = $state(false);
  let copiedTimer: ReturnType<typeof setTimeout>;

  async function share() {
    const url = window.location.href;
    if (navigator.share) {
      await navigator.share({ title: figurine.name, text: figurine.shortText ?? figurine.name, url })
        .catch(() => {});
    } else {
      await navigator.clipboard.writeText(url).catch(() => {});
      copied = true;
      clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => { copied = false; }, 2200);
    }
  }

  // ── Keyboard gallery navigation ───────────────────────────────────────────
  function handleKeydown(e: KeyboardEvent) {
    if (showLightbox || showOrderModal) return;
    if (e.key === 'ArrowLeft')  selectImage(Math.max(0, selectedImageIndex - 1));
    if (e.key === 'ArrowRight') selectImage(Math.min(sortedImages.length - 1, selectedImageIndex + 1));
  }

  // ── Sticky condensed nav — три фазы ─────────────────────────────────────
  let scrollY = $state(0);
  let scrolled    = $derived(scrollY > 80);

  // DOM-якоря для определения выхода секций из viewport
  let galleryRef:  HTMLElement | undefined = $state();
  let grimoireRef: HTMLElement | undefined = $state();

  let galleryExited  = $state(false); // Phase 2: галерея ушла за экран
  let grimoireExited = $state(false); // Phase 3: grimoire ушёл за экран

  // ── Wishlist ──────────────────────────────────────────────────────────────
  let isWishlisted = $state(false);

  function toggleWishlist() {
    const stored = JSON.parse(localStorage.getItem('gotiga_wishlist') ?? '[]') as string[];
    const set = new Set(stored);
    isWishlisted ? set.delete(figurine.id) : set.add(figurine.id);
    localStorage.setItem('gotiga_wishlist', JSON.stringify([...set]));
    isWishlisted = !isWishlisted;
  }

  function openModal(m: 'request' | 'question' | 'notify') {
    orderMode = m;
    showOrderModal = true;
  }

  function onScroll() {
    scrollY = window.scrollY;
    const threshold = 130; // высота SiteHeader + topnav
    if (galleryRef) {
      galleryExited = galleryRef.getBoundingClientRect().bottom < threshold;
    }
    if (grimoireRef) {
      grimoireExited = grimoireRef.getBoundingClientRect().bottom < threshold;
    }
  }

  function handleVisibility() {
    if (document.visibilityState === 'visible') verifyClaims();
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
    window.addEventListener('scroll', onScroll, { passive: true });
    document.addEventListener('visibilitychange', handleVisibility);
    const wl = JSON.parse(localStorage.getItem('gotiga_wishlist') ?? '[]') as string[];
    isWishlisted = wl.includes(figurine.id);
    api.getFigurineSchedule(figurine.id).then(s => { figurineSchedule = s; });
    loadClaims();
    verifyClaims();
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
    window.removeEventListener('scroll', onScroll);
    document.removeEventListener('visibilitychange', handleVisibility);
    clearTimeout(copiedTimer);
    if (audioRef) { audioRef.pause(); audioRef = null; }
  });
</script>

{#if figurine.ambiencePath}
  <audio bind:this={audioRef} src={resolveUrl(figurine.ambiencePath)} loop></audio>
{/if}

<CandleReveal isActive={isCandleLit} />

<div class="page-root" class:page-root--has-cta={figurine.status === 'available'}>
  <OrderModal
    isOpen={showOrderModal}
    mode={orderMode}
    figurineName={figurine.name}
    figurineId={figurine.id}
    schedule={figurineSchedule}
    onClose={() => (showOrderModal = false)}
  />

  <BookingModal
    isOpen={showBookingModal}
    {onBookingCreated}
    figurineName={figurine.name}
    figurineId={figurine.id}
    schedule={figurineSchedule}
    onClose={() => (showBookingModal = false)}
  />

  {#if showLightbox}
    <Lightbox images={lightboxImages} startIndex={lightboxStartIndex} onClose={() => (showLightbox = false)} />
  {/if}

  <div class="page-container">

    <!-- ── NAV ── -->
    <nav class="topnav" class:topnav--scrolled={scrolled} in:fade={{ duration: 600 }}>

      <!-- Left: back + prev/next -->
      <div class="topnav-left">
        <a href="/figurines" class="back-link" aria-label={$t('figurineBackToArchive')}>
          <svg class="back-arrow" width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M9 2.5L4.5 7 9 11.5"/>
          </svg>
          <span class="back-label">{$t('figurineBackToArchive')}</span>
        </a>

        {#if prev || next}
          <div class="topnav-fig-nav" role="group" aria-label="Figurine navigation">
            {#if prev}
              <a
                href="/figurines/{prev.id}"
                class="fig-nav-pill"
                title={prev.name}
                aria-label="{$t('figurineNavPrev')}: {prev.name}"
                data-sveltekit-preload-data="hover"
              >
                <svg class="fig-nav-arrow" width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.6">
                  <path d="M6.5 2L3.5 5 6.5 8"/>
                </svg>
                <span class="fig-nav-name">{prev.name}</span>
              </a>
            {:else}
              <span class="fig-nav-pill fig-nav-pill--off" aria-hidden="true">
                <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.6">
                  <path d="M6.5 2L3.5 5 6.5 8"/>
                </svg>
              </span>
            {/if}
            {#if next}
              <a
                href="/figurines/{next.id}"
                class="fig-nav-pill fig-nav-pill--next"
                title={next.name}
                aria-label="{$t('figurineNavNext')}: {next.name}"
                data-sveltekit-preload-data="hover"
              >
                <span class="fig-nav-name">{next.name}</span>
                <svg class="fig-nav-arrow" width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.6">
                  <path d="M3.5 2l3 3L3.5 8"/>
                </svg>
              </a>
            {:else}
              <span class="fig-nav-pill fig-nav-pill--off fig-nav-pill--next" aria-hidden="true">
                <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.6">
                  <path d="M3.5 2l3 3L3.5 8"/>
                </svg>
              </span>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Center: прогрессивная идентификация (3 фазы) -->
      <div class="topnav-center" aria-hidden="true">

        <!-- Phase 1: только имя -->
        {#if scrolled && !galleryExited}
          <span class="topnav-p1-name" transition:fade={{ duration: 220 }}>
            {figurine.name}
          </span>
        {/if}

        <!-- Phase 2+: полная идентичность с галереей и годом -->
        {#if galleryExited}
          <div class="topnav-identity" transition:fade={{ duration: 280 }}>
            {#if currentImage?.url}
              <button
                class="topnav-mini-img"
                onclick={() => openLightbox(selectedImageIndex)}
                tabindex="-1"
                title="Open in fullscreen"
              >
                <img src={resolveUrl(currentImage.url)} alt="" loading="eager" />
              </button>
            {/if}

            <span class="topnav-ident-name">{figurine.name}</span>

            {#if sortedImages.length > 1}
              <div class="topnav-dots" role="group" aria-label="Gallery navigation">
                {#each sortedImages as _, i}
                  <button
                    class="topnav-dot {i === selectedImageIndex ? 'topnav-dot--on' : ''}"
                    onclick={() => selectImage(i)}
                    tabindex="-1"
                    aria-label="Image {i + 1}"
                  ></button>
                {/each}
              </div>
            {/if}

            {#if figurine.year}
              <span class="topnav-ident-year">Anno {figurine.year}</span>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Right: controls -->
      <div class="topnav-controls">
        <button
          onclick={toggleCandle}
          class="control-btn {isCandleLit ? 'control-btn--lit' : ''}"
          aria-label={isCandleLit ? $t('figurineExtinguish') : $t('figurineCandle')}
          title={isCandleLit ? $t('figurineExtinguish') : $t('figurineCandle')}
        >
          <span class="control-icon">{isCandleLit ? '🔥' : '🕯️'}</span>
          <span class="btn-label">{isCandleLit ? $t('figurineExtinguish') : $t('figurineCandle')}</span>
        </button>

        <button
          onclick={toggleWishlist}
          class="control-btn {isWishlisted ? 'control-btn--lit' : ''}"
          aria-label={isWishlisted ? $t('figurineWishlisted') : $t('figurineWishlist')}
          title={isWishlisted ? $t('figurineWishlisted') : $t('figurineWishlist')}
        >
          <svg width="13" height="12" viewBox="0 0 13 12" fill={isWishlisted ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="1.4">
            <path d="M6.5 10.5S1 7 1 3.5A2.5 2.5 0 0 1 6.5 2 2.5 2.5 0 0 1 12 3.5C12 7 6.5 10.5 6.5 10.5z"/>
          </svg>
          <span class="btn-label">{isWishlisted ? $t('figurineWishlisted') : $t('figurineWishlist')}</span>
        </button>

        {#if figurine.ambiencePath}
          <button
            onclick={toggleAudio}
            class="control-btn {isAudioPlaying ? 'control-btn--active' : ''}"
            aria-label={isAudioPlaying ? $t('figurineSilence') : $t('figurineWhisper')}
            title={isAudioPlaying ? $t('figurineSilence') : $t('figurineWhisper')}
          >
            <span class="audio-indicator {isAudioPlaying ? 'audio-indicator--on' : ''}"></span>
            <span class="btn-label">{isAudioPlaying ? $t('figurineSilence') : $t('figurineWhisper')}</span>
          </button>
        {/if}

        <!-- Phase 2+: кнопка Memory Mirror появляется когда галерея ушла -->
        {#if figurine.processSteps && figurine.processSteps.length > 0 && galleryExited}
          <button
            onclick={() => { isGrimoireOpen = true; }}
            class="control-btn control-btn--eye {isGrimoireOpen ? 'control-btn--active' : ''} {grimoireExited && !isGrimoireOpen ? 'control-btn--eye-pulse' : ''}"
            aria-label={$t('figurineGrimoire')}
            title={$t('figurineGrimoire')}
            transition:fade={{ duration: 200 }}
          >
            <svg width="12" height="14" viewBox="0 0 12 14" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="2.5" y="2" width="7" height="10" rx="1.2"/>
              <path d="M2.5 2A1.5 1.5 0 0 0 1 3.5v7A1.5 1.5 0 0 0 2.5 12"/>
              <path d="M9.5 2A1.5 1.5 0 0 1 11 3.5v7A1.5 1.5 0 0 1 9.5 12"/>
              <path d="M4.5 5h3M4.5 7h3M4.5 9h1.5"/>
            </svg>
            <span class="btn-label">{$t('figurineGrimoire')}</span>
          </button>
        {/if}

        <button
          onclick={share}
          class="control-btn {copied ? 'control-btn--active' : ''}"
          aria-label={$t('figurineShare')}
          title={$t('figurineShare')}
        >
          {#if copied}
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M2 6l3 3 5-5"/>
            </svg>
            <span class="btn-label">{$t('figurineCopied')}</span>
          {:else}
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M9 1.5a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3zM3 4.5a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3zM9 7.5a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3z"/>
              <path d="M7.5 2.7l-3 1.8M7.5 9.3l-3-1.8"/>
            </svg>
            <span class="btn-label">{$t('figurineShare')}</span>
          {/if}
        </button>

        <!-- Phase 2+: компактная CTA в хидере -->
        {#if galleryExited && figurine.status === 'available'}
          <button
            onclick={() => openModal('request')}
            class="control-btn"
            aria-label={$t('figurineRequest')}
            transition:fade={{ duration: 200 }}
          >
            <svg width="11" height="12" viewBox="0 0 14 15" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
              <path d="M4.5 5V3.5a2.5 2.5 0 0 1 5 0V5"/>
              <rect x="2" y="5" width="10" height="8.5" rx="1.2"/>
            </svg>
            <span class="btn-label">{$t('figurineRequest')}</span>
          </button>
        {/if}

        <span class="ref-tag">ARC-{id.toUpperCase()}</span>
      </div>
    </nav>

    <!-- ── MAIN GRID ── -->
    <div class="main-grid">

      <!-- LEFT: Gallery with vertical thumbnail strip -->
      <div class="gallery-col" bind:this={galleryRef}>
        <div class="gallery-layout" class:gallery-layout--solo={sortedImages.length <= 1}>

          {#if sortedImages.length > 1}
            <nav class="thumbs-strip" aria-label="Gallery thumbnails">
              {#each sortedImages as img, i}
                <button
                  class="thumb-v {selectedImageIndex === i ? 'thumb-v--active' : ''}"
                  onclick={() => selectImage(i)}
                  aria-label="{$t('figurineShowView')} {i + 1}"
                  aria-current={selectedImageIndex === i ? 'true' : undefined}
                >
                  <img src={resolveUrl(img.thumbUrl ?? img.url)} alt="" class="thumb-v-img" loading="lazy" />
                  <div class="thumb-v-bar" aria-hidden="true"></div>
                </button>
              {/each}
            </nav>
          {/if}

          <div class="image-frame">
            <div class="image-stage" style="view-transition-name: figurine-{id}">
              {#key currentImage?.id}
                <div class="image-layer" in:fade={{ duration: 220 }}>
                  <BrassLens
                    src={currentImage?.url}
                    alt={figurine.name}
                    class="w-full h-full"
                    onOpenLightbox={() => openLightbox(selectedImageIndex)}
                  />
                </div>
              {/key}

              {#if sortedImages.length > 1}
                <div class="img-counter" aria-hidden="true">
                  {selectedImageIndex + 1}<span class="img-counter-sep">/</span>{sortedImages.length}
                </div>
              {/if}

              <button
                onclick={() => openLightbox(selectedImageIndex)}
                class="expand-btn"
                aria-label={$t('figurineFullscreen')}
              >
                <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5">
                  <path d="M1 4V1h3M6 1h3v3M9 6v3H6M4 9H1V6"/>
                </svg>
                {$t('figurineFullscreen')}
              </button>

              <div class="image-vignette"></div>
            </div>
          </div>
        </div>
      </div>

      <!-- RIGHT: Details — on warm page bg, no card wrapper -->
      <div class="details-col">

        {#if figurine.secretText}
          <div class="secret-anchor">
            <SecretText text={figurine.secretText} isCandleLit={isCandleLit} />
          </div>
        {/if}

        <!-- Eyebrow: year + status pill -->
        <div class="d-eyebrow">
          <div class="eyebrow-tags">
            {#if figurine.year}
              <span class="eyebrow-year">Anno {figurine.year}</span>
            {/if}
          </div>
          <span class="status-pill status-pill--{figurine.status}">
            {figurine.status === 'sold'
              ? $t('figurineStatusSold')
              : figurine.status === 'reserved'
                ? $t('figurineStatusReserved')
                : $t('figurineStatusAvailable')}
          </span>
        </div>

        <h1 class="figurine-title">{figurine.name}</h1>

        {#if figurine.shortText}
          <p class="lore-short">{figurine.shortText}</p>
        {/if}

        <!-- History FIRST — читаем перед решением -->
        {#if figurine.fullDescription}
          <div class="d-history">
            <header class="d-section-header">
              <span class="sec-label">{$t('figurineHistory')}</span>
              <div class="sec-rule" aria-hidden="true"></div>
            </header>
            <p class="history-body drop-cap">{figurine.fullDescription}</p>
          </div>
        {/if}

        <!-- Attributes — compact spec rows -->
        {#if figurine.dimensions || figurine.material || figurine.technique}
          <div class="d-attrs">
            <header class="d-section-header">
              <span class="sec-label">{$t('figurineAttributes')}</span>
              <div class="sec-rule" aria-hidden="true"></div>
            </header>
            <dl class="attrs-specs">
              {#if figurine.dimensions}
                <div class="spec-row">
                  <span class="spec-icon" aria-hidden="true">
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.2">
                      <rect x="0.7" y="4" width="12.6" height="6" rx="0.8"/>
                      <path d="M3 4V6.5M5.5 4V7.5M8 4V6.5M10.5 4V7.5"/>
                    </svg>
                  </span>
                  <dt class="spec-label">{$t('figurineDimensions')}</dt>
                  <dd class="spec-value">{figurine.dimensions}</dd>
                </div>
              {/if}
              {#if figurine.material}
                <div class="spec-row">
                  <span class="spec-icon" aria-hidden="true">
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.2">
                      <path d="M7 1L12.5 4.5v5L7 13 1.5 9.5v-5L7 1z"/>
                      <path d="M1.5 4.5h11"/>
                    </svg>
                  </span>
                  <dt class="spec-label">{$t('figurineMaterial')}</dt>
                  <dd class="spec-value">{figurine.material}</dd>
                </div>
              {/if}
              {#if figurine.technique}
                <div class="spec-row">
                  <span class="spec-icon" aria-hidden="true">
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.2">
                      <path d="M9 1.5l3.5 3.5L4.5 13H1v-3.5L9 1.5z"/>
                      <path d="M7.5 3l3 3"/>
                    </svg>
                  </span>
                  <dt class="spec-label">{$t('figurineTechnique')}</dt>
                  <dd class="spec-value">{figurine.technique}</dd>
                </div>
              {/if}
            </dl>
          </div>
        {/if}

        <!-- Timeline показов -->
        {#if figurineSchedule.entries.length > 0}
          <ShowingsTimeline schedule={figurineSchedule} />
        {/if}

        <!-- ── CLAIM TOKEN: user's bookings (shown only when figurine is still available) ── -->
        {#if figurine.status === 'available'}
        {#each claims as c (c.token)}
          {#if c.status === 'confirmed'}
            <div class="claim-block claim-block--confirmed">
              <div class="claim-head">
                <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.5">
                  <path d="M1.5 6.5l3.5 3.5 6.5-7"/>
                </svg>
                <span>{$t('claimConfirmed')}</span>
              </div>
              <div class="claim-row">
                <span class="claim-dates">{fmtDate(c.startsAt)} — {fmtDate(c.endsAt)}</span>
                <span class="claim-code-small">{c.token}</span>
              </div>
            </div>
          {:else}
            <div class="claim-block">
              <div class="claim-head">
                <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.3">
                  <rect x="0.5" y="1.5" width="12" height="11" rx="0.8"/>
                  <path d="M3.5 1.5V0.5M9.5 1.5V0.5M0.5 5h12"/>
                </svg>
                <span>{$t('claimPendingBooking')}</span>
              </div>
              <div class="claim-row">
                <span class="claim-dates">{fmtDate(c.startsAt)} — {fmtDate(c.endsAt)}</span>
                <span class="claim-code-small">{c.token}</span>
                {#if claimErrors[c.token]}
                  <p class="claim-err">{claimErrors[c.token]}</p>
                {/if}
                <button
                  onclick={() => cancelClaim(c)}
                  disabled={cancellingToken === c.token}
                  class="claim-cancel-btn"
                >{cancellingToken === c.token ? $t('claimCancelling') : $t('claimCancelBtn')}</button>
              </div>
            </div>
          {/if}
        {/each}

        {#if cancelledTokens.size > 0 && claims.length === 0}
          <div class="claim-block claim-block--done">
            <p class="claim-done">{$t('claimCancelDone')}</p>
          </div>
        {/if}

        <!-- Manual token entry — only while figurine is still available -->
        <div class="claim-lookup">
          {#if !showTokenForm}
            <button onclick={() => showTokenForm = true} class="claim-lookup-link">{$t('claimHaveCode')}</button>
          {:else}
            <div class="claim-lookup-form">
              <input type="text" bind:value={tokenInput} placeholder="XXXX-XXXX" maxlength="9"
                class="claim-lookup-input" oninput={() => { tokenLookupInfo = null; tokenLookupErr = ''; }} />
              <button onclick={lookupToken} disabled={tokenLooking} class="claim-lookup-btn">
                {tokenLooking ? '…' : $t('claimLookupBtn')}
              </button>
              <button onclick={() => { showTokenForm = false; tokenInput = ''; tokenLookupInfo = null; }} class="claim-lookup-close">✕</button>
            </div>
            {#if tokenLookupErr}<p class="claim-err">{tokenLookupErr}</p>{/if}
            {#if tokenLookupInfo}
              <div class="claim-lookup-result">
                <p class="claim-dates">{fmtDate(tokenLookupInfo.startsAt)} — {fmtDate(tokenLookupInfo.endsAt)}</p>
                {#if tokenLookupInfo.status === 'pending'}
                  <button onclick={cancelFromLookup} disabled={lookupCancelling} class="claim-cancel-btn">
                    {lookupCancelling ? $t('claimCancelling') : $t('claimCancelBtn')}
                  </button>
                {:else}
                  <p class="claim-status">{$t('claimStatus')}: {tokenLookupInfo.status}</p>
                {/if}
              </div>
            {/if}
          {/if}
        </div>
        {/if}
        <!-- end available-only claim section -->

        <!-- CTA at the bottom — после того как всё прочитано -->
        <div class="d-cta-zone">
          {#if figurine.status === 'available'}
            <!-- Showings block: shows when there are showings OR when there are bookings with no showings -->
            {#if upcomingShowings.length > 0}
              <div class="showing-block">
                <div class="showing-block-head">
                  <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3" class="flex-shrink-0">
                    <rect x="1" y="2" width="12" height="11" rx="1"/>
                    <path d="M4 2V0.5M10 2V0.5M1 5.5h12"/>
                  </svg>
                  <span>{hasActiveShowing ? $t('figurineActiveShowing') : $t('figurineShowingsBlock')}</span>
                </div>
                {#each upcomingShowings as s}
                  <p class="showing-block-entry">
                    <span class="showing-block-type">{s.showingType === 'exhibition' ? $t('bookingShowingExhibition') : $t('bookingShowingPrivate')}</span>
                    {#if s.title}«{s.title}»{/if}
                    — {new Date(s.startsAt + 'T00:00:00').toLocaleDateString(undefined, { day: '2-digit', month: 'short' })}
                    – {new Date(s.endsAt + 'T00:00:00').toLocaleDateString(undefined, { day: '2-digit', month: 'short', year: 'numeric' })}
                  </p>
                {/each}
                {#if nextAvailableDate}
                  <p class="showing-block-avail">
                    {$t('figurineAvailableFrom')}
                    <strong>{nextAvailableDate.toLocaleDateString(undefined, { day: 'numeric', month: 'long', year: 'numeric' })}</strong>
                  </p>
                {/if}
              </div>
            {:else if upcomingBookings.length > 0 && nextAvailableDate}
              <!-- Only bookings, no showings — show a compact availability note -->
              <div class="avail-note">
                <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.3" class="flex-shrink-0">
                  <rect x="0.5" y="1.5" width="12" height="11" rx="0.8"/>
                  <path d="M3.5 1.5V0.5M9.5 1.5V0.5M0.5 5h12"/>
                </svg>
                <span>
                  {$t('figurineAvailableFrom')}
                  <strong>{nextAvailableDate.toLocaleDateString(undefined, { day: 'numeric', month: 'long', year: 'numeric' })}</strong>
                </span>
              </div>
            {/if}

            <!-- Request button: blocked during active showings -->
            {#if hasActiveShowing}
              <div class="cta-row">
                <div class="cta-exhibition-block">
                  <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3">
                    <rect x="1" y="2" width="12" height="11" rx="1"/>
                    <path d="M4 2V0.5M10 2V0.5M1 5.5h12"/>
                  </svg>
                  <span>{$t('figurineTransferBlocked')}</span>
                </div>
                <button
                  onclick={toggleWishlist}
                  class="cta-heart {isWishlisted ? 'cta-heart--saved' : ''}"
                  aria-label={isWishlisted ? $t('figurineWishlisted') : $t('figurineWishlist')}
                  title={isWishlisted ? $t('figurineWishlisted') : $t('figurineWishlist')}
                >
                  <svg width="18" height="16" viewBox="0 0 18 16" fill={isWishlisted ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="1.5">
                    <path d="M9 14.5S1.5 9.5 1.5 5A3.5 3.5 0 0 1 9 2.8 3.5 3.5 0 0 1 16.5 5C16.5 9.5 9 14.5 9 14.5z"/>
                  </svg>
                </button>
              </div>
              <div class="cta-secondary-row">
                <button onclick={() => openModal('question')} class="cta-ask">{$t('figurineAskQuestion')}</button>
                <button onclick={() => openModal('notify')} class="cta-ask">{$t('figurineNotify')}</button>
              </div>
            {:else}
              <div class="cta-row">
                <button onclick={() => openModal('request')} class="cta-btn">
                  <span class="cta-btn-label">{$t('figurineRequest')}</span>
                  <svg class="cta-arrow" width="15" height="16" viewBox="0 0 14 15" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M4.5 5V3.5a2.5 2.5 0 0 1 5 0V5"/>
                    <rect x="2" y="5" width="10" height="8.5" rx="1.2"/>
                  </svg>
                </button>
                <button
                  onclick={toggleWishlist}
                  class="cta-heart {isWishlisted ? 'cta-heart--saved' : ''}"
                  aria-label={isWishlisted ? $t('figurineWishlisted') : $t('figurineWishlist')}
                  title={isWishlisted ? $t('figurineWishlisted') : $t('figurineWishlist')}
                >
                  <svg width="18" height="16" viewBox="0 0 18 16" fill={isWishlisted ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="1.5">
                    <path d="M9 14.5S1.5 9.5 1.5 5A3.5 3.5 0 0 1 9 2.8 3.5 3.5 0 0 1 16.5 5C16.5 9.5 9 14.5 9 14.5z"/>
                  </svg>
                </button>
              </div>
              <div class="cta-secondary-row">
                <p class="cta-note">{$t('figurineRequestNote')}</p>
                <button onclick={() => openModal('question')} class="cta-ask">
                  {$t('figurineAskQuestion')}
                </button>
              </div>
            {/if}

            <!-- Book button: always available when status is available -->
            <button onclick={() => (showBookingModal = true)} class="book-btn">
              <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.3">
                <rect x="0.5" y="1.5" width="12" height="11" rx="0.8"/>
                <path d="M3.5 1.5V0.5M9.5 1.5V0.5M0.5 5h12"/>
              </svg>
              {$t('figurineBook')}
            </button>

          {:else if figurine.status === 'reserved'}
            {#if claims.some(c => c.status === 'confirmed')}
              <!-- User's confirmed booking — show prominently -->
              {#each claims.filter(c => c.status === 'confirmed') as c (c.token)}
                <div class="reserved-notice reserved-notice--confirmed">
                  <svg class="reserved-icon" width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.8">
                    <path d="M3 9l4.5 4.5 8-8"/>
                  </svg>
                  <div>
                    <p class="reserved-title reserved-title--confirmed">{$t('claimConfirmed')}</p>
                    <p class="reserved-sub">{fmtDate(c.startsAt)} — {fmtDate(c.endsAt)}</p>
                    <p class="claim-code-small" style="margin-top:0.25rem">{c.token}</p>
                  </div>
                </div>
              {/each}
            {:else if claims.some(c => !c.status || c.status === 'pending')}
              <!-- User has a pending booking for this figurine -->
              {#each claims.filter(c => !c.status || c.status === 'pending') as c (c.token)}
                <div class="reserved-notice">
                  <svg class="reserved-icon" width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.3">
                    <circle cx="9" cy="9" r="7.5"/>
                    <path d="M9 5.5v3.5l2.5 2"/>
                  </svg>
                  <div>
                    <p class="reserved-title">{$t('claimPendingBooking')}</p>
                    <p class="reserved-sub">{fmtDate(c.startsAt)} — {fmtDate(c.endsAt)}</p>
                    <button onclick={() => cancelClaim(c)} disabled={cancellingToken === c.token}
                      class="claim-cancel-btn" style="margin-top:0.5rem">
                      {cancellingToken === c.token ? $t('claimCancelling') : $t('claimCancelBtn')}
                    </button>
                    {#if claimErrors[c.token]}<p class="claim-err">{claimErrors[c.token]}</p>{/if}
                  </div>
                </div>
              {/each}
            {:else}
              <!-- No token in this browser — generic reserved + lookup offer -->
              <div class="reserved-notice">
                <svg class="reserved-icon" width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.3">
                  <circle cx="9" cy="9" r="7.5"/>
                  <path d="M9 5.5v3.5l2.5 2"/>
                </svg>
                <div>
                  <p class="reserved-title">Зарезервирована</p>
                  <p class="reserved-sub">{$t('figurineNotifyNote')}</p>
                  {#if nextAvailableDate}
                    <p class="reserved-avail">
                      {$t('figurineAvailableFrom')}
                      <strong>{nextAvailableDate.toLocaleDateString(undefined, { day: 'numeric', month: 'long', year: 'numeric' })}</strong>
                    </p>
                  {/if}
                </div>
              </div>
              <!-- Token lookup in reserved state -->
              <div class="claim-lookup" style="margin-bottom:0.75rem">
                {#if !showTokenForm}
                  <button onclick={() => showTokenForm = true} class="claim-lookup-link">{$t('claimHaveCode')}</button>
                {:else}
                  <div class="claim-lookup-form">
                    <input type="text" bind:value={tokenInput} placeholder="XXXX-XXXX" maxlength="9"
                      class="claim-lookup-input" oninput={() => { tokenLookupInfo = null; tokenLookupErr = ''; }} />
                    <button onclick={lookupToken} disabled={tokenLooking} class="claim-lookup-btn">
                      {tokenLooking ? '…' : $t('claimLookupBtn')}
                    </button>
                    <button onclick={() => { showTokenForm = false; tokenInput = ''; tokenLookupInfo = null; }} class="claim-lookup-close">✕</button>
                  </div>
                  {#if tokenLookupErr}<p class="claim-err">{tokenLookupErr}</p>{/if}
                  {#if tokenLookupInfo}
                    <div class="claim-lookup-result">
                      <p class="claim-dates">{fmtDate(tokenLookupInfo.startsAt)} — {fmtDate(tokenLookupInfo.endsAt)}</p>
                      <p class="claim-status">{$t('claimStatus')}: <strong>{tokenLookupInfo.status}</strong></p>
                    </div>
                  {/if}
                {/if}
              </div>
              <button onclick={() => openModal('notify')} class="notify-btn">
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3">
                  <path d="M7 1a4 4 0 0 1 4 4v3l1.5 2H1.5L3 8V5a4 4 0 0 1 4-4z"/>
                  <path d="M5.5 11.5a1.5 1.5 0 0 0 3 0"/>
                </svg>
                {$t('figurineNotify')}
              </button>
            {/if}
          {:else}
            <div class="sold-notice">
              <p class="sold-text">{$t('figurineStatusSold')} — эта работа обрела своего хранителя.</p>
            </div>
            <button onclick={() => openModal('notify')} class="notify-btn">
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3">
                <path d="M7 1a4 4 0 0 1 4 4v3l1.5 2H1.5L3 8V5a4 4 0 0 1 4-4z"/>
                <path d="M5.5 11.5a1.5 1.5 0 0 0 3 0"/>
              </svg>
              {$t('figurineNotify')}
            </button>
          {/if}
        </div>

      </div>
    </div>

    <!-- ── GRIMOIRE ── -->
    {#if figurine.processSteps && figurine.processSteps.length > 0}
      <div class="grimoire-section" bind:this={grimoireRef}>
        <button onclick={toggleGrimoire} class="grimoire-trigger" aria-expanded={isGrimoireOpen}>
          <span class="grimoire-icon" aria-hidden="true">
            <svg width="22" height="22" viewBox="0 0 22 22" fill="none">
              <ellipse cx="11" cy="11" rx="9" ry="5.5" stroke="currentColor" stroke-width="1.2"/>
              <circle cx="11" cy="11" r="2.5" fill="currentColor" opacity="0.7"/>
              <circle cx="11" cy="11" r="1" fill="currentColor"/>
            </svg>
          </span>
          <span class="grimoire-body">
            <span class="grimoire-title">
              {$t('figurineGrimoire')}
              <span class="grimoire-dot"></span>
            </span>
            <span class="grimoire-sub">{figurine.processSteps.length} {$t('figurineGrimoireSub')}</span>
          </span>
          <span class="grimoire-arrow" aria-hidden="true">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"
              style="transform: rotate({isGrimoireOpen ? '90deg' : '0deg'}); transition: transform 0.3s ease">
              <path d="M3 8h10M9 4l4 4-4 4"/>
            </svg>
          </span>
        </button>
        <MemoryMirror
          isOpen={isGrimoireOpen}
          steps={figurine.processSteps}
          finalImage={resolveUrl(currentImage?.url)}
          onClose={() => (isGrimoireOpen = false)}
        />
      </div>
    {/if}

    <!-- ── VIDEO ── -->
    {#if figurine.videoUrl}
      <section class="video-section">
        <header class="section-row">
          <span class="sec-label">{$t('figurineVideo')}</span>
          <div class="sec-rule" aria-hidden="true"></div>
        </header>
        <div class="video-wrap group">
          <div class="video-frame card group">
            <span class="corner-tl"></span>
            <span class="corner-tr"></span>
            <span class="corner-bl"></span>
            <span class="corner-br"></span>
            <div class="video-stage">
              <video bind:this={videoRef} controls class="video-el"
                poster={resolveUrl(currentImage?.url)} preload="metadata">
                <source src={resolveUrl(figurine.videoUrl)} type="video/mp4" />
                {$t('figurineBrowserNoVideo')}
              </video>
              <button onclick={toggleFullscreen} class="video-fs-btn" title={$t('figurineFullscreen')}>
                <svg width="14" height="14" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5">
                  <path d="M1 4V1h3M6 1h3v3M9 6v3H6M4 9H1V6"/>
                </svg>
              </button>
            </div>
          </div>
        </div>
        <p class="video-caption text-label">{$t('figurineVideoFilm')}{id.slice(-3)}</p>
      </section>
    {/if}

    <!-- ── RELATED — горизонтальная кинолента ── -->
    {#if figurine.relatedItems && figurine.relatedItems.length > 0}
      <section class="related-section">
        <header class="section-row">
          <span class="sec-label">{$t('figurineRelated')}</span>
          <div class="sec-rule" aria-hidden="true"></div>
        </header>
        <div class="related-strip" role="list">
          {#each figurine.relatedItems as item}
            <a
              href="/figurines/{item.id}"
              class="related-card group"
              role="listitem"
              data-sveltekit-preload-data="hover"
            >
              <div class="related-img-wrap">
                <img
                  src={resolveUrl(item.faceImageUrl)}
                  alt={item.name}
                  class="related-img"
                  loading="lazy"
                />
                <div class="related-overlay" aria-hidden="true">
                  <span class="related-cta-hint">
                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
                      <path d="M1 6h10M7 2l4 4-4 4"/>
                    </svg>
                  </span>
                </div>
                <span class="related-status-badge related-status-badge--{item.status}">
                  {item.status === 'sold'
                    ? $t('figurineStatusSold')
                    : item.status === 'reserved'
                      ? $t('figurineStatusReserved')
                      : $t('figurineStatusAvailable')}
                </span>
              </div>
              <div class="related-meta">
                <h4 class="related-name">{item.name}</h4>
              </div>
            </a>
          {/each}
        </div>
      </section>
    {/if}

  </div>
</div>

<!-- Mobile sticky CTA — появляется при скролле на мобильных -->
{#if figurine.status === 'available' && scrollY > 300}
  <div class="mobile-cta" transition:fade={{ duration: 180 }}>
    <div class="mobile-cta-info">
      <span class="mobile-cta-name">{figurine.name}</span>
      <span class="mobile-cta-status">{$t('figurineStatusAvailable')}</span>
    </div>
    <button onclick={() => (showOrderModal = true)} class="mobile-cta-btn">
      {$t('figurineRequest')}
      <svg width="13" height="14" viewBox="0 0 14 15" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
        <path d="M4.5 5V3.5a2.5 2.5 0 0 1 5 0V5"/>
        <rect x="2" y="5" width="10" height="8.5" rx="1.2"/>
      </svg>
    </button>
  </div>
{/if}

<style>
  /* ── Page shell ── */
  .page-root {
    min-height: 100svh;
    background:
      radial-gradient(ellipse 80% 60% at 65% 20%, rgba(198,95,60,0.06) 0%, transparent 60%),
      radial-gradient(ellipse 60% 80% at 15% 75%, rgba(201,168,117,0.05) 0%, transparent 55%),
      #f8f1e7;
    color: #2c1710;
    padding-bottom: 8rem;
  }
  /* На мобильных: дополнительный отступ под sticky CTA-бар */
  @media (max-width: 1023px) {
    .page-root--has-cta { padding-bottom: calc(8rem + 68px); }
  }

  .page-container {
    max-width: 1280px;
    margin: 0 auto;
    padding: 2.5rem 1.5rem 0;
  }
  @media (min-width: 1024px) {
    .page-container { padding: 3rem 3.5rem 0; }
  }

  /* ── Top nav ── */
  .topnav {
    position: sticky;
    top: 68px;          /* высота SiteHeader */
    z-index: 40;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 3rem;
    padding: 0.6rem 0;
    /* плавный переход всех свойств */
    transition:
      background var(--duration-default) var(--ease-atelier),
      border-color var(--duration-default) var(--ease-atelier),
      padding var(--duration-default) var(--ease-atelier),
      box-shadow var(--duration-default) var(--ease-atelier);
  }

  /* condensed state — появляется при прокрутке */
  .topnav--scrolled {
    background: rgba(250, 246, 238, 0.90);
    backdrop-filter: blur(18px) saturate(1.4);
    -webkit-backdrop-filter: blur(18px) saturate(1.4);
    border-bottom: 1px solid var(--color-border-subtle);
    box-shadow: 0 1px 12px rgba(60, 25, 10, 0.06);
    padding: 0.5rem 1.5rem;
    /* выезжает за padding page-container чтобы растянуться на всю ширину */
    margin-left: -1.5rem;
    margin-right: -1.5rem;
  }
  @media (min-width: 1024px) {
    .topnav--scrolled {
      margin-left: -3.5rem;
      margin-right: -3.5rem;
      padding-left: 3.5rem;
      padding-right: 3.5rem;
    }
  }
  @media (max-width: 680px) {
    .topnav { top: 58px; }   /* высота мобильного SiteHeader */
  }

  /* ── Центральная зона — прогрессивная идентичность ── */
  .topnav-center {
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  /* Phase 1: просто имя */
  .topnav-p1-name {
    font-family: var(--font-display);
    font-size: 0.875rem;
    font-weight: 400;
    letter-spacing: 0.01em;
    color: var(--color-ink-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
  }

  /* Phase 2+: идентити-полоска */
  .topnav-identity {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
    overflow: hidden;
  }

  /* Мини-превью изображения */
  .topnav-mini-img {
    flex-shrink: 0;
    width: 22px;
    height: 28px;
    overflow: hidden;
    border-radius: 2px;
    border: 1px solid var(--color-border-subtle);
    cursor: pointer;
    background: var(--color-canvas-sunken);
    padding: 0;
    transition:
      border-color var(--duration-default) var(--ease-atelier),
      transform var(--duration-default) var(--ease-atelier);
  }
  .topnav-mini-img:hover {
    border-color: var(--color-border-default);
    transform: scale(1.1);
  }
  .topnav-mini-img img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 20%;
    display: block;
  }

  /* Имя в идентити */
  .topnav-ident-name {
    font-family: var(--font-display);
    font-size: 0.875rem;
    font-weight: 400;
    letter-spacing: 0.01em;
    color: var(--color-ink-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  /* Точки-индикаторы галереи */
  .topnav-dots {
    display: flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
  }

  .topnav-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    border: none;
    background: rgba(52,37,28,0.18);
    cursor: pointer;
    padding: 0;
    transition:
      background var(--duration-default) var(--ease-atelier),
      transform var(--duration-default) var(--ease-atelier);
  }
  .topnav-dot:hover { background: rgba(52,37,28,0.38); transform: scale(1.35); }
  .topnav-dot--on { background: var(--color-ember); transform: scale(1.25); }

  /* Год */
  .topnav-ident-year {
    font-family: var(--font-body);
    font-size: 0.5625rem;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.45);
    flex-shrink: 0;
    white-space: nowrap;
  }

  /* Скрываем год и точки на маленьких экранах */
  @media (max-width: 560px) {
    .topnav-ident-year { display: none; }
    .topnav-dots       { display: none; }
  }

  /* Eye/grimoire button в controls */
  .control-btn--eye-pulse {
    border-color: rgba(192,88,44,0.32);
    animation: eyePulse 2.6s ease-in-out infinite;
  }
  @keyframes eyePulse {
    0%, 100% { box-shadow: none; border-color: rgba(192,88,44,0.28); }
    50%       { box-shadow: 0 0 0 3px rgba(192,88,44,0.10); border-color: rgba(192,88,44,0.5); }
  }

  /* ── Левая зона: back + prev/next ── */
  .topnav-left {
    display: flex;
    align-items: center;
    gap: 0;
    min-width: 0;
    justify-self: start;
  }

  .back-link {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-family: var(--font-body);
    font-size: 0.6rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
    text-decoration: none;
    padding: 0.3rem 0.75rem 0.3rem 0;
    white-space: nowrap;
    transition: color 0.2s var(--ease-atelier);
  }
  .back-link:hover { color: var(--color-ink-primary); }

  .back-arrow {
    flex-shrink: 0;
    transition: transform 0.2s var(--ease-atelier);
  }
  .back-link:hover .back-arrow { transform: translateX(-2px); }

  .back-label {
    transition: opacity 0.22s var(--ease-atelier), max-width 0.22s var(--ease-atelier);
    max-width: 200px;
    overflow: hidden;
  }
  .topnav--scrolled .back-label { opacity: 0; max-width: 0; }

  /* Prev/Next nav группа — всегда видна */
  .topnav-fig-nav {
    display: flex;
    align-items: center;
    gap: 3px;
    border-left: 1px solid rgba(52,37,28,0.13);
    padding-left: 0.75rem;
    margin-left: 0.5rem;
  }

  .fig-nav-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.28rem 0.65rem;
    border: 1px solid rgba(52,37,28,0.13);
    border-radius: 100px;
    color: var(--color-ink-tertiary);
    background: transparent;
    text-decoration: none;
    font-family: var(--font-body);
    font-size: 0.5625rem;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    white-space: nowrap;
    overflow: hidden;
    max-width: 180px;
    flex-shrink: 0;
    transition:
      color 0.2s var(--ease-atelier),
      border-color 0.2s var(--ease-atelier),
      background 0.2s var(--ease-atelier),
      box-shadow 0.2s var(--ease-atelier),
      transform 0.2s var(--ease-atelier);
  }
  .fig-nav-pill:hover {
    color: var(--color-ink-primary);
    border-color: rgba(52,37,28,0.28);
    background: rgba(52,37,28,0.04);
    box-shadow: 0 1px 4px rgba(52,37,28,0.06);
  }
  .fig-nav-pill:not(.fig-nav-pill--off):hover { transform: translateX(-1px); }
  .fig-nav-pill--next:not(.fig-nav-pill--off):hover { transform: translateX(1px); }

  .fig-nav-pill--off {
    opacity: 0.28;
    cursor: default;
    padding: 0.28rem 0.5rem;
  }

  /* Имя фигурки внутри пилюли */
  .fig-nav-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100px;
    transition:
      max-width 0.25s var(--ease-atelier),
      opacity 0.22s var(--ease-atelier);
  }
  .topnav--scrolled .fig-nav-name {
    max-width: 0;
    opacity: 0;
  }

  .fig-nav-arrow {
    flex-shrink: 0;
    transition: transform 0.18s var(--ease-atelier);
  }
  .fig-nav-pill:not(.fig-nav-pill--off):hover .fig-nav-arrow { transform: translateX(-2px); }
  .fig-nav-pill--next:not(.fig-nav-pill--off):hover .fig-nav-arrow { transform: translateX(2px); }

  /* На совсем маленьких — скрываем имена сразу */
  @media (max-width: 480px) {
    .fig-nav-name { display: none; }
    .fig-nav-pill { padding: 0.28rem 0.5rem; max-width: none; }
  }

  /* CTA в хидере */
  .control-btn--cta {
    background: #2c1710;
    color: #fff9f0 !important;
    border-color: #2c1710 !important;
  }
  .control-btn--cta:hover {
    background: #6f3b24 !important;
    border-color: #6f3b24 !important;
    box-shadow: 0 4px 16px rgba(44,23,16,0.28) !important;
    transform: none !important;
  }

  /* ── Controls ── */
  .topnav-controls {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    justify-self: end;
    flex-wrap: nowrap;
  }

  .control-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.3rem 0.85rem;
    font-family: var(--font-body);
    font-size: 0.6875rem;
    font-weight: 500;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
    background: transparent;
    border: 1px solid var(--color-border-subtle);
    border-radius: 100px;
    cursor: pointer;
    white-space: nowrap;
    transition: all var(--duration-default) var(--ease-atelier);
  }
  .control-btn:hover {
    color: var(--color-ink-primary);
    border-color: var(--color-border-default);
    background: var(--color-canvas-raised);
    box-shadow: var(--shadow-xs);
  }
  .control-btn--active {
    color: var(--color-ink-primary);
    border-color: var(--color-border-default);
    background: var(--color-canvas-raised);
  }
  .control-btn--lit {
    color: var(--color-ember);
    border-color: var(--color-border-ember);
    background: var(--color-ember-subtle);
  }

  /* текстовые лейблы кнопок — скрываются в condensed */
  .btn-label {
    transition: opacity 0.22s var(--ease-atelier), max-width 0.22s var(--ease-atelier);
    max-width: 120px;
    overflow: hidden;
    white-space: nowrap;
  }
  .topnav--scrolled .btn-label {
    opacity: 0;
    max-width: 0;
  }
  /* в condensed кнопки становятся компактными */
  .topnav--scrolled .control-btn {
    padding: 0.3rem 0.5rem;
  }

  .control-icon { font-size: 0.85rem; line-height: 1; flex-shrink: 0; }

  .audio-indicator {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: var(--color-border-default);
    flex-shrink: 0;
    position: relative;
    transition: background var(--duration-default) var(--ease-atelier);
  }
  .audio-indicator--on { background: var(--color-ember); }
  .audio-indicator--on::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: var(--color-ember);
    opacity: 0.5;
    animation: audioPing 1s cubic-bezier(0,0,.2,1) infinite;
  }
  @keyframes audioPing { 75%,100% { transform: scale(2.2); opacity: 0; } }

  .ref-tag {
    font-family: var(--font-body);
    font-size: 0.625rem;
    font-weight: 500;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-ink-muted);
    border: 1px solid var(--color-border-subtle);
    border-radius: 100px;
    padding: 0.28rem 0.75rem;
    flex-shrink: 0;
  }

  /* ── Main grid ── */
  .main-grid {
    display: grid;
    gap: 2.5rem;
    align-items: start;
    margin-bottom: 5rem;
  }
  @media (min-width: 1024px) {
    .main-grid {
      grid-template-columns: minmax(0, 10fr) minmax(0, 9fr);
      gap: 5rem;
    }
    .gallery-col {
      position: sticky;
      top: calc(68px + 52px + 1.5rem);
    }
  }
  @media (max-width: 680px) {
    .gallery-col { top: calc(58px + 52px + 1rem); }
  }

  /* ── Gallery layout: thumb strip + main image ── */
  .gallery-layout {
    display: grid;
    grid-template-columns: 54px 1fr;
    gap: 0.6rem;
    align-items: start;
  }
  .gallery-layout--solo { grid-template-columns: 1fr; }

  /* Vertical thumbnail strip */
  .thumbs-strip {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .thumb-v {
    position: relative;
    width: 100%;
    aspect-ratio: 4/5;
    overflow: hidden;
    border: 1.5px solid var(--color-border-subtle);
    border-radius: 3px;
    cursor: pointer;
    background: none;
    padding: 0;
    transition:
      border-color var(--duration-default) var(--ease-atelier),
      transform var(--duration-default) var(--ease-atelier);
  }
  .thumb-v:hover { border-color: var(--color-border-default); transform: translateX(2px); }
  .thumb-v--active { border-color: var(--color-ember); }

  .thumb-v-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: grayscale(0.55);
    transition: filter var(--duration-slow) var(--ease-atelier);
  }
  .thumb-v:hover .thumb-v-img,
  .thumb-v--active .thumb-v-img { filter: grayscale(0); }

  /* Active indicator — left bar */
  .thumb-v-bar {
    position: absolute;
    left: 0; top: 0; bottom: 0;
    width: 2px;
    background: var(--color-ember);
    opacity: 0;
    transition: opacity var(--duration-default) var(--ease-atelier);
  }
  .thumb-v--active .thumb-v-bar { opacity: 1; }

  /* Mobile: thumbs go below gallery, horizontal */
  @media (max-width: 767px) {
    .gallery-layout { grid-template-columns: 1fr; }
    .thumbs-strip {
      flex-direction: row;
      overflow-x: auto;
      scrollbar-width: none;
      padding-bottom: 2px;
      order: 2;
      gap: 0.35rem;
    }
    .thumbs-strip::-webkit-scrollbar { display: none; }
    .thumb-v { width: 52px; height: 65px; aspect-ratio: auto; flex-shrink: 0; }
    .thumb-v:hover { transform: translateY(-2px); }
    .thumb-v-bar { left: 0; right: 0; top: auto; bottom: 0; width: 100%; height: 2px; }
  }

  /* ── Image frame ── */
  .image-frame {
    position: relative;
    border: 1px solid var(--color-border-subtle);
    border-radius: 4px;
    overflow: hidden;
    background: var(--color-canvas-raised);
    box-shadow:
      0 1px 0 rgba(255,255,255,0.8) inset,
      var(--shadow-xl);
    transition: box-shadow var(--duration-slow) var(--ease-atelier);
  }
  .image-frame:hover { box-shadow: 0 1px 0 rgba(255,255,255,0.8) inset, 0 28px 80px rgba(60,25,10,0.18); }

  .image-stage {
    position: relative;
    aspect-ratio: 4/5;
    overflow: hidden;
    background: var(--color-canvas-sunken);
  }

  .image-layer { position: absolute; inset: 0; }

  .image-vignette {
    position: absolute; inset: 0;
    pointer-events: none;
    box-shadow: inset 0 0 80px rgba(60,25,10,0.14);
  }

  /* Image counter badge */
  .img-counter {
    position: absolute;
    top: 0.75rem;
    left: 0.75rem;
    z-index: 10;
    font-family: var(--font-body);
    font-size: 0.5625rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    color: rgba(255,249,240,0.88);
    background: rgba(44,23,16,0.42);
    backdrop-filter: blur(6px);
    padding: 0.22rem 0.55rem;
    border-radius: 100px;
    pointer-events: none;
  }
  .img-counter-sep { margin: 0 0.15em; opacity: 0.55; }

  .expand-btn {
    position: absolute;
    bottom: 0.75rem; right: 0.75rem;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.35rem 0.65rem;
    font-family: var(--font-body);
    font-size: 0.5625rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: rgba(255,249,240,0.85);
    background: rgba(44,23,16,0.42);
    backdrop-filter: blur(6px);
    border: 1px solid rgba(255,249,240,0.18);
    border-radius: 4px;
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--duration-default) var(--ease-atelier), background var(--duration-default) var(--ease-atelier);
  }
  .image-stage:hover .expand-btn { opacity: 1; }
  .expand-btn:hover { background: rgba(44,23,16,0.62); color: #fff9f0; }

  /* ═══════════════════════════════════════════
     DETAILS COLUMN — editorial, no card wrapper
  ═══════════════════════════════════════════ */
  .details-col {
    display: flex;
    flex-direction: column;
    padding: 0.25rem 0 2rem;
    position: relative;
  }

  .secret-anchor {
    position: absolute;
    top: -1.5rem;
    right: 0;
    max-width: 18rem;
    text-align: right;
    transform: rotate(1.2deg);
    z-index: 10;
    pointer-events: auto;
  }

  /* Eyebrow row */
  .d-eyebrow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .eyebrow-tags {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .eyebrow-year {
    font-family: var(--font-body);
    font-size: 0.6875rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.72);
  }
  .eyebrow-sep {
    color: rgba(52,37,28,0.24);
    font-size: 0.75rem;
  }
  .eyebrow-series {
    font-family: var(--font-body);
    font-size: 0.6875rem;
    font-weight: 500;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.55);
  }

  /* Status pill */
  .status-pill {
    display: inline-flex;
    align-items: center;
    padding: 0.22rem 0.7rem;
    font-family: var(--font-body);
    font-size: 0.6rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    border-radius: 100px;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .status-pill--available { background: var(--color-sage-subtle); color: var(--color-sage-ink); border: 1px solid rgba(107,138,86,0.25); }
  .status-pill--sold      { background: var(--color-ember-subtle); color: var(--color-ember-ink); border: 1px solid rgba(192,88,44,0.22); }
  .status-pill--reserved  { background: var(--color-ochre-subtle); color: var(--color-ochre-ink); border: 1px solid rgba(176,136,32,0.22); }

  /* Title */
  .figurine-title {
    font-family: var(--font-display);
    font-size: clamp(2.4rem, 4.2vw, 3.8rem);
    font-weight: 400;
    letter-spacing: -0.03em;
    line-height: 1.02;
    color: var(--color-ink-primary);
    margin: 0 0 1.25rem;
    text-wrap: balance;
  }

  /* Short lore */
  .lore-short {
    font-family: var(--font-serif);
    font-size: 1.05rem;
    font-style: italic;
    line-height: 1.72;
    color: #6f3b24;
    margin: 0 0 2rem;
    padding-top: 1rem;
    border-top: 1px solid rgba(52,37,28,0.08);
    letter-spacing: 0.01em;
  }

  /* CTA zone */
  .d-cta-zone {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-bottom: 2rem;
  }

  .cta-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.6rem;
    padding: 1rem 1.5rem;
    font-family: var(--font-body);
    font-size: 0.8125rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    border-radius: 10px;
    background: #2c1710;
    color: #fff9f0;
    border: none;
    cursor: pointer;
    position: relative;
    overflow: hidden;
    transition: background 220ms ease, transform 140ms ease, box-shadow 220ms ease;
    box-shadow:
      0 1px 0 rgba(255,255,255,0.12) inset,
      0 8px 24px rgba(44,23,16,0.28);
  }
  .cta-btn::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(105deg, transparent 30%, rgba(255,255,255,0.06) 50%, transparent 70%);
    transform: translateX(-100%);
    transition: transform 0.5s ease;
  }
  .cta-btn:hover { background: #6f3b24; transform: translateY(-1px); box-shadow: 0 1px 0 rgba(255,255,255,0.12) inset, 0 12px 32px rgba(44,23,16,0.32); }
  .cta-btn:hover::after { transform: translateX(100%); }
  .cta-btn:active { transform: translateY(0); }

  .cta-btn-label { position: relative; z-index: 1; }
  .cta-arrow { position: relative; z-index: 1; flex-shrink: 0; transition: transform 200ms ease; }
  .cta-btn:hover .cta-arrow { transform: translateX(4px); }

  .cta-note {
    font-family: var(--font-serif);
    font-size: 0.75rem;
    font-style: italic;
    color: rgba(95,70,54,0.65);
    margin: 0;
    line-height: 1.5;
    text-align: center;
  }

  /* Reserved notice */
  .reserved-notice {
    display: flex;
    align-items: flex-start;
    gap: 0.875rem;
    padding: 1rem 1.25rem;
    background: var(--color-ochre-subtle);
    border: 1px solid rgba(176,136,32,0.22);
    border-radius: 10px;
  }
  .reserved-icon {
    flex-shrink: 0;
    color: var(--color-ochre);
    margin-top: 0.1rem;
  }
  .reserved-title {
    font-family: var(--font-body);
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--color-ochre-ink);
    margin: 0 0 0.3rem;
  }
  .reserved-notice--confirmed {
    background: rgba(6,95,70,0.06);
    border-color: rgba(6,95,70,0.22);
    border-left: 3px solid #059669;
  }
  .reserved-title--confirmed { color: #065f46 !important; }
  .reserved-sub {
    font-family: var(--font-body);
    font-size: 0.8125rem;
    line-height: 1.55;
    color: rgba(90,52,16,0.72);
    margin: 0;
  }

  /* ── Attributes — compact spec rows ── */
  .d-attrs { margin-top: 1.75rem; }

  .attrs-specs {
    display: flex;
    flex-direction: column;
    margin: 0;
  }

  .spec-row {
    display: grid;
    grid-template-columns: 18px minmax(80px, max-content) 1fr;
    align-items: baseline;
    gap: 0 0.75rem;
    padding: 0.7rem 0;
    border-bottom: 1px solid rgba(52,37,28,0.07);
  }
  .spec-row:last-child { border-bottom: none; }

  .spec-icon {
    color: var(--color-ember);
    display: flex;
    align-items: flex-start;
    padding-top: 1px;
    grid-row: 1;
  }

  .spec-label {
    font-family: var(--font-body);
    font-size: 0.5625rem;
    font-weight: 700;
    letter-spacing: 0.13em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.52);
    white-space: nowrap;
    padding-top: 2px;
    margin: 0;
  }

  .spec-value {
    font-family: var(--font-body);
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--color-ink-primary);
    line-height: 1.45;
    margin: 0;
  }

  /* ── History inside right column ── */
  .d-history {
    margin-top: 1.75rem;
  }

  .d-section-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.25rem;
  }

  .sec-label {
    font-family: var(--font-body);
    font-size: 0.6rem;
    font-weight: 800;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--color-ember);
    flex-shrink: 0;
  }

  .sec-rule {
    flex: 1;
    height: 1px;
    background: rgba(52,37,28,0.10);
  }

  .history-body {
    font-family: var(--font-serif);
    font-size: 1.04rem;
    line-height: 1.88;
    color: rgba(52,37,28,0.88);
    margin: 0;
    font-weight: 400;
  }
  .history-body.drop-cap::first-letter {
    font-family: var(--font-display);
    font-size: 3.2rem;
    font-weight: 500;
    float: left;
    line-height: 0.82;
    margin-right: 0.28rem;
    margin-top: 0.07em;
    color: var(--color-ember);
  }

  /* ── CTA row with heart ── */
  .cta-row {
    display: flex;
    gap: 0.625rem;
    align-items: stretch;
  }

  .cta-row .cta-btn { flex: 1; }

  .cta-heart {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 48px;
    border: 1.5px solid var(--color-border-default);
    border-radius: 10px;
    background: transparent;
    color: var(--color-ink-tertiary);
    cursor: pointer;
    transition: all var(--duration-default) var(--ease-atelier);
  }
  .cta-heart:hover {
    border-color: rgba(192,88,44,0.45);
    color: var(--color-ember);
    background: var(--color-ember-subtle);
  }
  .cta-heart--saved {
    border-color: rgba(192,88,44,0.5);
    color: var(--color-ember);
    background: var(--color-ember-subtle);
  }

  .cta-secondary-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }

  .cta-ask {
    flex-shrink: 0;
    font-family: var(--font-body);
    font-size: 0.6875rem;
    font-weight: 500;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0.25rem 0;
    border-bottom: 1px solid rgba(95,70,54,0.2);
    transition: color var(--duration-default) var(--ease-atelier), border-color var(--duration-default) var(--ease-atelier);
    white-space: nowrap;
  }
  .cta-ask:hover {
    color: var(--color-ink-primary);
    border-color: rgba(95,70,54,0.5);
  }

  /* Notify button */
  .notify-btn {
    margin-top: 0.875rem;
    width: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.75rem 1.25rem;
    font-family: var(--font-body);
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-ochre-ink);
    background: var(--color-ochre-subtle);
    border: 1px solid rgba(176,136,32,0.28);
    border-radius: 10px;
    cursor: pointer;
    transition: all var(--duration-default) var(--ease-atelier);
  }
  .notify-btn:hover {
    background: var(--color-ochre-light);
    border-color: rgba(176,136,32,0.5);
    transform: translateY(-1px);
    box-shadow: 0 4px 16px rgba(176,136,32,0.14);
  }

  /* Showings block — clear transfer restriction notice */
  .showing-block {
    padding: 0.875rem 1rem;
    background: rgba(217,119,6,0.07);
    border: 1px solid rgba(217,119,6,0.25);
    border-left: 3px solid #d97706;
    border-radius: 4px;
    margin-bottom: 1rem;
  }
  .showing-block-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.68rem;
    font-family: var(--font-sans);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #92400e;
    margin-bottom: 0.5rem;
  }
  .showing-block-entry {
    font-size: 0.75rem;
    font-family: var(--font-sans);
    color: #78350f;
    margin: 0.2rem 0 0;
    line-height: 1.4;
  }
  .showing-block-type {
    font-weight: 600;
    margin-right: 0.25rem;
  }
  .showing-block-avail {
    margin-top: 0.625rem;
    padding-top: 0.5rem;
    border-top: 1px solid rgba(217,119,6,0.2);
    font-size: 0.75rem;
    font-family: var(--font-sans);
    color: #34251c;
  }

  /* Availability note — compact version for bookings-only case */
  .avail-note {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background: rgba(52,37,28,0.04);
    border: 1px solid rgba(52,37,28,0.1);
    border-radius: 4px;
    margin-bottom: 1rem;
    font-size: 0.75rem;
    font-family: var(--font-sans);
    color: #34251c;
  }
  .avail-note svg { flex-shrink: 0; color: rgba(95,70,54,0.6); }

  /* Exhibition block — replaces request button when figurine is on active showing */
  .cta-exhibition-block {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.625rem 1rem;
    background: rgba(217,119,6,0.06);
    border: 1px solid rgba(217,119,6,0.2);
    border-radius: 4px;
    color: #92400e;
    font-family: var(--font-sans);
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.04em;
  }

  /* reserved-avail: date under reserved notice */
  .reserved-avail {
    margin-top: 0.5rem;
    font-family: var(--font-sans);
    font-size: 0.75rem;
    color: rgba(90,52,16,0.8);
  }

  /* Book button */
  .book-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.75rem;
    padding: 0.5rem 1rem;
    border: 1px solid rgba(95,70,54,0.25);
    background: transparent;
    color: var(--color-warm-medium);
    font-family: var(--font-sans);
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
    transition: border-color 0.2s, color 0.2s, background 0.2s;
    border-radius: 2px;
  }
  .book-btn:hover {
    border-color: rgba(95,70,54,0.5);
    color: var(--color-warm-dark);
    background: rgba(95,70,54,0.04);
  }

  /* Sold notice */
  .sold-notice {
    padding: 0.875rem 1rem;
    border: 1px solid rgba(52,37,28,0.10);
    border-radius: 8px;
    background: rgba(52,37,28,0.03);
  }
  .sold-text {
    font-family: var(--font-serif);
    font-size: 0.875rem;
    font-style: italic;
    color: rgba(95,70,54,0.65);
    margin: 0;
    line-height: 1.55;
  }

  /* Shared section-row header pattern (video, related) */
  .section-row {
    display: flex;
    align-items: center;
    gap: 1.25rem;
    margin-bottom: 2rem;
  }

  /* ── Grimoire ── */
  .grimoire-section {
    margin: 0 0 3rem;
  }

  .grimoire-trigger {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 1.25rem;
    padding: 1.25rem 1.5rem;
    background: linear-gradient(135deg, var(--color-ember-subtle) 0%, rgba(251,240,212,0.45) 100%);
    border: 1px solid rgba(192,88,44,0.22);
    border-radius: 8px;
    cursor: pointer;
    text-align: left;
    transition:
      border-color var(--duration-default) var(--ease-atelier),
      box-shadow var(--duration-default) var(--ease-atelier),
      background var(--duration-default) var(--ease-atelier),
      transform var(--duration-default) var(--ease-atelier);
    position: relative;
    overflow: hidden;
  }

  /* subtle shimmer sweep on hover */
  .grimoire-trigger::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(105deg, transparent 40%, rgba(255,255,255,0.18) 50%, transparent 60%);
    transform: translateX(-100%);
    transition: transform 0.55s var(--ease-atelier);
  }
  .grimoire-trigger:hover::before { transform: translateX(100%); }

  .grimoire-trigger:hover {
    border-color: rgba(192,88,44,0.44);
    box-shadow: 0 4px 24px rgba(192,88,44,0.12), 0 1px 0 rgba(255,255,255,0.7) inset;
    transform: translateY(-1px);
  }

  .grimoire-trigger:active { transform: translateY(0); }

  .grimoire-icon {
    flex-shrink: 0;
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: rgba(192,88,44,0.10);
    border: 1px solid rgba(192,88,44,0.20);
    color: var(--color-ember);
    transition: background var(--duration-default) var(--ease-atelier);
  }
  .grimoire-trigger:hover .grimoire-icon {
    background: rgba(192,88,44,0.16);
  }

  .grimoire-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .grimoire-title {
    position: relative;
    display: inline-block;
    font-family: var(--font-display);
    font-size: 1.1rem;
    font-weight: 400;
    letter-spacing: -0.01em;
    color: var(--color-ink-primary);
    line-height: 1.2;
  }

  .grimoire-sub {
    font-family: var(--font-body);
    font-size: 0.6875rem;
    font-weight: 400;
    letter-spacing: 0.04em;
    color: var(--color-ink-tertiary);
    line-height: 1.4;
  }

  .grimoire-arrow {
    flex-shrink: 0;
    color: var(--color-ember);
    opacity: 0.7;
    transition: opacity var(--duration-default) var(--ease-atelier);
  }
  .grimoire-trigger:hover .grimoire-arrow { opacity: 1; }

  .grimoire-dot {
    position: absolute;
    top: -0.1rem;
    right: -0.55rem;
    width: 0.35rem;
    height: 0.35rem;
    border-radius: 50%;
    background: var(--color-ember);
    animation: audioPing 1.4s cubic-bezier(0,0,.2,1) infinite;
  }

  /* ── Video section ── */
  .video-section { margin-top: 5rem; }

  .video-heading {
    margin-bottom: 2rem;
  }

  .video-wrap {
    max-width: 54rem;
    margin: 0 auto;
    position: relative;
  }

  .video-frame {
    padding: 0.5rem;
    border-radius: 10px;
    box-shadow: var(--shadow-xl);
    transition: transform var(--duration-slow) var(--ease-atelier), box-shadow var(--duration-slow) var(--ease-atelier);
  }
  .video-frame:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow-xl), 0 0 0 1px rgba(192,88,44,0.08);
  }

  .video-stage {
    position: relative;
    aspect-ratio: 16/9;
    overflow: hidden;
    border-radius: 6px;
    background: var(--color-canvas-deep);
  }

  .video-el {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .video-fs-btn {
    position: absolute;
    top: 0.6rem;
    right: 0.6rem;
    z-index: 20;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    background: rgba(253,250,245,0.85);
    border: 1px solid var(--color-border-default);
    border-radius: 4px;
    color: var(--color-ink-secondary);
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--duration-default) var(--ease-atelier);
  }
  .video-stage:hover .video-fs-btn { opacity: 1; }
  .video-fs-btn:hover { background: var(--color-canvas-raised); color: var(--color-ink-primary); }

  .video-caption {
    text-align: center;
    color: var(--color-ink-muted);
    margin-top: 1rem;
  }

  /* ── Related — горизонтальная лента ── */
  .related-section { margin-top: 5rem; }

  .related-strip {
    display: flex;
    gap: 1.25rem;
    overflow-x: auto;
    overscroll-behavior-x: contain;
    scroll-snap-type: x mandatory;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: none;
    padding-bottom: 0.5rem;
    /* negative margin for full-bleed feel on mobile */
    margin-left: -1.5rem;
    margin-right: -1.5rem;
    padding-left: 1.5rem;
    padding-right: 1.5rem;
  }
  .related-strip::-webkit-scrollbar { display: none; }

  @media (min-width: 1024px) {
    .related-strip { margin-left: -3.5rem; margin-right: -3.5rem; padding-left: 3.5rem; padding-right: 3.5rem; }
  }

  .related-card {
    flex-shrink: 0;
    width: 196px;
    scroll-snap-align: start;
    text-decoration: none;
    color: inherit;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .related-img-wrap {
    position: relative;
    aspect-ratio: 3/4;
    overflow: hidden;
    border-radius: 6px;
    background: var(--color-canvas-sunken);
    margin-bottom: 0.75rem;
  }

  .related-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 30%;
    filter: grayscale(0.35) saturate(0.88);
    transition: filter var(--duration-slow) var(--ease-atelier), transform var(--duration-slow) var(--ease-atelier);
  }
  .related-card:hover .related-img {
    filter: grayscale(0) saturate(1);
    transform: scale(1.03);
  }

  .related-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(to top, rgba(44,23,16,0.52) 0%, transparent 55%);
    display: flex;
    align-items: flex-end;
    padding: 0.875rem;
    opacity: 0;
    transition: opacity var(--duration-default) var(--ease-atelier);
  }
  .related-card:hover .related-overlay { opacity: 1; }

  .related-cta-hint {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: rgba(255,249,240,0.18);
    border: 1px solid rgba(255,249,240,0.35);
    backdrop-filter: blur(4px);
    color: #fff9f0;
    transform: translateX(-4px);
    transition: transform var(--duration-default) var(--ease-atelier);
  }
  .related-card:hover .related-cta-hint { transform: translateX(0); }

  .related-status-badge {
    position: absolute;
    top: 0.6rem;
    right: 0.6rem;
    font-family: var(--font-body);
    font-size: 0.5rem;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 0.2rem 0.5rem;
    border-radius: 100px;
  }
  .related-status-badge--available { background: var(--color-sage-subtle); color: var(--color-sage-ink); }
  .related-status-badge--sold      { background: var(--color-ember-subtle); color: var(--color-ember-ink); }
  .related-status-badge--reserved  { background: var(--color-ochre-subtle); color: var(--color-ochre-ink); }

  .related-meta {
    padding: 0 0.25rem;
  }

  .related-name {
    font-family: var(--font-display);
    font-size: 1.05rem;
    font-weight: 400;
    letter-spacing: -0.01em;
    line-height: 1.2;
    color: var(--color-ink-primary);
    margin: 0;
    transition: color var(--duration-default) var(--ease-atelier);
  }
  .related-card:hover .related-name { color: var(--color-ember); }

  /* ── Mobile sticky CTA ── */
  .mobile-cta {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    z-index: 110;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.875rem 1.25rem;
    background: rgba(250,246,238,0.94);
    backdrop-filter: blur(16px) saturate(1.3);
    -webkit-backdrop-filter: blur(16px) saturate(1.3);
    border-top: 1px solid rgba(180,140,100,0.22);
    box-shadow: 0 -4px 24px rgba(44,23,16,0.08);
  }
  /* Скрываем на desktop */
  @media (min-width: 1024px) {
    .mobile-cta { display: none; }
  }

  .mobile-cta-info {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
  }

  .mobile-cta-name {
    font-family: var(--font-display);
    font-size: 0.9375rem;
    font-weight: 400;
    color: var(--color-ink-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mobile-cta-status {
    font-family: var(--font-body);
    font-size: 0.5625rem;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-sage);
  }

  .mobile-cta-btn {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1.25rem;
    background: #2c1710;
    color: #fff9f0;
    border: none;
    border-radius: 8px;
    font-family: var(--font-body);
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    cursor: pointer;
    transition: background var(--duration-default) var(--ease-atelier);
  }
  .mobile-cta-btn:hover { background: #6f3b24; }

  .related-name {
    font-family: var(--font-display);
    font-size: 1.1rem;
    font-weight: 400;
    letter-spacing: -0.015em;
    color: var(--color-ink-primary);
    margin: 0;
    transition: color var(--duration-default) var(--ease-atelier);
  }
  .related-card:hover .related-name { color: var(--color-ember); }

  .related-status { align-self: flex-start; }

  /* ── Utility ── */
  .section-label {
    display: inline-flex;
    align-items: center;
    font-family: var(--font-body);
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.13em;
    text-transform: uppercase;
    color: #a84f2f;
  }

  .divider-text {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .divider-text::before,
  .divider-text::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--color-border-subtle);
  }

  /* ── Claim token / self-cancel ── */
  .claim-block {
    padding: 0.75rem 1rem;
    background: rgba(52,37,28,0.04);
    border: 1px solid rgba(52,37,28,0.12);
    border-radius: 4px;
    margin-bottom: 0.75rem;
  }
  .claim-block--done      { background: rgba(6,95,70,0.04); border-color: rgba(6,95,70,0.15); }
  .claim-block--confirmed {
    background: rgba(6,95,70,0.05);
    border-color: rgba(6,95,70,0.2);
    border-left: 3px solid #059669;
  }
  .claim-block--confirmed .claim-head { color: #065f46; }
  .claim-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem;
    padding: 0.4rem 0;
    border-top: 1px solid rgba(52,37,28,0.07);
  }
  .claim-row:first-of-type { border-top: none; }
  .claim-code-small {
    font-family: 'Fraunces', serif;
    font-size: 0.7rem;
    letter-spacing: 0.06em;
    color: rgba(95,70,54,0.5);
    margin-left: auto;
  }
  .claim-head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.625rem;
    font-family: var(--font-sans);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: rgba(95,70,54,0.7);
    margin-bottom: 0.35rem;
  }
  .claim-dates {
    font-size: 0.8rem;
    font-family: var(--font-sans);
    color: #34251c;
    margin: 0 0 0.5rem;
  }
  .claim-done {
    font-size: 0.78rem;
    font-family: var(--font-sans);
    color: #065f46;
    margin: 0;
  }
  .claim-err {
    font-size: 0.7rem;
    color: #991b1b;
    font-family: var(--font-sans);
    margin: 0 0 0.4rem;
  }
  .claim-status {
    font-size: 0.7rem;
    font-family: var(--font-sans);
    color: rgba(95,70,54,0.7);
    margin: 0.25rem 0 0;
    text-transform: capitalize;
  }
  .claim-cancel-btn {
    font-family: var(--font-sans);
    font-size: 0.65rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #9e452d;
    background: transparent;
    border: 1px solid rgba(158,69,45,0.3);
    border-radius: 3px;
    padding: 0.3rem 0.75rem;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .claim-cancel-btn:hover:not(:disabled) { border-color: #9e452d; background: rgba(158,69,45,0.05); }
  .claim-cancel-btn:disabled { opacity: 0.5; cursor: default; }

  /* Manual lookup */
  .claim-lookup { margin-bottom: 0.5rem; }
  .claim-lookup-link {
    font-family: var(--font-sans);
    font-size: 0.65rem;
    color: rgba(95,70,54,0.5);
    background: none;
    border: none;
    cursor: pointer;
    text-decoration: underline dotted;
    padding: 0;
  }
  .claim-lookup-link:hover { color: rgba(95,70,54,0.8); }
  .claim-lookup-form {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    margin-bottom: 0.35rem;
  }
  .claim-lookup-input {
    font-family: 'Fraunces', serif;
    font-size: 0.95rem;
    letter-spacing: 0.08em;
    width: 7rem;
    border: 1px solid rgba(52,37,28,0.2);
    border-radius: 3px;
    padding: 0.25rem 0.5rem;
    background: #fff9f0;
    color: #34251c;
    text-transform: uppercase;
  }
  .claim-lookup-input:focus { outline: none; border-color: rgba(52,37,28,0.4); }
  .claim-lookup-btn {
    font-family: var(--font-sans);
    font-size: 0.65rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    background: rgba(52,37,28,0.08);
    border: 1px solid rgba(52,37,28,0.15);
    border-radius: 3px;
    padding: 0.28rem 0.6rem;
    color: #34251c;
    cursor: pointer;
  }
  .claim-lookup-close {
    font-size: 0.7rem;
    color: rgba(95,70,54,0.4);
    background: none;
    border: none;
    cursor: pointer;
  }
  .claim-lookup-result { margin-top: 0.4rem; }

</style>
