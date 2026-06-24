/**
 * `use:dwellReveal` — lift a sealed card's shadow when the visitor lingers.
 *
 * Starts a timer on pointer-enter; if the pointer stays `ms` milliseconds the
 * card reveals itself (no need to open the work). Leaving before the timer
 * fires cancels it, so a passing glance does nothing — only a sustained look is
 * rewarded. `ms <= 0` disables it (used when the feature is off, or the card is
 * already revealed, so it never re-fires).
 *
 * Pointer-based, so it's a desktop "linger" affordance; on touch a tap opens
 * the work directly, which reveals it anyway.
 */
export interface DwellRevealParams {
  ms: number;
  /** Pointer entered — a look has begun (drive the in-progress "thinning" visual). */
  onStart?: () => void;
  /** Pointer left before completing — the look was abandoned (settle back). */
  onStop?: () => void;
  /** The dwell completed — commit the glance. */
  onReveal: () => void;
}

export function dwellReveal(node: HTMLElement, params: DwellRevealParams) {
  let current = params;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const clearTimer = () => {
    if (timer) {
      clearTimeout(timer);
      timer = undefined;
    }
  };

  const onEnter = () => {
    clearTimer();
    if (!current.ms || current.ms <= 0) return;
    current.onStart?.();
    timer = setTimeout(() => {
      timer = undefined;
      current.onReveal();
    }, current.ms);
  };

  const onLeave = () => {
    clearTimer();
    current.onStop?.();
  };

  node.addEventListener('pointerenter', onEnter);
  node.addEventListener('pointerleave', onLeave);
  node.addEventListener('pointercancel', onLeave);

  return {
    update(next: DwellRevealParams) {
      current = next;
    },
    destroy() {
      clearTimer();
      node.removeEventListener('pointerenter', onEnter);
      node.removeEventListener('pointerleave', onLeave);
      node.removeEventListener('pointercancel', onLeave);
    },
  };
}
