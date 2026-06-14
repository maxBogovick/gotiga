<script lang="ts">
  // "The Becoming" — a hand-driven reveal between the raw first stage and the
  // finished work. The headline "how this figure took shape" made operable.
  let {
    beforeSrc,
    afterSrc,
    beforeLabel = '',
    afterLabel = '',
    hint = '',
  }: {
    beforeSrc: string;
    afterSrc: string;
    beforeLabel?: string;
    afterLabel?: string;
    hint?: string;
  } = $props();

  let pos = $state(50);
  let frame = $state<HTMLDivElement | null>(null);
  let dragging = $state(false);

  function setFromClientX(clientX: number) {
    if (!frame) return;
    const rect = frame.getBoundingClientRect();
    pos = Math.min(100, Math.max(0, ((clientX - rect.left) / rect.width) * 100));
  }

  function onPointerDown(e: PointerEvent) {
    dragging = true;
    frame?.setPointerCapture?.(e.pointerId);
    setFromClientX(e.clientX);
  }

  function onPointerMove(e: PointerEvent) {
    if (dragging) setFromClientX(e.clientX);
  }

  function onPointerUp(e: PointerEvent) {
    dragging = false;
    frame?.releasePointerCapture?.(e.pointerId);
  }

  function onKey(e: KeyboardEvent) {
    const step = e.shiftKey ? 10 : 4;
    if (e.key === 'ArrowLeft') { pos = Math.max(0, pos - step); e.preventDefault(); }
    else if (e.key === 'ArrowRight') { pos = Math.min(100, pos + step); e.preventDefault(); }
    else if (e.key === 'Home') { pos = 0; e.preventDefault(); }
    else if (e.key === 'End') { pos = 100; e.preventDefault(); }
  }
</script>

<div
  class="becoming"
  class:becoming--dragging={dragging}
  bind:this={frame}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
  role="presentation"
>
  <img class="becoming-img becoming-img--after" src={afterSrc} alt="" draggable="false" />
  <img
    class="becoming-img becoming-img--before"
    src={beforeSrc}
    alt=""
    draggable="false"
    style="clip-path: inset(0 {100 - pos}% 0 0);"
  />

  {#if beforeLabel}
    <span class="becoming-tag becoming-tag--before" style="opacity:{pos > 12 ? 1 : 0}">{beforeLabel}</span>
  {/if}
  {#if afterLabel}
    <span class="becoming-tag becoming-tag--after" style="opacity:{pos < 88 ? 1 : 0}">{afterLabel}</span>
  {/if}

  <div class="becoming-divider" style="left:{pos}%">
    <button
      type="button"
      class="becoming-handle"
      role="slider"
      tabindex="0"
      aria-label={hint}
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={Math.round(pos)}
      aria-orientation="horizontal"
      onkeydown={onKey}
    >
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
        <path d="M8 6 4 10l4 4M12 6l4 4-4 4" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>
  </div>
</div>

<style>
  .becoming {
    position: relative;
    aspect-ratio: 1 / 1;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 10%, transparent);
    border-radius: 3px;
    background: var(--color-canvas-sunken);
    cursor: ew-resize;
    touch-action: none;
    user-select: none;
  }

  .becoming-img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    -webkit-user-drag: none;
  }

  .becoming-img--before {
    filter: grayscale(0.22) saturate(0.88);
  }

  /* atmospheric opening cast onto the work, matching the gallery passe-partout */
  .becoming::after {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--color-ink-primary) 10%, transparent),
      inset 0 0 22px -6px color-mix(in srgb, var(--color-ink-primary) 30%, transparent);
  }

  .becoming-divider {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 0;
    transform: translateX(-0.5px);
    border-left: 1px solid color-mix(in srgb, var(--color-canvas-raised) 88%, transparent);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-ink-primary) 20%, transparent);
    pointer-events: none;
  }

  .becoming-handle {
    position: absolute;
    top: 50%;
    left: 0;
    display: grid;
    place-items: center;
    width: 2.4rem;
    height: 2.4rem;
    transform: translate(-50%, -50%);
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 26%, transparent);
    border-radius: 50%;
    color: var(--color-ink-primary);
    background: color-mix(in srgb, var(--color-canvas-raised) 92%, white);
    box-shadow: 0 2px 7px color-mix(in srgb, var(--color-ink-primary) 24%, transparent);
    cursor: ew-resize;
    pointer-events: auto;
    transition:
      border-color var(--duration-fast) var(--ease-atelier),
      color var(--duration-fast) var(--ease-atelier);
  }

  .becoming-handle:hover,
  .becoming--dragging .becoming-handle {
    border-color: var(--color-ember);
    color: var(--color-ember-deep);
  }

  .becoming-handle:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--color-ember) 55%, transparent);
    outline-offset: 3px;
  }

  .becoming-tag {
    position: absolute;
    bottom: 0.6rem;
    padding: 0.18rem 0.48rem;
    border-radius: 2px;
    color: color-mix(in srgb, var(--color-canvas-raised) 94%, white);
    background: color-mix(in srgb, var(--color-ink-primary) 48%, transparent);
    backdrop-filter: blur(4px);
    font-family: var(--font-body);
    font-size: 0.54rem;
    font-weight: 800;
    letter-spacing: 0.12em;
    line-height: 1.2;
    text-transform: uppercase;
    pointer-events: none;
    transition: opacity var(--duration-fast) var(--ease-atelier);
  }

  .becoming-tag--before {
    left: 0.6rem;
  }

  .becoming-tag--after {
    right: 0.6rem;
  }
</style>
