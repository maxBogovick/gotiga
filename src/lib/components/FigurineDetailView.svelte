<script lang="ts">
  import { onMount, onDestroy, tick, setContext } from 'svelte';
  import { fade } from 'svelte/transition';
  import type { Figurine, FigurineSchedule, FigurineStatus, DisplayConfig } from '$lib/types/api';
  import type { FigurineListItem } from '$lib/types/api';
  import UnifiedRequestModal from '$lib/components/UnifiedRequestModal.svelte';
  import CandleReveal from '$lib/components/CandleReveal.svelte';
  import MemoryMirror from '$lib/components/MemoryMirror.svelte';
  import Lightbox from '$lib/components/Lightbox.svelte';
  import TurnSoundSwitcher from '$lib/components/TurnSoundSwitcher.svelte';
  import SealedDoor from '$lib/components/SealedDoor.svelte';
  import SpecimenLayout from '$lib/components/figurine-detail/layouts/SpecimenLayout.svelte';
  import ShowcaseLayout from '$lib/components/figurine-detail/layouts/ShowcaseLayout.svelte';
  import CodexLayout from '$lib/components/figurine-detail/layouts/CodexLayout.svelte';
  import DiptychLayout from '$lib/components/figurine-detail/layouts/DiptychLayout.svelte';
  import BroadsideLayout from '$lib/components/figurine-detail/layouts/BroadsideLayout.svelte';
  import { goto, replaceState } from '$app/navigation';
  import { page } from '$app/stores';
  import { browser } from '$app/environment';
  import { api, resolveMediaUrl, resolveWebpUrl } from '$lib/api';
  import { createFigurineAnalytics } from '$lib/analytics';
  import { t } from '$lib/i18n';
  import { FigurineClaimsStore, type ClaimData } from '$lib/stores/figurine-claims.svelte';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
  import { pageTurn } from '$lib/stores/page-turn.svelte';
  import { turnSound } from '$lib/stores/page-turn-sound.svelte';
  import { playTurnSound } from '$lib/audio/page-turn-sounds';
  import { focusTrap } from '$lib/actions/focusTrap';
  import '$lib/styles/figurine-detail.css';
  import { houseClock } from '$lib/stores/house-clock.svelte';
  import { showingRooms } from '$lib/stores/showing-rooms.svelte';
  import { visitorMarks } from '$lib/stores/visitor-marks.svelte';
  import { isGated, isShowingOpen, resolveWindow } from '$lib/showing-window';

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

  let win = $derived(resolveWindow(figurine, showingRooms.list));
  let doorClosed = $derived(isGated(win) && !isShowingOpen(win, houseClock.nowDate));
  let sealedFace = $derived.by(() => {
    const images = figurine.images ?? [];
    return images.find(i => i.imageType === 'face') ?? images[0] ?? null;
  });
  let layout = $derived(figurine.displayLayout ?? 'specimen');

  function parseDisplayConfig(raw: string | null | undefined): DisplayConfig | null {
    if (!raw) return null;
    try { return JSON.parse(raw) as DisplayConfig; } catch { return null; }
  }
  let displayConfig = $derived(parseDisplayConfig(figurine.displayConfig));

  const BG_PRESETS: Record<string, string> = {
    parchment: '#f8f1e7',
    aged: '#ede3cf',
    linen: '#f4efe8',
    dark: '#181210',
    slate: '#dce0e4',
  };
  function getBgStyle(config: DisplayConfig | null): string {
    const bg = config?.background;
    if (!bg?.preset) return '';
    const color = bg.preset === 'custom' ? (bg.customColor ?? '') : (BG_PRESETS[bg.preset] ?? '');
    return color ? `--dc-bg:${color};` : '';
  }
  let pageRootBgStyle = $derived(getBgStyle(displayConfig));

  // ?photo=N (1-indexed) lets a shared/reloaded link reopen on the photo the
  // visitor was looking at, instead of always resetting to the first image.
  function readInitialPhotoIndex(): number {
    if (!browser) return 0;
    const raw = $page.url.searchParams.get('photo');
    if (!raw) return 0;
    const n = parseInt(raw, 10);
    return Number.isFinite(n) && n >= 1 ? n - 1 : 0;
  }
  let selectedImageIndex = $state(readInitialPhotoIndex());
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
  let analyticsClient: ReturnType<typeof createFigurineAnalytics> | null = null;
  let analyticsMountedAt = 0;
  let analyticsEngagedTimer: ReturnType<typeof setTimeout> | null = null;
  let analyticsScrollSent = false;

  let queueJoin = $state<{ token: string; position: number } | null>(null);
  let notifyJoin = $state<string | null>(null);

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
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '') || 'figurine';
    return `ritunia-${slug}-story.jpg`;
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
    const file = new File([storyBlob], 'ritunia-story.jpg', { type: 'image/jpeg' });
    try {
      await navigator.share({ files: [file], title: figurine.name });
      closeStoryModal();
    } catch { /* user cancelled */ }
  }

  function closeStoryModal() {
    showStoryModal = false;
    storyError = '';
    if (storyObjectUrl) { URL.revokeObjectURL(storyObjectUrl); storyObjectUrl = ''; }
    storyBlob = null;
  }

  $effect(() => {
    if (!showStoryModal || !storyModalRef) return;
    void tick().then(() => storyModalRef?.focus());
  });

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

  // Preload the neighbouring photos (preview + thumb, whichever format the
  // <picture> elements would actually pick) so paging with the new arrow
  // buttons / swipe feels instant instead of showing a fresh blur-up each
  // time. Fire-and-forget Image() objects — the browser cache does the rest.
  function preloadImage(path: string | null | undefined) {
    if (typeof Image === 'undefined' || !path) return;
    const url = resolveMediaUrl(path);
    if (!url) return;
    const img = new Image();
    img.src = resolveWebpUrl(url) ?? url;
  }

  $effect(() => {
    const idx = activeImageIndex;
    const images = sortedImages;
    const prevImg = images[idx - 1];
    const nextImg = images[idx + 1];
    preloadImage(prevImg?.url);
    preloadImage(prevImg?.thumbUrl);
    preloadImage(nextImg?.url);
    preloadImage(nextImg?.thumbUrl);
  });

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

  // Raking-light examination ("осмотр под косым светом"): a conservator's mode,
  // mutually exclusive with the lens and the resting daguerreotype — all three
  // contend for the same pointer over the same plate. Same desktop/motion/fit
  // and page-turn guards as the daguerreotype (a WebGL canvas can't survive a
  // view-transition snapshot, so it yields to the plain <img> during a turn).
  let isRakingEnabled = $state(false);
  let showRakingButton = $derived(isPointerFine && !prefersReducedMotion);
  function setImageViewMode(mode: 'fit' | 'detail') { imageViewMode = mode; }
  let useRaking = $derived(
    isPointerFine && !prefersReducedMotion && isRakingEnabled && imageViewMode === 'fit'
      && !pageTurn.direction
  );
  let useDaguerreotype = $derived(
    isPointerFine && !prefersReducedMotion && !isLensEnabled && !isRakingEnabled
      && imageViewMode === 'fit' && !pageTurn.direction
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
    sortedImages.map((img) => ({
      url: resolveUrl(img.originalUrl ?? img.url),
      alt: img.altText ?? '',
      thumbUrl: resolveUrl(img.thumbUrl ?? img.url) || undefined,
      focalX: img.focalX,
      focalY: img.focalY,
    }))
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
  let hasClaimRecords = $derived(cs.claims.length > 0 || cs.cancelledTokens.size > 0);
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
  // Keep the open photo shareable/reloadable: ?photo=2 is 1-indexed for
  // readability in a URL bar, omitted entirely for the first (default) photo
  // so plain figurine links stay clean. replaceState avoids spamming browser
  // history with one entry per swipe/arrow click.
  function syncPhotoParam(index: number) {
    if (typeof window === 'undefined') return;
    const url = new URL(window.location.href);
    if (index > 0) url.searchParams.set('photo', String(index + 1));
    else url.searchParams.delete('photo');
    replaceState(url, {});
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
      isRakingEnabled = false;
      syncPhotoParam(nextIndex);
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
    if (isLensEnabled) isRakingEnabled = false; // one examination tool at a time
  }
  function toggleRaking() {
    isRakingEnabled = !isRakingEnabled;
    if (isRakingEnabled) {
      isLensEnabled = false;     // mutually exclusive with the magnifier
      imageViewMode = 'fit';     // raking light reads the whole plate, not a crop
    }
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

  // "Mark of attention" — a wax seal overlaid on the image itself (see the
  // per-layout gallery-mark button). Clicking the seal opens a row of all 3
  // tone icons at once so the visitor picks the one they mean directly,
  // instead of blind-cycling through clicks. No count or tone is ever shown
  // publicly; the seal just confirms this visitor's own state on this piece.
  let markTone = $derived(visitorMarks.toneOf(figurine.id));
  let markPickerOpen = $state(false);
  let markToggling = $state(false);
  let markPressing = $state(false);
  let markPressTimer: ReturnType<typeof setTimeout> | null = null;
  // Brief in-context acknowledgment right when a mark is set — closes the
  // reward loop at the moment of the action itself, rather than only ever
  // paying off later on the home page's "Marked by you" shelf.
  let markThanksVisible = $state(false);
  let markThanksTimer: ReturnType<typeof setTimeout> | null = null;

  function markToneText(tone: import('$lib/types/api').MarkTone) {
    return tone === 'touched' ? $t('figurineMarkTouched')
      : tone === 'mesmerized' ? $t('figurineMarkMesmerized')
      : $t('figurineMarkDesired');
  }
  let markIconTone = $derived<import('$lib/types/api').MarkTone | 'bookmark'>(markTone ?? 'bookmark');
  let markLabel = $derived(
    markTone ? `${markToneText(markTone)} — ${$t('figurineMarkChangeHint')}` : $t('figurineMarkNone')
  );
  let markToneOptions = $derived(
    (['touched', 'mesmerized', 'desired'] as const).map((tone) => ({ tone, label: markToneText(tone) }))
  );

  function toggleMarkPicker() {
    markPickerOpen = !markPickerOpen;
  }

  async function setMarkTone(tone: import('$lib/types/api').MarkTone) {
    if (markToggling) return;
    markPickerOpen = false;
    const clearing = markTone === tone;
    if (!clearing) {
      markPressing = false;
      requestAnimationFrame(() => { markPressing = true; });
      if (markPressTimer) clearTimeout(markPressTimer);
      markPressTimer = setTimeout(() => { markPressing = false; }, 520);
    }
    markToggling = true;
    try {
      const resolved = await visitorMarks.set(figurine.id, clearing ? null : tone);
      if (!clearing && resolved === tone) {
        markThanksVisible = true;
        if (markThanksTimer) clearTimeout(markThanksTimer);
        markThanksTimer = setTimeout(() => { markThanksVisible = false; }, 2200);
      }
    } finally {
      markToggling = false;
    }
  }

  // Close the tone picker on Escape or on any click outside it. Centralized
  // here (rather than duplicated per layout) since only one layout renders
  // at a time for a given figurine and all of them read markPickerOpen from
  // this same context.
  $effect(() => {
    if (!markPickerOpen) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') markPickerOpen = false;
    }
    function onClickAway(e: MouseEvent) {
      const target = e.target as HTMLElement | null;
      if (!target?.closest('.gallery-mark, .gallery-mark-option')) markPickerOpen = false;
    }
    window.addEventListener('keydown', onKey);
    // Defer attaching so the click that opened the picker doesn't immediately close it.
    const timer = setTimeout(() => window.addEventListener('click', onClickAway), 0);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('click', onClickAway);
      clearTimeout(timer);
    };
  });

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
    if (!isPointerFine) return;
    pageTurn.arm(direction);
    const sound = turnSound.value;
    if (sound !== 'off') playTurnSound(sound, direction);
  }

  // Paper bleed-through: while a neighbour pill is hovered/focused, a faint ghost
  // of that work shows through the spine edge of the current leaf, as if the page
  // were thin enough to see the next one underneath. The image is already in cache
  // (the pills preload on hover), so this costs no extra request.
  let bleedDir = $state<'prev' | 'next' | null>(null);
  let bleedImage = $derived(
    bleedDir === 'prev'
      ? resolveUrl(prev?.thumbUrl ?? prev?.faceImageUrl)
      : bleedDir === 'next'
        ? resolveUrl(next?.thumbUrl ?? next?.faceImageUrl)
        : ''
  );
  // Keep the last shown side+image while fading out, so the ghost doesn't pop to
  // full-bleed when the pointer leaves and bleedDir clears.
  let lastBleed = $state<{ dir: 'prev' | 'next'; img: string } | null>(null);
  $effect(() => {
    if (bleedDir && bleedImage) lastBleed = { dir: bleedDir, img: bleedImage };
  });

  // ── Mobile swipe navigation (page-level: prev/next figurine) ────────────
  let swipeTouchStartX = 0;
  let swipeTouchStartY = 0;
  let swipeTouchTarget: EventTarget | null = null;

  function handlePageTouchStart(e: TouchEvent) {
    if (e.touches.length !== 1) return;
    swipeTouchStartX = e.touches[0].clientX;
    swipeTouchStartY = e.touches[0].clientY;
    swipeTouchTarget = e.target;
  }

  function handlePageTouchEnd(e: TouchEvent) {
    if (showLightbox || showRequestModal || showStoryModal || isGrimoireOpen) return;
    if (e.changedTouches.length !== 1) return;
    const dx = e.changedTouches[0].clientX - swipeTouchStartX;
    const dy = e.changedTouches[0].clientY - swipeTouchStartY;
    if (Math.abs(dx) < 72 || Math.abs(dx) < Math.abs(dy) * 1.6) return;
    if (swipeTouchTarget instanceof Element && swipeTouchTarget.closest('.gallery-col')) return;
    if (dx > 0 && prev) {
      void goto(`/figurines/${prev.id}`);
    } else if (dx < 0 && next) {
      void goto(`/figurines/${next.id}`);
    }
  }

  // ── Keyboard gallery navigation ───────────────────────────────────────────
  function handleKeydown(e: KeyboardEvent) {
    if (showStoryModal && e.key === 'Escape') {
      closeStoryModal();
      return;
    }
    if (showLightbox || showRequestModal || showStoryModal || isGrimoireOpen) return;
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
  let scrolled = $derived(scrollY > 80);

  // Gallery element is registered by the active layout component via setGalleryEl.
  let galleryEl = $state<HTMLElement | undefined>(undefined);
  let galleryObserver: IntersectionObserver | null = null;
  const TOPNAV_THRESHOLD = 130;

  let galleryExited = $state(false);

  function setGalleryEl(el: HTMLElement | undefined) { galleryEl = el; }

  $effect(() => {
    if (!galleryEl) { galleryObserver?.disconnect(); return; }
    galleryObserver?.disconnect();
    galleryObserver = new IntersectionObserver(
      ([entry]) => {
        galleryExited = !entry.isIntersecting && entry.boundingClientRect.bottom < TOPNAV_THRESHOLD;
      },
      { rootMargin: `-${TOPNAV_THRESHOLD}px 0px 0px 0px`, threshold: 0 }
    );
    galleryObserver.observe(galleryEl);
    return () => { galleryObserver?.disconnect(); galleryObserver = null; };
  });

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
      if (analyticsEngagedTimer) { clearTimeout(analyticsEngagedTimer); analyticsEngagedTimer = null; }
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
    houseClock.start();
    showingRooms.load();
    analyticsClient = createFigurineAnalytics(id);
    analyticsMountedAt = Date.now();
    analyticsClient.view();
    analyticsEngagedTimer = setTimeout(() => {
      if (analyticsScrollSent) return;
      analyticsScrollSent = true;
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
    visitorMarks.load();
    cs.verify();
    cs.startPolling();
    turnSound.load();
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
    if (markPressTimer) clearTimeout(markPressTimer);
    if (markThanksTimer) clearTimeout(markThanksTimer);
    galleryObserver?.disconnect();
    clearTodayRefresh();
    clearAudioFade();
    if (storyObjectUrl) URL.revokeObjectURL(storyObjectUrl);
    if (audioRef) { audioRef.pause(); audioRef = null; }
    cs.dispose();
  });

  setContext<App.FigurineDetailContext>('figurine-detail', {
    get figurine() { return figurine; },
    get id() { return id; },
    get prev() { return prev; },
    get next() { return next; },
    get sortedImages() { return sortedImages; },
    get attributes() { return attributes; },
    get visibleProcessSteps() { return visibleProcessSteps; },
    get visibleRelatedItems() { return visibleRelatedItems; },
    get figurineSchedule() { return figurineSchedule; },
    get activeImageIndex() { return activeImageIndex; },
    get currentImage() { return currentImage; },
    get currentImageFit() { return currentImageFit; },
    get imageViewMode() { return imageViewMode; },
    get isLensEnabled() { return isLensEnabled; },
    get isRakingEnabled() { return isRakingEnabled; },
    get useDaguerreotype() { return useDaguerreotype; },
    get useRaking() { return useRaking; },
    get showRakingButton() { return showRakingButton; },
    get isSaved() { return isSaved; },
    get markTone() { return markTone; },
    get markIconTone() { return markIconTone; },
    get markLabel() { return markLabel; },
    get markPressing() { return markPressing; },
    get markPickerOpen() { return markPickerOpen; },
    get markToneOptions() { return markToneOptions; },
    get markThanksVisible() { return markThanksVisible; },
    get noticedByOthers() { return Boolean(figurine.noticedByOthers); },
    get houseFavorite() { return Boolean(figurine.houseFavorite); },
    get canOpenLightbox() { return canOpenLightbox; },
    get bleedDir() { return bleedDir; },
    get lastBleed() { return lastBleed; },
    get plateStyle() { return plateStyle; },
    get viewTransitionName() { return viewTransitionName; },
    get isCandleLit() { return isCandleLit; },
    get hasHistorySection() { return hasHistorySection; },
    get hasMakingSection() { return hasMakingSection; },
    get hasVideoSection() { return hasVideoSection; },
    get hasWorkStorySection() { return hasWorkStorySection; },
    get hasAttributesSection() { return hasAttributesSection; },
    get hasScheduleSection() { return hasScheduleSection; },
    get hasFactsSection() { return hasFactsSection; },
    get hasPersonalRecord() { return hasPersonalRecord; },
    get hasClaimRecords() { return hasClaimRecords; },
    get hasClaimLookupState() { return hasClaimLookupState; },
    get canShowPersonalRecord() { return canShowPersonalRecord; },
    get hasBecoming() { return hasBecoming; },
    get showMirrorLink() { return showMirrorLink; },
    get becomingBefore() { return becomingBefore; },
    get becomingAfter() { return becomingAfter; },
    get firstStep() { return firstStep; },
    get lastStep() { return lastStep; },
    get queueJoin() { return queueJoin; },
    get notifyJoin() { return notifyJoin; },
    get hasActiveShowing() { return hasActiveShowing; },
    get nextAvailableDate() { return nextAvailableDate; },
    get scheduleLoadFailed() { return scheduleLoadFailed; },
    get displayConfig() { return displayConfig; },
    get isGrimoireOpen() { return isGrimoireOpen; },
    get cs() { return cs; },
    get analyticsClient() { return analyticsClient; },
    get statusUi() { return statusUi; },
    selectImage,
    openLightbox,
    toggleSaved,
    toggleMarkPicker,
    setMarkTone,
    toggleLens,
    toggleRaking,
    setImageViewMode,
    openRequestModal,
    openGrimoire,
    closeGrimoire,
    handlePersonalRecordToggle,
    openClaimLookup,
    closeClaimLookup,
    armPageTurn,
    setGalleryEl,
    resolveUrl,
    imageTypeLabel,
    imageRoleNote,
    processStepLabel,
    fmtDate,
    lookupStatusLabel,
    toRoman,
    hasText,
    statusLabel,
  });
</script>

{#if figurine.ambiencePath}
  <audio bind:this={audioRef} src={resolveUrl(figurine.ambiencePath)} loop></audio>
{/if}

<CandleReveal isActive={isCandleLit} />

<div class="page-root" class:page-root--has-cta={scrollY > 300} class:page-root--candle={isCandleLit}
  style={pageRootBgStyle}
  ontouchstart={handlePageTouchStart}
  ontouchend={handlePageTouchEnd}
>
  <UnifiedRequestModal
    isOpen={showRequestModal}
    figurineName={figurine.name}
    figurineId={figurine.id}
    status={figurine.status}
    schedule={figurineSchedule}
    initialIntent={requestInitialIntent}
    onJoined={(tok: string, pos: number) => { queueJoin = { token: tok, position: pos }; }}
    onNotified={(tok: string) => { notifyJoin = tok; }}
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
                onpointerenter={() => (bleedDir = 'prev')}
                onpointerleave={() => (bleedDir = null)}
                onfocus={() => (bleedDir = 'prev')}
                onblur={() => (bleedDir = null)}
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
                onpointerenter={() => (bleedDir = 'next')}
                onpointerleave={() => (bleedDir = null)}
                onfocus={() => (bleedDir = 'next')}
                onblur={() => (bleedDir = null)}
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

      <!-- Right: controls — turn-sound, candle (mood + reveal), whisper (if audio), share -->
      <div class="topnav-controls">
        <TurnSoundSwitcher />

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
    {#if doorClosed}
      <div class="sealed-body">
        <h1 class="sealed-title">{figurine.name}</h1>
        <div class="sealed-door-wrap">
          <SealedDoor
            openFromMin={win.openFromMin}
            openUntilMin={win.openUntilMin}
            daysMask={win.daysMask}
            monthDay={win.monthDay}
            dateFrom={win.dateFrom}
            dateUntil={win.dateUntil}
            imageUrl={sealedFace?.url}
            thumbUrl={sealedFace?.thumbUrl}
            name={figurine.name}
          />
        </div>
      </div>
    {:else}

    {#if storyError}
      <p class="detail-inline-alert" role="alert">
        {storyError}
        <button type="button" class="detail-inline-alert-dismiss" onclick={() => (storyError = '')} aria-label={$t('lightboxClose')}>×</button>
      </p>
    {/if}

    {#if layout === 'specimen'}
      <SpecimenLayout />
    {:else if layout === 'showcase'}
      <ShowcaseLayout />
    {:else if layout === 'codex'}
      <CodexLayout />
    {:else if layout === 'diptych'}
      <DiptychLayout />
    {:else if layout === 'broadside'}
      <BroadsideLayout />
    {/if}

    {/if}
  </div>
</div>

<!-- Mobile swipe edge indicators — only on touch devices, only when prev/next exist -->
{#if prev && !showLightbox && !showRequestModal && !showStoryModal && !isGrimoireOpen}
  <a
    href="/figurines/{prev.id}"
    class="swipe-edge swipe-edge--prev"
    aria-label="{$t('figurineNavPrev')}: {prev.name}"
    data-sveltekit-preload-data="hover"
  >
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
      <path d="M6.5 2L3.5 5 6.5 8" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
    <span class="swipe-edge-name">{prev.name}</span>
  </a>
{/if}
{#if next && !showLightbox && !showRequestModal && !showStoryModal && !isGrimoireOpen}
  <a
    href="/figurines/{next.id}"
    class="swipe-edge swipe-edge--next"
    aria-label="{$t('figurineNavNext')}: {next.name}"
    data-sveltekit-preload-data="hover"
  >
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
      <path d="M3.5 2l3 3L3.5 8" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
    <span class="swipe-edge-name">{next.name}</span>
  </a>
{/if}

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
    <a href={createSimilarHref} class="mobile-cta-link" onclick={() => analyticsClient?.cta('create_similar')}>{$t('commissionCreateSimilarShort')}</a>
  </div>
{/if}
