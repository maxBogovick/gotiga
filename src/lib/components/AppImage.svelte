<script lang="ts">
  type Props = {
    src: string | undefined | null;
    alt?: string;
    class?: string;
    loading?: 'lazy' | 'eager';
    fetchpriority?: 'high' | 'low' | 'auto';
    decoding?: 'async' | 'sync' | 'auto';
    [key: string]: unknown;
  };

  let {
    src,
    alt = '',
    class: cls = '',
    loading = 'lazy',
    fetchpriority,
    decoding = 'async',
    ...rest
  }: Props = $props();

  // Derive a sibling .webp URL for paths the server produces:
  //   /static/images/preview/{uuid}.jpg  →  /static/images/preview/{uuid}.webp
  //   /static/images/thumb/{uuid}.jpg    →  /static/images/thumb/{uuid}.webp
  // External URLs (http/https) or already-webp src are passed through unchanged.
  function deriveWebp(url: string): string | null {
    if (!url || url.startsWith('http') || url.endsWith('.webp')) return null;
    const derived = url.replace(/\.(jpe?g|png)(\?.*)?$/i, '.webp$2');
    return derived !== url ? derived : null;
  }

  let webpSrc = $derived(src ? deriveWebp(src) : null);
</script>

{#if src}
  <picture>
    {#if webpSrc}
      <source srcset={webpSrc} type="image/webp" />
    {/if}
    <img
      {src}
      {alt}
      class={cls}
      {loading}
      {decoding}
      fetchpriority={fetchpriority}
      {...rest}
    />
  </picture>
{/if}
