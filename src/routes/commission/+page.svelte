<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { page } from '$app/state';
  import { api, resolveMediaUrl } from '$lib/api';
  import { t, lang , brandName } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';
  import { isValidEmail } from '$lib/validation';
  import { createSiteAnalytics } from '$lib/analytics';
  import type { AttachmentInput, Figurine, FigurineListItem } from '$lib/types/api';

  const STORE_KEY = 'gotiga_commissions';
  const PENDING_CLAIM_KEY = 'gotiga_pending_claim';

  const siteAnalytics = createSiteAnalytics();

  onMount(() => {
    siteAnalytics.pageView();
  });

  // Fired from the step-1 title/description fields' own input handler — not a
  // reactive effect — so a programmatic value-set (e.g. the source-figurine
  // effect below prefilling `title`) never counts as "the visitor started
  // the form." cta() dedupes internally, so this is safe to call on every keystroke.
  function markFormStarted() {
    siteAnalytics.cta('commission_form_start');
  }

  let step = $state(1);
  const TOTAL_STEPS = 3;

  // Step 1 — the idea
  let title = $state('');
  let description = $state('');
  let similarKeepNote = $state('');
  let similarChangeNote = $state('');
  let sourceFigurine = $state<Figurine | null>(null);
  let sourceLoadError = $state(false);
  let sourceTitleApplied = $state(false);
  let sourceFigurineId = $derived(
    page.url.searchParams.get('source') ?? page.url.searchParams.get('sourceFigurineId') ?? '',
  );
  let sourceImageUrl = $derived.by(() => {
    const image = sourceFigurine?.images.find((img) => img.imageType === 'face') ?? sourceFigurine?.images[0];
    return resolveMediaUrl(image?.thumbUrl ?? image?.url);
  });
  let similarTags = $derived.by(() => {
    if (!sourceFigurine) return [] as string[];
    return [sourceFigurine.series, sourceFigurine.technique, sourceFigurine.material]
      .map((tag) => tag?.trim())
      .filter((tag): tag is string => Boolean(tag))
      .slice(0, 6);
  });

  // Step 2 — details & references
  let sizeNote = $state('');
  let mood = $state('');
  let deadline = $state('');
  let budgetNote = $state('');
  let occasionChoice = $state('');
  let occasionCustom = $state('');
  let occasionValue = $derived(occasionChoice === '__custom__' ? occasionCustom.trim() : occasionChoice);
  let attachments = $state<AttachmentInput[]>([]);
  let uploading = $state(false);
  let uploadError = $state('');

  // Reference works — only on the general form (no source figurine from the URL).
  // The visitor can tick one or several archive pieces whose mood/craft inspires them.
  let showRefs = $state(false);
  let refItems = $state<FigurineListItem[]>([]);
  let refsLoaded = $state(false);
  let loadingRefs = $state(false);
  let selectedRefIds = $state<string[]>([]);

  async function loadReferences() {
    if (refsLoaded || loadingRefs) return;
    loadingRefs = true;
    try {
      refItems = await api.getAllFigurines();
      refsLoaded = true;
    } catch {
      /* leave empty — picker simply shows nothing to choose */
    } finally {
      loadingRefs = false;
    }
  }

  function toggleRef(id: string) {
    selectedRefIds = selectedRefIds.includes(id)
      ? selectedRefIds.filter((x) => x !== id)
      : [...selectedRefIds, id];
  }

  // Search / filter / paginate the archive so a large catalogue stays browsable.
  const REF_PAGE_SIZE = 8;
  let refSearch = $state('');
  let refSeries = $state('');
  let refPage = $state(1);

  let refSeriesOptions = $derived.by(() => {
    const set = new Set<string>();
    for (const f of refItems) if (f.series) set.add(f.series);
    return [...set].sort((a, b) => a.localeCompare(b));
  });

  let filteredRefs = $derived.by(() => {
    const q = refSearch.trim().toLowerCase();
    return refItems.filter(
      (f) => (!q || f.name.toLowerCase().includes(q)) && (!refSeries || f.series === refSeries),
    );
  });

  let refTotalPages = $derived(Math.max(1, Math.ceil(filteredRefs.length / REF_PAGE_SIZE)));
  let pagedRefs = $derived(filteredRefs.slice((refPage - 1) * REF_PAGE_SIZE, refPage * REF_PAGE_SIZE));

  // Snap back to the first page whenever the filters change.
  $effect(() => {
    refSearch; refSeries;
    refPage = 1;
  });

  function refName(id: string) {
    return refItems.find((f) => f.id === id)?.name ?? id;
  }

  // Step 3 — contact
  let name = $state('');
  let email = $state('');
  let phone = $state('');
  let website = $state(''); // honeypot

  let isSubmitting = $state(false);
  let submitError = $state('');
  let isSealed = $state(false);
  let claimToken = $state('');

  let canAdvanceFromStep1 = $derived(description.trim().length > 0);

  $effect(() => {
    const id = sourceFigurineId.trim();
    sourceLoadError = false;
    sourceFigurine = null;
    sourceTitleApplied = false;
    if (!id) return;

    let cancelled = false;
    api.getFigurine(id)
      .then((fig) => {
        if (cancelled) return;
        sourceFigurine = fig;
        sourceLoadError = !fig;
        if (fig && !sourceTitleApplied && !title.trim()) {
          title = `${$t('commissionInspiredTitlePrefix')} ${fig.name}`;
          sourceTitleApplied = true;
        }
      })
      .catch(() => {
        if (!cancelled) sourceLoadError = true;
      });

    return () => {
      cancelled = true;
    };
  });

  function next() {
    submitError = '';
    if (step === 1 && !canAdvanceFromStep1) { submitError = $t('commissionNeedIdea'); return; }
    if (step < TOTAL_STEPS) step += 1;
  }
  function back() {
    submitError = '';
    if (step > 1) step -= 1;
  }

  async function handleFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files || !authStore.sessionToken) return;
    uploadError = '';
    const files = Array.from(input.files);
    for (const file of files) {
      if (attachments.length >= 5) { uploadError = $t('commissionTooManyFiles'); break; }
      if (file.size > 8 * 1024 * 1024) { uploadError = $t('commissionFileTooLarge'); continue; }
      uploading = true;
      try {
        const att = await api.uploadUserMedia(authStore.sessionToken, file);
        attachments = [...attachments, att];
      } catch {
        uploadError = $t('commissionUploadError');
      } finally {
        uploading = false;
      }
    }
    input.value = '';
  }

  function removeAttachment(i: number) {
    attachments = attachments.filter((_, idx) => idx !== i);
  }

  function rememberToken(token: string) {
    try {
      const raw = localStorage.getItem(STORE_KEY);
      const list: string[] = raw ? JSON.parse(raw) : [];
      list.push(token);
      localStorage.setItem(STORE_KEY, JSON.stringify(list));
    } catch { /* ignore */ }
  }

  async function submit() {
    submitError = '';
    const effectiveEmail = authStore.isLoggedIn ? (authStore.user?.email ?? '') : email.trim();
    if (!effectiveEmail) { submitError = $t('formFillFields'); return; }
    if (!authStore.isLoggedIn && !isValidEmail(effectiveEmail)) { submitError = $t('formInvalidEmail'); return; }
    if (!description.trim()) { submitError = $t('commissionNeedIdea'); step = 1; return; }

    // On the general form (no URL source) carry the checkbox selection: the first
    // ticked work becomes the linked source (thumbnail in admin/profile), and every
    // selected work's name goes into the tags so all references stay visible.
    const manualRefIds = sourceFigurineId.trim() ? [] : selectedRefIds;
    const manualRefNames = manualRefIds
      .map((id) => refItems.find((f) => f.id === id)?.name)
      .filter((n): n is string => Boolean(n));
    const effectiveSourceId = sourceFigurineId.trim() || manualRefIds[0] || null;
    const effectiveTags = manualRefNames.length ? manualRefNames : similarTags;

    isSubmitting = true;
    try {
      const res = await api.submitCommission(
        {
          requesterName: authStore.isLoggedIn ? (authStore.user?.displayName ?? null) : (name.trim() || null),
          requesterEmail: effectiveEmail,
          requesterPhone: phone.trim() || null,
          title: title.trim() || null,
          description: description.trim(),
          sizeNote: sizeNote.trim() || null,
          mood: mood.trim() || null,
          deadline: deadline || null,
          budgetNote: budgetNote.trim() || null,
          occasion: occasionValue || null,
          sourceFigurineId: effectiveSourceId,
          similarKeepNote: similarKeepNote.trim() || null,
          similarChangeNote: similarChangeNote.trim() || null,
          similarTags: effectiveTags,
          attachmentUrls: attachments,
          website: website || null,
          lang: $lang,
        },
        authStore.sessionToken ?? undefined,
      );
      claimToken = res.claimToken;
      rememberToken(res.claimToken);
      if (!authStore.isLoggedIn) {
        try { localStorage.setItem(PENDING_CLAIM_KEY, res.claimToken); } catch { /* ignore */ }
      }
      isSealed = true;
    } catch {
      submitError = $t('commissionSubmitError');
    } finally {
      isSubmitting = false;
    }
  }
</script>

<svelte:head>
  <title>{$t('commissionTitle')} — {$brandName}</title>
  <meta name="description" content={$t('commissionSubtitle')} />
  <!-- Fonts loaded once globally in app.html -->
</svelte:head>

<div class="root">
  <div class="grain" aria-hidden="true"></div>

  <div class="page">
    <nav class="back-nav" in:fade={{ duration: 600 }}>
      <a href="/" class="back-link">{$t('commissionBackHome')}</a>
    </nav>

    {#if isSealed}
      <!-- Success: a sealed letter -->
      <div class="sealed" in:fade={{ duration: 700 }}>
        <div class="wax" in:fly={{ y: -10, duration: 800, easing: cubicOut }}>✦</div>
        <h1 class="page-title">{$t('commissionSentTitle')}</h1>
        <p class="page-subtitle">{$t('commissionSentBody')}</p>

        <div class="token-box">
          <span class="token-label">{$t('commissionTokenLabel')}</span>
          <code class="token">{claimToken}</code>
          <span class="token-hint">{$t('commissionTokenHint')}</span>
        </div>

        {#if authStore.isLoggedIn}
          <a class="cta" href="/profile">{$t('commissionGoToProfile')}</a>
        {:else}
          <a class="cta" href="/login">{$t('commissionLoginToFollow')}</a>
          <p class="quiet">{$t('commissionGuestNote')}</p>
        {/if}
      </div>
    {:else}
      <header class="page-header" in:fly={{ x: -20, duration: 900, delay: 100, easing: cubicOut }}>
        <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('commissionKicker')}</p>
        <h1 class="page-title">{$t('commissionTitle')}</h1>
        <p class="page-subtitle">{$t('commissionSubtitle')}</p>
      </header>

      {#if sourceFigurineId}
        <aside class="source-card" aria-label={$t('commissionSourceLabel')}>
          {#if sourceImageUrl}
            <img src={sourceImageUrl} alt="" class="source-card-img" />
          {:else}
            <div class="source-card-img source-card-img--empty" aria-hidden="true">GT</div>
          {/if}
          <div class="source-card-copy">
            <span class="source-card-kicker">{$t('commissionSourceKicker')}</span>
            <strong>{sourceFigurine?.name ?? $t('commissionSourceLoading')}</strong>
            <span>
              {#if sourceLoadError}
                {$t('commissionSourceUnavailable')}
              {:else if sourceFigurine}
                {$t('commissionSourceHint')}
              {:else}
                {$t('commissionSourceLoading')}
              {/if}
            </span>
          </div>
        </aside>
      {/if}

      <!-- Progress -->
      <div class="progress" aria-hidden="true">
        {#each [1, 2, 3] as s}
          <div class="progress-step {step >= s ? 'on' : ''}">
            <span class="dot">{s}</span>
            <span class="lbl">
              {s === 1 ? $t('commissionStep1') : s === 2 ? $t('commissionStep2') : $t('commissionStep3')}
            </span>
          </div>
        {/each}
      </div>

      <form class="letter" onsubmit={(e) => { e.preventDefault(); if (step < TOTAL_STEPS) next(); else submit(); }}>
        <!-- Honeypot -->
        <input class="hp" type="text" name="website" tabindex="-1" autocomplete="off" bind:value={website} aria-hidden="true" />

        {#if step === 1}
          <div class="step" in:fade={{ duration: 350 }}>
            <label class="field">
              <span class="field-label">{$t('commissionFieldTitle')}</span>
              <input class="input" type="text" bind:value={title} oninput={markFormStarted} placeholder={$t('commissionFieldTitlePh')} maxlength="120" />
            </label>
            <label class="field">
              <span class="field-label">{$t('commissionFieldIdea')} *</span>
              <textarea class="input area" bind:value={description} oninput={markFormStarted} rows="5" placeholder={$t('commissionFieldIdeaPh')} maxlength="5000"></textarea>
            </label>
            {#if sourceFigurineId}
              <div class="similar-fields">
                <label class="field">
                  <span class="field-label">{$t('commissionSimilarKeep')}</span>
                  <textarea class="input area--compact" bind:value={similarKeepNote} rows="3" placeholder={$t('commissionSimilarKeepPh')} maxlength="1000"></textarea>
                </label>
                <label class="field">
                  <span class="field-label">{$t('commissionSimilarChange')}</span>
                  <textarea class="input area--compact" bind:value={similarChangeNote} rows="3" placeholder={$t('commissionSimilarChangePh')} maxlength="1000"></textarea>
                </label>
              </div>
            {:else}
              <div class="ref-works">
                <span class="field-label">{$t('commissionRefWorksLabel')}</span>
                {#if !showRefs}
                  <button type="button" class="ref-works-toggle" onclick={() => { showRefs = true; loadReferences(); }}>
                    {$t('commissionRefWorksAdd')} +
                  </button>
                {:else}
                  <p class="quiet ref-works-hint">{$t('commissionRefWorksHint')}</p>
                  {#if loadingRefs}
                    <p class="quiet">{$t('commissionRefWorksLoading')}</p>
                  {:else if refItems.length === 0}
                    <p class="quiet">{$t('commissionRefWorksEmpty')}</p>
                  {:else}
                    <div class="ref-tools">
                      <input class="input ref-search" type="text" bind:value={refSearch} placeholder={$t('commissionRefWorksSearchPh')} />
                      {#if refSeriesOptions.length > 0}
                        <select class="input input--select ref-series" bind:value={refSeries}>
                          <option value="">{$t('commissionRefWorksAllSeries')}</option>
                          {#each refSeriesOptions as s}
                            <option value={s}>{s}</option>
                          {/each}
                        </select>
                      {/if}
                    </div>

                    {#if selectedRefIds.length > 0}
                      <div class="ref-chosen">
                        <span class="ref-chosen-label">{$t('commissionRefWorksSelected')} {selectedRefIds.length}</span>
                        {#each selectedRefIds as id (id)}
                          <button type="button" class="ref-chip" onclick={() => toggleRef(id)}>{refName(id)} ×</button>
                        {/each}
                      </div>
                    {/if}

                    {#if filteredRefs.length === 0}
                      <p class="quiet">{$t('commissionRefWorksNothing')}</p>
                    {:else}
                      <ul class="ref-list">
                        {#each pagedRefs as fig (fig.id)}
                          {@const on = selectedRefIds.includes(fig.id)}
                          {@const thumb = resolveMediaUrl(fig.thumbUrl ?? fig.faceImageUrl)}
                          <li>
                            <label class="ref-row" class:ref-row--on={on}>
                              <input type="checkbox" class="ref-row-input" checked={on} onchange={() => toggleRef(fig.id)} />
                              <span class="ref-row-tick" aria-hidden="true">{on ? '✓' : ''}</span>
                              {#if thumb}
                                <img src={thumb} alt="" class="ref-row-img" />
                              {:else}
                                <span class="ref-row-img ref-row-img--empty" aria-hidden="true">GT</span>
                              {/if}
                              <span class="ref-row-name">{fig.name}</span>
                              {#if fig.series}<span class="ref-row-series">{fig.series}</span>{/if}
                            </label>
                          </li>
                        {/each}
                      </ul>

                      {#if refTotalPages > 1}
                        <div class="ref-pager">
                          <button type="button" class="ref-pager-btn" disabled={refPage <= 1} onclick={() => refPage -= 1} aria-label="←">←</button>
                          <span class="ref-pager-count">{refPage} / {refTotalPages}</span>
                          <button type="button" class="ref-pager-btn" disabled={refPage >= refTotalPages} onclick={() => refPage += 1} aria-label="→">→</button>
                        </div>
                      {/if}
                    {/if}
                  {/if}
                {/if}
              </div>
            {/if}
          </div>
        {:else if step === 2}
          <div class="step" in:fade={{ duration: 350 }}>
            <label class="field">
              <span class="field-label">{$t('commissionFieldSize')}</span>
              <select class="input input--select" bind:value={sizeNote}>
                <option value="">{$t('commissionSizeUnsure')}</option>
                <option value={$t('commissionSizeXs')}>{$t('commissionSizeXs')}</option>
                <option value={$t('commissionSizeS')}>{$t('commissionSizeS')}</option>
                <option value={$t('commissionSizeM')}>{$t('commissionSizeM')}</option>
                <option value={$t('commissionSizeL')}>{$t('commissionSizeL')}</option>
                <option value={$t('commissionSizeXl')}>{$t('commissionSizeXl')}</option>
              </select>
            </label>
            <div class="row">
              <label class="field">
                <span class="field-label">{$t('commissionFieldMood')}</span>
                <input class="input" type="text" bind:value={mood} placeholder={$t('commissionFieldMoodPh')} />
              </label>
              <label class="field">
                <span class="field-label">{$t('commissionFieldDeadline')}</span>
                <input class="input" type="date" bind:value={deadline} />
              </label>
            </div>
            <label class="field">
              <span class="field-label">{$t('commissionFieldOccasion')}</span>
              <select class="input input--select" bind:value={occasionChoice}>
                <option value="">{$t('commissionOccasionChoose')}</option>
                <option value={$t('commissionOccasionGift')}>{$t('commissionOccasionGift')}</option>
                <option value={$t('commissionOccasionSelf')}>{$t('commissionOccasionSelf')}</option>
                <option value={$t('commissionOccasionCollection')}>{$t('commissionOccasionCollection')}</option>
                <option value={$t('commissionOccasionMemorial')}>{$t('commissionOccasionMemorial')}</option>
                <option value="__custom__">{$t('commissionOccasionOther')}</option>
              </select>
              {#if occasionChoice === '__custom__'}
                <input class="input" type="text" bind:value={occasionCustom} placeholder={$t('commissionOccasionCustomPh')} style="margin-top:0.5rem" />
              {/if}
            </label>
            <label class="field">
              <span class="field-label">{$t('commissionFieldBudget')}</span>
              <input class="input" type="text" bind:value={budgetNote} placeholder={$t('commissionFieldBudgetPh')} />
            </label>

            <!-- References -->
            <div class="field">
              <span class="field-label">{$t('commissionFieldRefs')}</span>
              {#if authStore.isLoggedIn}
                <div class="refs">
                  {#each attachments as att, i (att.url)}
                    <div class="ref-thumb">
                      <img src={resolveMediaUrl(att.thumbUrl ?? att.url)} alt="" />
                      <button type="button" class="ref-x" onclick={() => removeAttachment(i)} aria-label="×">×</button>
                    </div>
                  {/each}
                  {#if attachments.length < 5}
                    <label class="ref-add">
                      <input type="file" accept="image/*" multiple onchange={handleFiles} hidden />
                      {uploading ? '…' : '+'}
                    </label>
                  {/if}
                </div>
                {#if uploadError}<p class="err">{uploadError}</p>{/if}
              {:else}
                <p class="quiet">{$t('commissionRefsGuestNote')}</p>
              {/if}
            </div>
          </div>
        {:else}
          <div class="step" in:fade={{ duration: 350 }}>
            {#if authStore.isLoggedIn}
              <p class="as-user">{$t('commissionAsUser')} <strong>{authStore.user?.displayName}</strong> ({authStore.user?.email})</p>
            {:else}
              <label class="field">
                <span class="field-label">{$t('commissionFieldName')}</span>
                <input class="input" type="text" bind:value={name} placeholder={$t('commissionFieldNamePh')} />
              </label>
              <label class="field">
                <span class="field-label">{$t('commissionFieldEmail')} *</span>
                <input class="input" type="email" bind:value={email} placeholder="you@example.com" />
              </label>
              <label class="field">
                <span class="field-label">{$t('commissionFieldPhone')}</span>
                <input class="input" type="tel" bind:value={phone} placeholder={$t('commissionFieldPhonePh')} />
              </label>
            {/if}
            <p class="quiet decline-note">{$t('commissionMayDecline')}</p>
            <div class="terms-note">
              <span class="terms-title">{$t('commissionTermsTitle')}</span>
              <p>{$t('commissionTerms')}</p>
              <a class="terms-link" href="/acquire">{$t('navAcquire')} →</a>
            </div>
          </div>
        {/if}

        {#if submitError}<p class="err center">{submitError}</p>{/if}

        <div class="actions">
          {#if step > 1}
            <button type="button" class="btn ghost" onclick={back} disabled={isSubmitting}>← {$t('commissionBack')}</button>
          {/if}
          {#if step < TOTAL_STEPS}
            <button type="submit" class="btn" disabled={step === 1 && !canAdvanceFromStep1}>{$t('commissionNext')} →</button>
          {:else}
            <button type="submit" class="btn" disabled={isSubmitting}>{isSubmitting ? $t('commissionSending') : $t('commissionSend')}</button>
          {/if}
        </div>
      </form>
    {/if}
  </div>
</div>

<style>
  .root {
    position: relative;
    min-height: 100vh;
    background: #f8f1e7;
    color: #34251c;
    font-family: 'Instrument Sans', sans-serif;
    overflow-x: hidden;
  }
  .grain {
    position: fixed; inset: 0; pointer-events: none; opacity: 0.4; z-index: 1;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='120'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)' opacity='0.5'/%3E%3C/svg%3E");
  }
  .page { position: relative; z-index: 2; max-width: 640px; margin: 0 auto; padding: 2.5rem 1.5rem 5rem; }
  .back-nav { margin-bottom: 2.5rem; }
  .back-link { font-size: 0.8rem; letter-spacing: 0.08em; text-transform: uppercase; color: #6f3b24; text-decoration: none; opacity: 0.7; transition: opacity 0.2s; }
  .back-link:hover { opacity: 1; }

  .eyebrow { display: flex; align-items: center; gap: 0.75rem; font-size: 0.7rem; letter-spacing: 0.22em; text-transform: uppercase; color: #c65f3c; margin-bottom: 1rem; }
  .eyebrow-rule { width: 2.5rem; height: 1px; background: #c65f3c; }
  .page-title { font-family: 'Fraunces', Georgia, serif; font-size: clamp(2rem, 6vw, 3rem); font-weight: 400; line-height: 1.05; margin: 0 0 0.75rem; }
  .page-subtitle { font-family: 'Cormorant Garamond', Georgia, serif; font-size: 1.2rem; font-style: italic; color: #6f3b24; max-width: 36ch; }

  .source-card {
    display: grid;
    grid-template-columns: 72px 1fr;
    gap: 0.9rem;
    align-items: center;
    margin: 1.75rem 0 0;
    padding: 0.75rem;
    border: 1px solid #d8c6b1;
    background: rgba(255, 250, 242, 0.72);
  }
  .source-card-img {
    width: 72px;
    aspect-ratio: 1;
    object-fit: cover;
    border: 1px solid #d8c6b1;
    background: #f0e6d6;
  }
  .source-card-img--empty {
    display: grid;
    place-items: center;
    color: #6f3b24;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 0.9rem;
    letter-spacing: 0.08em;
  }
  .source-card-copy { display: flex; min-width: 0; flex-direction: column; gap: 0.18rem; }
  .source-card-kicker { font-size: 0.65rem; letter-spacing: 0.12em; text-transform: uppercase; color: #c65f3c; }
  .source-card-copy strong { font-family: 'Fraunces', Georgia, serif; font-size: 1.05rem; color: #34251c; }
  .source-card-copy span:last-child { font-size: 0.86rem; color: #6f3b24; }

  .progress { display: flex; gap: 0.5rem; margin: 2.5rem 0 1.5rem; }
  .progress-step { display: flex; align-items: center; gap: 0.5rem; flex: 1; opacity: 0.4; transition: opacity 0.3s; }
  .progress-step.on { opacity: 1; }
  .progress-step .dot { display: grid; place-items: center; width: 1.6rem; height: 1.6rem; border-radius: 50%; border: 1px solid #c65f3c; color: #c65f3c; font-size: 0.8rem; flex-shrink: 0; }
  .progress-step.on .dot { background: #c65f3c; color: #f8f1e7; }
  .progress-step .lbl { font-size: 0.7rem; letter-spacing: 0.05em; text-transform: uppercase; color: #6f3b24; }

  .letter { position: relative; background: #fffaf2; border: 1px solid #d8c6b1; box-shadow: 0 1px 0 #d8c6b1, 0 18px 40px -28px rgba(111,59,36,0.5); padding: 2rem; transform: rotate(-0.4deg); }
  .step { display: flex; flex-direction: column; gap: 1.1rem; }
  .row { display: flex; gap: 1rem; }
  .row .field { flex: 1; }
  .field { display: flex; flex-direction: column; gap: 0.4rem; }
  .field-label { font-size: 0.72rem; letter-spacing: 0.1em; text-transform: uppercase; color: #6f3b24; }
  .input { box-sizing: border-box; width: 100%; min-height: 2.9rem; background: #f8f1e7; border: 1px solid #d8c6b1; padding: 0.6rem 0.7rem; font-family: inherit; font-size: 0.95rem; line-height: 1.4; color: #34251c; transition: border-color 0.2s; }
  .input:focus { outline: none; border-color: #c65f3c; }
  /* Selects: strip the native chrome and draw one consistent chevron so they match the inputs. */
  .input--select {
    appearance: none;
    -webkit-appearance: none;
    padding-right: 2.2rem;
    cursor: pointer;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M1 1l4 4 4-4' fill='none' stroke='%236f3b24' stroke-width='1.4'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.85rem center;
    background-size: 0.7rem;
  }
  /* type=date keeps a consistent baseline with the text inputs. */
  .input[type="date"] { line-height: normal; }
  .area { resize: vertical; min-height: 8rem; font-family: 'Cormorant Garamond', Georgia, serif; font-size: 1.05rem; line-height: 1.5; }
  .area--compact { resize: vertical; min-height: 5rem; font-family: inherit; font-size: 0.95rem; line-height: 1.45; }
  /* Full-width, stacked — these are free-text fields and need room to type in. */
  .similar-fields {
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
    margin-top: 0.4rem;
    padding-top: 1.1rem;
    border-top: 1px solid #e7d8c4;
  }

  .refs { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  .ref-thumb { position: relative; width: 64px; height: 64px; border: 1px solid #d8c6b1; overflow: hidden; }
  .ref-thumb img { width: 100%; height: 100%; object-fit: cover; }
  .ref-x { position: absolute; top: 0; right: 0; width: 18px; height: 18px; background: rgba(52,37,28,0.8); color: #fff; border: none; cursor: pointer; line-height: 1; font-size: 0.8rem; }
  .ref-add { display: grid; place-items: center; width: 64px; height: 64px; border: 1px dashed #c65f3c; color: #c65f3c; font-size: 1.4rem; cursor: pointer; }

  /* Reference-works picker (general form): searchable, paginated list of checkboxes */
  .ref-works { display: flex; flex-direction: column; gap: 0.4rem; margin-top: 0.4rem; padding-top: 1.1rem; border-top: 1px solid #e7d8c4; }
  .ref-works-toggle { align-self: flex-start; margin-top: 0.2rem; padding: 0.55rem 1rem; background: transparent; border: 1px dashed #c65f3c; color: #6f3b24; font-family: inherit; font-size: 0.82rem; letter-spacing: 0.02em; cursor: pointer; transition: background 0.2s, border-color 0.2s; }
  .ref-works-toggle:hover { background: #f0e6d6; border-style: solid; }
  .ref-works-hint { margin: 0.1rem 0 0.3rem; }

  .ref-tools { display: flex; gap: 0.6rem; margin: 0.3rem 0 0.2rem; }
  .ref-search { flex: 1 1 auto; }
  .ref-series { flex: 0 0 auto; max-width: 42%; }

  .ref-chosen { display: flex; flex-wrap: wrap; align-items: center; gap: 0.4rem; margin: 0.5rem 0 0.2rem; }
  .ref-chosen-label { font-size: 0.72rem; letter-spacing: 0.06em; text-transform: uppercase; color: #6f3b24; }
  .ref-chip { display: inline-flex; align-items: center; gap: 0.3rem; padding: 0.25rem 0.6rem; border: 1px solid #c65f3c; background: rgba(198,95,60,0.1); color: #6f3b24; font-family: inherit; font-size: 0.78rem; cursor: pointer; transition: background 0.2s; }
  .ref-chip:hover { background: rgba(198,95,60,0.2); }

  .ref-list { list-style: none; margin: 0.4rem 0 0; padding: 0; border: 1px solid #e0d0bb; }
  .ref-list li + li .ref-row { border-top: 1px solid #ece0cf; }
  .ref-row { display: flex; align-items: center; gap: 0.75rem; padding: 0.5rem 0.7rem; cursor: pointer; transition: background 0.15s; }
  .ref-row:hover { background: #f8f1e7; }
  .ref-row--on { background: rgba(198,95,60,0.08); }
  .ref-row-input { position: absolute; opacity: 0; width: 1px; height: 1px; pointer-events: none; }
  .ref-row-tick { flex: 0 0 auto; display: grid; place-items: center; width: 1.25rem; height: 1.25rem; border: 1px solid #d8c6b1; border-radius: 3px; background: #fffaf2; color: #c65f3c; font-size: 0.8rem; line-height: 1; }
  .ref-row--on .ref-row-tick { background: #c65f3c; border-color: #c65f3c; color: #fffaf2; }
  .ref-row-input:focus-visible ~ .ref-row-tick { border-color: #c65f3c; box-shadow: 0 0 0 2px rgba(198,95,60,0.35); }
  .ref-row-img { flex: 0 0 auto; width: 44px; height: 44px; object-fit: cover; border: 1px solid #d8c6b1; background: #f0e6d6; }
  .ref-row-img--empty { display: grid; place-items: center; color: #6f3b24; font-family: 'Fraunces', Georgia, serif; font-size: 0.7rem; letter-spacing: 0.06em; }
  .ref-row-name { flex: 1 1 auto; min-width: 0; font-size: 0.9rem; color: #34251c; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .ref-row-series { flex: 0 0 auto; font-size: 0.72rem; letter-spacing: 0.04em; text-transform: uppercase; color: #9a7c5c; }

  .ref-pager { display: flex; align-items: center; justify-content: center; gap: 1rem; margin-top: 0.7rem; }
  .ref-pager-btn { width: 2.2rem; height: 2.2rem; border: 1px solid #d8c6b1; background: #fffaf2; color: #6f3b24; font-size: 1rem; cursor: pointer; transition: background 0.2s, border-color 0.2s; }
  .ref-pager-btn:hover:not(:disabled) { background: #f0e6d6; border-color: #c65f3c; }
  .ref-pager-btn:disabled { opacity: 0.4; cursor: default; }
  .ref-pager-count { font-size: 0.82rem; color: #6f3b24; min-width: 3.5rem; text-align: center; }

  .as-user { font-family: 'Cormorant Garamond', Georgia, serif; font-size: 1.1rem; color: #34251c; }
  .quiet { font-family: 'Cormorant Garamond', Georgia, serif; font-style: italic; color: #6f3b24; font-size: 1rem; }
  .decline-note { margin-top: 0.5rem; padding-top: 0.75rem; border-top: 1px solid #d8c6b1; }
  .terms-note { margin-top: 1rem; padding: 0.9rem 1rem; background: #f8f1e7; border: 1px solid #d8c6b1; }
  .terms-title { display: block; font-size: 0.68rem; letter-spacing: 0.12em; text-transform: uppercase; color: #c65f3c; margin-bottom: 0.4rem; }
  .terms-note p { font-family: 'Cormorant Garamond', Georgia, serif; font-size: 1rem; line-height: 1.5; color: #6f3b24; margin: 0; }
  .terms-link { display: inline-block; margin-top: 0.6rem; font-size: 0.8rem; letter-spacing: 0.04em; color: #c65f3c; text-decoration: none; }
  .terms-link:hover { text-decoration: underline; }

  .err { color: #a3361d; font-size: 0.85rem; }
  .err.center { text-align: center; }

  .actions { display: flex; gap: 0.75rem; margin-top: 1.75rem; }
  .btn { margin-left: auto; background: #6f3b24; color: #f8f1e7; border: none; padding: 0.7rem 1.6rem; font-family: inherit; font-size: 0.85rem; letter-spacing: 0.08em; text-transform: uppercase; cursor: pointer; transition: background 0.2s; }
  .btn:hover:not(:disabled) { background: #c65f3c; }
  .btn:disabled { opacity: 0.4; cursor: default; }
  .btn.ghost { margin-left: 0; background: transparent; color: #6f3b24; border: 1px solid #d8c6b1; }
  .btn.ghost:hover:not(:disabled) { background: #f0e6d6; }

  .hp { position: absolute; left: -9999px; width: 1px; height: 1px; opacity: 0; }

  /* Success */
  .sealed { text-align: center; padding: 2rem 0; }
  .wax { width: 4.5rem; height: 4.5rem; margin: 0 auto 1.5rem; display: grid; place-items: center; border-radius: 50%; background: radial-gradient(circle at 35% 30%, #d8714f, #8f2e1a); color: #f8f1e7; font-size: 1.6rem; box-shadow: 0 10px 24px -10px rgba(143,46,26,0.7); }
  .token-box { display: inline-flex; flex-direction: column; gap: 0.4rem; margin: 1.5rem auto; padding: 1rem 1.5rem; background: #fffaf2; border: 1px solid #d8c6b1; }
  .token-label { font-size: 0.68rem; letter-spacing: 0.12em; text-transform: uppercase; color: #6f3b24; }
  .token { font-family: 'Fraunces', monospace; font-size: 0.95rem; letter-spacing: 0.05em; word-break: break-all; color: #34251c; }
  .token-hint { font-size: 0.75rem; color: #6f3b24; opacity: 0.7; }
  .cta { display: inline-block; margin-top: 1rem; background: #6f3b24; color: #f8f1e7; padding: 0.7rem 1.8rem; text-decoration: none; font-size: 0.85rem; letter-spacing: 0.08em; text-transform: uppercase; transition: background 0.2s; }
  .cta:hover { background: #c65f3c; }

  @media (max-width: 520px) {
    .row { flex-direction: column; gap: 1.1rem; }
    .source-card { grid-template-columns: 56px 1fr; }
    .source-card-img { width: 56px; }
    .ref-tools { flex-direction: column; }
    .ref-series { max-width: none; }
    .letter { padding: 1.3rem; }
  }
</style>
