<script lang="ts">
  /**
   * «Приобрести»: объясняет, почему оплата идёт через Etsy, и ведёт на страницу
   * работы там (`etsyUrl`) или, если её нет, в магазин. Сайт платежей не
   * принимает (§ 1).
   */
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { t, lang } from '$lib/i18n';
  import { interact } from '$lib/analytics';
  import { ETSY_SHOP_URL } from '$lib/site';
  import { focusTrap } from '$lib/actions/focusTrap';
  import { lockBodyScroll } from '$lib/actions/lockBodyScroll';
  import { portal } from '$lib/actions/portal';
  import { workText } from '$lib/workText';
  import AppImage from '$lib/components/AppImage.svelte';
  import EtsyWhy from '$lib/components/EtsyWhy.svelte';
  import type { Figurine, FigurineListItem } from '$lib/types/api';

  let {
    isOpen = false,
    work,
    onClose = () => {},
    onSimilar = () => {},
  }: {
    isOpen?: boolean;
    work: Figurine | FigurineListItem | null;
    onClose?: () => void;
    onSimilar?: () => void;
  } = $props();

  let name = $derived(work ? workText(work, 'name', $lang) || work.name : '');
  let etsyUrl = $derived(work?.etsyUrl?.trim() || null);
  let href = $derived(etsyUrl ?? ETSY_SHOP_URL);
  let photo = $derived.by(() => {
    if (!work) return null;
    if ('faceImageUrl' in work) return work.faceImageUrl;
    const imgs = work.images ?? [];
    const face = imgs.find((i) => i.imageType === 'face') ?? imgs[0];
    return face?.thumbUrl ?? face?.url ?? null;
  });
  let wasOpen = false;

  $effect(() => {
    if (isOpen && !wasOpen) interact('buy_open', etsyUrl ? 'work' : 'shop');
    wasOpen = isOpen;
  });

  function handleKeydown(e: KeyboardEvent) {
    if (isOpen && e.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen && work}
  <div
    class="buy-backdrop"
    transition:fade={{ duration: 250 }}
    onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    role="presentation"
    use:portal
    use:lockBodyScroll
  >
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="buy-dialog-title"
      tabindex="-1"
      class="buy-dialog"
      in:fly={{ y: 36, duration: 420, easing: cubicOut }}
      use:focusTrap
    >
      <button type="button" class="buy-close" onclick={onClose} aria-label={$t('lightboxClose')}>
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
          <path d="M1 1l12 12M13 1L1 13" />
        </svg>
      </button>

      <header class="buy-head">
        {#if photo}
          <div class="buy-photo">
            <AppImage src={photo} alt="" sizes="72px" />
          </div>
        {/if}
        <div>
          <h3 id="buy-dialog-title">{name}</h3>
          <p class="buy-via">{$t('buyViaEtsy')}</p>
        </div>
      </header>

      <EtsyWhy />

      {#if !etsyUrl}
        <p class="buy-soon">{$t('buySoonOnEtsy')}</p>
      {/if}

      <div class="buy-actions">
        <a
          class="buy-go"
          {href}
          target="_blank"
          rel="noopener"
          onclick={() => interact('etsy_out', etsyUrl ? 'work' : 'shop', null, false)}
        >
          {etsyUrl ? $t('buyGoEtsy') : $t('buyOpenShop')}
        </a>
        <button type="button" class="buy-similar" onclick={() => { onSimilar(); onClose(); }}>
          {$t('buySimilar')}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .buy-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: rgba(111, 59, 36, 0.35);
    overscroll-behavior: none;
  }
  .buy-dialog {
    position: relative;
    display: grid;
    gap: 1rem;
    width: min(100%, 34rem);
    max-height: min(90vh, 44rem);
    overflow: auto;
    overscroll-behavior: contain;
    padding: 1.25rem;
    border: 1px solid #d8c6b1;
    border-radius: 8px;
    color: #34251c;
    background: #fff9f0;
    box-shadow: 0 24px 70px rgba(52, 37, 28, 0.2);
  }
  .buy-close {
    position: absolute;
    top: 0.75rem;
    right: 0.75rem;
    display: grid;
    place-items: center;
    width: 2rem;
    height: 2rem;
    border: 0;
    color: rgba(52, 37, 28, 0.52);
    background: transparent;
    cursor: pointer;
  }
  .buy-head {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    padding-right: 2rem;
  }
  .buy-photo {
    flex: none;
    width: 4.5rem;
    height: 4.5rem;
    overflow: hidden;
    border: 1px solid #d8c6b1;
    border-radius: 6px;
    background: #f8f1e7;
  }
  .buy-photo :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  h3 {
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.25rem;
    line-height: 1.25;
  }
  .buy-via {
    margin: 0.2rem 0 0;
    font-size: 0.85rem;
    color: #6f3b24;
  }
  .buy-soon {
    margin: 0;
    font-size: 0.9rem;
    color: #6f3b24;
  }
  .buy-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
  }
  .buy-go,
  .buy-similar {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: 2.75rem;
    padding: 0 1.2rem;
    border-radius: 6px;
    font-size: 0.9rem;
    font-weight: 600;
    text-decoration: none;
    cursor: pointer;
  }
  .buy-go {
    border: 1px solid #c65f3c;
    color: #fff9f0;
    background: #c65f3c;
  }
  .buy-go:hover {
    background: #6f3b24;
    border-color: #6f3b24;
  }
  .buy-similar {
    border: 1px solid #d8c6b1;
    color: #34251c;
    background: transparent;
  }
  .buy-similar:hover {
    border-color: #c65f3c;
  }
  .buy-go:focus-visible,
  .buy-similar:focus-visible,
  .buy-close:focus-visible {
    outline: 2px solid #c65f3c;
    outline-offset: 2px;
  }
</style>
