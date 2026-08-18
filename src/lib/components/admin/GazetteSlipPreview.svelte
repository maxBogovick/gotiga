<script lang="ts">
  import { t } from '$lib/i18n';
  import { GAZETTE_KIND_KEY } from '$lib/gazette';
  import type { GazetteKind } from '$lib/types/api';
  import AppImage from '$lib/components/AppImage.svelte';

  let {
    kind,
    title,
    dek,
    imageUrl,
    extraUrls = [],
    dateLabel = '',
  }: {
    kind: GazetteKind;
    title: string;
    dek: string;
    imageUrl: string;
    extraUrls?: string[];
    dateLabel?: string;
  } = $props();
</script>

<aside class="slip" aria-label={$t('adminGazettePreview')}>
  <p class="slip-kicker">{$t('adminGazettePreview')}</p>
  <div class="card">
    {#if imageUrl}
      <span class="face">
        <AppImage src={imageUrl} alt="" class="face-img" sizes="72px" />
      </span>
      {#if extraUrls.length > 0}
        <span class="stack">
          {#each extraUrls.slice(0, 3) as url (url)}
            <span class="stack-item">
              <AppImage src={url} alt="" sizes="36px" />
            </span>
          {/each}
        </span>
      {/if}
    {/if}
    <p class="meta">
      <span>{$t(GAZETTE_KIND_KEY[kind])}</span>
      {#if dateLabel}<span>{dateLabel}</span>{/if}
    </p>
    <p class="title">{title.trim() || '…'}</p>
    {#if dek.trim()}
      <p class="dek">{dek.trim()}</p>
    {/if}
  </div>
</aside>

<style>
  .slip { min-width: 0; }
  .slip-kicker {
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #8a6a55;
    margin: 0 0 8px;
  }
  .card {
    background: #fbf6ee;
    border: 1px solid #d8c6b1;
    box-shadow:
      0 1px 0 rgba(52, 37, 28, 0.04),
      0 10px 24px -18px rgba(52, 37, 28, 0.35);
    padding: 16px 18px 14px;
    transform: rotate(-1.1deg);
    transform-origin: 20% 0;
  }
  .face {
    display: block;
    width: 72px;
    height: 90px;
    overflow: hidden;
    float: right;
    margin: 0 0 8px 14px;
    border: 1px solid #d8c6b1;
    background: #1a120e;
  }
  .face :global(.app-image-wrap),
  .face :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .stack {
    display: flex;
    gap: 4px;
    float: right;
    clear: right;
    margin: 0 0 8px 14px;
  }
  .stack-item {
    display: block;
    width: 36px;
    height: 44px;
    overflow: hidden;
    border: 1px solid #d8c6b1;
    background: #1a120e;
  }
  .stack-item :global(.app-image-wrap),
  .stack-item :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .meta {
    display: flex;
    gap: 10px;
    align-items: baseline;
    flex-wrap: wrap;
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #6f3b24;
    margin: 0 0 6px;
  }
  .title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 22px;
    line-height: 1.15;
    color: #34251c;
    margin: 0;
  }
  .dek {
    font-size: 14px;
    line-height: 1.5;
    color: #5f4636;
    margin: 8px 0 0;
  }
</style>
