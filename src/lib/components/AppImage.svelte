<script lang="ts">
  type Props = {
    src: string | undefined | null;
    thumbUrl?: string | null;
    alt?: string;
    class?: string;
    loading?: 'lazy' | 'eager';
    fetchpriority?: 'high' | 'low' | 'auto';
    decoding?: 'async' | 'sync' | 'auto';
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
    ...rest
  }: Props = $props();

  let loaded  = $state(false);
  let failed  = $state(false);
  let mainImg = $state<HTMLImageElement | undefined>();

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
      <img
        src={thumbUrl}
        {alt}
        aria-hidden="true"
        class="app-image-thumb"
        loading="eager"
        decoding="async"
      />
    {/if}

    <picture class="app-image-picture" class:app-image-picture--loaded={loaded || failed}>
      <img
        bind:this={mainImg}
        {src}
        {alt}
        {loading}
        {decoding}
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

  /* Blurred thumb sits underneath, fills the space */
  .app-image-thumb {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: blur(12px);
    transform: scale(1.05); /* hide blur edges */
    transition: opacity 0.3s ease;
  }

  /* Main image: invisible until loaded, then fade in */
  .app-image-picture {
    display: contents;
  }

  /* Fills the wrapper and covers it by default, so consumers only need to size the
     wrapper (named class or Tailwind w-full/h-full). Named consumers can still override
     object-fit/object-position via more specific :global rules. */
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
