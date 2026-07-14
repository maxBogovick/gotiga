/**
 * Keep a `<style id="…">` in the document head in sync with a string of CSS.
 *
 * Call it from an `$effect` and it behaves like one: re-running writes the new CSS into
 * the same element, and the returned teardown removes it.
 *
 * There used to be TWO ways to do this on the home page — this one for the reel theme, and
 * `{@html '<style>…</style>'}` inside `<svelte:head>` for the layout's element overrides —
 * on the belief, written into a comment, that the `{@html}` form "is rendered once and does
 * not re-run when the theme arrives from the server". That is not true of Svelte 5: the
 * block compiles to `$.html(node, () => …)` inside `$.head(…)`, i.e. a reactive effect, and
 * it does update. (Whatever the original bug was, it was not this.)
 *
 * Both forms work, then; what does not work is having both. This is the one kept, because
 * it is the one that does not build a CSS string with `{@html}`, and because a single
 * mechanism is easier to reason about than two that disagree about why they exist.
 */
export function injectStyle(id: string, css: string): () => void {
    if (typeof document === 'undefined') return () => {};

    const existing = document.getElementById(id);
    const style = existing instanceof HTMLStyleElement ? existing : document.createElement('style');
    style.id = id;
    style.textContent = css;
    if (!style.parentNode) document.head.appendChild(style);

    return () => style.remove();
}
