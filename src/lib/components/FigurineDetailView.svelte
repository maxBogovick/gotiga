<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import type { Figurine, FigurineSchedule, FigurineStatus } from '$lib/types/api';
  import UnifiedRequestModal from '$lib/components/UnifiedRequestModal.svelte';
  import FigurineClaimRow from '$lib/components/FigurineClaimRow.svelte';
  import FigurineReceiptPanel from '$lib/components/FigurineReceiptPanel.svelte';
  import BrassLens from '$lib/components/BrassLens.svelte';
  import LivingDaguerreotype from '$lib/components/LivingDaguerreotype.svelte';
  import CandleReveal from '$lib/components/CandleReveal.svelte';
  import MemoryMirror from '$lib/components/MemoryMirror.svelte';
  import BecomingReveal from '$lib/components/BecomingReveal.svelte';
  import SecretText from '$lib/components/SecretText.svelte';
  import Lightbox from '$lib/components/Lightbox.svelte';
  import FontSwitcher from '$lib/components/FontSwitcher.svelte';
  import { api, resolveMediaUrl } from '$lib/api';
  import { createFigurineAnalytics } from '$lib/analytics';
  import { t } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';
  import ShowingsTimeline from '$lib/components/ShowingsTimeline.svelte';
  import FigurineComments from '$lib/components/FigurineComments.svelte';
  import { FigurineClaimsStore, type ClaimData } from '$lib/stores/figurine-claims.svelte';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
  import { pageTurn } from '$lib/stores/page-turn.svelte';
  import { focusTrap } from '$lib/actions/focusTrap';
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
  let showRequestModal = $state(false);
  let requestInitialIntent = $state<RequestIntent>('request');
  let createSimilarHref = $derived(`/commission?source=${encodeURIComponent(id)}`);
  let figurineSchedule = $state<FigurineSchedule>({ entries: [] });
  let scheduleLoadFailed = $state(false);
  let isAudioPlaying = $state(false);
  let isCandleLit = $state(false);
  let showLightbox = $state(false);
  let lightboxStartIndex = $state(0);
  let audioRef = $state<HTMLAudioElement | null>(null);
  let videoRef = $state<HTMLVideoElement | null>(null);
  let analyticsClient: ReturnType<typeof createFigurineAnalytics> | null = null;
  let analyticsMountedAt = 0;
  let analyticsEngagedTimer: ReturnType<typeof setTimeout> | null = null;
  let analyticsScrollSent = false;

  function readStoredToken(key: string): string | null {
    try { return localStorage.getItem(key); } catch { return null; }
  }

  function writeStoredToken(key: string, token: string) {
    try { localStorage.setItem(key, token); } catch {}
  }

  function removeStoredToken(key: string) {
    try { localStorage.removeItem(key); } catch {}
  }

  // ── Queue (waitlist) receipt: persisted per-figurine token, like booking claims ──
  let queueKey = $derived(`gotiga_queue_${id}`);
  let queuePosition = $state(0);
  let queueLeaving = $state(false);
  let queueLeft = $state(false);
  let queueLookupStale = $state(false);

  async function loadQueue() {
    const token = readStoredToken(queueKey);
    if (!token) return;
    try {
      const info = await api.getWaitlistByToken(token);
      queueLookupStale = false;
      if (info) {
        queuePosition = info.position;
        queueLeft = false;
      } else {
        // Token no longer valid (notified/removed) - clear it.
        removeStoredToken(queueKey);
        queuePosition = 0;
      }
    } catch {
      // Keep the local token; this can be a transient network/backend error.
      queueLookupStale = true;
    }
  }

  function onQueueJoined(token: string, position: number) {
    writeStoredToken(queueKey, token);
    queuePosition = position;
    queueLeft = false;
    queueLookupStale = false;
  }

  async function leaveQueue() {
    const token = readStoredToken(queueKey);
    if (!token || queueLeaving) return;
    queueLeaving = true;
    try {
      await api.leaveWaitlistByToken(token);
      removeStoredToken(queueKey);
      queuePosition = 0;
      queueLeft = true;
      queueLookupStale = false;
    } finally {
      queueLeaving = false;
    }
  }

  // ── Notify-me subscription receipt: same pattern as the queue ──
  let notifyKey = $derived(`gotiga_notify_${id}`);
  let notifyActive = $state(false);
  let notifyStopping = $state(false);
  let notifyStopped = $state(false);
  let notifyLookupStale = $state(false);

  async function loadNotify() {
    const token = readStoredToken(notifyKey);
    if (!token) return;
    try {
      const info = await api.getNotifyByToken(token);
      notifyLookupStale = false;
      if (info) {
        notifyActive = true;
        notifyStopped = false;
      } else {
        removeStoredToken(notifyKey);
        notifyActive = false;
      }
    } catch {
      // Keep the local token; this can be a transient network/backend error.
      notifyLookupStale = true;
    }
  }

  function onNotifySubscribed(token: string) {
    writeStoredToken(notifyKey, token);
    notifyActive = true;
    notifyStopped = false;
    notifyLookupStale = false;
  }

  async function stopNotify() {
    const token = readStoredToken(notifyKey);
    if (!token || notifyStopping) return;
    notifyStopping = true;
    try {
      await api.cancelNotifyByToken(token);
      removeStoredToken(notifyKey);
      notifyActive = false;
      notifyStopped = true;
      notifyLookupStale = false;
    } finally {
      notifyStopping = false;
    }
  }

  let showings = $derived(figurineSchedule.entries.filter(e => e.entryType === 'showing'));

  // Nearest date when figurine is fully free (after all showings + blocking bookings).
  function localIsoDate(date = new Date()): string {
    const y = date.getFullYear();
    const m = String(date.getMonth() + 1).padStart(2, '0');
    const d = String(date.getDate()).padStart(2, '0');
    return `${y}-${m}-${d}`;
  }

  let todayStr = $state(localIsoDate());
  let todayRefreshTimer: ReturnType<typeof setTimeout> | null = null;

  function refreshToday() {
    todayStr = localIsoDate();
  }

  function clearTodayRefresh() {
    if (!todayRefreshTimer) return;
    clearTimeout(todayRefreshTimer);
    todayRefreshTimer = null;
  }

  function scheduleTodayRefresh() {
    if (typeof window === 'undefined') return;
    clearTodayRefresh();
    const nextMidnight = new Date();
    nextMidnight.setHours(24, 0, 0, 50);
    todayRefreshTimer = setTimeout(() => {
      refreshToday();
      scheduleTodayRefresh();
    }, Math.max(1000, nextMidnight.getTime() - Date.now()));
  }

  let nextAvailableDate = $derived.by(() => {
    const today = todayStr;
    const blocking = figurineSchedule.entries.filter((e) => {
      if (e.entryType !== 'showing' && e.entryType !== 'booking') return false;
      if (figurine.status === 'available') return e.startsAt <= today && e.endsAt >= today;
      return e.endsAt >= today;
    });
    if (blocking.length === 0) return null;
    const latestEnd = blocking.reduce<string | null>((max, e) => !max || e.endsAt > max ? e.endsAt : max, null);
    if (!latestEnd) return null;
    if (latestEnd < today) return null;
    const d = new Date(latestEnd + 'T00:00:00');
    d.setDate(d.getDate() + 1);
    return d;
  });

  // Showing that is happening TODAY (started but not yet ended)
  let hasActiveShowing = $derived(showings.some(s => s.startsAt <= todayStr && s.endsAt >= todayStr));

  type RequestIntent = 'request' | 'reserve' | 'waitlist' | 'viewing' | 'similar' | 'question' | 'notify';
  type AttributeKind = 'dimensions' | 'material' | 'technique';

  function statusLabel(status: FigurineStatus): string {
    switch (status) {
      case 'available': return $t('figurineStatusAvailable');
      case 'reserved': return $t('figurineStatusReserved');
      case 'in_progress': return $t('figurineStatusInProgress');
      case 'sold': return $t('figurineStatusSold');
    }
  }

  let statusUi = $derived.by(() => {
    const isAvailable = figurine.status === 'available';
    const title = isAvailable
      ? (hasActiveShowing ? $t('detailRegistryViewingTitle') : $t('detailRegistryAvailableTitle'))
      : figurine.status === 'reserved'
        ? $t('detailRegistryReservedTitle')
        : figurine.status === 'in_progress'
          ? $t('detailRegistryProgressTitle')
          : $t('detailRegistrySoldTitle');
    const note = figurine.status === 'reserved'
      ? $t('unifiedReservedNote')
      : figurine.status === 'in_progress'
        ? $t('unifiedProgressNote')
        : figurine.status === 'sold'
          ? $t('unifiedSoldNote')
          : hasActiveShowing
            ? $t('unifiedShowingNote')
            : $t('unifiedAvailableNote');
    const mobileSubtitle = isAvailable
      ? (hasActiveShowing ? $t('detailMobileShowingSub') : $t('figurinePriceOnRequest'))
      : figurine.status === 'reserved'
        ? $t('detailMobileReservedSub')
        : figurine.status === 'in_progress'
          ? $t('detailMobileProgressSub')
          : $t('detailMobileSoldSub');
    const defaultIntent: RequestIntent = figurine.status === 'reserved'
      ? 'waitlist'
      : figurine.status === 'in_progress' || figurine.status === 'sold'
        ? 'notify'
        : 'request';

    return {
      label: statusLabel(figurine.status),
      title,
      note,
      mobileSubtitle,
      mobileCtaLabel: isAvailable ? $t('detailMobileRequestCta') : $t('unifiedOpenRequest'),
      mobileIcon: isAvailable ? 'lock' : 'arrow',
      defaultIntent,
    };
  });


  // === CLAIM TOKEN (self-cancellation) ===
  // figurine.id captured once — component is never remounted with a different figurine
  // svelte-ignore state_referenced_locally
  const cs = new FigurineClaimsStore(id, () => {
    void loadSchedule();
  });

  async function loadSchedule() {
    try {
      figurineSchedule = await api.getFigurineSchedule(id);
      scheduleLoadFailed = false;
    } catch {
      scheduleLoadFailed = true;
    }
  }

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

  function themeColor(name: string, fallback: string): string {
    if (typeof document === 'undefined') return fallback;
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
  }

  function normalizeCssColor(color: string, fallback: string): string {
    if (typeof document === 'undefined') return fallback;
    const probe = document.createElement('span');
    probe.style.color = color;
    probe.style.display = 'none';
    document.body.appendChild(probe);
    const computed = getComputedStyle(probe).color;
    document.body.removeChild(probe);
    return computed || fallback;
  }

  function withAlpha(color: string, alpha: number, fallback: string): string {
    const hex = color.trim().match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i);
    if (hex) {
      const value = hex[1].length === 3
        ? hex[1].split('').map(ch => ch + ch).join('')
        : hex[1];
      const r = parseInt(value.slice(0, 2), 16);
      const g = parseInt(value.slice(2, 4), 16);
      const b = parseInt(value.slice(4, 6), 16);
      return `rgba(${r},${g},${b},${alpha})`;
    }

    const normalized = normalizeCssColor(color, fallback);
    const rgb = normalized.match(/^rgba?\((.+)\)$/i);
    if (!rgb) return fallback;
    const parts = rgb[1]
      .replace(/\//g, ' ')
      .replace(/,/g, ' ')
      .trim()
      .split(/\s+/);
    const [r, g, b] = parts;
    if (!r || !g || !b) return fallback;
    return `rgba(${r},${g},${b},${alpha})`;
  }

  // ── Instagram Story share ────────────────────────────────────────────────
  let storySaving    = $state(false);
  let storyBlob      = $state<Blob | null>(null);
  let storyObjectUrl = $state('');
  let showStoryModal = $state(false);
  let canNativeShare = $state(false);
  let storyModalRef = $state<HTMLElement | null>(null);
  let storyError = $state('');

  async function openStoryModal() {
    if (storySaving) return;
    storySaving = true;
    storyError = '';
    try {
      const images = figurine.images ?? [];
      const faceImg = images.find(i => i.imageType === 'face') ?? images[0];
      const imgSrc  = faceImg?.originalUrl ?? faceImg?.url ?? '';
      const W = 1080, H = 1920;
      const storyBase = themeColor('--color-canvas-base', '#f8f1e7');
      const storyInk = themeColor('--color-ink-primary', '#34251c');

      async function buildCanvas(withImage: boolean): Promise<HTMLCanvasElement> {
        const cv = document.createElement('canvas');
        cv.width = W; cv.height = H;
        const ctx = cv.getContext('2d');
        if (!ctx) throw new Error('Canvas 2D context is unavailable');
        ctx.fillStyle = storyBase;
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
        grad.addColorStop(0, withAlpha(storyInk, 0, 'rgba(34,15,10,0)'));
        grad.addColorStop(0.55, withAlpha(storyInk, 0.75, 'rgba(34,15,10,0.75)'));
        grad.addColorStop(1, withAlpha(storyInk, 0.94, 'rgba(34,15,10,0.94)'));
        ctx.fillStyle = grad;
        ctx.fillRect(0, H * 0.45, W, H * 0.55);
        ctx.strokeStyle = withAlpha(storyBase, 0.15, 'rgba(248,241,231,0.15)');
        ctx.lineWidth = 2;
        ctx.strokeRect(40, 40, W - 80, H - 80);
        ctx.fillStyle = storyBase;
        ctx.textAlign = 'center';
        ctx.font = `500 ${Math.round(W * 0.072)}px Georgia, serif`;
        ctx.fillText(figurine.name, W / 2, Math.round(H * 0.825), W - 140);
        ctx.strokeStyle = withAlpha(storyBase, 0.22, 'rgba(248,241,231,0.22)');
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(W * 0.31, H * 0.875); ctx.lineTo(W * 0.69, H * 0.875);
        ctx.stroke();
        ctx.font = `400 ${Math.round(W * 0.048)}px Georgia, serif`;
        ctx.fillStyle = withAlpha(storyBase, 0.55, 'rgba(248,241,231,0.55)');
        ctx.fillText('G O T I G A', W / 2, Math.round(H * 0.916), W - 160);
        return cv;
      }

      let blob: Blob | null = null;
      try {
        const cv = await buildCanvas(true);
        blob = await new Promise<Blob | null>(res => cv.toBlob(res, 'image/jpeg', 0.92));
      } catch {
        try {
          const cv = await buildCanvas(false);
          blob = await new Promise<Blob | null>(res => cv.toBlob(res, 'image/jpeg', 0.92));
        } catch {
          storyError = $t('storyBuildError');
          return;
        }
      }

      if (!blob) {
        storyError = $t('storyBuildError');
        return;
      }

      if (storyObjectUrl) URL.revokeObjectURL(storyObjectUrl);
      storyBlob      = blob;
      storyObjectUrl = URL.createObjectURL(blob);
      const testFile = new File([blob], 'story.jpg', { type: 'image/jpeg' });
      canNativeShare = !!navigator.canShare?.({ files: [testFile] });
      showStoryModal = true;
    } finally {
      storySaving = false;
    }
  }

  function storyFileName() {
    const slug = figurine.name
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/gi, '-')
      .replace(/^-+|-+$/g, '') || 'figurine';
    return `gotiga-${slug}-story.jpg`;
  }

  function downloadStory() {
    if (!storyObjectUrl) return;
    const a = document.createElement('a');
    a.href = storyObjectUrl;
    a.download = storyFileName();
    document.body.appendChild(a); a.click(); document.body.removeChild(a);
    setTimeout(closeStoryModal, 250);
  }

  async function nativeShareStory() {
    if (!storyBlob) return;
    const file = new File([storyBlob], 'gotiga-story.jpg', { type: 'image/jpeg' });
    try {
      await navigator.share({ files: [file], title: figurine.name });
      closeStoryModal();
    } catch { /* user cancelled */ }
  }

  function closeStoryModal() {
    showStoryModal = false;
    if (storyObjectUrl) { URL.revokeObjectURL(storyObjectUrl); storyObjectUrl = ''; }
    storyBlob = null;
  }

  $effect(() => {
    if (!showStoryModal || !storyModalRef) return;
    void tick().then(() => storyModalRef?.focus());
  });

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

  function imagePriority(type: string | undefined | null): number {
    switch (type) {
      case 'face': return 0;
      case 'full': return 1;
      case 'detail': return 2;
      default: return 3;
    }
  }

  let sortedImages = $derived(
    (figurine.images ?? [])
      .map((img, order) => ({ img, order }))
      .sort((a, b) => imagePriority(a.img.imageType) - imagePriority(b.img.imageType) || a.order - b.order)
      .map(({ img }) => img)
  );

  function clampImageIndex(index: number, imageCount = sortedImages.length) {
    const maxIndex = imageCount - 1;
    if (maxIndex < 0) return 0;
    return Math.max(0, Math.min(maxIndex, index));
  }

  let activeImageIndex = $derived(clampImageIndex(selectedImageIndex, sortedImages.length));
  let currentImage = $derived(sortedImages[activeImageIndex]);
  let imageViewMode = $state<'fit' | 'detail'>('fit');
  let isLensEnabled = $state(false);
  let currentImageFit = $derived<'cover' | 'contain'>(imageViewMode === 'detail' ? 'cover' : 'contain');

  // Living daguerreotype (2.5D depth parallax) is the resting presentation of the
  // main plate. It only takes over on desktop pointers, with motion allowed, while
  // the lens is off and we're in fit (contain) mode — otherwise BrassLens keeps its
  // lens / mobile pinch-zoom / lightbox behaviour untouched.
  //
  // It is also suppressed while a prev/next page-turn is armed: a WebGL canvas does
  // not survive a view-transition snapshot reliably across browsers (Safari leaves
  // it blank), so during the turn we render the plain <img> the browser can capture
  // cleanly, and the daguerreotype re-mounts once the leaf has settled.
  let isPointerFine = $state(false);
  let prefersReducedMotion = $state(false);
  let useDaguerreotype = $derived(
    isPointerFine && !prefersReducedMotion && !isLensEnabled && imageViewMode === 'fit'
      && !pageTurn.direction
  );

  // Stage adapts to the work's real proportion. The gallery grid itself stays
  // stable while the image probe resolves, which avoids the tiny-photo/blank-mat
  // layout shifts that came from changing the grid after first paint.
  let aspectNum = $state(0.8);

  function resetImageAspect() {
    aspectNum = 0.8;
  }

  $effect(() => {
    const url = resolveUrl(currentImage?.url);
    if (!url || typeof Image === 'undefined') {
      resetImageAspect();
      return;
    }
    let cancelled = false;
    const probe = new Image();
    probe.onload = () => {
      if (!cancelled && probe.naturalWidth > 0 && probe.naturalHeight > 0) {
        aspectNum = probe.naturalWidth / probe.naturalHeight;
        void tick().then(onScroll);
      }
    };
    probe.onerror = () => {
      if (!cancelled) {
        resetImageAspect();
        void tick().then(onScroll);
      }
    };
    probe.src = url;
    return () => { cancelled = true; };
  });

  let viewerAspect = $derived(aspectNum < 0.9 ? '5 / 4' : '16 / 10');
  let plateStyle = $derived(`--viewer-aspect-ratio: ${viewerAspect};`);

  let lightboxImages = $derived(
    sortedImages.map((img) => ({ url: resolveUrl(img.originalUrl ?? img.url), alt: img.altText ?? '' }))
  );
  let canOpenLightbox = $derived(lightboxImages.length > 0);
  let isSaved = $derived(savedFigurines.has(id));

  function hasText(value: string | null | undefined): value is string {
    return Boolean(value?.trim());
  }

  let visibleProcessSteps = $derived(
    (figurine.processSteps ?? []).filter((step) => hasText(step.imageUrl) || hasText(step.description))
  );
  let visibleRelatedItems = $derived(
    (figurine.relatedItems ?? []).filter((item) => hasText(item.id) && hasText(item.name))
  );
  let hasHistorySection = $derived(hasText(figurine.fullDescription));
  let hasMakingSection = $derived(visibleProcessSteps.length > 0);

  // "The Becoming" reveal: first stage (raw) ↔ the finished work.
  let firstStep = $derived(visibleProcessSteps[0]);
  let lastStep = $derived(visibleProcessSteps[visibleProcessSteps.length - 1]);
  let becomingBefore = $derived(firstStep ? resolveUrl(firstStep.imageUrl) : '');
  let becomingAfter = $derived(currentImage?.url ? resolveUrl(currentImage.url) : '');
  let hasBecoming = $derived(
    hasText(firstStep?.imageUrl) && hasText(currentImage?.url) && becomingBefore !== becomingAfter
  );
  // With the reveal in place, the Memory Mirror is the way to all the in-between
  // stages — surface it whenever there is a real sequence (≥2), not only >4.
  let showMirrorLink = $derived(
    hasBecoming ? visibleProcessSteps.length >= 2 : visibleProcessSteps.length > 4
  );

  function toRoman(n: number): string {
    if (!Number.isFinite(n) || n <= 0) return String(n);
    if (n >= 4000) return String(n);
    const table: [number, string][] = [
      [1000, 'M'], [900, 'CM'], [500, 'D'], [400, 'CD'],
      [100, 'C'], [90, 'XC'], [50, 'L'], [40, 'XL'],
      [10, 'X'], [9, 'IX'], [5, 'V'], [4, 'IV'], [1, 'I'],
    ];
    let out = '';
    for (const [v, s] of table) { while (n >= v) { out += s; n -= v; } }
    return out;
  }

  function safeCssIdentifier(value: string): string {
    return value.replace(/[^a-zA-Z0-9_-]/g, '-').replace(/^-+/, 'id-') || 'figurine';
  }

  // During a prev/next page-turn the whole leaf turns as one, so the plate must
  // stay inside the root snapshot — drop its name while a turn is armed. The
  // card→detail morph (no direction armed) keeps the shared name.
  let viewTransitionName = $derived(
    pageTurn.direction ? 'none' : `figurine-${safeCssIdentifier(id)}`
  );
  let hasVideoSection = $derived(hasText(figurine.videoUrl));
  let hasWorkStorySection = $derived(hasHistorySection || hasMakingSection || hasVideoSection);
  let attributes = $derived.by(() => {
    const items: { kind: AttributeKind; label: string; value: string }[] = [];
    if (hasText(figurine.dimensions)) items.push({ kind: 'dimensions', label: $t('figurineDimensions'), value: figurine.dimensions });
    if (hasText(figurine.material)) items.push({ kind: 'material', label: $t('figurineMaterial'), value: figurine.material });
    if (hasText(figurine.technique)) items.push({ kind: 'technique', label: $t('figurineTechnique'), value: figurine.technique });
    return items;
  });
  let hasAttributesSection = $derived(attributes.length > 0);
  let hasScheduleSection = $derived(figurineSchedule.entries.length > 0);
  let hasFactsSection = $derived(hasScheduleSection);
  let canShowPersonalRecord = $derived(figurine.status === 'available' || figurine.status === 'reserved');
  let hasClaimRecords = $derived(cs.claims.length > 0 || (cs.cancelledTokens.size > 0 && cs.claims.length === 0));
  let hasClaimLookupState = $derived(cs.showTokenForm || Boolean(cs.tokenLookupInfo) || Boolean(cs.tokenLookupErr));
  let hasPersonalRecord = $derived(
    canShowPersonalRecord && (hasClaimRecords || hasClaimLookupState)
  );

  function resolveUrl(path: string | undefined | null) { return resolveMediaUrl(path) ?? ''; }
  function imageTypeLabel(type: string | undefined | null) {
    switch (type) {
      case 'face': return $t('detailImageMain');
      case 'detail': return $t('detailImageDetail');
      case 'full': return $t('detailImageScale');
      default: return $t('detailImageView');
    }
  }
  function imageRoleNote(type: string | undefined | null) {
    switch (type) {
      case 'face': return $t('detailImageMainNote');
      case 'detail': return $t('detailImageDetailNote');
      case 'full': return $t('detailImageScaleNote');
      default: return $t('detailImageViewNote');
    }
  }
  function processStepLabel(type: string | undefined | null) {
    switch (type) {
      case 'sketch': return $t('figurineStepSketch');
      case 'prototype': return $t('figurineStepPrototype');
      case 'modeling': return $t('figurineStepModeling');
      case 'painting': return $t('figurineStepPainting');
      case 'finish': return $t('figurineStepFinish');
      default: return $t('detailMakingRecordStep');
    }
  }
  function selectImage(index: number) {
    const maxIndex = sortedImages.length - 1;
    if (maxIndex < 0) {
      selectedImageIndex = 0;
      return;
    }
    const nextIndex = clampImageIndex(index, sortedImages.length);
    if (nextIndex !== selectedImageIndex) {
      selectedImageIndex = nextIndex;
      imageViewMode = 'fit';
      isLensEnabled = false;
    }
  }
  function openLightbox(index: number) {
    if (!canOpenLightbox) return;
    lightboxStartIndex = Math.max(0, Math.min(lightboxImages.length - 1, index));
    showLightbox = true;
  }
  function toggleSaved() {
    analyticsClient?.cta('wishlist');
    savedFigurines.toggle(id);
  }
  function toggleLens() {
    isLensEnabled = !isLensEnabled;
  }
  let lastGrimoireCloseAt = $state(0);

  function openGrimoire() {
    if (Date.now() - lastGrimoireCloseAt < 350) return;
    isGrimoireOpen = true;
  }

  function closeGrimoire() {
    lastGrimoireCloseAt = Date.now();
    isGrimoireOpen = false;
  }

  function toggleCandle() { isCandleLit = !isCandleLit; }

  function toggleFullscreen() {
    if (!videoRef) return;
    document.fullscreenElement ? document.exitFullscreen() : videoRef.requestFullscreen().catch(() => {});
  }

  let audioFadeTimer: ReturnType<typeof setInterval> | null = null;

  function clearAudioFade() {
    if (!audioFadeTimer) return;
    clearInterval(audioFadeTimer);
    audioFadeTimer = null;
  }

  function toggleAudio() {
    if (!audioRef || !figurine.ambiencePath) return;
    if (isAudioPlaying) {
      fadeOutAudio();
    } else {
      clearAudioFade();
      audioRef.volume = 0;
      audioRef.play()
        .then(() => {
          isAudioPlaying = true;
          fadeInAudio();
        })
        .catch(() => {
          clearAudioFade();
          isAudioPlaying = false;
        });
    }
  }

  function fadeInAudio() {
    if (!audioRef) return;
    clearAudioFade();
    let vol = 0;
    audioFadeTimer = setInterval(() => {
      if (!audioRef || vol >= 0.5) {
        clearAudioFade();
        return;
      }
      vol = Math.min(0.5, vol + 0.05);
      audioRef.volume = vol;
    }, 100);
  }

  function fadeOutAudio() {
    if (!audioRef) return;
    clearAudioFade();
    let vol = audioRef.volume;
    audioFadeTimer = setInterval(() => {
      if (!audioRef || vol <= 0.05) {
        clearAudioFade();
        audioRef?.pause();
        isAudioPlaying = false;
        return;
      }
      vol = Math.max(0, vol - 0.05);
      audioRef.volume = vol;
    }, 100);
  }

  // ── Share ────────────────────────────────────────────────────────────────
  let copied = $state(false);
  let copiedTimer: ReturnType<typeof setTimeout> | null = null;

  async function copyShareUrl(url: string) {
    if (!navigator.clipboard?.writeText) return;
    await navigator.clipboard.writeText(url);
    copied = true;
    if (copiedTimer) clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => { copied = false; copiedTimer = null; }, 2200);
  }

  async function share() {
    const url = window.location.href;
    if (navigator.share) {
      try {
        await navigator.share({ title: figurine.name, text: figurine.shortText ?? figurine.name, url });
        return;
      } catch {
        // User cancellation is harmless; other share failures can still fall back
        // to copying the URL where clipboard access is available.
      }
    }
    try {
      await copyShareUrl(url);
    } catch {
      copied = false;
    }
  }

  // Arm the book page-turn for prev/next paging — but only for a plain in-tab
  // navigation. A modified click (cmd/ctrl/shift/middle → new tab/window) won't
  // fire onNavigate, so arming there would strand the direction and blank the
  // current plate's morph name.
  function armPageTurn(e: MouseEvent, direction: 'forward' | 'backward') {
    if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    pageTurn.arm(direction);
  }

  // ── Keyboard gallery navigation ───────────────────────────────────────────
  function handleKeydown(e: KeyboardEvent) {
    if (showStoryModal && e.key === 'Escape') {
      closeStoryModal();
      return;
    }
    if (showLightbox || showRequestModal || showStoryModal) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest('input, textarea, select, button, a, [contenteditable="true"]')) return;
    if (sortedImages.length <= 1) return;
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      selectImage(activeImageIndex - 1);
    }
    if (e.key === 'ArrowRight') {
      e.preventDefault();
      selectImage(activeImageIndex + 1);
    }
  }

  // ── Sticky condensed nav — три фазы ─────────────────────────────────────
  let scrollY = $state(0);
  let scrolled    = $derived(scrollY > 80);

  // DOM anchor for the sticky navigation identity.
  let galleryRef:  HTMLElement | undefined = $state();
  let galleryObserver: IntersectionObserver | null = null;
  const TOPNAV_THRESHOLD = 130; // высота SiteHeader + topnav

  let galleryExited  = $state(false); // Phase 2: галерея ушла за экран

  function defaultRequestIntent(): RequestIntent {
    return statusUi.defaultIntent;
  }

  function openRequestModal(intent = defaultRequestIntent()) {
    const analyticsIntent = intent === 'similar'
      ? 'create_similar'
      : intent === 'viewing'
        ? 'booking'
        : intent;
    analyticsClient?.cta(analyticsIntent);
    requestInitialIntent = intent;
    showRequestModal = true;
  }

  function openClaimLookup() {
    cs.showTokenForm = true;
  }

  function closeClaimLookup() {
    cs.showTokenForm = false;
    cs.tokenInput = '';
    cs.tokenLookupInfo = null;
    cs.tokenLookupErr = '';
  }

  function handlePersonalRecordToggle(e: Event) {
    if (!(e.currentTarget as HTMLDetailsElement).open) {
      closeClaimLookup();
    }
  }

  // Scroll path stays cheap: only read window.scrollY (drives `scrolled` and the
  // CTA threshold). Whether the gallery has left the top of the viewport is tracked
  // by an IntersectionObserver below, so we no longer call getBoundingClientRect on
  // every scroll frame (that forced a synchronous layout reflow).
  function onScroll() {
    scrollY = window.scrollY;
    if (!analyticsClient || analyticsScrollSent) return;
    const doc = document.documentElement;
    const maxScroll = Math.max(1, doc.scrollHeight - window.innerHeight);
    const depth = Math.round((window.scrollY / maxScroll) * 100);
    if (depth >= 50) {
      analyticsScrollSent = true;
      analyticsClient.engaged({
        durationMs: Math.max(0, Date.now() - analyticsMountedAt),
        scrollDepth: Math.min(100, depth),
      });
    }
  }

  function handleVisibility() {
    if (document.visibilityState === 'visible') {
      refreshToday();
      scheduleTodayRefresh();
      cs.verify();
    }
  }

  onMount(() => {
    analyticsClient = createFigurineAnalytics(id);
    analyticsMountedAt = Date.now();
    analyticsClient.view();
    analyticsEngagedTimer = setTimeout(() => {
      analyticsClient?.engaged({
        durationMs: Math.max(0, Date.now() - analyticsMountedAt),
        scrollDepth: Math.min(
          100,
          Math.round((window.scrollY / Math.max(1, document.documentElement.scrollHeight - window.innerHeight)) * 100)
        ),
      });
    }, 8000);
    isPointerFine = window.matchMedia('(pointer: fine)').matches;
    prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    window.addEventListener('keydown', handleKeydown);
    window.addEventListener('scroll', onScroll, { passive: true });
    document.addEventListener('visibilitychange', handleVisibility);
    refreshToday();
    scheduleTodayRefresh();
    onScroll();
    void loadSchedule();
    cs.load();
    savedFigurines.load();
    cs.verify();
    cs.startPolling();
    void loadQueue();
    void loadNotify();
    if (galleryRef) {
      // The observer reports the gallery's geometry in its own callback (no reflow on
      // our side) and also refires on layout/resize changes — e.g. when the image
      // aspect probe resolves — so it replaces the old ResizeObserver+rect combo.
      galleryObserver = new IntersectionObserver(
        ([entry]) => {
          galleryExited = !entry.isIntersecting && entry.boundingClientRect.bottom < TOPNAV_THRESHOLD;
        },
        { rootMargin: `-${TOPNAV_THRESHOLD}px 0px 0px 0px`, threshold: 0 }
      );
      galleryObserver.observe(galleryRef);
    }
  });

  onDestroy(() => {
    // onDestroy runs during SSR teardown in Svelte 5; these listeners were only added
    // in onMount (client), so guard the browser-only cleanup.
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', handleKeydown);
      window.removeEventListener('scroll', onScroll);
      document.removeEventListener('visibilitychange', handleVisibility);
    }
    if (copiedTimer) clearTimeout(copiedTimer);
    if (analyticsEngagedTimer) clearTimeout(analyticsEngagedTimer);
    galleryObserver?.disconnect();
    clearTodayRefresh();
    clearAudioFade();
    if (storyObjectUrl) URL.revokeObjectURL(storyObjectUrl);
    if (audioRef) { audioRef.pause(); audioRef = null; }
    cs.stopPolling();
  });
</script>

{#if figurine.ambiencePath}
  <audio bind:this={audioRef} src={resolveUrl(figurine.ambiencePath)} loop></audio>
{/if}

<CandleReveal isActive={isCandleLit} />

<div class="page-root" class:page-root--has-cta={scrollY > 300} class:page-root--candle={isCandleLit}>
  <UnifiedRequestModal
    isOpen={showRequestModal}
    figurineName={figurine.name}
    figurineId={figurine.id}
    status={figurine.status}
    schedule={figurineSchedule}
    initialIntent={requestInitialIntent}
    onJoined={onQueueJoined}
    onNotified={onNotifySubscribed}
    onBookingCreated={(claim: ClaimData) => cs.onBookingCreated(claim)}
    onClose={() => (showRequestModal = false)}
  />

  <!-- ── Story share modal ──────────────────────────────────────────────── -->
  {#if showStoryModal}
    <div class="story-backdrop" transition:fade={{ duration: 200 }}>
      <button type="button" class="story-backdrop-dismiss" onclick={closeStoryModal} aria-label={$t('lightboxClose')}></button>
      <div class="story-modal" bind:this={storyModalRef} transition:fade={{ duration: 150 }}
           role="dialog" aria-modal="true" aria-labelledby="story-modal-title" tabindex="-1" use:focusTrap>
        <button type="button" class="story-close" onclick={closeStoryModal} aria-label={$t('lightboxClose')}>
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
            <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke-linecap="round" />
          </svg>
        </button>

        <p id="story-modal-title" class="story-modal-title">{$t('figurineStoryShare')}</p>

        <!-- 9:16 preview -->
        {#if storyObjectUrl}
          <div class="story-preview-wrap">
            <img src={storyObjectUrl} alt={$t('figurineStoryShare')} class="story-preview-img" />
          </div>
        {/if}

        <div class="story-actions">
          {#if canNativeShare}
            <button type="button" class="story-btn story-btn--primary" onclick={nativeShareStory}>
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3">
                <circle cx="11" cy="3" r="1.5"/><circle cx="3" cy="7" r="1.5"/><circle cx="11" cy="11" r="1.5"/>
                <path d="M4.4 6.1l5.2-2.6M4.4 7.9l5.2 2.6"/>
              </svg>
              {$t('storyShare')}
            </button>
          {/if}
          <button type="button" class="story-btn {canNativeShare ? 'story-btn--secondary' : 'story-btn--primary'}" onclick={downloadStory}>
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

  <MemoryMirror
    isOpen={isGrimoireOpen}
    steps={visibleProcessSteps}
    finalImage={resolveUrl(currentImage?.url)}
    onClose={closeGrimoire}
  />

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
          <div class="topnav-fig-nav" role="group" aria-label="{$t('figurineNavPrev')} / {$t('figurineNavNext')}">
            {#if prev}
              <a
                href="/figurines/{prev.id}"
                class="fig-nav-pill"
                title={prev.name}
                aria-label="{$t('figurineNavPrev')}: {prev.name}"
                data-sveltekit-preload-data="hover"
                onclick={(e) => armPageTurn(e, 'backward')}
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
                onclick={(e) => armPageTurn(e, 'forward')}
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

      <!-- Center: progressive identity once the gallery leaves the viewport -->
      <div class="topnav-center">

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
                type="button"
                class="topnav-mini-img"
                onclick={() => openLightbox(activeImageIndex)}
                title={$t('figurineFullscreen')}
                aria-label={$t('figurineFullscreen')}
              >
                <img src={resolveUrl(currentImage.url)} alt="" loading="eager" />
              </button>
            {/if}

            <span class="topnav-ident-name">{figurine.name}</span>

            {#if sortedImages.length > 1}
              <div class="topnav-dots" role="group" aria-label={$t('figurineShowView')}>
                {#each sortedImages as _, i}
                  <button
                    type="button"
                    class="topnav-dot {i === activeImageIndex ? 'topnav-dot--on' : ''}"
                    onclick={() => selectImage(i)}
                    aria-label="{$t('figurineShowView')} {i + 1}"
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

      <!-- Right: controls — candle (mood + reveal), whisper (if audio), share -->
      <div class="topnav-controls">
        <button
          type="button"
          onclick={toggleCandle}
          class="control-btn {isCandleLit ? 'control-btn--lit' : ''}"
          aria-label={isCandleLit ? $t('figurineExtinguish') : $t('figurineCandle')}
          title={isCandleLit ? $t('figurineExtinguish') : $t('figurineCandle')}
        >
          <svg class="control-svg" width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.35" aria-hidden="true">
            <path d="M7 1.3c1.1 1.15 2.5 2.88 2.5 5.05a2.5 2.5 0 0 1-5 0C4.5 4.8 5.42 3.72 6.2 2.8c.32-.38.6-.72.8-1.5z" fill="currentColor" fill-opacity="0.12"/>
            <path d="M4.25 12.2h5.5M5.15 9.6h3.7M7 1.3c1.1 1.15 2.5 2.88 2.5 5.05a2.5 2.5 0 0 1-5 0C4.5 4.8 5.42 3.72 6.2 2.8c.32-.38.6-.72.8-1.5z"/>
          </svg>
          <span class="btn-label">{isCandleLit ? $t('figurineExtinguish') : $t('figurineCandle')}</span>
        </button>

        {#if figurine.ambiencePath}
          <button
            type="button"
            onclick={toggleAudio}
            class="control-btn {isAudioPlaying ? 'control-btn--active' : ''}"
            aria-label={isAudioPlaying ? $t('figurineSilence') : $t('figurineWhisper')}
            title={isAudioPlaying ? $t('figurineSilence') : $t('figurineWhisper')}
          >
            <span class="audio-indicator {isAudioPlaying ? 'audio-indicator--on' : ''}"></span>
            <span class="btn-label">{isAudioPlaying ? $t('figurineSilence') : $t('figurineWhisper')}</span>
          </button>
        {/if}

        <button
          type="button"
          onclick={openStoryModal}
          disabled={storySaving}
          class="control-btn control-btn--utility {storySaving ? 'control-btn--active' : ''}"
          aria-label={storySaving ? $t('figurineStorySaving') : $t('figurineStoryShare')}
          title={storySaving ? $t('figurineStorySaving') : $t('figurineStoryShare')}
        >
          {#if storySaving}
            <span class="control-spinner" aria-hidden="true"></span>
            <span class="btn-label">{$t('figurineStorySaving')}</span>
          {:else}
            <svg class="control-svg" width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.35" aria-hidden="true">
              <rect x="2" y="1.5" width="9" height="10" rx="1.2"/>
              <path d="M4.1 8.5 6 6.8l1.3 1.1 1.6-2 1.1 1.4"/>
              <circle cx="4.6" cy="4.1" r="0.65" fill="currentColor" stroke="none"/>
            </svg>
            <span class="btn-label">{$t('figurineStoryShare')}</span>
          {/if}
        </button>

        <button
          type="button"
          onclick={share}
          class="control-btn control-btn--utility {copied ? 'control-btn--active' : ''}"
          aria-label={$t('figurineShare')}
          title={$t('figurineShare')}
        >
          {#if copied}
            <svg class="control-svg" width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
              <path d="M2 6l3 3 5-5"/>
            </svg>
            <span class="btn-label">{$t('figurineCopied')}</span>
          {:else}
            <svg class="control-svg" width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
              <path d="M9 1.5a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3zM3 4.5a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3zM9 7.5a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3z"/>
              <path d="M7.5 2.7l-3 1.8M7.5 9.3l-3-1.8"/>
            </svg>
            <span class="btn-label">{$t('figurineShare')}</span>
          {/if}
        </button>
      </div>
    </nav>

    <!-- ── MAIN GRID ── -->
    {#if storyError}
      <p class="detail-inline-alert" role="alert">{storyError}</p>
    {/if}

    <div class="main-grid">

      <!-- LEFT: Gallery with vertical thumbnail strip -->
      <div class="gallery-col" bind:this={galleryRef}>
	        <div
	          class="gallery-layout"
	          class:gallery-layout--solo={sortedImages.length <= 1}
	          style={plateStyle}
	        >

          {#if sortedImages.length > 1}
            <nav class="thumbs-strip" aria-label={$t('figurineShowView')}>
              {#each sortedImages as img, i}
                <button
                  type="button"
                  class="thumb-v {activeImageIndex === i ? 'thumb-v--active' : ''}"
                  onclick={() => selectImage(i)}
                  aria-label="{imageTypeLabel(img.imageType)}: {imageRoleNote(img.imageType)}"
                  aria-current={activeImageIndex === i ? 'true' : undefined}
                >
                  <span class="thumb-v-media">
                    <img src={resolveUrl(img.thumbUrl ?? img.url)} alt="" class="thumb-v-img" loading="lazy" />
                  </span>
                  <span class="thumb-v-copy">
                    <span class="thumb-v-label">{imageTypeLabel(img.imageType)}</span>
                    <span class="thumb-v-note">{imageRoleNote(img.imageType)}</span>
                  </span>
                  <div class="thumb-v-bar" aria-hidden="true"></div>
                </button>
              {/each}
            </nav>
          {/if}

	          <figure class="image-col">
            <div class="image-frame">
              <div
                class="image-stage"
                class:image-stage--detail={imageViewMode === 'detail'}
                data-figurine-plate
                style="view-transition-name: {viewTransitionName};"
              >
                {#if useDaguerreotype}
                  <!-- Persistent across image switches (NO {#key}) so the WebGL
                       context/shaders survive ←/→ paging; the component reloads
                       only its textures and crossfades them internally. -->
                  <div class="image-layer">
                    <LivingDaguerreotype
                      src={resolveUrl(currentImage?.url)}
                      depthSrc={resolveUrl(currentImage?.depthUrl) || null}
                      intensity={currentImage?.parallaxIntensity ?? undefined}
                      alt={currentImage?.altText ?? figurine.name}
                      class="w-full h-full"
                      onActivate={() => canOpenLightbox && openLightbox(activeImageIndex)}
                    />
                  </div>
                {:else}
                  {#key currentImage?.id}
                    <div class="image-layer" in:fade={{ duration: 220 }}>
                      <BrassLens
                        src={resolveUrl(currentImage?.url)}
                        alt={currentImage?.altText ?? figurine.name}
                        class="w-full h-full"
                        imageFit={currentImageFit}
                        objectPosition="center center"
                        lensEnabled={isLensEnabled}
                        onOpenLightbox={() => canOpenLightbox && openLightbox(activeImageIndex)}
                      />
                    </div>
                  {/key}
                {/if}

                {#if sortedImages.length > 1}
                  <div class="img-counter" aria-hidden="true">
                    <span class="img-counter-type">{imageTypeLabel(currentImage?.imageType)}</span>
                    <span class="img-counter-num">{activeImageIndex + 1}<span class="img-counter-sep">/</span>{sortedImages.length}</span>
                  </div>
                {/if}

                <button
                  type="button"
                  class="gallery-heart"
                  class:gallery-heart--saved={isSaved}
                  onclick={toggleSaved}
                  aria-label={isSaved ? $t('cardSaved') : $t('cardSave')}
                  title={isSaved ? $t('cardSaved') : $t('cardSave')}
                  aria-pressed={isSaved}
                >
                  <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                    <path
                      d="M7 12.5C7 12.5 1 8.5 1 4.5C1 2.5 2.5 1 4.5 1C5.5 1 6.5 1.8 7 3C7.5 1.8 8.5 1 9.5 1C11.5 1 13 2.5 13 4.5C13 8.5 7 12.5 7 12.5Z"
                      fill={isSaved ? 'currentColor' : 'none'}
                      stroke="currentColor"
                      stroke-width="1.15"
                      stroke-linejoin="round"
                    />
                  </svg>
                </button>

                <button
                  type="button"
                  class="gallery-lens"
                  class:gallery-lens--active={isLensEnabled}
                  onclick={toggleLens}
                  aria-label={isLensEnabled ? $t('detailImageLensOff') : $t('detailImageLensOn')}
                  title={isLensEnabled ? $t('detailImageLensOff') : $t('detailImageLensOn')}
                  aria-pressed={isLensEnabled}
                >
                  <svg width="15" height="15" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" aria-hidden="true">
                    <circle cx="6" cy="6" r="3.7" />
                    <path d="M8.8 8.8L12 12" />
                  </svg>
                </button>

                <div class="image-view-tools" aria-label={$t('detailImageViewMode')}>
                  <button
                    type="button"
                    class="image-view-tool {imageViewMode === 'fit' ? 'image-view-tool--active' : ''}"
                    onclick={() => (imageViewMode = 'fit')}
                    aria-pressed={imageViewMode === 'fit'}
                  >
                    {$t('detailImageFit')}
                  </button>
                  <button
                    type="button"
                    class="image-view-tool {imageViewMode === 'detail' ? 'image-view-tool--active' : ''}"
                    onclick={() => (imageViewMode = 'detail')}
                    aria-pressed={imageViewMode === 'detail'}
                  >
                    {$t('detailImageDetailView')}
                  </button>
                </div>

                {#if canOpenLightbox}
                  <button
                    type="button"
                    onclick={() => openLightbox(activeImageIndex)}
                    class="expand-btn"
                    aria-label={$t('figurineFullscreen')}
                  >
                    <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                      <path d="M1 4V1h3M6 1h3v3M9 6v3H6M4 9H1V6"/>
                    </svg>
                    {$t('figurineFullscreen')}
                  </button>
                {/if}

                <div class="image-vignette"></div>
              </div>
            </div>

            {#if currentImage}
              <figcaption class="plate-caption">
                <span class="plate-caption-label">{imageTypeLabel(currentImage.imageType)}</span>
                <span class="plate-caption-note">{imageRoleNote(currentImage.imageType)}</span>
                {#if hasText(figurine.dimensions)}
                  <span class="plate-caption-dim">{figurine.dimensions}</span>
                {/if}
              </figcaption>
            {/if}
          </figure>
        </div>
      </div>

      <!-- RIGHT: Details — on warm page bg, no card wrapper -->
      <div class="details-col">

        <!-- Colophon: specimen ref + year, centered above the name -->
        <div class="d-eyebrow">
          <div class="eyebrow-tags">
            <span class="colophon-ref">ARC-{id.slice(0, 8).toUpperCase()}</span>
            {#if figurine.year}
              <span class="eyebrow-sep">·</span>
              <span class="eyebrow-year">Anno {figurine.year}</span>
            {/if}
          </div>
        </div>

        <h1 class="figurine-title {figurine.name.length > 60 ? 'figurine-title--long' : figurine.name.length > 30 ? 'figurine-title--medium' : ''}">{figurine.name}</h1>

        <span class="colophon-kind">{$t('detailKind')}</span>

        {#if hasText(figurine.shortText)}
          <p class="lore-short">{figurine.shortText}</p>
        {/if}

        {#if hasAttributesSection}
          <dl class="hero-facts" aria-label={$t('figurineAttributes')}>
            {#each attributes as attr (attr.kind)}
              <div>
                <dt>{attr.label}</dt>
                <dd>{attr.value}</dd>
              </div>
            {/each}
          </dl>
        {/if}

        {#if hasText(figurine.secretText) && isCandleLit}
          <div class="secret-anchor">
            <SecretText text={figurine.secretText} isCandleLit={isCandleLit} />
          </div>
        {/if}

        <!-- ── Status & enquiry: commerce collapsed to one quiet marginal line ── -->
        <div class="entry-status entry-status--{figurine.status}">
          <div class="entry-status-head">
              <span class="entry-status-marque">
                <span class="entry-status-kind">
                  {statusUi.label}
                </span>
                <span class="entry-wax" aria-hidden="true">GT</span>
              </span>
            <span class="entry-registry">{$t('detailRegistryNo')} {id.slice(0, 3).toUpperCase()}</span>
          </div>

          <div class="entry-status-body">
            <div class="entry-status-copy">
              <h2 class="entry-status-title">
                {statusUi.title}
              </h2>

              <p class="entry-status-line">
                <span class="entry-price">{$t('figurinePriceOnRequest')}</span>
                {#if figurine.status === 'available'}
                  {#if hasActiveShowing}
                    <span class="entry-sep">·</span>{$t('detailPresenceOnExhibition')}{#if nextAvailableDate} <span class="entry-sep">·</span>{$t('figurineAvailableFrom')} {nextAvailableDate.toLocaleDateString(undefined, { day: 'numeric', month: 'long', year: 'numeric' })}{/if}
                  {:else if nextAvailableDate}
                    <span class="entry-sep">·</span>{$t('figurineAvailableFrom')} {nextAvailableDate.toLocaleDateString(undefined, { day: 'numeric', month: 'long', year: 'numeric' })}
                  {:else}
                    <span class="entry-sep">·</span>{$t('detailPresenceAvailableNow')}
                  {/if}
                {:else if figurine.status === 'reserved'}
                  <span class="entry-sep">·</span>{#if nextAvailableDate}{$t('detailPresenceMayFree')} {nextAvailableDate.toLocaleDateString(undefined, { day: 'numeric', month: 'long', year: 'numeric' })}{:else}{$t('figurineReserved')}{/if}
                {/if}
              </p>
            </div>

            <div class="entry-action-stack">
              <button type="button" onclick={() => openRequestModal()} class="entry-action">
                {$t('unifiedOpenRequest')} →
              </button>
              {#if figurine.status === 'available'}
                <button type="button" class="entry-action entry-action--secondary" onclick={() => openRequestModal('reserve')}>
                  {$t('unifiedReserveShort')} →
                </button>
              {:else}
                <a href={createSimilarHref} class="entry-action entry-action--secondary" onclick={() => analyticsClient?.cta('create_similar')}>
                  {$t('commissionCreateSimilarCta')} →
                </a>
              {/if}
              <p class="entry-action-note">
                {statusUi.note}
              </p>
            </div>
          </div>

          <div class="entry-status-facts" aria-label={$t('detailRegistryFacts')}>
            <span>{$t('detailReplyWindow')}</span>
            <span>{$t('detailNoObligation')}</span>
            <span>{$t('detailPersonalTransfer')}</span>
          </div>

          <section class="trust-ledger" aria-label={$t('detailTrustBlockLabel')}>
            <p class="trust-ledger-mark">
              <span class="trust-ledger-lozenge" aria-hidden="true"></span>
              {$t('detailTrustUnique')}
            </p>
            <div class="trust-ledger-next">
              {#if figurine.status === 'available'}
                <p>{$t('detailTrustNextAvailable')}</p>
              {:else}
                <p>
                  {figurine.status === 'reserved'
                    ? $t('detailTrustNextReserved')
                    : figurine.status === 'in_progress'
                      ? $t('detailTrustNextProgress')
                      : $t('detailTrustNextSold')}
                </p>
              {/if}
              <a class="trust-ledger-link" href="/figurines/{id}/passport" onclick={() => analyticsClient?.cta('passport')}>
                {$t('detailOpenPassport')} →
              </a>
            </div>
          </section>

          {#if scheduleLoadFailed}
            <p class="queue-receipt-left queue-receipt-left--warning">{$t('detailScheduleLoadStale')}</p>
          {/if}

          {#if figurine.status === 'reserved'}
            {#if queuePosition > 0}
              <FigurineReceiptPanel
                title={$t('detailQueuePanelTitle')}
                note={$t('detailQueueNote')}
                stale={queueLookupStale}
                position={queuePosition}
                positionLabel={$t('detailQueuePositionLabel')}
                actionLabel={$t('detailQueueLeave')}
                actionBusyLabel={$t('detailQueueLeaving')}
                busy={queueLeaving}
                onAction={leaveQueue}
              />
            {:else if queueLookupStale}
              <p class="queue-receipt-left queue-receipt-left--warning">{$t('detailReceiptStale')}</p>
            {:else if queueLeft}
              <p class="queue-receipt-left">{$t('detailQueueLeft')}</p>
            {/if}
          {:else if figurine.status === 'in_progress' || figurine.status === 'sold'}
            {#if notifyActive}
              <FigurineReceiptPanel
                title={$t('detailNotifyPanelTitle')}
                note={$t('detailNotifyNote')}
                stale={notifyLookupStale}
                actionLabel={$t('detailNotifyStop')}
                actionBusyLabel={$t('detailNotifyStopping')}
                busy={notifyStopping}
                variant="notify"
                onAction={stopNotify}
              />
            {:else if notifyLookupStale}
              <p class="queue-receipt-left queue-receipt-left--warning">{$t('detailReceiptStale')}</p>
            {:else if notifyStopped}
              <p class="queue-receipt-left">{$t('detailNotifyStopped')}</p>
            {/if}
          {/if}
        </div>

        <!-- Personal record for this work: folded into a disclosure, not a dashboard on the leaf. -->
        {#if canShowPersonalRecord}
          <details class="entry-record" class:entry-record--empty={!hasPersonalRecord} ontoggle={handlePersonalRecordToggle}>
            <summary>{hasPersonalRecord ? $t('detailYourRecord') : $t('claimHaveCode')}</summary>
            <div class="entry-record-body">
              {#if hasClaimRecords}
                <section class="entry-record-section entry-record-section--claims" aria-label={$t('detailYourRecord')}>
                  <div class="claims-panel {cs.claims.some(c => c.status === 'confirmed') ? 'claims-panel--has-confirmed' : ''}">
                    <div class="claims-panel-header">
                      {cs.claims.some(c => c.status === 'confirmed') ? $t('claimsYours') : $t('claimsPending')}
                    </div>
                    {#if cs.cancelledTokens.size > 0 && cs.claims.length === 0}
                      <div class="cp-row cp-row--done">
                        <p class="cp-done">{$t('claimCancelDone')}</p>
                      </div>
                    {/if}
                    {#each cs.claims as c (c.token)}
                      <FigurineClaimRow
                        claim={c}
                        isLoggedIn={authStore.isLoggedIn}
                        isCancelling={cs.cancellingToken === c.token}
                        error={cs.claimErrors[c.token]}
                        formatDate={fmtDate}
                        onCancel={(claim) => cs.cancel(claim)}
                      />
                    {/each}
                  </div>
                </section>
              {/if}

              <section class="entry-record-section entry-record-section--lookup" aria-label={$t('claimHaveCode')}>
                <div class="claim-lookup">
                  {#if !cs.showTokenForm}
                    <button type="button" onclick={openClaimLookup} class="claim-lookup-link">{$t('claimHaveCode')}</button>
                  {:else}
                    <div class="claim-lookup-form">
                      <input type="text" bind:value={cs.tokenInput} placeholder="XXXX-XXXX" maxlength="9"
                        class="claim-lookup-input" oninput={() => { cs.tokenLookupInfo = null; cs.tokenLookupErr = ''; }} />
                      <button type="button" onclick={() => cs.lookupToken()} disabled={cs.tokenLooking} class="claim-lookup-btn">
                        {cs.tokenLooking ? '...' : $t('claimLookupBtn')}
                      </button>
                      <button type="button" onclick={closeClaimLookup} class="claim-lookup-close" aria-label={$t('lightboxClose')}>
                        <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                          <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke-linecap="round" />
                        </svg>
                      </button>
                    </div>
                    {#if cs.tokenLookupErr}<p class="claim-err">{cs.tokenLookupErr}</p>{/if}
                    {#if cs.tokenLookupInfo}
                      <div class="claim-lookup-result">
                        <p class="claim-dates">{fmtDate(cs.tokenLookupInfo.startsAt)} - {fmtDate(cs.tokenLookupInfo.endsAt)}</p>
                        {#if cs.tokenLookupInfo.status === 'pending'}
                          <button type="button" onclick={() => cs.cancelFromLookup()} disabled={cs.lookupCancelling} class="claim-cancel-btn">
                            {cs.lookupCancelling ? $t('claimCancelling') : $t('claimCancelBtn')}
                          </button>
                        {:else}
                          <p class="claim-status">{$t('claimStatus')}: {lookupStatusLabel(cs.tokenLookupInfo.status)}</p>
                        {/if}
                      </div>
                    {/if}
                  {/if}
                </div>
              </section>
            </div>
          </details>
        {/if}

        {#if hasWorkStorySection}
          <div class="act-divider" aria-hidden="true"></div>
        {/if}

        <!-- ACT II — THE WORK: story → making → motion -->
        {#if hasHistorySection}
          <div class="d-history">
            <header class="d-section-header">
              <span class="sec-label">{$t('figurineHistory')}</span>
              <div class="sec-rule" aria-hidden="true"></div>
              <FontSwitcher variant="colophon" />
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

        <!-- ── MAKING RECORD ── -->
        {#if hasMakingSection}
          <div class="grimoire-section {figurine.status === 'in_progress' ? 'grimoire-section--live' : ''}">
            <div class="making-record">
              <div class="making-copy">
                <span class="making-kicker">
                  {#if figurine.status === 'in_progress'}
                    <span class="making-live" aria-hidden="true"></span>{$t('detailMakingProgressKicker')}
                  {:else}
                    {$t('detailMakingRecordKicker')}
                  {/if}
                </span>
                <h2 class="making-title">
                  {figurine.status === 'in_progress' ? $t('detailMakingProgressTitle') : $t('detailMakingRecordTitle')}
                </h2>
                <p class="making-colophon">
                  {toRoman(visibleProcessSteps.length)} {$t('detailMakingStages')}<span class="mc-sep">·</span>{processStepLabel(firstStep?.stepType)} → {#if figurine.status === 'in_progress'}{$t('detailMakingLive')}{:else}{processStepLabel(lastStep?.stepType)}<span class="mc-sep">·</span>{$t('detailMakingByOneHand')}{/if}
                </p>
              </div>

              {#if hasBecoming}
                <BecomingReveal
                  beforeSrc={becomingBefore}
                  afterSrc={becomingAfter}
                  beforeLabel={processStepLabel(firstStep?.stepType)}
                  afterLabel={$t('detailBecomingFinished')}
                  hint={$t('detailBecomingHint')}
                />
              {:else}
              <div class="making-strip" aria-label={$t('detailMakingRecordTitle')}>
                {#each visibleProcessSteps.slice(0, 4) as step, i (step.id)}
                  <article class="making-card">
                    <div class="making-img-wrap">
                      {#if hasText(step.imageUrl)}
                        <img src={resolveUrl(step.imageUrl)} alt="" class="making-img" loading="lazy" />
                      {:else}
                        <div class="making-img-placeholder" aria-hidden="true"></div>
                      {/if}
                      <span class="making-count">{String(i + 1).padStart(2, '0')}</span>
                    </div>
                    <div class="making-card-copy">
                      <h3>{processStepLabel(step.stepType)}</h3>
                      {#if step.description}
                        <p>{step.description}</p>
                      {/if}
                    </div>
                  </article>
                {/each}
              </div>
              {/if}

            </div>

            <!-- Memory Mirror demoted to a quiet continuation of this same act —
                 with the reveal as teaser, it leads to every in-between stage. -->
            {#if showMirrorLink}
              <button type="button" onclick={openGrimoire} class="mirror-link" aria-expanded={isGrimoireOpen}>
                <span class="mirror-link-mark" aria-hidden="true"></span>
                <span class="mirror-link-label">{$t('figurineGrimoire')}</span>
                <span class="mirror-link-count">{visibleProcessSteps.length} {$t('figurineGrimoireSub')}</span>
                <svg
                  class="mirror-link-arrow"
                  class:mirror-link-arrow--open={isGrimoireOpen}
                  width="15"
                  height="15"
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.5"
                  aria-hidden="true"
                >
                  <path d="M3 8h10M9 4l4 4-4 4"/>
                </svg>
              </button>
            {/if}
          </div>
        {/if}

        <!-- ── VIDEO ── -->
        {#if hasVideoSection}
          <section class="video-section">
            <header class="section-row">
              <span class="sec-label">{$t('figurineVideo')}</span>
              <div class="sec-rule" aria-hidden="true"></div>
            </header>
            <div class="video-wrap">
              <div class="video-frame">
                <div class="video-stage">
                  <video bind:this={videoRef} controls class="video-el"
                    poster={resolveUrl(currentImage?.url)} preload="metadata">
                    <source src={resolveUrl(figurine.videoUrl)} type="video/mp4" />
                    {$t('figurineBrowserNoVideo')}
                  </video>
                  <button type="button" onclick={toggleFullscreen} class="video-fs-btn" title={$t('figurineFullscreen')} aria-label={$t('figurineFullscreen')}>
                    <svg width="14" height="14" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                      <path d="M1 4V1h3M6 1h3v3M9 6v3H6M4 9H1V6"/>
                    </svg>
                  </button>
                </div>
              </div>
            </div>
            <p class="video-caption text-label">{$t('figurineVideoFilm')}{id.slice(-3)}</p>
          </section>
        {/if}

    <!-- ── PRESENCE & SCHEDULE (Act III — logistics only; attributes already live near the title) ── -->
    {#if hasFactsSection}
      <div class="act-divider" aria-hidden="true"></div>

      {#if hasScheduleSection}
        <section id="presence" class="presence-section">
          <header class="section-row">
            <span class="sec-label">{$t('detailPresenceLabel')}</span>
            <div class="sec-rule" aria-hidden="true"></div>
          </header>
          <ShowingsTimeline schedule={figurineSchedule} />
          {#if hasActiveShowing}
            <p class="presence-note">{$t('figurineTransferBlocked')}</p>
          {/if}
        </section>
      {/if}
    {/if}

    <!-- ── RELATED NEXT CHOICES ── -->
    {#if visibleRelatedItems.length > 0}
      <div class="act-divider" aria-hidden="true"></div>
      <section id="related-works" class="related-section">
        <header class="related-head">
          <div>
            <span class="sec-label">{$t('figurineRelated')}</span>
            <h2 class="related-title">{$t('detailRelatedTitle')}</h2>
          </div>
          <p class="related-intro">{$t('detailRelatedText')}</p>
        </header>

        <div class="related-strip">
          {#each visibleRelatedItems as item}
            {@const relatedImageUrl = resolveUrl(item.faceImageUrl ?? item.thumbUrl)}
            <a
              href="/figurines/{item.id}"
              onclick={() => analyticsClient?.cta('related_figurine')}
              class="related-card"
              data-sveltekit-preload-data="hover"
            >
              <div class="related-img-wrap">
                {#if relatedImageUrl}
                  <img
                    src={relatedImageUrl}
                    alt={item.name}
                    class="related-img"
                    loading="lazy"
                  />
                {:else}
                  <div class="related-placeholder" aria-hidden="true">
                    <span>{item.name.slice(0, 1)}</span>
                  </div>
                {/if}
                <div class="related-overlay" aria-hidden="true">
                  <span class="related-cta-hint">
                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
                      <path d="M1 6h10M7 2l4 4-4 4"/>
                    </svg>
                  </span>
                </div>
                <span class="related-status-badge related-status-badge--{item.status}">
                  {statusLabel(item.status)}
                </span>
              </div>
              <div class="related-meta">
                <h4 class="related-name">{item.name}</h4>
                <p class="related-line">
                  {#if item.material}
                    {item.material}
                  {:else if item.technique}
                    {item.technique}
                  {:else if item.series}
                    {item.series}
                  {:else}
                    {$t('detailRelatedArchivePiece')}
                  {/if}
                </p>
                <div class="related-foot">
                  {#if item.year}
                    <span>Anno {item.year}</span>
                  {/if}
                  <span class="related-action">
                    {item.status === 'available' ? $t('detailRelatedRequestable') : $t('detailRelatedOpen')}
                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                      <path d="M1.5 6h9M7 2.5L10.5 6 7 9.5"/>
                    </svg>
                  </span>
                </div>
              </div>
            </a>
          {/each}
        </div>
      </section>
    {/if}

    <FigurineComments figurineId={id} />

      </div>
    </div>
  </div>
</div>

<!-- Mobile sticky CTA after the first screen — present for every status. -->
{#if scrollY > 300 && !showRequestModal}
  <div class="mobile-cta" transition:fade={{ duration: 180 }}>
    <div class="mobile-cta-info">
      <span class="mobile-cta-name">{figurine.name}</span>
      <span class="mobile-cta-status">
        {statusUi.mobileSubtitle}
      </span>
    </div>
    <button type="button" onclick={() => openRequestModal()} class="mobile-cta-btn">
      {statusUi.mobileCtaLabel}
      {#if statusUi.mobileIcon === 'lock'}
        <svg width="13" height="14" viewBox="0 0 14 15" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M4.5 5V3.5a2.5 2.5 0 0 1 5 0V5"/>
          <rect x="2" y="5" width="10" height="8.5" rx="1.2"/>
        </svg>
      {:else}
        <svg width="13" height="13" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M2 7h9M8 4l3 3-3 3"/>
        </svg>
      {/if}
    </button>
    {#if figurine.status === 'available'}
      <button type="button" onclick={() => openRequestModal('reserve')} class="mobile-cta-link">{$t('unifiedReserveShort')}</button>
    {:else}
      <a href={createSimilarHref} class="mobile-cta-link" onclick={() => analyticsClient?.cta('create_similar')}>{$t('commissionCreateSimilarShort')}</a>
    {/if}
  </div>
{/if}
