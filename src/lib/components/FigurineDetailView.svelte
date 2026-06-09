<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';
  import type { Figurine, FigurineSchedule } from '$lib/types/api';
  import OrderModal from '$lib/components/OrderModal.svelte';
  import BookingModal from '$lib/components/BookingModal.svelte';
  import WaitlistModal from '$lib/components/WaitlistModal.svelte';
  import BrassLens from '$lib/components/BrassLens.svelte';
  import CandleReveal from '$lib/components/CandleReveal.svelte';
  import MemoryMirror from '$lib/components/MemoryMirror.svelte';
  import SecretText from '$lib/components/SecretText.svelte';
  import Lightbox from '$lib/components/Lightbox.svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import ShowingsTimeline from '$lib/components/ShowingsTimeline.svelte';
  import FigurineComments from '$lib/components/FigurineComments.svelte';
  import { FigurineClaimsStore, type ClaimData } from '$lib/stores/figurine-claims.svelte';
  import '$lib/styles/figurine-detail.css';

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
  let showWaitlistModal = $state(false);
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
  // figurine.id captured once — component is never remounted with a different figurine
  const cs = new FigurineClaimsStore(id, () => {
    api.getFigurineSchedule(id).then(s => { figurineSchedule = s; });
  });

  function fmtDate(ds: string) {
    return new Date(ds + 'T00:00:00').toLocaleDateString(undefined, { day: 'numeric', month: 'long', year: 'numeric' });
  }

  function lookupStatusLabel(s: string): string {
    switch (s) {
      case 'pending':   return $t('cancelStatusPending');
      case 'confirmed': return $t('cancelStatusConfirmed');
      case 'rejected':  return $t('cancelStatusRejected');
      case 'cancelled': return $t('cancelStatusCancelled');
      case 'completed': return $t('cancelStatusCompleted');
      default:          return s;
    }
  }

  // ── Instagram Story share ────────────────────────────────────────────────
  let storySaving    = $state(false);
  let storyBlob      = $state<Blob | null>(null);
  let storyObjectUrl = $state('');
  let showStoryModal = $state(false);
  let canNativeShare = $state(false);

  async function openStoryModal() {
    if (storySaving) return;
    storySaving = true;
    try {
      const faceImg = figurine.images.find(i => i.imageType === 'face') ?? figurine.images[0];
      const imgSrc  = faceImg?.originalUrl ?? faceImg?.url ?? '';
      const W = 1080, H = 1920;

      async function buildCanvas(withImage: boolean): Promise<HTMLCanvasElement> {
        const cv = document.createElement('canvas');
        cv.width = W; cv.height = H;
        const ctx = cv.getContext('2d')!;
        ctx.fillStyle = '#f8f1e7';
        ctx.fillRect(0, 0, W, H);
        if (withImage && imgSrc) {
          const img = new Image();
          img.crossOrigin = 'anonymous';
          await new Promise<void>(res => { img.onload = () => res(); img.onerror = () => res(); img.src = imgSrc; });
          if (img.naturalWidth > 0) {
            const zone = H * 0.75;
            const scale = Math.max(W / img.naturalWidth, zone / img.naturalHeight);
            const iw = img.naturalWidth * scale, ih = img.naturalHeight * scale;
            ctx.save();
            ctx.beginPath(); ctx.rect(0, 0, W, zone); ctx.clip();
            ctx.drawImage(img, (W - iw) / 2, (zone - ih) / 2, iw, ih);
            ctx.restore();
          }
        }
        const grad = ctx.createLinearGradient(0, H * 0.45, 0, H);
        grad.addColorStop(0, 'rgba(34,15,10,0)');
        grad.addColorStop(0.55, 'rgba(34,15,10,0.75)');
        grad.addColorStop(1, 'rgba(34,15,10,0.94)');
        ctx.fillStyle = grad;
        ctx.fillRect(0, H * 0.45, W, H * 0.55);
        ctx.strokeStyle = 'rgba(248,241,231,0.15)';
        ctx.lineWidth = 2;
        ctx.strokeRect(40, 40, W - 80, H - 80);
        ctx.fillStyle = '#f8f1e7';
        ctx.textAlign = 'center';
        ctx.font = `500 ${Math.round(W * 0.072)}px Georgia, serif`;
        ctx.fillText(figurine.name, W / 2, Math.round(H * 0.825), W - 140);
        ctx.strokeStyle = 'rgba(248,241,231,0.22)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(W * 0.31, H * 0.875); ctx.lineTo(W * 0.69, H * 0.875);
        ctx.stroke();
        ctx.font = `400 ${Math.round(W * 0.048)}px Georgia, serif`;
        ctx.fillStyle = 'rgba(248,241,231,0.55)';
        ctx.fillText('G O T I G A', W / 2, Math.round(H * 0.916), W - 160);
        return cv;
      }

      let blob: Blob | null = null;
      try {
        const cv = await buildCanvas(true);
        blob = await new Promise<Blob | null>(res => cv.toBlob(res, 'image/jpeg', 0.92));
      } catch {
        const cv = await buildCanvas(false);
        blob = await new Promise<Blob | null>(res => cv.toBlob(res, 'image/jpeg', 0.92));
      }

      if (!blob) return;

      storyBlob      = blob;
      storyObjectUrl = URL.createObjectURL(blob);
      const testFile = new File([blob], 'story.jpg', { type: 'image/jpeg' });
      canNativeShare = !!navigator.canShare?.({ files: [testFile] });
      showStoryModal = true;
    } finally {
      storySaving = false;
    }
  }

  function downloadStory() {
    if (!storyObjectUrl) return;
    const a = document.createElement('a');
    a.href = storyObjectUrl;
    a.download = `gotiga-${figurine.name.replace(/\s+/g, '-').toLowerCase()}-story.jpg`;
    document.body.appendChild(a); a.click(); document.body.removeChild(a);
  }

  async function nativeShareStory() {
    if (!storyBlob) return;
    const file = new File([storyBlob], 'gotiga-story.jpg', { type: 'image/jpeg' });
    try { await navigator.share({ files: [file], title: figurine.name }); } catch { /* user cancelled */ }
  }

  function closeStoryModal() {
    showStoryModal = false;
    if (storyObjectUrl) { URL.revokeObjectURL(storyObjectUrl); storyObjectUrl = ''; }
    storyBlob = null;
  }

  // ── Ink reveal ───────────────────────────────────────────────────────────
  let historyRef = $state<HTMLElement | null>(null);
  let inkReady   = $state(false);

  $effect(() => {
    if (!historyRef || inkReady) return;
    if (typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      inkReady = true; return;
    }
    const io = new IntersectionObserver((entries) => {
      if (entries[0].isIntersecting) { inkReady = true; io.disconnect(); }
    }, { threshold: 0.05 });
    io.observe(historyRef);
    return () => io.disconnect();
  });

  function buildInkHtml(text: string): string {
    const words = text.split(' ');
    return words.map((word, i) => {
      const esc = word.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
      const delay = Math.min(i, 80) * 25;
      return `<span class="ink-word" style="animation-delay:${delay}ms">${esc}</span>`;
    }).join(' ');
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
    if (document.visibilityState === 'visible') cs.verify();
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
    window.addEventListener('scroll', onScroll, { passive: true });
    document.addEventListener('visibilitychange', handleVisibility);
    const wl = JSON.parse(localStorage.getItem('gotiga_wishlist') ?? '[]') as string[];
    isWishlisted = wl.includes(figurine.id);
    api.getFigurineSchedule(figurine.id).then(s => { figurineSchedule = s; });
    cs.load();
    cs.verify();
    cs.startPolling();
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
    window.removeEventListener('scroll', onScroll);
    document.removeEventListener('visibilitychange', handleVisibility);
    clearTimeout(copiedTimer);
    if (audioRef) { audioRef.pause(); audioRef = null; }
    cs.stopPolling();
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
    relatedAvailable={figurine.relatedItems.filter(r => r.status === 'available').slice(0, 3)}
    onClose={() => (showOrderModal = false)}
  />

  <WaitlistModal
    isOpen={showWaitlistModal}
    figurineId={id}
    figurineName={figurine.name}
    onClose={() => (showWaitlistModal = false)}
  />

  <BookingModal
    isOpen={showBookingModal}
    onBookingCreated={(claim: ClaimData) => cs.onBookingCreated(claim)}
    figurineName={figurine.name}
    figurineId={figurine.id}
    schedule={figurineSchedule}
    onClose={() => (showBookingModal = false)}
  />

  <!-- ── Story share modal ──────────────────────────────────────────────── -->
  {#if showStoryModal}
    <div class="story-backdrop" transition:fade={{ duration: 200 }}
         onclick={closeStoryModal} onkeydown={(e) => e.key === 'Escape' && closeStoryModal()}
         role="presentation">
      <div class="story-modal" onclick={(e) => e.stopPropagation()} transition:fade={{ duration: 150 }}
           role="dialog" aria-modal="true" tabindex="-1">
        <button class="story-close" onclick={closeStoryModal} aria-label="Закрыть">✕</button>

        <p class="story-modal-title">{$t('figurineStoryShare')}</p>

        <!-- 9:16 preview -->
        {#if storyObjectUrl}
          <div class="story-preview-wrap">
            <img src={storyObjectUrl} alt="Story preview" class="story-preview-img" />
          </div>
        {/if}

        <div class="story-actions">
          {#if canNativeShare}
            <button class="story-btn story-btn--primary" onclick={nativeShareStory}>
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3">
                <circle cx="11" cy="3" r="1.5"/><circle cx="3" cy="7" r="1.5"/><circle cx="11" cy="11" r="1.5"/>
                <path d="M4.4 6.1l5.2-2.6M4.4 7.9l5.2 2.6"/>
              </svg>
              {$t('storyShare')}
            </button>
          {/if}
          <button class="story-btn {canNativeShare ? 'story-btn--secondary' : 'story-btn--primary'}" onclick={downloadStory}>
            <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.3">
              <path d="M6.5 1v7.5M4 6l2.5 2.5L9 6" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M1 10v1.5A0.5 0.5 0 0 0 1.5 12h10a0.5 0.5 0 0 0 0.5-0.5V10"/>
            </svg>
            {$t('storyDownload')}
          </button>
        </div>

        {#if !canNativeShare}
          <p class="story-hint">{$t('storyHint')}</p>
        {/if}
      </div>
    </div>
  {/if}

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

        <button
          onclick={openStoryModal}
          class="control-btn"
          aria-label={$t('figurineStoryShare')}
          title={$t('figurineStoryShare')}
          disabled={storySaving}
        >
          {#if storySaving}
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.4" style="animation:spin 1s linear infinite">
              <path d="M6 1.5A4.5 4.5 0 1 1 1.5 6" stroke-linecap="round"/>
            </svg>
          {:else}
            <svg width="12" height="14" viewBox="0 0 12 14" fill="none" stroke="currentColor" stroke-width="1.3">
              <rect x="1" y="1" width="10" height="12" rx="1"/>
              <path d="M3.5 5h5M3.5 7.5h5M3.5 10h3"/>
            </svg>
          {/if}
          <span class="btn-label">{storySaving ? $t('figurineStorySaving') : $t('figurineStoryShare')}</span>
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
            <p bind:this={historyRef} class="history-body drop-cap">
              {#if inkReady}
                {@html buildInkHtml(figurine.fullDescription ?? '')}
              {:else}
                {figurine.fullDescription}
              {/if}
            </p>
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
        {#each cs.claims as c (c.token)}
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
              <p class="claim-next">{$t('claimConfirmedNext')}</p>
              <div class="claim-confirmed-actions">
                <a href="/cancel/{c.token}" target="_blank" rel="noopener" class="claim-manage-link">{$t('claimManageLink')}</a>
                <button onclick={() => cs.cancel(c)} disabled={cs.cancellingToken === c.token} class="claim-cancel-btn">
                  {cs.cancellingToken === c.token ? $t('claimCancelling') : $t('claimCancelBtn')}
                </button>
              </div>
              {#if cs.claimErrors[c.token]}<p class="claim-err">{cs.claimErrors[c.token]}</p>{/if}
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
                {#if cs.claimErrors[c.token]}
                  <p class="claim-err">{cs.claimErrors[c.token]}</p>
                {/if}
                <button
                  onclick={() => cs.cancel(c)}
                  disabled={cs.cancellingToken === c.token}
                  class="claim-cancel-btn"
                >{cs.cancellingToken === c.token ? $t('claimCancelling') : $t('claimCancelBtn')}</button>
              </div>
            </div>
          {/if}
        {/each}

        {#if cs.cancelledTokens.size > 0 && cs.claims.length === 0}
          <div class="claim-block claim-block--done">
            <p class="claim-done">{$t('claimCancelDone')}</p>
          </div>
        {/if}

        <!-- Manual token entry — only while figurine is still available -->
        <div class="claim-lookup">
          {#if !cs.showTokenForm}
            <button onclick={() => cs.showTokenForm = true} class="claim-lookup-link">{$t('claimHaveCode')}</button>
          {:else}
            <div class="claim-lookup-form">
              <input type="text" bind:value={cs.tokenInput} placeholder="XXXX-XXXX" maxlength="9"
                class="claim-lookup-input" oninput={() => { cs.tokenLookupInfo = null; cs.tokenLookupErr = ''; }} />
              <button onclick={() => cs.lookupToken()} disabled={cs.tokenLooking} class="claim-lookup-btn">
                {cs.tokenLooking ? '…' : $t('claimLookupBtn')}
              </button>
              <button onclick={() => { cs.showTokenForm = false; cs.tokenInput = ''; cs.tokenLookupInfo = null; }} class="claim-lookup-close">✕</button>
            </div>
            {#if cs.tokenLookupErr}<p class="claim-err">{cs.tokenLookupErr}</p>{/if}
            {#if cs.tokenLookupInfo}
              <div class="claim-lookup-result">
                <p class="claim-dates">{fmtDate(cs.tokenLookupInfo.startsAt)} — {fmtDate(cs.tokenLookupInfo.endsAt)}</p>
                {#if cs.tokenLookupInfo.status === 'pending'}
                  <button onclick={() => cs.cancelFromLookup()} disabled={cs.lookupCancelling} class="claim-cancel-btn">
                    {cs.lookupCancelling ? $t('claimCancelling') : $t('claimCancelBtn')}
                  </button>
                {:else}
                  <p class="claim-status">{$t('claimStatus')}: {lookupStatusLabel(cs.tokenLookupInfo.status)}</p>
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
              <p class="price-on-request">{$t('figurinePriceOnRequest')}</p>
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
            <!-- All confirmed claims -->
            {#each cs.claims.filter(c => c.status === 'confirmed') as c (c.token)}
              <div class="reserved-notice reserved-notice--confirmed">
                <svg class="reserved-icon" width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.8">
                  <path d="M3 9l4.5 4.5 8-8"/>
                </svg>
                <div>
                  <p class="reserved-title reserved-title--confirmed">{$t('claimConfirmed')}</p>
                  <p class="reserved-sub">{fmtDate(c.startsAt)} — {fmtDate(c.endsAt)}</p>
                  <p class="claim-code-small" style="margin-top:0.25rem">{c.token}</p>
                  <p class="claim-next" style="margin-top:0.6rem">{$t('claimConfirmedNext')}</p>
                  <div class="claim-confirmed-actions" style="margin-top:0.6rem">
                    <a href="/cancel/{c.token}" target="_blank" rel="noopener" class="claim-manage-link">{$t('claimManageLink')}</a>
                    <button onclick={() => cs.cancel(c)} disabled={cs.cancellingToken === c.token} class="claim-cancel-btn">
                      {cs.cancellingToken === c.token ? $t('claimCancelling') : $t('claimCancelBtn')}
                    </button>
                  </div>
                  {#if cs.claimErrors[c.token]}<p class="claim-err" style="margin-top:0.4rem">{cs.claimErrors[c.token]}</p>{/if}
                </div>
              </div>
            {/each}

            <!-- All pending claims -->
            {#each cs.claims.filter(c => !c.status || c.status === 'pending') as c (c.token)}
              <div class="reserved-notice">
                <svg class="reserved-icon" width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.3">
                  <circle cx="9" cy="9" r="7.5"/>
                  <path d="M9 5.5v3.5l2.5 2"/>
                </svg>
                <div>
                  <p class="reserved-title">{$t('claimPendingBooking')}</p>
                  <p class="reserved-sub">{fmtDate(c.startsAt)} — {fmtDate(c.endsAt)}</p>
                  <button onclick={() => cs.cancel(c)} disabled={cs.cancellingToken === c.token}
                    class="claim-cancel-btn" style="margin-top:0.5rem">
                    {cs.cancellingToken === c.token ? $t('claimCancelling') : $t('claimCancelBtn')}
                  </button>
                  {#if cs.claimErrors[c.token]}<p class="claim-err">{cs.claimErrors[c.token]}</p>{/if}
                </div>
              </div>
            {/each}

            <!-- No claims in this browser — generic reserved + lookup -->
            {#if cs.claims.length === 0}
              <div class="reserved-notice">
                <svg class="reserved-icon" width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.3">
                  <circle cx="9" cy="9" r="7.5"/>
                  <path d="M9 5.5v3.5l2.5 2"/>
                </svg>
                <div>
                  <p class="reserved-title">{$t('figurineReserved')}</p>
                  <p class="reserved-sub">{$t('figurineNotifyNote')}</p>
                  {#if nextAvailableDate}
                    <p class="reserved-avail">
                      {$t('figurineAvailableFrom')}
                      <strong>{nextAvailableDate.toLocaleDateString(undefined, { day: 'numeric', month: 'long', year: 'numeric' })}</strong>
                    </p>
                  {/if}
                </div>
              </div>
              <div class="claim-lookup" style="margin-bottom:0.75rem">
                {#if !cs.showTokenForm}
                  <button onclick={() => cs.showTokenForm = true} class="claim-lookup-link">{$t('claimHaveCode')}</button>
                {:else}
                  <div class="claim-lookup-form">
                    <input type="text" bind:value={cs.tokenInput} placeholder="XXXX-XXXX" maxlength="9"
                      class="claim-lookup-input" oninput={() => { cs.tokenLookupInfo = null; cs.tokenLookupErr = ''; }} />
                    <button onclick={() => cs.lookupToken()} disabled={cs.tokenLooking} class="claim-lookup-btn">
                      {cs.tokenLooking ? '…' : $t('claimLookupBtn')}
                    </button>
                    <button onclick={() => { cs.showTokenForm = false; cs.tokenInput = ''; cs.tokenLookupInfo = null; }} class="claim-lookup-close">✕</button>
                  </div>
                  {#if cs.tokenLookupErr}<p class="claim-err">{cs.tokenLookupErr}</p>{/if}
                  {#if cs.tokenLookupInfo}
                    <div class="claim-lookup-result">
                      <p class="claim-dates">{fmtDate(cs.tokenLookupInfo.startsAt)} — {fmtDate(cs.tokenLookupInfo.endsAt)}</p>
                      {#if cs.tokenLookupInfo.status === 'pending'}
                        <button onclick={() => cs.cancelFromLookup()} disabled={cs.lookupCancelling} class="claim-cancel-btn">
                          {cs.lookupCancelling ? $t('claimCancelling') : $t('claimCancelBtn')}
                        </button>
                      {:else}
                        <p class="claim-status">{$t('claimStatus')}: {lookupStatusLabel(cs.tokenLookupInfo.status)}</p>
                      {/if}
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
              <button onclick={() => (showWaitlistModal = true)} class="notify-btn">
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3">
                  <path d="M2 7h10M2 4h10M2 10h6"/>
                </svg>
                {$t('waitlistJoinBtn')}
              </button>
            {/if}

            <!-- Notify always shown in reserved state -->
            {#if cs.claims.length > 0}
              <button onclick={() => openModal('notify')} class="notify-btn">
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3">
                  <path d="M7 1a4 4 0 0 1 4 4v3l1.5 2H1.5L3 8V5a4 4 0 0 1 4-4z"/>
                  <path d="M5.5 11.5a1.5 1.5 0 0 0 3 0"/>
                </svg>
                {$t('figurineNotify')}
              </button>
              <button onclick={() => (showWaitlistModal = true)} class="notify-btn">
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3">
                  <path d="M2 7h10M2 4h10M2 10h6"/>
                </svg>
                {$t('waitlistJoinBtn')}
              </button>
            {/if}

            <!-- Book button always available — calendar handles date conflicts -->
            <button onclick={() => (showBookingModal = true)} class="book-btn">
              <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.3">
                <rect x="0.5" y="1.5" width="12" height="11" rx="0.8"/>
                <path d="M3.5 1.5V0.5M9.5 1.5V0.5M0.5 5h12"/>
              </svg>
              {$t('figurineBook')}
            </button>

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

    <FigurineComments figurineId={id} />

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

