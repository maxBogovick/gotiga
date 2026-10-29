<script lang="ts">
  /**
   * «Подписи для поиска» — скрытый текст работы, по которому её находит поиск.
   *
   * Одна работа за раз: выбрать работу, скопировать задание для модели, приложить
   * к нему фото, вставить ответ в поле, нажать «Сохранить». Сохранение идёт теми же
   * ручками, что и у скрипта подписей (`getFigurineCaption`, `setFigurineCaption`):
   * сервер сам обновляет поиск по смыслу, а поиск по словам видит текст сразу.
   * Что вставили, проверяет `checkCaption`: он не мешает сохранить, а говорит, чего
   * не хватает (нет строки со словами, меньше шести языков).
   */
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import { fade } from 'svelte/transition';
  import type { Figurine, FigurineListItem } from '$lib/types/api';
  import { captionPrompt, checkCaption, type CaptionNote } from '$lib/captionPrompt';

  let {
    figurines,
    onMessage,
  }: {
    figurines: FigurineListItem[];
    onMessage: (text: string, type?: string) => void;
  } = $props();

  let query = $state('');
  let picked = $state<string | null>(null);
  let work = $state<Figurine | null>(null);
  /** Что лежит в базе, и что сейчас в поле. */
  let stored = $state('');
  let text = $state('');
  let loading = $state(false);
  let saving = $state(false);
  let copied = $state(false);
  /** Работы, у которых подпись сохранена на этой странице. */
  let savedHere = $state<Record<string, boolean>>({});

  let shown = $derived(
    figurines.filter((f) => !query.trim() || f.name.toLowerCase().includes(query.trim().toLowerCase())),
  );
  let check = $derived(checkCaption(text));
  let dirty = $derived(check.text !== checkCaption(stored).text);

  async function pick(f: FigurineListItem) {
    if (f.id === picked || loading) return;
    if (dirty && !confirm($t('adminCaptionsUnsaved'))) return;
    picked = f.id;
    loading = true;
    work = null;
    try {
      const [fresh, caption] = await Promise.all([api.getFigurineFresh(f.id), api.getFigurineCaption(f.id)]);
      if (picked !== f.id) return; // за это время выбрали другую
      work = fresh;
      stored = caption ?? '';
      text = stored;
    } catch (e) {
      onMessage($t('adminCaptionsLoadError') + e, 'error');
      picked = null;
    } finally {
      loading = false;
    }
  }

  async function copyPrompt() {
    if (!work) return;
    try {
      await navigator.clipboard.writeText(captionPrompt(work.name, work.shortText ?? ''));
      copied = true;
      setTimeout(() => (copied = false), 2500);
    } catch (e) {
      onMessage($t('adminCaptionsCopyFailed') + e, 'error');
    }
  }

  async function save() {
    if (!work || saving || !check.text) return;
    saving = true;
    try {
      await api.setFigurineCaption(work.id, check.text);
      stored = check.text;
      text = check.text;
      savedHere[work.id] = true;
      onMessage($t('adminCaptionsSaved'), 'success');
    } catch (e) {
      onMessage($t('adminCaptionsError') + e, 'error');
    } finally {
      saving = false;
    }
  }

  async function clear() {
    if (!work || saving || !stored || !confirm($t('adminCaptionsClearConfirm'))) return;
    saving = true;
    try {
      await api.setFigurineCaption(work.id, '');
      stored = '';
      text = '';
      delete savedHere[work.id];
      onMessage($t('adminCaptionsCleared'), 'success');
    } catch (e) {
      onMessage($t('adminCaptionsError') + e, 'error');
    } finally {
      saving = false;
    }
  }

  function noteText(n: CaptionNote): string {
    switch (n.code) {
      case 'noKeywords':
        return $t('adminCaptionsWarnNoKeywords');
      case 'segments':
        return $t('adminCaptionsWarnSegments').replace('{n}', String(n.n));
      case 'few':
        return $t('adminCaptionsWarnFew').replace('{lang}', n.lang).replace('{n}', String(n.n));
      case 'markdown':
        return $t('adminCaptionsWarnMarkdown');
    }
  }
</script>

<div in:fade class="h-full overflow-auto p-6 sm:p-8 max-w-5xl mx-auto w-full">
  <h2 class="font-['Fraunces'] text-2xl text-[#34251c] mb-1">{$t('adminTabCaptions')}</h2>
  <p class="text-[12px] text-[#7c6554] mb-2 leading-snug max-w-prose">{$t('adminCaptionsIntro')}</p>
  <ol class="text-[12px] text-[#5f4636] mb-5 leading-snug list-decimal pl-5 space-y-0.5">
    <li>{$t('adminCaptionsStep1')}</li>
    <li>{$t('adminCaptionsStep2')}</li>
    <li>{$t('adminCaptionsStep3')}</li>
    <li>{$t('adminCaptionsStep4')}</li>
  </ol>

  <div class="grid gap-6 md:grid-cols-[16rem_1fr] items-start">
    <div>
      <input
        bind:value={query}
        placeholder={$t('adminCaptionsFilter')}
        class="w-full mb-2 bg-[#fdf9f2] border border-[#d8c6b1] rounded-[3px] px-2 py-1.5 text-[12px] text-[#34251c] focus:outline-none focus:border-[#c65f3c]"
      />
      <ul class="border border-[#d8c6b1] rounded-[3px] max-h-[60vh] overflow-y-auto divide-y divide-[#e5d7c4]">
        {#each shown as f (f.id)}
          <li>
            <button
              onclick={() => pick(f)}
              class="w-full flex items-center gap-2 px-2 py-1.5 text-left text-[12px] hover:bg-[#efe4d3] {picked === f.id ? 'bg-[#efe4d3]' : ''}"
            >
              {#if f.faceImageUrl}
                <img src={f.faceImageUrl} alt="" loading="lazy" class="w-9 h-9 object-cover rounded-[2px] border border-[#d8c6b1] shrink-0" />
              {:else}
                <span class="w-9 h-9 shrink-0 rounded-[2px] border border-[#d8c6b1] bg-[#f3e9d9]"></span>
              {/if}
              <span class="min-w-0 flex-1 truncate text-[#34251c]">{f.name}</span>
              {#if savedHere[f.id]}
                <span class="text-[10px] text-[#2f6b3a] shrink-0" title={$t('adminCaptionsSavedHere')}>✓</span>
              {/if}
            </button>
          </li>
        {:else}
          <li class="px-3 py-4 text-[12px] text-[#7c6554]">{$t('adminCaptionsNoWorks')}</li>
        {/each}
      </ul>
    </div>

    <div class="min-w-0">
      {#if !picked}
        <p class="text-[13px] text-[#7c6554] py-10 text-center border border-dashed border-[#d8c6b1] rounded-[3px]">{$t('adminCaptionsPick')}</p>
      {:else if loading || !work}
        <p class="text-[13px] text-[#7c6554] py-10 text-center">{$t('adminCaptionsLoading')}</p>
      {:else}
        <div class="flex items-start justify-between gap-3 flex-wrap mb-3">
          <div class="min-w-0">
            <h3 class="font-['Fraunces'] text-xl text-[#34251c] leading-tight">{work.name}</h3>
            <p class="text-[11px] mt-0.5 {stored ? 'text-[#2f6b3a]' : 'text-[#9a6b2f]'}">
              {stored ? $t('adminCaptionsHas') : $t('adminCaptionsNone')}
            </p>
          </div>
          <button
            onclick={copyPrompt}
            class="px-3 py-1.5 text-[12px] border border-[#6f3b24] text-[#6f3b24] rounded-[3px] hover:bg-[#6f3b24] hover:text-[#f8f1e7] transition-colors"
          >{copied ? $t('adminCaptionsCopied') : $t('adminCaptionsCopyPrompt')}</button>
        </div>

        {#if work.images.length}
          <p class="text-[11px] text-[#7c6554] mb-1">{$t('adminCaptionsPhotos')}</p>
          <div class="flex flex-wrap gap-2 mb-4">
            {#each work.images as img (img.id)}
              <a href={img.originalUrl ?? img.url} target="_blank" rel="noopener" title={$t('adminCaptionsPhotoOpen')}>
                <img src={img.thumbUrl ?? img.url} alt="" loading="lazy" class="h-20 w-auto rounded-[2px] border border-[#d8c6b1]" />
              </a>
            {/each}
          </div>
        {/if}

        <label for="caption-text" class="block text-[11px] uppercase tracking-[0.08em] text-[#5f4636] mb-1">{$t('adminCaptionsField')}</label>
        <textarea
          id="caption-text"
          bind:value={text}
          rows="14"
          spellcheck="false"
          placeholder={$t('adminCaptionsPlaceholder')}
          class="w-full bg-[#fdf9f2] border border-[#d8c6b1] rounded-[3px] p-2 text-[12px] leading-relaxed text-[#34251c] focus:outline-none focus:border-[#c65f3c] font-mono"
        ></textarea>

        {#if check.text}
          <p class="text-[11px] mt-1 {check.notes.length ? 'text-[#9a6b2f]' : 'text-[#2f6b3a]'}">
            {#if check.languages}
              {$t('adminCaptionsCounts').replace('{l}', String(check.languages)).replace('{w}', String(check.words))}
            {/if}
          </p>
          {#each check.notes as n}
            <p class="text-[11px] text-[#9a6b2f]">⚠ {noteText(n)}</p>
          {/each}
        {/if}

        <div class="flex items-center gap-3 mt-3 flex-wrap">
          <button
            onclick={save}
            disabled={saving || !check.text || !dirty}
            class="px-4 py-1.5 text-[12px] border border-[#6f3b24] bg-[#6f3b24] text-[#f8f1e7] rounded-[3px] hover:bg-[#5a2f1c] transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
          >{saving ? $t('adminCaptionsSaving') : $t('adminCaptionsSave')}</button>
          {#if stored}
            <button
              onclick={clear}
              disabled={saving}
              class="px-2 py-1 text-[11px] text-[#9a8571] hover:text-[#a33] underline-offset-2 hover:underline disabled:opacity-40"
            >{$t('adminCaptionsClear')}</button>
          {/if}
          {#if dirty && check.text}
            <span class="text-[11px] text-[#7c6554]">{$t('adminCaptionsDirty')}</span>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</div>
