// Svelte action: keep the page behind a full-screen overlay from moving.
// iOS Safari will otherwise scroll the document under a `position: fixed`
// layer — and once that starts mid-gesture, sticky columns, 100vh frames
// and the visual viewport desync until the page is reloaded.

export function lockBodyScroll(node: HTMLElement) {
  const html = document.documentElement;
  const body = document.body;
  const prev = {
    htmlOverflow: html.style.overflow,
    bodyOverflow: body.style.overflow,
    htmlOverscroll: html.style.overscrollBehavior,
    bodyOverscroll: body.style.overscrollBehavior,
  };

  html.style.overflow = 'hidden';
  body.style.overflow = 'hidden';
  html.style.overscrollBehavior = 'none';
  body.style.overscrollBehavior = 'none';

  function onTouchMove(e: TouchEvent) {
    if (node.contains(e.target as Node)) return;
    e.preventDefault();
  }

  document.addEventListener('touchmove', onTouchMove, { passive: false });

  return {
    destroy() {
      document.removeEventListener('touchmove', onTouchMove);
      html.style.overflow = prev.htmlOverflow;
      body.style.overflow = prev.bodyOverflow;
      html.style.overscrollBehavior = prev.htmlOverscroll;
      body.style.overscrollBehavior = prev.bodyOverscroll;
    },
  };
}
