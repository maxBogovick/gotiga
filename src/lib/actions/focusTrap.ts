// Svelte action: trap keyboard focus inside a modal dialog and restore focus to the
// previously-focused element when the dialog unmounts.
//
// Usage:  <div role="dialog" tabindex="-1" use:focusTrap> … </div>
//
// Pair with an Escape handler in the host component for full modal a11y.
//
// Focus always uses { preventScroll: true } and lands on the dialog node, not
// the first tiny-font control. On iOS, `.focus()` on a fixed overlay otherwise
// pans the visual viewport; scrolling after that desyncs the layout.

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]),' +
  ' textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

const FOCUS_OPTS: FocusOptions = { preventScroll: true };

export function focusTrap(node: HTMLElement) {
  const previouslyFocused = document.activeElement as HTMLElement | null;

  if (!node.hasAttribute('tabindex')) node.tabIndex = -1;

  function focusables(): HTMLElement[] {
    return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE))
      // skip elements that are hidden (display:none / visibility:hidden → no rendered rects).
      // offsetParent check fails for position:fixed elements; getClientRects() is universal.
      .filter((el) => el.getClientRects().length > 0 || el === document.activeElement);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key !== 'Tab') return;
    const els = focusables();
    if (els.length === 0) {
      e.preventDefault();
      node.focus(FOCUS_OPTS);
      return;
    }
    const first = els[0];
    const last = els[els.length - 1];
    const active = document.activeElement;
    if (e.shiftKey) {
      if (active === first || !node.contains(active)) {
        e.preventDefault();
        last.focus(FOCUS_OPTS);
      }
    } else if (active === last || !node.contains(active)) {
      e.preventDefault();
      first.focus(FOCUS_OPTS);
    }
  }

  // Move focus into the dialog after it mounts — the dialog itself, not a
  // child button. Tab still reaches the first control from here.
  queueMicrotask(() => {
    if (!node.contains(document.activeElement)) {
      node.focus(FOCUS_OPTS);
    }
  });

  node.addEventListener('keydown', onKeydown);

  return {
    destroy() {
      node.removeEventListener('keydown', onKeydown);
      previouslyFocused?.focus?.(FOCUS_OPTS);
    },
  };
}
