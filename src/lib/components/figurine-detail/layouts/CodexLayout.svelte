<script lang="ts">
  import { getContext } from 'svelte';
  import { figurineHref } from '$lib/figurineHref';
  import { t } from '$lib/i18n';
  import { resolveWebpUrl } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import GalleryPlateActions from '$lib/components/figurine-detail/GalleryPlateActions.svelte';
  import BrassLens from '$lib/components/BrassLens.svelte';
  import FigurineStatusPanel from '$lib/components/FigurineStatusPanel.svelte';
  import FigurineClaimRow from '$lib/components/FigurineClaimRow.svelte';
  import BecomingReveal from '$lib/components/BecomingReveal.svelte';
  import SecretText from '$lib/components/SecretText.svelte';
  import ShowingsTimeline from '$lib/components/ShowingsTimeline.svelte';
  import FigurineComments from '$lib/components/FigurineComments.svelte';
  import '$lib/styles/figurine-detail/layout-codex.css';

  import { computeSectionOrderStyle, isBlockVisible, computeBlockStyle, computeElementStyle } from '$lib/components/figurine-detail/display-config';

  const ctx = getContext<App.FigurineDetailContext>('figurine-detail');
  let sectionStyle = $derived(computeSectionOrderStyle(ctx.displayConfig));

  let historyRef = $state<HTMLElement | null>(null);
  let inkReady = $state(false);

  $effect(() => {
    if (!historyRef || inkReady) return;
    if (typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      inkReady = true; return;
    }
    const io = new IntersectionObserver(entries => {
      if (entries[0].isIntersecting) { inkReady = true; io.disconnect(); }
    }, { threshold: 0.05 });
    io.observe(historyRef);
    return () => io.disconnect();
  });

  function buildInkHtml(text: string): string {
    return text.split(/\s+/).filter(Boolean).map((word, i) => {
      const esc = word.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
      return `<span class="ink-word" style="animation-delay:${Math.min(i,80)*25}ms">${esc}</span>`;
    }).join(' ');
  }

  let videoRef = $state<HTMLVideoElement | null>(null);
  function toggleFullscreen() {
    if (!videoRef) return;
    document.fullscreenElement ? document.exitFullscreen() : videoRef.requestFullscreen().catch(() => {});
  }

  let titleClass = $derived(
    ctx.figurine.name.length > 60 ? 'figurine-title--long'
    : ctx.figurine.name.length > 30 ? 'figurine-title--medium'
    : ''
  );
</script>

<div class="cx-root">

  <!-- ── Рубрика: тип и год ── -->
  <div class="cx-rubric">
    <span class="cx-rubric-kind">{$t('detailKind')}</span>
    {#if ctx.figurine.year}
      <span class="cx-rubric-sep">·</span>
      <span class="cx-rubric-year">{ctx.toRoman(ctx.figurine.year)}</span>
    {/if}
  </div>

  <!-- ── Заголовок ── -->
  <h1 class="figurine-title {titleClass} cx-title" style={computeElementStyle(ctx.displayConfig, 'name')}>{ctx.figurine.name}</h1>

  <div class="cx-head-rule" aria-hidden="true"></div>

  <!-- ── Открытие: двухколонный grid (текст | пластина) ── -->
  <div class="cx-opening">

    <!-- Левая колонка: текст, атрибуты, миниатюры -->
    <div class="cx-opening-left">
      {#if ctx.hasText(ctx.figurine.shortText)}
        <p class="cx-lore" style={computeElementStyle(ctx.displayConfig, 'shortText')}>{ctx.figurine.shortText}</p>
      {/if}

      {#if ctx.hasAttributesSection}
        <dl class="cx-ledger" aria-label={$t('figurineAttributes')}>
          {#each ctx.attributes as attr (attr.kind)}
            <div class="cx-ledger-item">
              <dt class="cx-ledger-label">{attr.label}</dt>
              <span class="cx-ledger-dots" aria-hidden="true"></span>
              <dd class="cx-ledger-value">{attr.value}</dd>
            </div>
          {/each}
        </dl>
      {/if}

      {#if ctx.sortedImages.length > 1}
        <nav class="cx-thumbs" aria-label={$t('figurineShowView')}>
          {#each ctx.sortedImages as img, i}
            <button
              type="button"
              class="cx-thumb {ctx.activeImageIndex === i ? 'cx-thumb--on' : ''}"
              onclick={() => ctx.selectImage(i)}
              aria-label="{ctx.imageTypeLabel(img.imageType)}: {ctx.imageRoleNote(img.imageType)}"
            >
              <picture>
                <source type="image/webp" srcset={resolveWebpUrl(img.thumbUrl ?? img.url) ?? undefined} />
                <img
                  src={ctx.resolveUrl(img.thumbUrl ?? img.url)}
                  alt={ctx.altTextFor(img)}
                  class="cx-thumb-img"
                  loading="lazy"
                  decoding="async"
                  style={img.focalX != null && img.focalY != null
                    ? `object-position: ${img.focalX * 100}% ${img.focalY * 100}%;`
                    : undefined}
                />
              </picture>
            </button>
          {/each}
        </nav>
      {/if}

      {#if ctx.hasText(ctx.figurine.secretText) && ctx.isCandleLit}
        <SecretText text={ctx.figurine.secretText} isCandleLit={ctx.isCandleLit} />
      {/if}

      <!-- Статус и запись — прижаты ко дну левой колонки -->
      <div class="cx-section-rule" aria-hidden="true"></div>

      <FigurineStatusPanel
        figurine={ctx.figurine}
        id={ctx.id}
        hasActiveShowing={ctx.hasActiveShowing}
        nextAvailableDate={ctx.nextAvailableDate}
        scheduleLoadFailed={ctx.scheduleLoadFailed}
        onOpenModal={ctx.openRequestModal}
        analyticsClient={ctx.analyticsClient}
        queueJoin={ctx.queueJoin}
        notifyJoin={ctx.notifyJoin}
      />

      {#if ctx.canShowPersonalRecord}
        <details
          class="entry-record"
          class:entry-record--empty={!ctx.hasPersonalRecord}
          ontoggle={ctx.handlePersonalRecordToggle}
        >
          <summary>{ctx.hasPersonalRecord ? $t('detailYourRecord') : $t('claimHaveCode')}</summary>
          <div class="entry-record-body">
            {#if ctx.hasClaimRecords}
              <section class="entry-record-section entry-record-section--claims" aria-label={$t('detailYourRecord')}>
                <div class="claims-panel {ctx.cs.claims.some(c => c.status === 'confirmed') ? 'claims-panel--has-confirmed' : ''}">
                  <div class="claims-panel-header">
                    {ctx.cs.claims.some(c => c.status === 'confirmed') ? $t('claimsYours') : $t('claimsPending')}
                  </div>
                  {#if ctx.cs.cancelledTokens.size > 0 && ctx.cs.claims.length === 0}
                    <div class="cp-row cp-row--done"><p class="cp-done">{$t('claimCancelDone')}</p></div>
                  {/if}
                  {#each ctx.cs.claims as c (c.token)}
                    <FigurineClaimRow
                      claim={c} isLoggedIn={authStore.isLoggedIn}
                      isCancelling={ctx.cs.cancellingToken === c.token}
                      error={ctx.cs.claimErrors[c.token]}
                      formatDate={ctx.fmtDate}
                      onCancel={(claim) => ctx.cs.cancel(claim)}
                    />
                  {/each}
                </div>
              </section>
            {/if}
            <section class="entry-record-section entry-record-section--lookup" aria-label={$t('claimHaveCode')}>
              <div class="claim-lookup">
                {#if !ctx.cs.showTokenForm}
                  <button type="button" onclick={ctx.openClaimLookup} class="claim-lookup-link">{$t('claimHaveCode')}</button>
                {:else}
                  <div class="claim-lookup-form">
                    <input type="text" bind:value={ctx.cs.tokenInput} placeholder="XXXX-XXXX" maxlength="9"
                      id="claim-token" name="claim-token" autocomplete="off"
                      class="claim-lookup-input" oninput={() => { ctx.cs.tokenLookupInfo = null; ctx.cs.tokenLookupErr = ''; }} />
                    <button type="button" onclick={() => ctx.cs.lookupToken()} disabled={ctx.cs.tokenLooking} class="claim-lookup-btn">
                      {ctx.cs.tokenLooking ? '...' : $t('claimLookupBtn')}
                    </button>
                    <button type="button" onclick={ctx.closeClaimLookup} class="claim-lookup-close" aria-label={$t('lightboxClose')}>
                      <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                        <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke-linecap="round"/>
                      </svg>
                    </button>
                  </div>
                  {#if ctx.cs.tokenLookupErr}<p class="claim-err">{ctx.cs.tokenLookupErr}</p>{/if}
                  {#if ctx.cs.tokenLookupInfo}
                    <div class="claim-lookup-result">
                      <p class="claim-dates">{ctx.fmtDate(ctx.cs.tokenLookupInfo.startsAt)} — {ctx.fmtDate(ctx.cs.tokenLookupInfo.endsAt)}</p>
                      {#if ctx.cs.tokenLookupInfo.status === 'pending'}
                        <button type="button" onclick={() => ctx.cs.cancelFromLookup()} disabled={ctx.cs.lookupCancelling} class="claim-cancel-btn">
                          {ctx.cs.lookupCancelling ? $t('claimCancelling') : $t('claimCancelBtn')}
                        </button>
                      {:else}
                        <p class="claim-status">{$t('claimStatus')}: {ctx.lookupStatusLabel(ctx.cs.tokenLookupInfo.status)}</p>
                      {/if}
                    </div>
                  {/if}
                {/if}
              </div>
            </section>
          </div>
        </details>
      {/if}
    </div>

    <!-- Правая колонка: иллюминированная пластина -->
    <div
      class="cx-illum"
      data-figurine-plate
      style="view-transition-name: {ctx.viewTransitionName}; {ctx.plateStyle}"
    >
      {#if ctx.useRaking}
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
            sizes="(min-width: 900px) 50vw, 100vw"
            alt={ctx.altTextFor(ctx.currentImage)}
            class="w-full h-full"
            imageFit="cover"
            objectPosition={ctx.currentImage?.focalX != null && ctx.currentImage?.focalY != null
              ? `${ctx.currentImage.focalX * 100}% ${ctx.currentImage.focalY * 100}%`
              : 'center top'}
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

      {#if ctx.sortedImages.length > 1}
        <div class="img-counter" aria-hidden="true">
          <span class="img-counter-num">{ctx.activeImageIndex + 1}<span class="img-counter-sep">/</span>{ctx.sortedImages.length}</span>
        </div>
      {/if}

      <GalleryPlateActions />

      <div class="image-vignette"></div>
    </div>
    <!-- /cx-illum -->

    {#if ctx.currentImage}
      <!-- Per-photo museum tag: which shot ("Detail — Craft close-up") this is. Name/
           material already sit in the ledger above (.cx-ledger) — this only adds what
           that static block can't: a caption that changes as the visitor browses. -->
      <div class="plate-caption">
        <span class="plate-caption-label">{ctx.imageTypeLabel(ctx.currentImage.imageType)}</span>
        <span class="plate-caption-note">{ctx.imageRoleNote(ctx.currentImage.imageType)}</span>
      </div>
    {/if}

  </div>
  <!-- /cx-opening -->

  <div class="dc-sections" style={sectionStyle}>
  <!-- ── История ── -->
  <div class="dc-block--description" class:dc-block--hidden={!isBlockVisible(ctx.displayConfig, 'description')} style={computeBlockStyle(ctx.displayConfig, 'description')}>
  {#if ctx.hasHistorySection}
    <div class="cx-div" aria-hidden="true"><span>◆</span></div>
    <p bind:this={historyRef} class="history-body drop-cap">
      {#if inkReady}
        {@html buildInkHtml(ctx.figurine.fullDescription ?? '')}
      {:else}
        {ctx.figurine.fullDescription}
      {/if}
    </p>
  {/if}
  </div>
  <div class="dc-block--making" class:dc-block--hidden={!isBlockVisible(ctx.displayConfig, 'making')} style={computeBlockStyle(ctx.displayConfig, 'making')}>
  <!-- ── Процесс ── -->
  {#if ctx.hasMakingSection}
    <div class="cx-div" aria-hidden="true"><span>◆</span></div>

    <div class="cx-making {ctx.figurine.status === 'in_progress' ? 'cx-making--live' : ''}">
      <p class="cx-making-kicker">
        {#if ctx.figurine.status === 'in_progress'}
          <span class="making-live" aria-hidden="true"></span>{$t('detailMakingProgressKicker')}
        {:else}
          {$t('detailMakingRecordKicker')}
        {/if}
      </p>

      {#if ctx.hasBecoming}
        <BecomingReveal
          beforeSrc={ctx.becomingBefore} afterSrc={ctx.becomingAfter}
          beforeLabel={ctx.processStepLabel(ctx.firstStep?.stepType)}
          afterLabel={$t('detailBecomingFinished')} hint={$t('detailBecomingHint')}
        />
      {:else}
        <div class="cx-making-strip" aria-label={$t('detailMakingRecordTitle')}>
          {#each ctx.visibleProcessSteps.slice(0, 6) as step, i (step.id)}
            <article class="cx-making-card">
              <div class="cx-making-img-wrap">
                {#if ctx.hasText(step.imageUrl)}
                  <img src={ctx.resolveUrl(step.imageUrl)} alt="" class="cx-making-img" loading="lazy" />
                {:else}
                  <div class="cx-making-placeholder" aria-hidden="true"></div>
                {/if}
                <span class="cx-making-num">{String(i + 1).padStart(2, '0')}</span>
              </div>
            </article>
          {/each}
        </div>
      {/if}

      {#if ctx.showMirrorLink}
        <button type="button" onclick={ctx.openGrimoire} class="mirror-link" aria-expanded={ctx.isGrimoireOpen}>
          <span class="mirror-link-mark" aria-hidden="true"></span>
          <span class="mirror-link-label">{$t('figurineGrimoire')}</span>
          <span class="mirror-link-count">{ctx.visibleProcessSteps.length} {$t('figurineGrimoireSub')}</span>
          <svg class="mirror-link-arrow" class:mirror-link-arrow--open={ctx.isGrimoireOpen}
            width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
            <path d="M3 8h10M9 4l4 4-4 4"/>
          </svg>
        </button>
      {/if}
    </div>
  {/if}
  </div>
  <div class="dc-block--video" class:dc-block--hidden={!isBlockVisible(ctx.displayConfig, 'video')} style={computeBlockStyle(ctx.displayConfig, 'video')}>
  <!-- ── Видео ── -->
  {#if ctx.hasVideoSection}
    <div class="cx-div" aria-hidden="true"><span>◆</span></div>
    <section class="video-section">
      <header class="section-row">
        <span class="sec-label">{$t('figurineVideo')}</span>
        <div class="sec-rule" aria-hidden="true"></div>
      </header>
      <div class="video-wrap"><div class="video-frame"><div class="video-stage">
        <video bind:this={videoRef} controls class="video-el"
          poster={ctx.resolveUrl(ctx.currentImage?.url)} preload="metadata">
          <source src={ctx.resolveUrl(ctx.figurine.videoUrl)} type="video/mp4" />
          {$t('figurineBrowserNoVideo')}
        </video>
        <button type="button" onclick={toggleFullscreen} class="video-fs-btn" aria-label={$t('figurineFullscreen')}>
          <svg width="14" height="14" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
            <path d="M1 4V1h3M6 1h3v3M9 6v3H6M4 9H1V6"/>
          </svg>
        </button>
      </div></div></div>
      <p class="video-caption text-label">{$t('figurineVideoFilm')}{ctx.id.slice(-3)}</p>
    </section>
  {/if}
  </div>
  <div class="dc-block--showings" class:dc-block--hidden={!isBlockVisible(ctx.displayConfig, 'showings')} style={computeBlockStyle(ctx.displayConfig, 'showings')}>
  <!-- ── Расписание ── -->
  {#if ctx.hasFactsSection && ctx.hasScheduleSection}
    <div class="cx-div" aria-hidden="true"><span>◆</span></div>
    <section id="presence" class="presence-section">
      <header class="section-row">
        <span class="sec-label">{$t('detailPresenceLabel')}</span>
        <div class="sec-rule" aria-hidden="true"></div>
      </header>
      <ShowingsTimeline schedule={ctx.figurineSchedule} />
      {#if ctx.hasActiveShowing}
        <p class="presence-note">{$t('figurineTransferBlocked')}</p>
      {/if}
    </section>
  {/if}
  </div>
  <div class="dc-block--related" class:dc-block--hidden={!isBlockVisible(ctx.displayConfig, 'related')} style={computeBlockStyle(ctx.displayConfig, 'related')}>
  <!-- ── Связанные работы ── -->
  {#if ctx.visibleRelatedItems.length > 0}
    <div class="cx-div" aria-hidden="true"><span>◆</span></div>
    <section id="related-works">
      <p class="cx-related-label">{$t('figurineRelated')}</p>
      <div class="cx-related-strip">
        {#each ctx.visibleRelatedItems as item}
          {@const relatedImageUrl = ctx.resolveUrl(item.faceImageUrl ?? item.thumbUrl)}
          <a href={figurineHref(item)} onclick={() => ctx.analyticsClient?.cta('related_figurine')}
            class="cx-related-card" data-sveltekit-preload-data="hover">
            <div class="cx-related-img-wrap">
              {#if relatedImageUrl}
                <img src={relatedImageUrl} alt={item.name} class="cx-related-img" loading="lazy" />
              {:else}
                <div class="cx-related-placeholder" aria-hidden="true"><span>{item.name.slice(0,1)}</span></div>
              {/if}
              <span class="cx-related-status cx-related-status--{item.status}">{ctx.statusLabel(item.status)}</span>
            </div>
            <p class="cx-related-name">{item.name}</p>
          </a>
        {/each}
      </div>
    </section>
  {/if}
  </div>
  <div class="dc-block--comments" class:dc-block--hidden={!isBlockVisible(ctx.displayConfig, 'comments')} style={computeBlockStyle(ctx.displayConfig, 'comments')}>
    <div class="cx-div" aria-hidden="true"><span>◆</span></div>
    <FigurineComments figurineId={ctx.id} />
  </div>
  </div>
</div>
