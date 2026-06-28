<script lang="ts">
  import { getContext } from 'svelte';
  import { fade } from 'svelte/transition';
  import BrassLens from '$lib/components/BrassLens.svelte';
  import LivingDaguerreotype from '$lib/components/LivingDaguerreotype.svelte';
  import RakingLight from '$lib/components/RakingLight.svelte';
  import { t } from '$lib/i18n';

  const ctx = getContext<App.FigurineDetailContext>('figurine-detail');
</script>

<div
  class="gallery-layout"
  class:gallery-layout--solo={ctx.sortedImages.length <= 1}
  style={ctx.plateStyle}
>
  {#if ctx.sortedImages.length > 1}
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
            <img src={ctx.resolveUrl(img.thumbUrl ?? img.url)} alt="" class="thumb-v-img" loading="lazy" />
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
        {#if ctx.useRaking}
          <div class="image-layer">
            <RakingLight
              src={ctx.resolveUrl(ctx.currentImage?.url)}
              heightSrc={ctx.resolveUrl(ctx.currentImage?.depthUrl) || null}
              alt={ctx.currentImage?.altText ?? ctx.figurine.name}
              class="w-full h-full"
              onActivate={() => ctx.canOpenLightbox && ctx.openLightbox(ctx.activeImageIndex)}
            />
          </div>
        {:else if ctx.useDaguerreotype}
          <div class="image-layer">
            <LivingDaguerreotype
              src={ctx.resolveUrl(ctx.currentImage?.url)}
              depthSrc={ctx.resolveUrl(ctx.currentImage?.depthUrl) || null}
              intensity={ctx.currentImage?.parallaxIntensity ?? undefined}
              alt={ctx.currentImage?.altText ?? ctx.figurine.name}
              class="w-full h-full"
              onActivate={() => ctx.canOpenLightbox && ctx.openLightbox(ctx.activeImageIndex)}
            />
          </div>
        {:else}
          {#key ctx.currentImage?.id}
            <div class="image-layer" transition:fade={{ duration: 220 }}>
              <BrassLens
                src={ctx.resolveUrl(ctx.currentImage?.url)}
                alt={ctx.currentImage?.altText ?? ctx.figurine.name}
                class="w-full h-full"
                imageFit={ctx.currentImageFit}
                objectPosition="center center"
                lensEnabled={ctx.isLensEnabled}
                onOpenLightbox={() => ctx.canOpenLightbox && ctx.openLightbox(ctx.activeImageIndex)}
                onSwipeLeft={() => ctx.sortedImages.length > 1 && ctx.selectImage(ctx.activeImageIndex + 1)}
                onSwipeRight={() => ctx.sortedImages.length > 1 && ctx.selectImage(ctx.activeImageIndex - 1)}
              />
            </div>
          {/key}
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
            <span class="img-counter-type">{ctx.imageTypeLabel(ctx.currentImage?.imageType)}</span>
            <span class="img-counter-num">{ctx.activeImageIndex + 1}<span class="img-counter-sep">/</span>{ctx.sortedImages.length}</span>
          </div>
        {/if}

        <button
          type="button"
          class="gallery-heart"
          class:gallery-heart--saved={ctx.isSaved}
          onclick={ctx.toggleSaved}
          aria-label={ctx.isSaved ? $t('cardSaved') : $t('cardSave')}
          title={ctx.isSaved ? $t('cardSaved') : $t('cardSave')}
          aria-pressed={ctx.isSaved}
        >
          <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <path
              d="M7 12.5C7 12.5 1 8.5 1 4.5C1 2.5 2.5 1 4.5 1C5.5 1 6.5 1.8 7 3C7.5 1.8 8.5 1 9.5 1C11.5 1 13 2.5 13 4.5C13 8.5 7 12.5 7 12.5Z"
              fill={ctx.isSaved ? 'currentColor' : 'none'}
              stroke="currentColor"
              stroke-width="1.15"
              stroke-linejoin="round"
            />
          </svg>
        </button>

        <button
          type="button"
          class="gallery-lens"
          class:gallery-lens--active={ctx.isLensEnabled}
          onclick={ctx.toggleLens}
          aria-label={ctx.isLensEnabled ? $t('detailImageLensOff') : $t('detailImageLensOn')}
          title={ctx.isLensEnabled ? $t('detailImageLensOff') : $t('detailImageLensOn')}
          aria-pressed={ctx.isLensEnabled}
        >
          <svg width="15" height="15" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" aria-hidden="true">
            <circle cx="6" cy="6" r="3.7" />
            <path d="M8.8 8.8L12 12" />
          </svg>
        </button>

        {#if ctx.showRakingButton}
          <button
            type="button"
            class="gallery-rake"
            class:gallery-rake--active={ctx.isRakingEnabled}
            onclick={ctx.toggleRaking}
            aria-label={ctx.isRakingEnabled ? $t('detailImageRakeOff') : $t('detailImageRakeOn')}
            title={ctx.isRakingEnabled ? $t('detailImageRakeOff') : $t('detailImageRakeOn')}
            aria-pressed={ctx.isRakingEnabled}
          >
            <svg width="15" height="15" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <circle cx="3.2" cy="3.2" r="1.5" />
              <path d="M4.3 4.3L11 11" />
              <path d="M2 11.4h10" />
              <path d="M5.2 11.4l1.1-2M7.7 11.4l1.1-2" />
            </svg>
          </button>
        {/if}

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

        {#if ctx.canOpenLightbox}
          <button
            type="button"
            onclick={() => ctx.openLightbox(ctx.activeImageIndex)}
            class="expand-btn"
            aria-label={$t('figurineFullscreen')}
          >
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
              <path d="M1 4V1h3M6 1h3v3M9 6v3H6M4 9H1V6"/>
            </svg>
            {$t('figurineFullscreen')}
          </button>
        {/if}

        <div class="image-vignette"></div>
      </div>
    </div>

    {#if ctx.currentImage}
      <figcaption class="plate-caption">
        <span class="plate-caption-label">{ctx.imageTypeLabel(ctx.currentImage.imageType)}</span>
        <span class="plate-caption-note">{ctx.imageRoleNote(ctx.currentImage.imageType)}</span>
        {#if ctx.hasText(ctx.figurine.dimensions)}
          <span class="plate-caption-dim">{ctx.figurine.dimensions}</span>
        {/if}
      </figcaption>
    {/if}
  </figure>
</div>
