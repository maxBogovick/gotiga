<script lang="ts">
  import { getContext } from 'svelte';
  import { figurineHref } from '$lib/figurineHref';
  import { resolveWebpUrl } from '$lib/api';
  import { t } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';
  import FigurineImageViewer from '../FigurineImageViewer.svelte';
  import FigurineStatusPanel from '$lib/components/FigurineStatusPanel.svelte';
  import FigurineClaimRow from '$lib/components/FigurineClaimRow.svelte';
  import BecomingReveal from '$lib/components/BecomingReveal.svelte';
  import SecretText from '$lib/components/SecretText.svelte';
  import FontSwitcher from '$lib/components/FontSwitcher.svelte';
  import ShowingsTimeline from '$lib/components/ShowingsTimeline.svelte';
  import FigurineComments from '$lib/components/FigurineComments.svelte';
  import CatalogGlyph from '$lib/components/figurine-detail/CatalogGlyph.svelte';
  import {
    enabledCustomLines,
    isCatalogKeyOn,
    parseCatalogLists,
  } from '$lib/catalog-lists';
  import '$lib/styles/figurine-detail/layout-catalog.css';

  import { computeSectionOrderStyle, isBlockVisible, computeBlockStyle, computeElementStyle } from '$lib/components/figurine-detail/display-config';

  const ctx = getContext<App.FigurineDetailContext>('figurine-detail');
  let sectionStyle = $derived(computeSectionOrderStyle(ctx.displayConfig));

  function registerGallery(el: HTMLElement) {
    ctx.setGalleryEl(el);
    return { destroy() { ctx.setGalleryEl(undefined); } };
  }

  let historyRef = $state<HTMLElement | null>(null);
  let inkReady = $state(false);

  $effect(() => {
    if (!historyRef || inkReady) return;
    if (typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      inkReady = true; return;
    }
    const io = new IntersectionObserver((entries) => {
      if (entries[0].isIntersecting) { inkReady = true; io.disconnect(); }
    }, { threshold: 0.05 });
    io.observe(historyRef);
    return () => io.disconnect();
  });

  function buildInkHtml(text: string): string {
    const words = text.split(/\s+/).filter(Boolean);
    return words.map((word, i) => {
      const esc = word.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
      const delay = Math.min(i, 80) * 25;
      return `<span class="ink-word" style="animation-delay:${delay}ms">${esc}</span>`;
    }).join(' ');
  }

  let videoRef = $state<HTMLVideoElement | null>(null);

  function toggleFullscreen() {
    if (!videoRef) return;
    document.fullscreenElement ? document.exitFullscreen() : videoRef.requestFullscreen().catch(() => {});
  }

  let titleClass = $derived(
    ctx.figurine.name.length > 60 ? 'catalog-title--long'
    : ctx.figurine.name.length > 30 ? 'catalog-title--medium'
    : ''
  );

  let heroParagraphs = $derived.by(() => splitProse(ctx.figurine.shortText ?? ''));
  let showHistory = $derived(
    ctx.hasHistorySection && isBlockVisible(ctx.displayConfig, 'description')
  );

  let filmEl = $state<HTMLElement | null>(null);

  $effect(() => {
    const index = ctx.activeImageIndex;
    const root = filmEl;
    if (!root) return;
    const thumb = root.querySelector<HTMLElement>(`[data-film-index="${index}"]`);
    if (!thumb) return;
    const left = thumb.offsetLeft - (root.clientWidth - thumb.offsetWidth) / 2;
    root.scrollTo({ left: Math.max(0, left), behavior: 'smooth' });
  });

  function splitProse(text: string): string[] {
    const raw = text.trim();
    if (!raw) return [];
    const parts = raw.split(/\n{2,}/).map((s) => s.trim()).filter(Boolean);
    if (parts.length >= 2) return parts.slice(0, 3);
    const one = parts[0] ?? '';
    if (one.length < 320) return [one];
    const mid = Math.floor(one.length / 2);
    const window = one.slice(Math.max(80, mid - 80), mid + 80);
    const relative = window.search(/\.\s+/);
    if (relative < 0) return [one];
    const cut = Math.max(80, mid - 80) + relative + 1;
    return [one.slice(0, cut).trim(), one.slice(cut).trim()].filter(Boolean);
  }

  type CatalogLine = { icon: string; text: string };

  let catalogLists = $derived(parseCatalogLists(ctx.figurine.catalogLists));

  let featureLines = $derived.by(() => {
    const on = (key: string) => isCatalogKeyOn(catalogLists.featuresSelected, key);
    const lines: CatalogLine[] = [];
    if (on('unique')) {
      lines.push({ icon: 'star', text: $t('catalogFeatureUnique') });
    }
    if (on('material') && ctx.hasText(ctx.figurine.material)) {
      lines.push({ icon: 'face', text: `${$t('catalogFeatureMaterial')} ${ctx.figurine.material.trim()}` });
    }
    if (on('technique') && ctx.hasText(ctx.figurine.technique)) {
      lines.push({ icon: 'pencil', text: ctx.figurine.technique.trim() });
    } else if (on('handPainted')) {
      lines.push({ icon: 'pencil', text: $t('catalogFeatureHandPainted') });
    }
    if (on('handFinished')) {
      lines.push({ icon: 'needle', text: $t('catalogFeatureHandFinished') });
    }
    if (on('recorded') && (ctx.hasText(ctx.figurine.passportNumber) || ctx.hasText(ctx.figurine.authenticityNote))) {
      lines.push({ icon: 'lock', text: $t('catalogFeatureRecorded') });
    }
    if (on('included') && ctx.hasText(ctx.figurine.includedItems)) {
      const first = ctx.figurine.includedItems.trim().split(/\n/)[0]?.trim() ?? '';
      if (first) lines.push({ icon: 'gift', text: first });
    }
    if (on('quietRoom')) {
      lines.push({ icon: 'figure', text: $t('catalogFeatureQuietRoom') });
    }
    for (const line of enabledCustomLines(catalogLists.featuresCustom)) {
      lines.push({ icon: 'star', text: line.text.trim() });
    }
    return lines;
  });

  let perfectLines = $derived.by(() => {
    const on = (key: string) => isCatalogKeyOn(catalogLists.perfectSelected, key);
    const lines: CatalogLine[] = [];
    if (on('collectors')) lines.push({ icon: 'heart', text: $t('catalogPerfectCollectors') });
    if (on('cabinet')) lines.push({ icon: 'house', text: $t('catalogPerfectCabinet') });
    if (on('looking')) lines.push({ icon: 'spool', text: $t('catalogPerfectLooking') });
    if (on('closeWork')) lines.push({ icon: 'scissors', text: $t('catalogPerfectCloseWork') });
    if (on('display')) lines.push({ icon: 'frame', text: $t('catalogPerfectDisplay') });
    if (on('gift')) lines.push({ icon: 'gift', text: $t('catalogPerfectGift') });
    for (const line of enabledCustomLines(catalogLists.perfectCustom)) {
      lines.push({ icon: 'heart', text: line.text.trim() });
    }
    return lines;
  });

  let requestFacts = $derived<CatalogLine[]>([
    { icon: 'quill', text: $t('detailReplyWindow') },
    { icon: 'seal', text: $t('detailNoObligation') },
    { icon: 'crate', text: $t('detailPersonalTransfer') },
  ]);
</script>

<div class="catalog-root">
  <div class="catalog-leaf">
      <header class="catalog-head">
        <h1
          class="figurine-title catalog-title {titleClass}"
          style={computeElementStyle(ctx.displayConfig, 'name')}
        >
          <svg class="catalog-flourish" viewBox="0 0 48 16" fill="none" stroke="currentColor" stroke-width="1.15" stroke-linecap="round" aria-hidden="true">
            <path d="M46 8.2C36 8.2 33.5 3.2 27.5 3.2c-7.2 0-9.2 9.6-16.4 9.6C7.4 12.8 3.8 10.4 2 8.2" />
          </svg>
          <span class="catalog-title-text">{ctx.figurine.name}</span>
          <svg class="catalog-flourish catalog-flourish--end" viewBox="0 0 48 16" fill="none" stroke="currentColor" stroke-width="1.15" stroke-linecap="round" aria-hidden="true">
            <path d="M46 8.2C36 8.2 33.5 3.2 27.5 3.2c-7.2 0-9.2 9.6-16.4 9.6C7.4 12.8 3.8 10.4 2 8.2" />
          </svg>
        </h1>
        <p class="catalog-subtitle">{$t('catalogSubtitle')}</p>
        {#if ctx.hasText(ctx.figurine.dimensions)}
          <p class="catalog-dims">{$t('catalogDimsPrefix')} {ctx.figurine.dimensions}</p>
        {/if}
      </header>

      <div class="catalog-main">
        <div class="catalog-copy">
          {#if heroParagraphs.length > 0 || (ctx.hasText(ctx.figurine.secretText) && ctx.isCandleLit)}
          <div class="catalog-prose" style={computeElementStyle(ctx.displayConfig, 'shortText')}>
            {#each heroParagraphs as para}
              <p>{para}</p>
            {/each}
            {#if ctx.hasText(ctx.figurine.secretText) && ctx.isCandleLit}
              <div class="secret-anchor">
                <SecretText text={ctx.figurine.secretText} isCandleLit={ctx.isCandleLit} />
              </div>
            {/if}
          </div>
          {/if}

          {#if showHistory}
            <section class="catalog-history dc-block--description" style={computeBlockStyle(ctx.displayConfig, 'description')}>
              <header class="d-section-header">
                <span class="sec-label">{$t('figurineHistory')}</span>
                <div class="sec-rule" aria-hidden="true"></div>
                <FontSwitcher variant="colophon" />
              </header>
              <p bind:this={historyRef} class="history-body drop-cap">
                {#if inkReady}
                  {@html buildInkHtml(ctx.figurine.fullDescription ?? '')}
                {:else}
                  {ctx.figurine.fullDescription}
                {/if}
              </p>
            </section>
          {/if}

          {#if featureLines.length > 0}
          <section class="catalog-features">
            <h2 class="catalog-list-title">{$t('catalogFeaturesTitle')}</h2>
            <ul class="catalog-list">
              {#each featureLines as line}
                <li class="catalog-list-item">
                  <CatalogGlyph name={line.icon} />
                  <span>{line.text}</span>
                </li>
              {/each}
            </ul>
          </section>
          {/if}

          <section class="catalog-request">
            <h2 class="catalog-notes-title">{ctx.statusUi.title}</h2>
            <div class="catalog-request-facts">
              {#each requestFacts as line}
                <p class="catalog-list-item">
                  <CatalogGlyph name={line.icon} />
                  <span>{line.text}</span>
                </p>
              {/each}
            </div>
            <div class="catalog-request-actions">
              <button
                type="button"
                class="catalog-request-btn"
                onclick={() => ctx.openRequestModal(ctx.statusUi.defaultIntent)}
              >
                {$t('unifiedOpenRequest')}
              </button>
              <a class="catalog-passport" href="/figurines/{ctx.id}/passport" onclick={() => ctx.analyticsClient?.cta('passport')}>
                {$t('detailOpenPassport')}
              </a>
            </div>
          </section>
        </div>

        <div class="catalog-visual">
          <div class="catalog-plate" use:registerGallery>
            <FigurineImageViewer hideThumbs hideCaption aspect="4 / 5" />
          </div>

          {#if ctx.sortedImages.length > 1}
            <div class="catalog-film">
              <nav bind:this={filmEl} class="catalog-film-track" aria-label={$t('figurineShowView')}>
                {#each ctx.sortedImages as img, i}
                  <button
                    type="button"
                    data-film-index={i}
                    class="catalog-thumb {ctx.activeImageIndex === i ? 'catalog-thumb--active' : ''}"
                    onclick={() => ctx.selectImage(i)}
                    aria-label="{ctx.imageTypeLabel(img.imageType)}: {ctx.imageRoleNote(img.imageType)}"
                    aria-current={ctx.activeImageIndex === i ? 'true' : undefined}
                    title="{ctx.imageTypeLabel(img.imageType)} · {i + 1}/{ctx.sortedImages.length}"
                  >
                    <picture>
                      <source type="image/webp" srcset={resolveWebpUrl(img.thumbUrl ?? img.url) ?? undefined} />
                      <img
                        src={ctx.resolveUrl(img.thumbUrl ?? img.url)}
                        alt={ctx.altTextFor(img)}
                        loading="lazy"
                        decoding="async"
                        style={img.focalX != null && img.focalY != null
                          ? `object-position: ${img.focalX * 100}% ${img.focalY * 100}%;`
                          : undefined}
                      />
                    </picture>
                    <span class="catalog-thumb-label">{ctx.imageTypeLabel(img.imageType)}</span>
                  </button>
                {/each}
              </nav>
              <p class="catalog-film-meta">
                <span>{ctx.imageTypeLabel(ctx.currentImage?.imageType)}</span>
                <span class="catalog-film-count">{String(ctx.activeImageIndex + 1).padStart(2, '0')} / {String(ctx.sortedImages.length).padStart(2, '0')}</span>
              </p>
            </div>
          {/if}

          {#if perfectLines.length > 0}
          <section class="catalog-perfect">
            <h2 class="catalog-list-title">{$t('catalogPerfectTitle')}</h2>
            <ul class="catalog-list">
              {#each perfectLines as line}
                <li class="catalog-list-item">
                  <CatalogGlyph name={line.icon} />
                  <span>{line.text}</span>
                </li>
              {/each}
            </ul>
          </section>
          {/if}

          <aside class="catalog-guarantee">
            <p>{$t('catalogGuarantee')}</p>
            <span class="catalog-guarantee-mark" aria-hidden="true">♡</span>
          </aside>
        </div>
      </div>

    <p class="catalog-thanks">{$t('catalogThanks')} ♡</p>
  </div>

  <div class="catalog-after details-col">
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
      omitLead={true}
    />

    {#if ctx.canShowPersonalRecord}
      <details class="entry-record" class:entry-record--empty={!ctx.hasPersonalRecord} ontoggle={ctx.handlePersonalRecordToggle}>
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
                    claim={c}
                    isLoggedIn={authStore.isLoggedIn}
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
                    class="claim-lookup-input" oninput={() => { ctx.cs.tokenLookupInfo = null; ctx.cs.tokenLookupErr = ''; }} />
                  <button type="button" onclick={() => ctx.cs.lookupToken()} disabled={ctx.cs.tokenLooking} class="claim-lookup-btn">
                    {ctx.cs.tokenLooking ? '...' : $t('claimLookupBtn')}
                  </button>
                  <button type="button" onclick={ctx.closeClaimLookup} class="claim-lookup-close" aria-label={$t('lightboxClose')}>
                    <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                      <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke-linecap="round" />
                    </svg>
                  </button>
                </div>
                {#if ctx.cs.tokenLookupErr}<p class="claim-err">{ctx.cs.tokenLookupErr}</p>{/if}
                {#if ctx.cs.tokenLookupInfo}
                  <div class="claim-lookup-result">
                    <p class="claim-dates">{ctx.fmtDate(ctx.cs.tokenLookupInfo.startsAt)} - {ctx.fmtDate(ctx.cs.tokenLookupInfo.endsAt)}</p>
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

    <div class="dc-sections" style={sectionStyle}>
    <div class="dc-block--making" class:dc-block--hidden={!isBlockVisible(ctx.displayConfig, 'making')} style={computeBlockStyle(ctx.displayConfig, 'making')}>
    {#if ctx.hasMakingSection}
      <div class="act-divider" aria-hidden="true"></div>
      <div class="grimoire-section {ctx.figurine.status === 'in_progress' ? 'grimoire-section--live' : ''}">
        <div class="making-record">
          <div class="making-copy">
            <span class="making-kicker">
              {#if ctx.figurine.status === 'in_progress'}
                <span class="making-live" aria-hidden="true"></span>{$t('detailMakingProgressKicker')}
              {:else}
                {$t('detailMakingRecordKicker')}
              {/if}
            </span>
            <h2 class="making-title">
              {ctx.figurine.status === 'in_progress' ? $t('detailMakingProgressTitle') : $t('detailMakingRecordTitle')}
            </h2>
            <p class="making-colophon">
              {ctx.toRoman(ctx.visibleProcessSteps.length)} {$t('detailMakingStages')}<span class="mc-sep">·</span>{ctx.processStepLabel(ctx.firstStep?.stepType)} → {#if ctx.figurine.status === 'in_progress'}{$t('detailMakingLive')}{:else}{ctx.processStepLabel(ctx.lastStep?.stepType)}<span class="mc-sep">·</span>{$t('detailMakingByOneHand')}{/if}
            </p>
          </div>

          {#if ctx.hasBecoming}
            <BecomingReveal
              beforeSrc={ctx.becomingBefore}
              afterSrc={ctx.becomingAfter}
              beforeLabel={ctx.processStepLabel(ctx.firstStep?.stepType)}
              afterLabel={$t('detailBecomingFinished')}
              hint={$t('detailBecomingHint')}
            />
          {:else}
            <div class="making-strip" aria-label={$t('detailMakingRecordTitle')}>
              {#each ctx.visibleProcessSteps.slice(0, 4) as step, i (step.id)}
                <article class="making-card">
                  <div class="making-img-wrap">
                    {#if ctx.hasText(step.imageUrl)}
                      <img src={ctx.resolveUrl(step.imageUrl)} alt="" class="making-img" loading="lazy" />
                    {:else}
                      <div class="making-img-placeholder" aria-hidden="true"></div>
                    {/if}
                    <span class="making-count">{String(i + 1).padStart(2, '0')}</span>
                  </div>
                  <div class="making-card-copy">
                    <h3>{ctx.processStepLabel(step.stepType)}</h3>
                    {#if step.description}<p>{step.description}</p>{/if}
                  </div>
                </article>
              {/each}
            </div>
          {/if}
        </div>

        {#if ctx.showMirrorLink}
          <button type="button" onclick={ctx.openGrimoire} class="mirror-link" aria-expanded={ctx.isGrimoireOpen}>
            <span class="mirror-link-mark" aria-hidden="true"></span>
            <span class="mirror-link-label">{$t('figurineGrimoire')}</span>
            <span class="mirror-link-count">{ctx.visibleProcessSteps.length} {$t('figurineGrimoireSub')}</span>
            <svg
              class="mirror-link-arrow"
              class:mirror-link-arrow--open={ctx.isGrimoireOpen}
              width="15" height="15" viewBox="0 0 16 16"
              fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"
            >
              <path d="M3 8h10M9 4l4 4-4 4"/>
            </svg>
          </button>
        {/if}
      </div>
    {/if}
    </div>

    <div class="dc-block--video" class:dc-block--hidden={!isBlockVisible(ctx.displayConfig, 'video')} style={computeBlockStyle(ctx.displayConfig, 'video')}>
    {#if ctx.hasVideoSection}
      <div class="act-divider" aria-hidden="true"></div>
      <section class="video-section">
        <header class="section-row">
          <span class="sec-label">{$t('figurineVideo')}</span>
          <div class="sec-rule" aria-hidden="true"></div>
        </header>
        <div class="video-wrap">
          <div class="video-frame">
            <div class="video-stage">
              <video bind:this={videoRef} controls class="video-el"
                poster={ctx.resolveUrl(ctx.currentImage?.url)} preload="metadata">
                <source src={ctx.resolveUrl(ctx.figurine.videoUrl)} type="video/mp4" />
                {$t('figurineBrowserNoVideo')}
              </video>
              <button type="button" onclick={toggleFullscreen} class="video-fs-btn" title={$t('figurineFullscreen')} aria-label={$t('figurineFullscreen')}>
                <svg width="14" height="14" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                  <path d="M1 4V1h3M6 1h3v3M9 6v3H6M4 9H1V6"/>
                </svg>
              </button>
            </div>
          </div>
        </div>
        <p class="video-caption text-label">{$t('figurineVideoFilm')}{ctx.id.slice(-3)}</p>
      </section>
    {/if}
    </div>

    <div class="dc-block--showings" class:dc-block--hidden={!isBlockVisible(ctx.displayConfig, 'showings')} style={computeBlockStyle(ctx.displayConfig, 'showings')}>
    {#if ctx.hasFactsSection && ctx.hasScheduleSection}
      <div class="act-divider" aria-hidden="true"></div>
      {#if ctx.hasScheduleSection}
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
    {/if}
    </div>

    <div class="dc-block--related" class:dc-block--hidden={!isBlockVisible(ctx.displayConfig, 'related')} style={computeBlockStyle(ctx.displayConfig, 'related')}>
    {#if ctx.visibleRelatedItems.length > 0}
      <div class="act-divider" aria-hidden="true"></div>
      <section id="related-works" class="related-section">
        <header class="related-head">
          <div>
            <span class="sec-label">{$t('figurineRelated')}</span>
            <h2 class="related-title">{$t('detailRelatedTitle')}</h2>
          </div>
          <p class="related-intro">{$t('detailRelatedText')}</p>
        </header>
        <div class="related-strip">
          {#each ctx.visibleRelatedItems as item}
            {@const relatedImageUrl = ctx.resolveUrl(item.faceImageUrl ?? item.thumbUrl)}
            <a href={figurineHref(item)}
              onclick={() => ctx.analyticsClient?.cta('related_figurine')}
              class="related-card" data-sveltekit-preload-data="hover">
              <div class="related-img-wrap">
                {#if relatedImageUrl}
                  <img src={relatedImageUrl} alt={item.name} class="related-img" loading="lazy" />
                {:else}
                  <div class="related-placeholder" aria-hidden="true"><span>{item.name.slice(0, 1)}</span></div>
                {/if}
                <div class="related-overlay" aria-hidden="true">
                  <span class="related-cta-hint">
                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
                      <path d="M1 6h10M7 2l4 4-4 4"/>
                    </svg>
                  </span>
                </div>
                <span class="related-status-badge related-status-badge--{item.status}">{ctx.statusLabel(item.status)}</span>
              </div>
              <div class="related-meta">
                <h4 class="related-name">{item.name}</h4>
                <p class="related-line">
                  {#if item.material}{item.material}{:else if item.technique}{item.technique}{:else if item.series}{item.series}{:else}{$t('detailRelatedArchivePiece')}{/if}
                </p>
                <div class="related-foot">
                  {#if item.year}<span>Anno {item.year}</span>{/if}
                  <span class="related-action">
                    {item.status === 'available' ? $t('detailRelatedRequestable') : $t('detailRelatedOpen')}
                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                      <path d="M1.5 6h9M7 2.5L10.5 6 7 9.5"/>
                    </svg>
                  </span>
                </div>
              </div>
            </a>
          {/each}
        </div>
      </section>
    {/if}
    </div>

    <div class="dc-block--comments" class:dc-block--hidden={!isBlockVisible(ctx.displayConfig, 'comments')} style={computeBlockStyle(ctx.displayConfig, 'comments')}>
      <div class="act-divider" aria-hidden="true"></div>
      <FigurineComments figurineId={ctx.id} />
    </div>
    </div>
  </div>
</div>
