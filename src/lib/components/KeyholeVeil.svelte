<script lang="ts">
  import { fade } from 'svelte/transition';
  /**
   * KeyholeVeil — the "sealed specimen" overlay.
   *
   * Lays a soft radial darkness over a framed image, leaving only a candle-lit
   * fragment visible around a focal point. The rest of the work stays in shadow
   * until the visitor chooses to step into the card (see HomeFigurineTile, which
   * lifts the veil once a piece has been opened — `gotiga_viewed`).
   *
   * Frame-relative by construction: focal point and radius are normalised 0..1
   * against the *rendered frame*, so the same numbers produce the same fragment
   * on any card size — as long as the frame keeps the same aspect/fit. The admin
   * picker reuses this very component (editable=true) over an identical 4/3
   * `contain` frame, so what the editor places is exactly what visitors see.
   *
   * Pure overlay: it never touches the <img> beneath it, so image loading, the
   * card→detail view-transition and reduced-motion all keep working untouched.
   * pointer-events stay off unless editing, so the card link underneath is live.
   */
  let {
    focalX = null,
    focalY = null,
    revealRadius = null,
    darkness = null,
    show = true,
    dwelling = false,
    partial = false,
    dwellMs = 0,
    editable = false,
    onpick = undefined,
  }: {
    focalX?: number | null;
    focalY?: number | null;
    revealRadius?: number | null;
    /**
     * Whether the shadow is present. When it flips to false the veil dissipates
     * with a soft fade (the work being revealed); flips back on without one.
     */
    show?: boolean;
    /**
     * Per-image darkness override (0..1). When null the veil inherits the global
     * `--kh-darkness` (theme setting), which itself falls back to a built-in
     * default — so depth can be tuned globally and overridden per work.
     */
    darkness?: number | null;
    /** A sustained look is in progress — the shadow eases toward "half-lit" over `dwellMs`. */
    dwelling?: boolean;
    /** A glance was completed (looked but not opened) — hold the shadow half-lit, not gone. */
    partial?: boolean;
    /** Dwell duration in ms; sets how slowly the shadow thins while dwelling. */
    dwellMs?: number;
    editable?: boolean;
    /** Called with normalised (x, y) when editing the focal point. */
    onpick?: (x: number, y: number) => void;
  } = $props();

  const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
  const clamp01 = (v: number) => clamp(v, 0, 1);

  let fx = $derived(clamp01(focalX ?? 0.5));
  let fy = $derived(clamp01(focalY ?? 0.5));
  // 0.30 of the frame is a fragment — enough to hook, not enough to give the work away.
  let r = $derived(clamp(revealRadius ?? 0.3, 0.08, 1));
  // Per-image darkness override, if any (else inherit the global --kh-darkness).
  let dark = $derived(darkness == null ? null : clamp01(darkness));

  let dragging = $state(false);

  function locate(e: PointerEvent) {
    const el = e.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();
    const x = clamp01((e.clientX - rect.left) / rect.width);
    const y = clamp01((e.clientY - rect.top) / rect.height);
    onpick?.(x, y);
  }

  function onDown(e: PointerEvent) {
    if (!editable) return;
    dragging = true;
    (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
    locate(e);
  }
  function onMove(e: PointerEvent) {
    if (!editable || !dragging) return;
    locate(e);
  }
  function onUp(e: PointerEvent) {
    dragging = false;
    (e.currentTarget as HTMLElement).releasePointerCapture?.(e.pointerId);
  }
</script>

{#if show}
  <div
    class="keyhole-veil"
    class:editable
    class:dwelling
    class:partial
    style="--kh-fx:{fx}; --kh-fy:{fy}; --kh-base:{r};{dark != null ? ` --kh-darkness:${dark};` : ''}{dwelling && dwellMs > 0 ? ` transition-duration:${dwellMs}ms;` : ''}"
    out:fade={{ duration: 750 }}
    onpointerdown={onDown}
    onpointermove={onMove}
    onpointerup={onUp}
    onpointercancel={onUp}
    role="presentation"
    aria-hidden="true"
  >
    {#if editable}
      <span class="kh-marker" style="left:{fx * 100}%; top:{fy * 100}%;"></span>
    {/if}
  </div>
{/if}

<style>
  .keyhole-veil {
    position: absolute;
    inset: 0;
    z-index: 1;
    pointer-events: none;
    border-radius: inherit;
    /* Effective radius: the editor's base, widened as a look is rewarded, plus a
       faint breathing pulse. */
    --kh-r: calc(max(var(--kh-base), var(--kh-spread)) + var(--kh-breathe));
    /* Shadow depth: per-image override (inline) → global theme → built-in 0.88. */
    --kh-dark: var(--kh-darkness, 0.88);
    background: radial-gradient(
      circle at calc(var(--kh-fx) * 100%) calc(var(--kh-fy) * 100%),
      transparent 0,
      transparent calc(var(--kh-r) * 72%),
      rgba(18, 11, 7, calc(var(--kh-dark) * 0.6)) calc(var(--kh-r) * 100% + 8%),
      rgba(14, 8, 5, var(--kh-dark)) calc(var(--kh-r) * 100% + 30%)
    );
    /* Default (settle) easing; while dwelling the duration is overridden inline
       to the configured dwell time so the shadow thins over exactly that long. */
    transition-property: --kh-spread, --kh-dark;
    transition-timing-function: ease;
    transition-duration: 0.8s;
  }

  /* A sustained look (in progress) or a completed glance: the shadow thins to a
     half-lit state and the keyhole widens — revealing more, but never all. Only
     opening the work clears it fully. */
  .keyhole-veil.dwelling,
  .keyhole-veil.partial {
    --kh-spread: 0.42;
    --kh-dark: calc(var(--kh-darkness, 0.88) * 0.42);
  }

  /* The breathing pulse animates gradient stops, which forces a paint every
     frame — so it runs ONLY on the card under an active look (at most one at a
     time), never across the whole gallery at rest. */
  .keyhole-veil.dwelling {
    animation: kh-breathe 7s ease-in-out infinite;
  }

  @keyframes kh-breathe {
    0%,
    100% {
      --kh-breathe: 0;
    }
    50% {
      --kh-breathe: 0.025;
    }
  }

  .keyhole-veil.editable {
    pointer-events: auto;
    cursor: crosshair;
    animation: none;
  }

  /* Focal marker — a quiet brass ring, shown only while editing. */
  .kh-marker {
    position: absolute;
    width: 22px;
    height: 22px;
    transform: translate(-50%, -50%);
    border: 1.5px solid rgba(255, 226, 170, 0.92);
    border-radius: 50%;
    box-shadow:
      0 0 0 1px rgba(20, 12, 7, 0.55),
      0 0 10px rgba(255, 200, 120, 0.5);
    pointer-events: none;
  }
  .kh-marker::after {
    content: '';
    position: absolute;
    inset: 9px;
    background: rgba(255, 226, 170, 0.92);
    border-radius: 50%;
  }

  @media (prefers-reduced-motion: reduce) {
    .keyhole-veil {
      animation: none;
      transition: none;
    }
  }
</style>
