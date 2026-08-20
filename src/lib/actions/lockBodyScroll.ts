// Svelte action: keep the page behind a full-screen overlay from moving.
// iOS Safari will otherwise scroll the document under a `position: fixed`
// layer — and once that starts mid-gesture, sticky columns, 100vh frames
// and the visual viewport desync until the page is reloaded.
//
// Never set overflow on <html>: WebKit then treats `position: fixed` as
// `position: absolute`, and the overlay dumps into the page as a giant photo.

function shouldFreezeBody(): boolean {
  if (typeof navigator === 'undefined' || typeof window === 'undefined') return false;
  // MacBooks report maxTouchPoints > 0; only freeze on a real iOS touch device.
  if (!window.matchMedia('(pointer: coarse)').matches) return false;
  const ua = navigator.userAgent;
  return /iP(ad|hone|od)/.test(ua)
    || (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1);
}

export function lockBodyScroll(node: HTMLElement) {
  const body = document.body;
  const scrollY = window.scrollY;
  const freeze = shouldFreezeBody();
  const scrollbar = Math.max(0, window.innerWidth - document.documentElement.clientWidth);
  const prev = {
    bodyOverflow: body.style.overflow,
    bodyOverscroll: body.style.overscrollBehavior,
    bodyPosition: body.style.position,
    bodyTop: body.style.top,
    bodyLeft: body.style.left,
    bodyRight: body.style.right,
    bodyWidth: body.style.width,
    bodyPaddingRight: body.style.paddingRight,
  };

  body.style.overflow = 'hidden';
  body.style.overscrollBehavior = 'none';
  if (scrollbar > 0) body.style.paddingRight = `${scrollbar}px`;

  if (freeze) {
    body.style.position = 'fixed';
    body.style.top = `-${scrollY}px`;
    body.style.left = '0';
    body.style.right = '0';
    body.style.width = '100%';
  }

  function onTouchMove(e: TouchEvent) {
    if (node.contains(e.target as Node)) return;
    e.preventDefault();
  }

  document.addEventListener('touchmove', onTouchMove, { passive: false });

  return {
    destroy() {
      document.removeEventListener('touchmove', onTouchMove);
      body.style.overflow = prev.bodyOverflow;
      body.style.overscrollBehavior = prev.bodyOverscroll;
      body.style.position = prev.bodyPosition;
      body.style.top = prev.bodyTop;
      body.style.left = prev.bodyLeft;
      body.style.right = prev.bodyRight;
      body.style.width = prev.bodyWidth;
      body.style.paddingRight = prev.bodyPaddingRight;
      if (freeze) window.scrollTo(0, scrollY);
    },
  };
}
