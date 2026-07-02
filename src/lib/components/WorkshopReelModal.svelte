<script lang="ts">
  /**
   * A cinematic reveal for the workshop reel — the clip visually grows out of
   * whichever hero locket was clicked (captured as `origin`, a viewport rect)
   * rather than simply fading in, echoing the site's card→detail shared-element
   * morph elsewhere. Chrome borrows the "important dialog" frame used by
   * Booking/Order modals (double border, slight rotate that settles on hover)
   * — but deliberately withholds the wax seal, which is reserved for confirmed
   * actions, not a video reveal.
   */
  import { onMount } from 'svelte';
  import { focusTrap } from '$lib/actions/focusTrap';

  type Rect = { x: number; y: number; width: number; height: number };

  let {
    webm,
    mp4,
    poster,
    caption,
    closeLabel,
    origin = null,
    onClose,
  }: {
    webm: string;
    mp4: string;
    poster: string;
    caption: string;
    closeLabel: string;
    origin?: Rect | null;
    onClose: () => void;
  } = $props();

  let panelEl = $state<HTMLElement>();
  let videoEl = $state<HTMLVideoElement>();
  let shown = $state(false);

  function handleKey(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }

  onMount(() => {
    const prevOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';

    if (panelEl && origin) {
      const panelRect = panelEl.getBoundingClientRect();
      const scale = Math.min(
        origin.width / panelRect.width,
        origin.height / panelRect.height,
      );
      const dx = (origin.x + origin.width / 2) - (panelRect.left + panelRect.width / 2);
      const dy = (origin.y + origin.height / 2) - (panelRect.top + panelRect.height / 2);
      panelEl.style.setProperty('--ox', `${dx}px`);
      panelEl.style.setProperty('--oy', `${dy}px`);
      panelEl.style.setProperty('--os', `${scale}`);
    }

    requestAnimationFrame(() => { shown = true; });
    videoEl?.play().catch(() => {});

    return () => {
      document.body.style.overflow = prevOverflow;
    };
  });
</script>

<svelte:window onkeydown={handleKey} />

<div
  class="reel-backdrop"
  onclick={handleBackdropClick}
  role="presentation"
>
  <div
    class="reel-panel"
    class:is-shown={shown}
    bind:this={panelEl}
    role="dialog"
    aria-modal="true"
    aria-label={caption}
    tabindex="-1"
    use:focusTrap
  >
    <div class="reel-panel-inner">
      <div class="reel-frame">
        <video
          bind:this={videoEl}
          class="reel-video"
          {poster}
          muted
          loop
          playsinline
          preload="auto"
        >
          <source src={webm} type="video/webm" />
          <source src={mp4} type="video/mp4" />
        </video>
        <div class="reel-vignette" aria-hidden="true"></div>
      </div>

      <p class="reel-caption">{caption}</p>
    </div>

    <button type="button" class="reel-close" onclick={onClose} aria-label={closeLabel}>
      <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.4">
        <path d="M1.5 1.5l10 10M11.5 1.5l-10 10"/>
      </svg>
    </button>
  </div>
</div>

<style>
  .reel-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(111, 59, 36, 0.4);
    backdrop-filter: blur(6px);
  }

  .reel-panel {
    position: relative;
    width: min(480px, 84vw);
    background: #fff9f0;
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    padding: 4px;
    box-shadow: 0 30px 80px rgba(52, 37, 28, 0.28);
    transform: translate(var(--ox, 0), var(--oy, 0)) scale(var(--os, 0.5)) rotate(-1deg);
    opacity: 0;
    transition:
      transform 0.55s cubic-bezier(0.16, 1, 0.3, 1),
      opacity 0.4s ease,
      box-shadow 0.4s ease;
  }

  .reel-panel:hover {
    transform: translate(0, 0) scale(1) rotate(0deg);
  }

  .reel-panel.is-shown {
    transform: translate(0, 0) scale(1) rotate(-1deg);
    opacity: 1;
  }

  .reel-panel-inner {
    border: 3px double rgba(201, 168, 117, 0.4);
    padding: 14px 14px 16px;
  }

  .reel-frame {
    position: relative;
    width: 100%;
    aspect-ratio: 4 / 5;
    overflow: hidden;
    background: #2a1c12;
  }

  .reel-video {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .reel-vignette {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background:
      radial-gradient(ellipse at 50% 50%, transparent 55%, rgba(40, 20, 8, 0.28) 100%);
  }

  .reel-caption {
    margin: 12px 2px 2px;
    font-family: 'Cormorant Garamond', 'Fraunces', Georgia, serif;
    font-style: italic;
    font-size: 15px;
    text-align: center;
    color: #6f3b24;
  }

  .reel-close {
    position: absolute;
    top: -14px;
    right: -14px;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid #d8c6b1;
    border-radius: 50%;
    background: #fff9f0;
    color: #6f3b24;
    cursor: pointer;
    box-shadow: 0 4px 14px rgba(52, 37, 28, 0.22);
    transition: color 0.18s ease, border-color 0.18s ease, transform 0.18s ease;
  }

  .reel-close:hover {
    color: #c65f3c;
    border-color: rgba(198, 95, 60, 0.5);
    transform: scale(1.06);
  }

  @media (prefers-reduced-motion: reduce) {
    .reel-panel {
      transition: opacity 0.3s ease;
      transform: none;
    }
    .reel-panel.is-shown {
      transform: none;
    }
  }
</style>
