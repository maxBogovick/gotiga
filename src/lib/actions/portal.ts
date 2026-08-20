// Move a node to document.body (or another host) so position:fixed is
// relative to the viewport — not a transformed / isolated ancestor.

export function portal(node: HTMLElement, target: HTMLElement | string = 'body') {
  function hostFor(to: HTMLElement | string) {
    if (typeof to === 'string') return document.querySelector(to) ?? document.body;
    return to;
  }

  hostFor(target).appendChild(node);

  return {
    update(to: HTMLElement | string) {
      const host = hostFor(to);
      if (node.parentNode !== host) host.appendChild(node);
    },
    destroy() {
      node.remove();
    },
  };
}
