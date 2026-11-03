<script lang="ts">
  import { cachedFigurine, loadFigurine } from '$lib/figurineFull';
  /**
   * Просмотр работы в архиве без ухода со страницы: на широком экране панель
   * стоит справа от списка, на телефоне — лист снизу. Кнопка действия («Приобрести»
   * или «Заказать похожую работу») всегда на одном месте внизу панели.
   * Листают стрелками, свайпом по фото и кнопками; «Открыть страницу работы»
   * ведёт на полную страницу.
   */
  import { tick } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { t, lang, LANG_META } from '$lib/i18n';
  import { figurineHref } from '$lib/figurineHref';
  import { localizeWork } from '$lib/workText';
  import AppImage from '$lib/components/AppImage.svelte';
  import ContactMessageForm from '$lib/components/ContactMessageForm.svelte';
  import type { Figurine, FigurineListItem } from '$lib/types/api';

  let {
    work,
    position,
    total,
    saved = false,
    onClose,
    onPrev,
    onNext,
    onBuy,
    onSimilar,
    onToggleSaved,
    shareCopied = false,
    onShare,
    onPin,
    startPhoto = 0,
    startAsk = false,
    similar = [],
    onOpenWork,
  }: {
    work: FigurineListItem;
    /** Номер работы в показанном списке, с единицы. */
    position: number;
    total: number;
    saved?: boolean;
    onClose: () => void;
    onPrev: () => void;
    onNext: () => void;
    onBuy: () => void;
    onSimilar: () => void;
    onToggleSaved: () => void;
    shareCopied?: boolean;
    onShare: () => void;
    onPin: () => void;
    /** Какое фото открыть первым (нажали на миниатюру в карточке). */
    startPhoto?: number;
    /** Открыть сразу с формой «Спросить». */
    startAsk?: boolean;
    /** Похожие работы, выбранные автором (до трёх). */
    similar?: FigurineListItem[];
    onOpenWork: (id: string) => void;
  } = $props();

  let full = $state<Figurine | null>(null);
  let photoIndex = $state(0);
  let asking = $state(false);
  let askEl = $state<HTMLElement | null>(null);
  let askPrefill = $derived.by(() => {
    const [open, close] = LANG_META[$lang].quotes;
    return `${open}${work.name}${close}: `;
  });
  function toggleAsk() {
    asking = !asking;
    if (asking) void tick().then(() => askEl?.scrollIntoView({ block: 'nearest', behavior: 'smooth' }));
  }
  let bodyEl = $state<HTMLElement | null>(null);

  $effect(() => {
    const id = work.id;
    photoIndex = startPhoto;
    asking = startAsk;
    if (startAsk) void tick().then(() => askEl?.scrollIntoView({ block: 'nearest' }));
    bodyEl?.scrollTo({ top: 0 });
    const known = cachedFigurine(id);
    if (known !== undefined) {
      full = known;
      return;
    }
    full = null;
    let current = true;
    void loadFigurine(id).then((f) => {
      if (current) full = f;
    });
    return () => {
      current = false;
    };
  });

  let fullText = $derived(full ? localizeWork(full, $lang) : null);

  let photos = $derived.by(() => {
    const imgs = fullText?.images ?? [];
    if (imgs.length > 0) {
      const rank = (type: string) => (type === 'face' ? 0 : type === 'full' ? 1 : 2);
      return imgs
        .slice()
        .sort((a, b) => rank(a.imageType) - rank(b.imageType))
        .map((i) => ({ url: i.url, thumb: i.thumbUrl ?? i.url, alt: i.altText || work.name }));
    }
    const out: { url: string; thumb: string; alt: string }[] = [];
    const face = work.faceImageLargeUrl ?? work.faceImageUrl;
    if (face) out.push({ url: face, thumb: work.faceImageUrl ?? face, alt: work.name });
    const detail = work.detailImageLargeUrl ?? work.detailImageUrl;
    if (detail) out.push({ url: detail, thumb: work.detailImageUrl ?? detail, alt: work.name });
    return out;
  });
  let photo = $derived(photos[Math.min(photoIndex, Math.max(photos.length - 1, 0))] ?? null);

  let statusLabel = $derived(
    work.status === 'available' ? $t('archiveStatusAvailableLabel')
      : work.status === 'reserved' ? $t('archiveStatusReservedLabel')
      : work.status === 'in_progress' ? $t('profileWishInProgress')
      : $t('archiveStatusSoldLabel'),
  );

  let facts = $derived(
    [
      { key: 'technique', label: $t('archiveTechniqueLabel'), value: fullText?.technique ?? work.technique },
      { key: 'material', label: $t('archiveMaterialLabel'), value: fullText?.material ?? work.material },
      { key: 'dimensions', label: $t('figurineDimensions'), value: fullText?.dimensions ?? work.dimensions },
    ].filter((f) => f.value && String(f.value).trim()),
  );

  let description = $derived((fullText?.fullDescription ?? '').trim() || (work.shortText ?? '').trim());

  // Свайп по фото листает работы; короткое касание остаётся нажатием.
  let touchX = 0;
  let touchY = 0;
  function onTouchStart(e: TouchEvent) {
    touchX = e.touches[0].clientX;
    touchY = e.touches[0].clientY;
  }
  function onTouchEnd(e: TouchEvent) {
    const dx = e.changedTouches[0].clientX - touchX;
    const dy = e.changedTouches[0].clientY - touchY;
    if (Math.abs(dx) < 56 || Math.abs(dx) < Math.abs(dy) * 1.4) return;
    if (dx < 0) onNext();
    else onPrev();
  }
</script>

<!-- Подложка есть только у листа на телефоне -->
<button
  type="button"
  class="peek-backdrop"
  aria-label={$t('lightboxClose')}
  onclick={onClose}
  transition:fade={{ duration: 180 }}
></button>

<aside
  class="peek"
  aria-label={$t('peekLabel')}
  in:fly={{ y: 24, duration: 260, easing: cubicOut }}
>
  <header class="peek__bar">
    <div class="peek__nav">
      <button type="button" class="peek__icon" onclick={onPrev} aria-label={$t('figurineNavPrev')} disabled={total < 2}>
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true"><path d="M9 2L4 7l5 5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
      </button>
      <span class="peek__count">{position} / {total}</span>
      <button type="button" class="peek__icon" onclick={onNext} aria-label={$t('figurineNavNext')} disabled={total < 2}>
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true"><path d="M5 2l5 5-5 5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
      </button>
    </div>
    <span class="peek__keys">{$t('peekKeysHint')}</span>
    <button type="button" class="peek__icon" onclick={onClose} aria-label={$t('lightboxClose')}>
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true"><path d="M1.5 1.5l9 9M10.5 1.5l-9 9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/></svg>
    </button>
  </header>

  <div class="peek__body" bind:this={bodyEl}>
    <div class="peek__photo" ontouchstart={onTouchStart} ontouchend={onTouchEnd}>
      {#key photo?.url}
        {#if photo}
          <div class="peek__photo-in" in:fade={{ duration: 220 }}>
            <AppImage
              src={photo.url}
              thumbUrl={photo.thumb}
              alt={photo.alt}
              class="peek__img"
              sizes="(max-width: 1023px) 100vw, 480px"
            />
          </div>
        {/if}
      {/key}
    </div>

    {#if photos.length > 1}
      <div class="peek__thumbs" role="group">
        {#each photos as p, i (p.url)}
          <button
            type="button"
            class="peek__thumb"
            class:peek__thumb--on={i === photoIndex}
            onclick={() => (photoIndex = i)}
            aria-label="{work.name} {i + 1}"
            aria-pressed={i === photoIndex}
          >
            <AppImage src={p.thumb} alt="" class="peek__thumb-img" loading="lazy" sizes="56px" />
          </button>
        {/each}
      </div>
    {/if}

    <div class="peek__text">
      <p class="peek__meta">
        <span class="peek__status peek__status--{work.status}">{statusLabel}</span>
        {#if work.year}<span aria-hidden="true">·</span><span>{work.year}</span>{/if}
        {#if work.series}<span aria-hidden="true">·</span><span>{work.series}</span>{/if}
      </p>
      <h2 class="peek__name">{work.name}</h2>

      {#if description}
        <p class="peek__desc">{description}</p>
      {/if}

      {#if similar.length > 0}
        <div class="peek__similar">
          <p class="peek__label">{$t('dockSimilar')}</p>
          <div class="peek__similar-row">
            {#each similar as sw (sw.id)}
              <button type="button" class="peek__sim" onclick={() => onOpenWork(sw.id)} title={sw.name} aria-label={sw.name}>
                <AppImage src={sw.faceImageUrl} thumbUrl={sw.thumbUrl} alt="" class="peek__sim-img" loading="lazy" sizes="90px" />
              </button>
            {/each}
          </div>
        </div>
      {/if}

      {#if facts.length > 0}
        <dl class="peek__facts">
          {#each facts as f (f.key)}
            <div>
              <dt>{f.label}</dt>
              <dd>{f.value}</dd>
            </div>
          {/each}
        </dl>
      {/if}
    </div>

    {#if asking}
      <div class="peek__ask" bind:this={askEl}>
        <p class="peek__label">{$t('headerContactNote')}</p>
        <ContactMessageForm source="dock" compact prefill={askPrefill} />
      </div>
    {/if}
  </div>

  <footer class="peek__foot">
    <div class="peek__foot-row">
      {#if work.status === 'available'}
        <button type="button" class="peek__primary" onclick={onBuy}>{$t('buyAction')}</button>
      {:else}
        <button type="button" class="peek__primary peek__primary--quiet" onclick={onSimilar}>{$t('commissionCreateSimilarCta')}</button>
      {/if}
      <button
        type="button"
        class="peek__icon peek__icon--lg"
        class:peek__icon--saved={saved}
        onclick={onToggleSaved}
        aria-pressed={saved}
        aria-label={saved ? $t('cardSaved') : $t('cardSave')}
        title={saved ? $t('cardSaved') : $t('cardSave')}
      >
        <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
          <path d="M7 12.5C7 12.5 1 8.5 1 4.5C1 2.5 2.5 1 4.5 1C5.5 1 6.5 1.8 7 3C7.5 1.8 8.5 1 9.5 1C11.5 1 13 2.5 13 4.5C13 8.5 7 12.5 7 12.5Z" fill={saved ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="1.1" stroke-linejoin="round"/>
        </svg>
      </button>
    </div>
    <div class="peek__more">
      <a class="peek__page" href={figurineHref(work)} data-sveltekit-preload-data="hover">
        {$t('peekOpenPage')} <span aria-hidden="true">→</span>
      </a>
      <button type="button" class="peek__ask-btn" class:peek__ask-btn--on={asking} onclick={toggleAsk} aria-expanded={asking}>{$t('dockAsk')}</button>
      <button type="button" class="peek__icon" onclick={onShare} aria-label={shareCopied ? $t('cardLinkCopied') : $t('cardShare')} title={shareCopied ? $t('cardLinkCopied') : $t('cardShare')}>
        {#if shareCopied}
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true"><path d="M2 7L5.5 10.5L12 3.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
        {:else}
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true"><path d="M9 2H12V5" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round"/><path d="M12 2L7 7" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/><path d="M6 3H3C2.4 3 2 3.4 2 4V11C2 11.6 2.4 12 3 12H10C10.6 12 11 11.6 11 11V8" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/></svg>
        {/if}
      </button>
      {#if work.faceImageUrl || work.detailImageUrl}
        <button type="button" class="peek__icon" onclick={onPin} aria-label={$t('cardPinterest')} title={$t('cardPinterest')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true"><path d="M5 12.6C4.6 11 4.4 9 4.9 7.1C4.9 7.1 4.6 6.4 4.6 5.5C4.6 4.1 5.4 3.1 6.4 3.1C7.3 3.1 7.7 3.8 7.7 4.6C7.7 5.5 7.1 6.8 6.8 8C6.6 9 7.3 9.8 8.3 9.8C10 9.8 11.3 7.6 11.3 5.4C11.3 3.4 9.9 2 7.6 2C5 2 3.4 3.9 3.4 6C3.4 6.9 3.7 7.6 4.1 8.1" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </button>
      {/if}
    </div>
  </footer>
</aside>

<style>
  /* Телефон и узкое окно: лист снизу поверх списка */
  .peek-backdrop {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: rgba(28, 16, 10, 0.45);
    border: 0;
    cursor: default;
  }
  .peek {
    position: fixed;
    left: 0;
    right: 0;
    /* над нижней панелью телефона (64px): страница лежит в своём слое наложения,
       поэтому выше неё лист поднять нельзя, а встать над ней можно */
    bottom: calc(64px + env(safe-area-inset-bottom, 0px));
    z-index: 201;
    display: flex;
    flex-direction: column;
    max-height: calc(88dvh - 64px);
    background: #f8f1e7;
    border-top: 1px solid #d8c6b1;
    border-radius: 14px 14px 0 0;
    border-bottom: 1px solid #d8c6b1;
    box-shadow: 0 -18px 50px rgba(44, 23, 16, 0.28);
    color: #34251c;
    overscroll-behavior: contain;
  }

  /* Широкое окно: панель рядом со списком, прилипает под шапкой */
  @media (min-width: 1024px) {
    .peek-backdrop { display: none; }
    .peek {
      bottom: auto;
      position: sticky;
      top: calc(var(--site-header-h, 58px) + 16px);
      z-index: 5;
      /* шапка к этому времени сжата до 58px, поэтому не var(--site-header-h) */
      scroll-margin-top: 74px;
      max-height: calc(100dvh - var(--site-header-h, 58px) - 32px);
      border: 1px solid #d8c6b1;
      border-radius: 3px;
      box-shadow: 0 12px 40px rgba(44, 23, 16, 0.12);
    }
  }

  .peek__bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 10px 8px 12px;
    border-bottom: 1px solid rgba(216, 198, 177, 0.7);
  }
  .peek__nav { display: flex; align-items: center; gap: 4px; }
  .peek__count {
    min-width: 46px;
    text-align: center;
    font-size: 12px;
    letter-spacing: 0.08em;
    color: #7c6554;
    font-variant-numeric: tabular-nums;
  }
  .peek__keys {
    display: none;
    margin-left: auto;
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #a08c7a;
  }
  @media (min-width: 1024px) and (hover: hover) { .peek__keys { display: block; } }

  .peek__icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: 999px;
    border: 1px solid rgba(52, 37, 28, 0.16);
    background: transparent;
    color: #5f4636;
    cursor: pointer;
    transition: background 0.2s, border-color 0.2s, color 0.2s;
  }
  .peek__icon:hover:not(:disabled) { background: rgba(52, 37, 28, 0.06); border-color: rgba(52, 37, 28, 0.35); color: #34251c; }
  .peek__icon:disabled { opacity: 0.35; cursor: default; }
  .peek__icon--lg { width: 44px; height: 44px; flex: none; }
  .peek__icon--saved { color: #c65f3c; border-color: rgba(198, 95, 60, 0.45); background: rgba(198, 95, 60, 0.1); }

  .peek__body {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .peek__photo {
    position: relative;
    aspect-ratio: 4 / 3;
    background: #efe5d6;
    overflow: hidden;
    touch-action: pan-y;
  }
  @media (min-width: 1024px) { .peek__photo { aspect-ratio: 1 / 1; } }
  .peek__photo-in { position: absolute; inset: 0; }
  .peek__photo :global(.peek__img) { width: 100%; height: 100%; object-fit: contain; }

  .peek__thumbs { display: flex; gap: 6px; padding: 8px 12px 0; overflow-x: auto; }
  .peek__thumb {
    flex: none;
    width: 56px;
    height: 56px;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 2px;
    background: #efe5d6;
    opacity: 0.6;
    cursor: pointer;
    overflow: hidden;
    transition: opacity 0.2s, border-color 0.2s;
  }
  .peek__thumb:hover { opacity: 0.9; }
  .peek__thumb--on { opacity: 1; border-color: #c65f3c; }
  .peek__thumb :global(.peek__thumb-img) { width: 100%; height: 100%; object-fit: cover; }

  .peek__text { padding: 14px 16px 18px; }
  .peek__meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0 0 6px;
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #7c6554;
  }
  .peek__status--available { color: #4d6b3a; }
  .peek__status--reserved { color: #a2742a; }
  .peek__status--sold { color: #8a7566; }
  .peek__name {
    margin: 0 0 10px;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 24px;
    line-height: 1.15;
    color: #34251c;
  }
  .peek__desc {
    margin: 0 0 14px;
    font-size: 14px;
    line-height: 1.6;
    color: #4a392e;
    white-space: pre-line;
  }
  .peek__facts { display: grid; gap: 8px; margin: 0; }
  .peek__facts > div { display: grid; grid-template-columns: 92px 1fr; gap: 10px; font-size: 13px; }
  .peek__facts dt { color: #8a7566; letter-spacing: 0.04em; }
  .peek__facts dd { margin: 0; color: #34251c; }

  .peek__foot {
    display: grid;
    gap: 8px;
    padding: 10px 12px;
    border-top: 1px solid rgba(216, 198, 177, 0.7);
    background: #f8f1e7;
  }
  .peek__foot-row { display: flex; gap: 8px; }
  .peek__primary {
    flex: 1;
    min-height: 44px;
    padding: 0 18px;
    border: 1px solid #c65f3c;
    border-radius: 2px;
    background: #c65f3c;
    color: #fff9f0;
    font-size: 12px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    cursor: pointer;
    transition: background 0.2s, border-color 0.2s;
  }
  .peek__primary:hover { background: #a94d2d; border-color: #a94d2d; }
  .peek__primary--quiet { background: transparent; color: #6f3b24; border-color: rgba(111, 59, 36, 0.45); }
  .peek__primary--quiet:hover { background: rgba(198, 95, 60, 0.1); border-color: #c65f3c; }
  .peek__more { display: flex; align-items: center; gap: 8px; }
  .peek__more .peek__page { margin-right: auto; }
  .peek__ask-btn {
    height: 34px;
    padding: 0 12px;
    border: 1px solid rgba(52, 37, 28, 0.16);
    border-radius: 999px;
    background: transparent;
    color: #5f4636;
    font-size: 11px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    cursor: pointer;
    transition: background 0.2s, border-color 0.2s;
  }
  .peek__ask-btn:hover, .peek__ask-btn--on { background: rgba(52, 37, 28, 0.06); border-color: rgba(52, 37, 28, 0.35); }
  .peek__label { margin: 0 0 8px; font-size: 11px; letter-spacing: 0.12em; text-transform: uppercase; color: #8a7566; }
  .peek__similar { margin: 0 0 16px; }
  .peek__similar-row { display: flex; gap: 8px; }
  .peek__sim { flex: none; width: 64px; height: 80px; padding: 0; border: 1px solid transparent; border-radius: 2px; overflow: hidden; background: #efe5d6; cursor: pointer; transition: border-color 0.2s; }
  .peek__sim:hover { border-color: #c65f3c; }
  .peek__sim :global(.peek__sim-img) { width: 100%; height: 100%; object-fit: cover; object-position: 50% 12%; }
  .peek__ask { padding: 0 16px 16px; }
  .peek__page {
    text-align: left;
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #7c6554;
    padding: 4px 0;
  }
  .peek__page:hover { color: #34251c; }

  @media (prefers-reduced-motion: reduce) {
    .peek__icon, .peek__primary, .peek__thumb { transition: none; }
  }
</style>
