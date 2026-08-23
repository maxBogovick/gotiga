<script lang="ts">
  import { getContext } from 'svelte';
  import BrassLens from '$lib/components/BrassLens.svelte';
  import GalleryPlateActions from '$lib/components/figurine-detail/GalleryPlateActions.svelte';
  import { resolveWebpUrl } from '$lib/api';
  import { t } from '$lib/i18n';

  const ctx = getContext<App.FigurineDetailContext>('figurine-detail');

  let {
    hideThumbs = false,
    hideCaption = false,
    quiet = false,
    aspect = '',
    imageFit = '',
  }: {
    hideThumbs?: boolean;
    hideCaption?: boolean;
    quiet?: boolean;
    aspect?: string;
    imageFit?: 'cover' | 'contain' | '';
  } = $props();

  let isNarrow = $state(false);
  let plateFit = $derived<'cover' | 'contain'>(
    isNarrow
      ? 'cover'
      : imageFit === 'cover' || imageFit === 'contain'
        ? imageFit
        : aspect
          ? 'contain'
          : ctx.currentImageFit
  );

  $effect(() => {
    if (typeof window === 'undefined') return;
    const mq = window.matchMedia('(max-width: 859px)');
    const sync = () => {
      isNarrow = mq.matches;
    };
    sync();
    mq.addEventListener('change', sync);
    return () => mq.removeEventListener('change', sync);
  });
</script>

<div
  class="gallery-layout"
  class:gallery-layout--solo={hideThumbs || ctx.sortedImages.length <= 1}
  class:gallery-layout--quiet={quiet}
  style={aspect ? `--viewer-aspect-ratio: ${aspect};` : ctx.plateStyle}
>
  {#if !hideThumbs && ctx.sortedImages.length > 1}
    <nav class="thumbs-strip" aria-label={$t('figurineShowView')}>
      {#each ctx.sortedImages as img, i}
        <button
          type="button"
          class="thumb-v {ctx.activeImageIndex === i ? 'thumb-v--active' : ''}"
          onclick={() => ctx.selectImage(i)}
          aria-label="{ctx.imageTypeLabel(img.imageType)}: {ctx.imageRoleNote(img.imageType)}"
          aria-current={ctx.activeImageIndex === i ? 'true' : undefined}
        >
          <span class="thumb-v-media">
            <picture>
              <source type="image/webp" srcset={resolveWebpUrl(img.thumbUrl ?? img.url) ?? undefined} />
              <img
                src={ctx.resolveUrl(img.thumbUrl ?? img.url)}
                alt={ctx.altTextFor(img)}
                class="thumb-v-img"
                loading="lazy"
                decoding="async"
                style={img.focalX != null && img.focalY != null
                  ? `object-position: ${img.focalX * 100}% ${img.focalY * 100}%;`
                  : undefined}
              />
            </picture>
          </span>
          <span class="thumb-v-copy">
            <span class="thumb-v-label">{ctx.imageTypeLabel(img.imageType)}</span>
            <span class="thumb-v-note">{ctx.imageRoleNote(img.imageType)}</span>
          </span>
          <div class="thumb-v-bar" aria-hidden="true"></div>
        </button>
      {/each}
    </nav>
  {/if}

  <figure class="image-col">
    <div class="image-frame">
      <div
        class="image-stage"
        class:image-stage--detail={ctx.imageViewMode === 'detail'}
        data-figurine-plate
        style="view-transition-name: {ctx.viewTransitionName};"
      >
        {#if !quiet && ctx.useRaking}
          <div class="image-layer">
            {#await import('$lib/components/RakingLight.svelte') then { default: RakingLight }}
              <RakingLight
                src={ctx.resolveUrl(ctx.currentImage?.url)}
                heightSrc={ctx.resolveUrl(ctx.currentImage?.depthUrl) || null}
                alt={ctx.altTextFor(ctx.currentImage)}
                class="w-full h-full"
                onActivate={() => ctx.canOpenLightbox && ctx.openLightbox(ctx.activeImageIndex)}
              />
            {/await}
          </div>
        {:else}
          <div class="image-layer">
            <BrassLens
              src={ctx.resolveUrl(ctx.currentImage?.url)}
              thumbSrc={ctx.resolveUrl(ctx.currentImage?.thumbUrl)}
              sizes={aspect ? '(min-width: 1025px) 52vw, (min-width: 860px) 42vw, 92vw' : undefined}
              alt={ctx.altTextFor(ctx.currentImage)}
              class="w-full h-full"
              imageFit={plateFit}
              objectPosition={ctx.currentImage?.focalX != null && ctx.currentImage?.focalY != null
                ? `${ctx.currentImage.focalX * 100}% ${ctx.currentImage.focalY * 100}%`
                : 'center center'}
              lensEnabled={ctx.isLensEnabled}
              onSwipeLeft={() => ctx.sortedImages.length > 1 && ctx.selectImage(ctx.activeImageIndex + 1)}
              onSwipeRight={() => ctx.sortedImages.length > 1 && ctx.selectImage(ctx.activeImageIndex - 1)}
            />
            {#if ctx.useDaguerreotype}
              {#await import('$lib/components/LivingDaguerreotype.svelte') then { default: LivingDaguerreotype }}
                <LivingDaguerreotype
                  src={ctx.resolveUrl(ctx.currentImage?.url)}
                  depthSrc={ctx.resolveUrl(ctx.currentImage?.depthUrl) || null}
                  intensity={ctx.currentImage?.parallaxIntensity ?? undefined}
                  alt={ctx.altTextFor(ctx.currentImage)}
                  class="w-full h-full"
                  imageFit={plateFit}
                  objectPosition={ctx.currentImage?.focalX != null && ctx.currentImage?.focalY != null
                    ? `${ctx.currentImage.focalX * 100}% ${ctx.currentImage.focalY * 100}%`
                    : 'center center'}
                  onActivate={() => ctx.canOpenLightbox && ctx.openLightbox(ctx.activeImageIndex)}
                />
              {/await}
            {/if}
          </div>
        {/if}

        {#if ctx.sortedImages.length > 1}
          <button
            type="button"
            class="gallery-nav-prev"
            onclick={() => ctx.selectImage(ctx.activeImageIndex - 1)}
            disabled={ctx.activeImageIndex === 0}
            aria-label={$t('detailImagePrevPhoto')}
            title={$t('detailImagePrevPhoto')}
          >
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
              <path d="M8 2L4 6l4 4"/>
            </svg>
          </button>
          <button
            type="button"
            class="gallery-nav-next"
            onclick={() => ctx.selectImage(ctx.activeImageIndex + 1)}
            disabled={ctx.activeImageIndex === ctx.sortedImages.length - 1}
            aria-label={$t('detailImageNextPhoto')}
            title={$t('detailImageNextPhoto')}
          >
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
              <path d="M4 2l4 4-4 4"/>
            </svg>
          </button>
        {/if}

        {#if ctx.lastBleed}
          <div
            class="leaf-bleed leaf-bleed--{ctx.lastBleed.dir}"
            class:leaf-bleed--on={ctx.bleedDir}
            aria-hidden="true"
            style="background-image: url('{ctx.lastBleed.img}');"
          ></div>
        {/if}

        {#if ctx.sortedImages.length > 1}
          <div class="img-counter" aria-hidden="true">
            <span class="img-counter-num">{ctx.activeImageIndex + 1}<span class="img-counter-sep">/</span>{ctx.sortedImages.length}</span>
          </div>
          <span class="sr-only" aria-live="polite">
            {ctx.imageTypeLabel(ctx.currentImage?.imageType)} — {ctx.activeImageIndex + 1} / {ctx.sortedImages.length}
          </span>
        {/if}

        {#if ctx.houseFavorite}
          <p class="notice-badge notice-badge--favorite">
            <svg width="11" height="11" viewBox="0 0 14 14" fill="none" aria-hidden="true">
              <path d="M2 12C2 7 4 3 4 3M2 12L4 9.5M2 12L4.5 11" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
              <path d="M12 12C12 7 10 3 10 3M12 12L10 9.5M12 12L9.5 11" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
              <circle cx="7" cy="3" r="1.1" fill="currentColor"/>
            </svg>
            {$t('houseFavoriteBadge')}
          </p>
        {:else if ctx.noticedByOthers}
          <p class="notice-badge notice-badge--spark">
            <span class="notice-spark-dot" aria-hidden="true">
              <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 0C12 6.6 6.6 12 0 12C6.6 12 12 17.4 12 24C12 17.4 17.4 12 24 12C17.4 12 12 6.6 12 0Z"/></svg>
            </span>
            {$t('figurineNoticedByOthers')}
          </p>
        {/if}

        <GalleryPlateActions {quiet} />

        {#if !quiet}
        <div class="image-view-tools" aria-label={$t('detailImageViewMode')}>
          <button
            type="button"
            class="image-view-tool {ctx.imageViewMode === 'fit' ? 'image-view-tool--active' : ''}"
            onclick={() => ctx.setImageViewMode('fit')}
            aria-pressed={ctx.imageViewMode === 'fit'}
          >
            {$t('detailImageFit')}
          </button>
          <button
            type="button"
            class="image-view-tool {ctx.imageViewMode === 'detail' ? 'image-view-tool--active' : ''}"
            onclick={() => ctx.setImageViewMode('detail')}
            aria-pressed={ctx.imageViewMode === 'detail'}
          >
            {$t('detailImageDetailView')}
          </button>
        </div>
        {/if}

        <div class="image-vignette"></div>
      </div>
    </div>

    {#if ctx.currentImage && !hideCaption}
      <!-- Museum object label: title first, so the photo is never just an
           unlabelled picture to a crawler reading the page's visible text —
           Google's image-SEO guidance weighs an on-page caption more heavily
           than the (also present) alt attribute, since visitors read it too. -->
      <figcaption class="plate-caption">
        <span class="plate-caption-title">{ctx.figurine.name}</span>
        <span class="plate-caption-label">{ctx.imageTypeLabel(ctx.currentImage.imageType)}</span>
        <span class="plate-caption-note">{ctx.imageRoleNote(ctx.currentImage.imageType)}</span>
        {#if ctx.hasText(ctx.figurine.material)}
          <span class="plate-caption-material">{ctx.figurine.material}</span>
        {/if}
        {#if ctx.hasText(ctx.figurine.dimensions)}
          <span class="plate-caption-dim">{ctx.figurine.dimensions}</span>
        {/if}
      </figcaption>
    {/if}
  </figure>
</div>
