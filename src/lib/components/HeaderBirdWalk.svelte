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

  function intervalMs(): number {
    const v = get(themeConfig).effects?.birdWalkInterval;
    const secs = typeof v === 'number' && v > 0 ? v : DEFAULT_EFFECTS.birdWalkInterval;
    return secs * 1000;
  }

  function drawFrame() {
    if (!canvasEl || !videoEl || videoEl.readyState < 2) return;
    const ctx = canvasEl.getContext('2d', { willReadFrequently: true });
    if (!ctx) return;
    ctx.clearRect(0, 0, W, H);
    ctx.drawImage(videoEl, 0, 0, W, H);
    const id = ctx.getImageData(0, 0, W, H);
    const d = id.data;
    for (let i = 0; i < d.length; i += 4) {
      const luma = 0.299 * d[i] + 0.587 * d[i + 1] + 0.114 * d[i + 2];
      if (luma <= 4) { d[i + 3] = 0; }
      else if (luma < 14) { d[i + 3] = Math.round((luma - 4) / 10 * 255); }
    }
    ctx.putImageData(id, 0, 0);
  }

  function animateLeg(from: number, to: number): Promise<void> {
    return new Promise(resolve => {
      const duration = (Math.abs(to - from) / WALK_SPEED) * 1000;
      let t0 = 0;
      function step(now: number) {
        if (!t0) t0 = now;
        const t = Math.min(1, (now - t0) / duration);
        posX = from + (to - from) * t;
        drawFrame();
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
    videoEl.pause();
  }

  function schedule() {
    timerId = setTimeout(async () => {
      await walk();
      schedule();
    }, intervalMs());
  }

  onMount(schedule);
  onDestroy(() => {
    clearTimeout(timerId);
    cancelAnimationFrame(rafId);
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
