<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { untrack } from 'svelte';
  import { get } from 'svelte/store';
  import { api, resolveMediaUrl } from '$lib/api';
  import { t } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';
  import { focusTrap } from '$lib/actions/focusTrap';
  import type { CommissionDto, AttachmentInput } from '$lib/types/api';

  let {
    commission,
    onClose = () => {},
    onSaved = (_c: CommissionDto) => {},
  }: {
    commission: CommissionDto;
    onClose?: () => void;
    onSaved?: (c: CommissionDto) => void;
  } = $props();

  // The modal is mounted fresh per petition, so seed the editable fields from a
  // one-time snapshot of the prop. A snapshot of the translator is enough to
  // detect which stored values correspond to preset options.
  const tr = get(t);
  const SIZE_PRESETS = [
    tr('commissionSizeXs'), tr('commissionSizeS'), tr('commissionSizeM'),
    tr('commissionSizeL'), tr('commissionSizeXl'),
  ];
  const OCCASION_PRESETS = [
    tr('commissionOccasionGift'), tr('commissionOccasionSelf'),
    tr('commissionOccasionCollection'), tr('commissionOccasionMemorial'),
  ];

  let title = $state(untrack(() => commission.title));
  let description = $state(untrack(() => commission.description));
  let sizeValue = $state(untrack(() => commission.sizeNote ?? ''));
  let mood = $state(untrack(() => commission.mood ?? ''));
  let deadline = $state(untrack(() => commission.deadline ?? ''));
  let budget = $state(untrack(() => commission.budgetNote ?? ''));

  // Occasion: preset select + custom fallback.
  const initialOccasion = untrack(() => commission.occasion ?? '');
  const occasionIsPreset = OCCASION_PRESETS.includes(initialOccasion);
  let occasionChoice = $state(initialOccasion === '' ? '' : (occasionIsPreset ? initialOccasion : '__custom__'));
  let occasionCustom = $state(occasionIsPreset ? '' : initialOccasion);
  let occasionValue = $derived(occasionChoice === '__custom__' ? occasionCustom.trim() : occasionChoice);

  let attachments = $state<AttachmentInput[]>(
    untrack(() => commission.attachments.map((a) => ({ url: a.url, thumbUrl: a.thumbUrl })))
  );
  let uploading = $state(false);
  let uploadError = $state('');

  let saving = $state(false);
  let error = $state('');

  // If the saved size is not one of the presets, expose it as a custom row.
  let sizeIsCustom = $derived(sizeValue !== '' && sizeValue !== null && !SIZE_PRESETS.includes(sizeValue));

  async function handleFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files) return;
    uploadError = '';
    for (const file of Array.from(input.files)) {
      if (attachments.length >= 5) { uploadError = $t('commissionTooManyFiles'); break; }
      if (file.size > 8 * 1024 * 1024) { uploadError = $t('commissionFileTooLarge'); continue; }
      uploading = true;
      try {
        if (authStore.sessionToken) {
          const att = await api.uploadUserMedia(authStore.sessionToken, file);
          attachments = [...attachments, att];
        }
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

  async function save() {
    error = '';
    if (!description.trim()) { error = $t('commissionNeedIdea'); return; }
    saving = true;
    const payload = {
      title: title.trim() || null,
      description: description.trim(),
      sizeNote: sizeValue.trim() || null,
      mood: mood.trim() || null,
      deadline: deadline || null,
      budgetNote: budget.trim() || null,
      occasion: occasionValue || null,
      attachmentUrls: attachments,
    };
    try {
      const updated = await api.editCommission(authStore.token!, commission.id, payload);
      onSaved(updated);
    } catch {
      error = $t('profileActionError');
    } finally {
      saving = false;
    }
  }
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape' && !saving) onClose(); }} />

<div
  class="backdrop"
  role="button"
  tabindex="0"
  aria-label={$t('lightboxClose')}
  transition:fade={{ duration: 300 }}
  onclick={(e) => { if (e.target === e.currentTarget && !saving) onClose(); }}
  onkeydown={(e) => { if ((e.key === 'Enter' || e.key === ' ') && e.target === e.currentTarget && !saving) onClose(); }}
>
  <div
    class="sheet"
    role="dialog"
    aria-modal="true"
    aria-label={$t('commissionEditTitle')}
    use:focusTrap
    transition:scale={{ duration: 360, start: 0.96, easing: cubicOut }}
  >
    <header class="sheet-head">
      <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('commissionEditKicker')}</p>
      <h2 class="sheet-title">{$t('commissionEditTitle')}</h2>
      <button class="close" onclick={() => { if (!saving) onClose(); }} aria-label={$t('lightboxClose')}>×</button>
    </header>

    <div class="sheet-body">
      <label class="field">
        <span class="field-label">{$t('commissionFieldTitle')}</span>
        <input class="input" type="text" bind:value={title} maxlength="120" placeholder={$t('commissionFieldTitlePh')} />
      </label>

      <label class="field">
        <span class="field-label">{$t('commissionFieldIdea')} *</span>
        <textarea class="input area" bind:value={description} rows="5" maxlength="5000" placeholder={$t('commissionFieldIdeaPh')}></textarea>
      </label>

      <div class="grid2">
        <label class="field">
          <span class="field-label">{$t('commissionFieldSize')}</span>
          <select class="input" bind:value={sizeValue}>
            <option value="">{$t('commissionSizeUnsure')}</option>
            {#if sizeIsCustom}<option value={sizeValue}>{sizeValue}</option>{/if}
            <option value={$t('commissionSizeXs')}>{$t('commissionSizeXs')}</option>
            <option value={$t('commissionSizeS')}>{$t('commissionSizeS')}</option>
            <option value={$t('commissionSizeM')}>{$t('commissionSizeM')}</option>
            <option value={$t('commissionSizeL')}>{$t('commissionSizeL')}</option>
            <option value={$t('commissionSizeXl')}>{$t('commissionSizeXl')}</option>
          </select>
        </label>
        <label class="field">
          <span class="field-label">{$t('commissionFieldMood')}</span>
          <input class="input" type="text" bind:value={mood} placeholder={$t('commissionFieldMoodPh')} />
        </label>
      </div>

      <div class="grid2">
        <label class="field">
          <span class="field-label">{$t('commissionFieldDeadline')}</span>
          <input class="input" type="date" bind:value={deadline} />
        </label>
        <label class="field">
          <span class="field-label">{$t('commissionFieldBudget')}</span>
          <input class="input" type="text" bind:value={budget} placeholder={$t('commissionFieldBudgetPh')} />
        </label>
      </div>

      <div class="field">
        <span class="field-label">{$t('commissionFieldOccasion')}</span>
        <select class="input" bind:value={occasionChoice}>
          <option value="">{$t('commissionOccasionChoose')}</option>
          <option value={$t('commissionOccasionGift')}>{$t('commissionOccasionGift')}</option>
          <option value={$t('commissionOccasionSelf')}>{$t('commissionOccasionSelf')}</option>
          <option value={$t('commissionOccasionCollection')}>{$t('commissionOccasionCollection')}</option>
          <option value={$t('commissionOccasionMemorial')}>{$t('commissionOccasionMemorial')}</option>
          <option value="__custom__">{$t('commissionOccasionOther')}</option>
        </select>
        {#if occasionChoice === '__custom__'}
          <input class="input mt" type="text" bind:value={occasionCustom} placeholder={$t('commissionOccasionCustomPh')} />
        {/if}
      </div>

      <div class="field">
        <span class="field-label">{$t('commissionFieldRefs')}</span>
        <div class="refs">
          {#each attachments as att, i (att.url)}
            <div class="ref">
              <img src={resolveMediaUrl(att.thumbUrl ?? att.url)} alt="" />
              <button type="button" class="ref-x" onclick={() => removeAttachment(i)} aria-label="×">×</button>
            </div>
          {/each}
          {#if attachments.length < 5}
            <label class="ref-add" title={$t('profileAttachImage')}>
              <input type="file" accept="image/*" multiple hidden onchange={handleFiles} />
              {uploading ? '…' : '+'}
            </label>
          {/if}
        </div>
        {#if uploadError}<p class="err">{uploadError}</p>{/if}
      </div>

      {#if error}<p class="err center">{error}</p>{/if}
    </div>

    <footer class="sheet-foot">
      <button class="btn ghost" onclick={() => { if (!saving) onClose(); }} disabled={saving}>{$t('commissionBack')}</button>
      <button class="btn" onclick={save} disabled={saving}>{saving ? $t('commissionSending') : $t('profileCommissionsSave')}</button>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed; inset: 0; z-index: 240;
    display: flex; align-items: center; justify-content: center;
    padding: 1.25rem;
    background: rgba(111, 59, 36, 0.4);
    backdrop-filter: blur(5px);
  }
  .sheet {
    position: relative;
    width: 100%; max-width: 560px;
    max-height: 92vh;
    display: flex; flex-direction: column;
    background: #fffaf2;
    border: 1px solid #d8c6b1;
    box-shadow: 0 1px 0 #d8c6b1, 0 30px 60px -30px rgba(111, 59, 36, 0.6);
    transform: rotate(-0.4deg);
    font-family: 'Instrument Sans', sans-serif;
  }
  .sheet-head {
    position: relative;
    padding: 1.5rem 1.75rem 1rem;
    border-bottom: 1px solid #e8dcc9;
    flex-shrink: 0;
  }
  .eyebrow { display: flex; align-items: center; gap: 0.6rem; font-size: 0.65rem; letter-spacing: 0.22em; text-transform: uppercase; color: #c65f3c; margin: 0 0 0.5rem; }
  .eyebrow-rule { width: 2rem; height: 1px; background: #c65f3c; }
  .sheet-title { font-family: 'Fraunces', Georgia, serif; font-weight: 400; font-size: 1.7rem; color: #34251c; margin: 0; line-height: 1.1; }
  .close { position: absolute; top: 1rem; right: 1.1rem; width: 2rem; height: 2rem; border: none; background: none; color: #6f3b24; font-size: 1.6rem; line-height: 1; cursor: pointer; opacity: 0.6; transition: opacity 0.2s; }
  .close:hover { opacity: 1; }

  .sheet-body { padding: 1.5rem 1.75rem; overflow-y: auto; display: flex; flex-direction: column; gap: 1.1rem; }
  .grid2 { display: flex; gap: 1rem; }
  .grid2 .field { flex: 1; }
  .field { display: flex; flex-direction: column; gap: 0.4rem; }
  .field-label { font-size: 0.7rem; letter-spacing: 0.1em; text-transform: uppercase; color: #6f3b24; }
  .input { width: 100%; background: #f8f1e7; border: 1px solid #d8c6b1; padding: 0.6rem 0.7rem; font-family: inherit; font-size: 0.95rem; color: #34251c; transition: border-color 0.2s; }
  .input:focus { outline: none; border-color: #c65f3c; }
  .input.mt { margin-top: 0.5rem; }
  .area { resize: vertical; min-height: 6rem; font-family: 'Cormorant Garamond', Georgia, serif; font-size: 1.05rem; }

  .refs { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  .ref { position: relative; width: 68px; height: 68px; border: 1px solid #d8c6b1; overflow: hidden; }
  .ref img { width: 100%; height: 100%; object-fit: cover; }
  .ref-x { position: absolute; top: 0; right: 0; width: 18px; height: 18px; background: rgba(52,37,28,0.82); color: #fff; border: none; cursor: pointer; line-height: 1; font-size: 0.8rem; }
  .ref-add { display: grid; place-items: center; width: 68px; height: 68px; border: 1px dashed #c65f3c; color: #c65f3c; font-size: 1.5rem; cursor: pointer; }

  .err { color: #a3361d; font-size: 0.85rem; }
  .err.center { text-align: center; }

  .sheet-foot { display: flex; gap: 0.75rem; justify-content: flex-end; padding: 1rem 1.75rem 1.5rem; border-top: 1px solid #e8dcc9; flex-shrink: 0; }
  .btn { background: #6f3b24; color: #f8f1e7; border: none; padding: 0.65rem 1.6rem; font-family: inherit; font-size: 0.8rem; letter-spacing: 0.08em; text-transform: uppercase; cursor: pointer; transition: background 0.2s; }
  .btn:hover:not(:disabled) { background: #c65f3c; }
  .btn:disabled { opacity: 0.45; cursor: default; }
  .btn.ghost { background: transparent; color: #6f3b24; border: 1px solid #d8c6b1; }
  .btn.ghost:hover:not(:disabled) { background: #f0e6d6; }

  @media (max-width: 520px) {
    .grid2 { flex-direction: column; gap: 1.1rem; }
    .sheet { transform: none; }
  }
</style>
