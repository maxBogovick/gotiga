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

export type PictureWant = {
    /** srcset of the `<source type="image/webp">`, if the picture has one. */
    webp?: string | null;
    /** srcset of the fallback `<img>`. */
    srcset?: string | null;
    /** src of the fallback `<img>`. */
    src?: string | null;
};

/**
 * Svelte action: keep a `<picture>`'s `<source>` and `<img>` pointing at what the
 * component currently wants, including right after hydration (see `syncAttr`).
 *
 * The `<source>` is the one that bites: a browser that decodes WebP picks it over the
 * `<img>`'s own srcset, so a stale prerendered `<source>` shows the build-time photo on
 * top of a correct `<img>` — two different photographs stacked in one frame. Runs on
 * mount and on every change of the parameters, and is a no-op when the DOM already agrees.
 */
export function repairPicture(node: HTMLElement, want: PictureWant) {
    const apply = (w: PictureWant) => {
        const source = node.querySelector('source[type="image/webp"]');
        const img = node.querySelector('img');
        if (source) syncAttr(source, 'srcset', w.webp);
        // The <source> first: the browser re-runs its selection when the <img>'s own
        // src/srcset is touched, so the sources must already be correct by then.
        syncAttr(img, 'srcset', w.srcset);
        syncAttr(img, 'src', w.src);
    };
    apply(want);
    return { update: apply };
}

/** Same repair for a lone `<img>` that is not inside a `<picture>`. */
export function repairSrc(node: HTMLImageElement, src: string | null | undefined) {
    syncAttr(node, 'src', src);
    return { update: (next: string | null | undefined) => syncAttr(node, 'src', next) };
}
