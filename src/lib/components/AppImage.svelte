<script lang="ts">
  import { onMount } from 'svelte';
  import { resolveWebpUrl, resolveSrcset } from '$lib/api';

  type Props = {
    src: string | undefined | null;
    thumbUrl?: string | null;
    alt?: string;
    class?: string;
    loading?: 'lazy' | 'eager';
    fetchpriority?: 'high' | 'low' | 'auto';
    decoding?: 'async' | 'sync' | 'auto';
    /** How wide this image will actually be rendered, as a CSS `sizes` value.
     *  Without it the browser must assume the image spans the full viewport (100vw) and
     *  will pick a needlessly large rendition — for a grid tile that is the difference
     *  between fetching the 900px medium and the 1800px preview. Callers that render an
     *  image at less than full width should say so. */
    sizes?: string;
    /** Intrinsic pixel size, when the caller knows it. Purely a hint to the browser
     *  so it can reserve the box before the bytes arrive (prevents layout shift on
     *  the rare consumer that does NOT size .app-image-wrap itself). The CSS below
     *  still governs the rendered size, so passing these never changes the layout. */
    width?: number;
    height?: number;
    [key: string]: unknown;
  };

  let {
    src,
    thumbUrl = null,
    alt = '',
    class: cls = '',
    loading = 'lazy',
    fetchpriority,
    decoding = 'async',
    sizes = '100vw',
    width,
    height,
    ...rest
  }: Props = $props();

  // The three renditions (420 / 900 / 1800) the server wrote for this image, if it is a
  // figurine photo. `null` for anything else (bundled art, avatars, remote URLs), in which
  // case we fall back to serving `src` exactly as given — the old behaviour.
  let candidates = $derived(resolveSrcset(src));

  // The upload pipeline (save_image_variants in handlers.rs) writes a WebP sibling next
  // to every JPEG it produces, at the same path with a swapped extension — but only the
  // JPEG path is stored on the image record, so nothing was ever serving the WebP. It was
  // being encoded and written to disk for no one. Offering it as a <source> costs one line
  // and hands every modern browser a materially smaller file; anything that can't decode
  // WebP simply falls through to the <img> below, which is unchanged. resolveWebpUrl
  // returns null for non-JPEG sources (already-WebP, PNG, remote URLs), and a <source>
  // with an undefined srcset is inert — so this is safe for every caller.
  let srcWebp = $derived(resolveWebpUrl(src));
  let thumbWebp = $derived(resolveWebpUrl(thumbUrl));

  let loaded  = $state(false);
  let failed  = $state(false);
  let mainImg = $state<HTMLImageElement | undefined>();
  let mainPicture = $state<HTMLElement | undefined>();
  let thumbImg = $state<HTMLImageElement | undefined>();
  let thumbPicture = $state<HTMLElement | undefined>();

  /**
   * Repair the photo after hydration.
   *
   * Svelte's `set_attribute` deliberately SKIPS `src` and `srcset` while hydrating
   * (internal/client/dom/elements/attributes.js) — resetting them would fire a second
   * network request for a picture the server already pointed at, so it assumes the
   * server's markup is right. On a page that is PRERENDERED (the home page, the archive,
   * every /figurines/[id]) that assumption is false: the HTML is a snapshot of the
   * collection as it stood at BUILD time, while the load functions re-run in the browser
   * against the live API. Text nodes and hrefs get updated; the photo does not. A card
   * then wears whichever photo occupied its slot on the day of the deploy — a work titled
   * "Gnome" showing the monkey.
   *
   * So once hydration is over, compare what the DOM actually holds against what this
   * component was asked to show, and write the difference through by hand. In a pure
   * client render (CSR, Tauri) the two already agree and every branch below is a no-op.
   */
  function syncAttr(el: Element | null | undefined, name: string, want: string | null): void {
    if (!el) return;
    const have = el.getAttribute(name);
    if (want) {
      if (have !== want) el.setAttribute(name, want);
    } else if (have !== null) {
      el.removeAttribute(name);
    }
  }

  onMount(() => {
    // The <source> first: the browser re-runs its selection when the <img>'s own
    // src/srcset is touched, so the sources must already be correct by then.
    syncAttr(thumbPicture?.querySelector('source'), 'srcset', thumbWebp);
    syncAttr(thumbImg, 'src', thumbUrl);

    syncAttr(mainPicture?.querySelector('source'), 'srcset', candidates?.webp || srcWebp);
    syncAttr(mainImg, 'srcset', candidates?.jpeg ?? null);
    syncAttr(mainImg, 'src', src ?? null);

    // A repaired image starts loading from scratch, and the stale one it replaces may
    // have finished already (marking us `loaded`, which fades the blur-up out). Re-read
    // the element's real state so the fade follows the photo actually on screen.
    if (mainImg && !(mainImg.complete && mainImg.naturalWidth > 0)) loaded = false;
  });

  // Reset whenever the source changes. If the (possibly cached) image is already
  // complete, mark it loaded right away: browsers don't re-fire `load` for an
  // already-decoded src, so without this a re-run of this effect — e.g. a parent
  // that re-passes props on a timer — would leave a cached image stuck invisible.
  $effect(() => {
    void src;
    if (mainImg?.complete && mainImg.naturalWidth > 0) {
      loaded = true;
      failed = false;
    } else {
      loaded = false;
      failed = false;
    }
  });

  function onLoad() { loaded = true; }
  function onError() { failed = true; }
</script>

{#if src}
  <div class="app-image-wrap {cls}" {...rest}>
    {#if thumbUrl && !loaded}
      <!-- Blur-up placeholder. It used to be hardcoded loading="eager", which quietly
           cancelled the caller's loading="lazy": every off-screen tile in the archive and
           on the home grid fetched its placeholder immediately, so a long grid paid for a
           full screenful of images nobody had scrolled to yet. Inheriting the caller's
           loading mode keeps the blur-up for what's actually on screen and lets the rest
           stay off the network until it's near the viewport. -->
      <picture bind:this={thumbPicture}>
        {#if thumbWebp}
          <source type="image/webp" srcset={thumbWebp} />
        {/if}
        <img
          bind:this={thumbImg}
          src={thumbUrl}
          {alt}
          aria-hidden="true"
          class="app-image-thumb"
          {loading}
          decoding="async"
        />
      </picture>
    {/if}

    <picture
      bind:this={mainPicture}
      class="app-image-picture"
      class:app-image-picture--loaded={loaded || failed}
    >
      {#if candidates?.webp}
        <source type="image/webp" srcset={candidates.webp} {sizes} />
      {:else if srcWebp}
        <source type="image/webp" srcset={srcWebp} />
      {/if}
      <img
        bind:this={mainImg}
        {src}
        srcset={candidates?.jpeg}
        sizes={candidates ? sizes : undefined}
        {alt}
        {loading}
        {decoding}
        {width}
        {height}
        fetchpriority={fetchpriority}
        class="app-image-main"
        onload={onLoad}
        onerror={onError}
      />
    </picture>
  </div>
{/if}

<style>
  .app-image-wrap {
    position: relative;
    overflow: hidden;
  }

  /* Blurred thumb sits underneath, fills the space.
     The <picture> wrapper added around it must not become a layout box of its own —
     display: contents keeps the <img> as the wrapper's direct visual child, so the
     absolute positioning below still resolves against .app-image-wrap. */
  .app-image-wrap :global(picture) {
    display: contents;
  }

  .app-image-thumb {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: blur(6px);
    transform: scale(1.03); /* hide blur edges */
    transition: opacity 0.3s ease;
  }

  /* Main image: invisible until loaded, then fade in */
  .app-image-picture {
    display: contents;
  }

  /* Fills the wrapper and covers it by default, so consumers only need to size the
     wrapper (named class or Tailwind w-full/h-full). Named consumers can still override
     object-fit/object-position via more specific :global rules.
     width/height here also neutralise the optional intrinsic width/height attributes —
     those are a reservation hint for the browser, never a layout instruction. */
  .app-image-main {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity 0.4s ease;
  }

  .app-image-picture--loaded .app-image-main {
    opacity: 1;
  }
</style>
