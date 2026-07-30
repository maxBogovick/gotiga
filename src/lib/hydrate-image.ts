/**
 * Repair an <img>/<picture> after hydration.
 *
 * Svelte's `set_attribute` deliberately SKIPS `src` and `srcset` while hydrating
 * (internal/client/dom/elements/attributes.js) — re-setting them would fire a second
 * network request for a picture the server already pointed at, so it assumes the
 * server's markup is right. On a PRERENDERED page (the home page, the archive, every
 * /figurines/[id]) that assumption is false: the HTML is a snapshot of the collection
 * as it stood at BUILD time, while the load functions re-run in the browser against
 * the live API. Text nodes and hrefs get updated; the photo does not.
 *
 * So once hydration is over, compare what the DOM actually holds against what the
 * component was asked to show, and write the difference through by hand. In a pure
 * client render (CSR) the two already agree and every call is a no-op.
 */
export function syncAttr(el: Element | null | undefined, name: string, want: string | null | undefined): void {
    if (!el) return;
    const have = el.getAttribute(name);
    if (want) {
        if (have !== want) el.setAttribute(name, want);
    } else if (have !== null) {
        el.removeAttribute(name);
    }
}
