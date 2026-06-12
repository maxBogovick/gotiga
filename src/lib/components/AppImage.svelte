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

  function deriveWebp(url: string): string | null {
    if (!url || url.startsWith('http') || url.endsWith('.webp')) return null;
    const derived = url.replace(/\.(jpe?g|png)(\?.*)?$/i, '.webp$2');
    return derived !== url ? derived : null;
  }

  let webpSrc = $derived(src ? deriveWebp(src) : null);
  let loaded  = $state(false);

  // Reset on src change
  $effect(() => { void src; loaded = false; });

  function onLoad() { loaded = true; }
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

    <picture class="app-image-picture" class:app-image-picture--loaded={loaded}>
      {#if webpSrc}
        <source srcset={webpSrc} type="image/webp" />
      {/if}
      <img
        {src}
        {alt}
        {loading}
        {decoding}
        fetchpriority={fetchpriority}
        class="app-image-main"
        onload={onLoad}
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
