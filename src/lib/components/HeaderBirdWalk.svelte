<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { themeConfig, DEFAULT_EFFECTS } from '$lib/stores/theme.svelte';
  import { birdWalking, ravenCircleEl } from '$lib/stores/bird-walk';

  // Same size as the yellow circle in RavenWatcher
  const W = 36, H = 36;
  const WALK_SPEED = 85; // px per second

  let canvasEl: HTMLCanvasElement | null = null;
  let videoEl: HTMLVideoElement | null = null;

  let visible = $state(false);
  let posX = $state(0);
  let posY = $state(0);
  let flipped = $state(false);

  let rafId = 0;
  let timerId: ReturnType<typeof setTimeout>;

  // The 2D context is created once and reused. getContext() was being called on every
  // animation frame; it returns the same context object every time, and the options bag
  // is only honoured on the FIRST call — so the per-frame calls were pure overhead that
  // also made the code read as if willReadFrequently were being (re)applied each frame.
  let ctx: CanvasRenderingContext2D | null = null;

  // The bird is pure decoration. Visitors who ask the OS for reduced motion get no walk
  // at all — no scheduler, no video decode, no canvas work. This component lives in
  // SiteHeader, i.e. on every public page, so the gate is worth more here than anywhere.
  let motionOk = true;

  /** Frame-accurate chroma-key driver.
   *
   * The keying below has to run once per *video* frame, not once per *display* frame.
   * Driving it from rAF meant a ~60 Hz loop keying a video that only presents ~25–30 new
   * frames a second: roughly half of every getImageData/putImageData pair re-processed
   * pixels that had not changed. That matters more than the pixel count suggests, because
   * getImageData forces a synchronous GPU→CPU readback and stalls the pipeline.
   *
   * requestVideoFrameCallback is the platform API for exactly this — it fires when a new
   * frame is actually presented. Safari/Chrome/Edge have it; where it is missing we fall
   * back to keying inside the rAF loop, i.e. today's behaviour, so nothing regresses.
   */
  type RVFCVideo = HTMLVideoElement & {
    requestVideoFrameCallback?: (cb: () => void) => number;
    cancelVideoFrameCallback?: (handle: number) => void;
  };
  let rvfcHandle = 0;
  let hasRVFC = false;

  function intervalMs(): number {
    const v = get(themeConfig).effects?.birdWalkInterval;
    const secs = typeof v === 'number' && v > 0 ? v : DEFAULT_EFFECTS.birdWalkInterval;
    return secs * 1000;
  }

  function drawFrame() {
    if (!canvasEl || !videoEl || videoEl.readyState < 2) return;
    ctx ??= canvasEl.getContext('2d', { willReadFrequently: true });
    if (!ctx) return;
    ctx.clearRect(0, 0, W, H);
    ctx.drawImage(videoEl, 0, 0, W, H);
    // Chroma-key: the source clip is a bird on black, so luma near zero becomes
    // transparent and the narrow band above it feathers the edge.
    const id = ctx.getImageData(0, 0, W, H);
    const d = id.data;
    for (let i = 0; i < d.length; i += 4) {
      const luma = 0.299 * d[i] + 0.587 * d[i + 1] + 0.114 * d[i + 2];
      if (luma <= 4) { d[i + 3] = 0; }
      else if (luma < 14) { d[i + 3] = Math.round((luma - 4) / 10 * 255); }
    }
    ctx.putImageData(id, 0, 0);
  }

  /** Key each presented video frame, for as long as the bird is on screen. */
  function pumpVideoFrames() {
    const v = videoEl as RVFCVideo | null;
    if (!v?.requestVideoFrameCallback || !visible) return;
    rvfcHandle = v.requestVideoFrameCallback(() => {
      drawFrame();
      pumpVideoFrames();
    });
  }

  function stopVideoFrames() {
    const v = videoEl as RVFCVideo | null;
    if (rvfcHandle && v?.cancelVideoFrameCallback) v.cancelVideoFrameCallback(rvfcHandle);
    rvfcHandle = 0;
  }

  function animateLeg(from: number, to: number): Promise<void> {
    return new Promise(resolve => {
      const duration = (Math.abs(to - from) / WALK_SPEED) * 1000;
      let t0 = 0;
      function step(now: number) {
        if (!t0) t0 = now;
        const t = Math.min(1, (now - t0) / duration);
        posX = from + (to - from) * t;
        // Only key pixels here when the browser lacks requestVideoFrameCallback.
        // Where it exists, pumpVideoFrames() owns the keying and this loop is left to do
        // just what a display-rate loop should: move the thing.
        if (!hasRVFC) drawFrame();
        if (t < 1) { rafId = requestAnimationFrame(step); }
        else { resolve(); }
      }
      rafId = requestAnimationFrame(step);
    });
  }

  function sleep(ms: number): Promise<void> {
    return new Promise(r => setTimeout(r, ms));
  }

  async function walk() {
    const circleEl = get(ravenCircleEl);
    if (!circleEl || !videoEl || !canvasEl) return;

    const rect = circleEl.getBoundingClientRect();

    // Center the travel canvas on the circle center
    const startX = rect.left + rect.width / 2 - W / 2;
    const startY = rect.top + rect.height / 2 - H / 2;
    const endX = window.innerWidth - W - 16;

    posX = startX;
    posY = startY;
    flipped = false;
    visible = true;
    birdWalking.set(true);

    videoEl.currentTime = 0;
    await videoEl.play().catch(() => {});
    pumpVideoFrames(); // no-op without requestVideoFrameCallback; rAF keys instead

    // Leg 1: right
    await animateLeg(startX, endX);

    // Flip at edge
    flipped = true;
    await sleep(150);

    // Leg 2: return to circle
    await animateLeg(endX, startX);

    // Arrive — hide video, reveal circle
    visible = false;
    flipped = false;
    birdWalking.set(false);
    stopVideoFrames();
    videoEl.pause();
  }

  // The travelling bird canvas is hidden at ≤720px (see .bird-travel media query),
  // so a walk there would only make the raven circle fade to `is-away` with nothing
  // walking in its place — the emblem would just blink out. Skip the walk at that
  // width so the mobile circle stays put. Checked per-tick so a resize is picked up.
  function isNarrow() {
    return typeof window !== 'undefined' && window.matchMedia('(max-width: 720px)').matches;
  }

  function schedule() {
    timerId = setTimeout(async () => {
      // A walk is driven by requestAnimationFrame, which browsers do not fire on a hidden
      // tab. Starting one here would leave it suspended mid-stride — the video decoding,
      // `birdWalking` stuck true, and the whole chain resuming at a random point when the
      // visitor came back. Skip the turn instead and let the next tick pick it up.
      if (!document.hidden && !isNarrow()) await walk();
      schedule();
    }, intervalMs());
  }

  onMount(() => {
    motionOk = !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (!motionOk) return; // never schedule: no timer, no decode, no canvas
    hasRVFC = typeof (videoEl as RVFCVideo | null)?.requestVideoFrameCallback === 'function';
    schedule();
  });
  onDestroy(() => {
    if (typeof window !== 'undefined') {
      clearTimeout(timerId);
      cancelAnimationFrame(rafId);
      stopVideoFrames();
    }
  });
</script>

<video
  bind:this={videoEl}
  src="/images/bird-see/steps_3.mp4"
  muted
  loop
  playsinline
  preload="auto"
  aria-hidden="true"
  style="display:none; position:absolute"
></video>

<!-- Always in DOM so canvasEl is bound immediately on mount -->
<canvas
  bind:this={canvasEl}
  width={W}
  height={H}
  class="bird-travel"
  class:is-visible={visible}
  class:is-flipped={flipped}
  style="left:{posX}px; top:{posY}px"
  aria-hidden="true"
></canvas>

<style>
  .bird-travel {
    position: fixed;
    width: 36px;
    height: 36px;
    pointer-events: none;
    z-index: 205;
    opacity: 0;
    /* no background, no border-radius — just the bird on transparent */
  }

  .bird-travel.is-visible {
    opacity: 1;
  }

  .bird-travel.is-flipped {
    transform: scaleX(-1);
  }

  @media (max-width: 720px) {
    .bird-travel { display: none; }
  }
</style>
