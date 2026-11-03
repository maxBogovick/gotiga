<script lang="ts">
  /**
   * «Переводы» — массовый перевод текстов сайта на другие языки.
   *
   * Разделы: работы, байки, газета, карты, правки текстов сайта, поручения и
   * этюды, термины боя. В каждом три шага: загрузить список записей, которым не
   * хватает текстов на выбранных языках; скопировать задание (правила раздела и
   * сами тексты) и отдать его модели; вставить ответ и сохранить. Сохранение
   * заполняет только пустое, если не отмечено «Заменять уже переведённое».
   * Работы идут через свои ручки (`getMissingTranslations`, `importTranslations`),
   * остальные разделы — через общие (`getMissingSection`, `importSection`).
   * Разбор ответа и тексты заданий — в `$lib/translationPrompt`.
   */
  import { api } from '$lib/api';
  import { t, type TranslationKey } from '$lib/i18n';
  import { fade } from 'svelte/transition';
  import type { TranslationImportReport } from '$lib/types/api';
  import {
    SECTION_LANGS,
    TRANSLATION_SECTIONS,
    parseTranslations,
    sectionPrompt,
    translationPrompt,
    type TranslationSection,
  } from '$lib/translationPrompt';
  import { onMount } from 'svelte';

  let { onMessage }: { onMessage: (text: string, type?: string) => void } = $props();

  /** Строка списка порции: одинаковая для работ и остальных разделов. */
  type Row = { id: string; name: string; need: Record<string, string[]>; photos: number };
  type Batch = { total: number; rows: Row[]; copy: () => string };

  const SECTION_LABEL: Record<TranslationSection, TranslationKey> = {
    works: 'adminTranslationsSecWorks',
    tales: 'adminTranslationsSecTales',
    gazette: 'adminTranslationsSecGazette',
    cards: 'adminTranslationsSecCards',
    site: 'adminTranslationsSecSite',
    errands: 'adminTranslationsSecErrands',
    terms: 'adminTranslationsSecTerms',
  };
  const SECTION_HINT: Record<TranslationSection, TranslationKey> = {
    works: 'adminTranslationsHintWorks',
    tales: 'adminTranslationsHintTales',
    gazette: 'adminTranslationsHintGazette',
    cards: 'adminTranslationsHintCards',
    site: 'adminTranslationsHintSite',
    errands: 'adminTranslationsHintErrands',
    terms: 'adminTranslationsHintTerms',
  };

  let section = $state<TranslationSection>('works');
  let langs = $state<string[]>(['es', 'de', 'it']);
  let limit = $state(10);
  let counts = $state<Partial<Record<TranslationSection, number>>>({});
  let batch = $state<Batch | null>(null);
  let loading = $state(false);
  let copied = $state(false);
  let text = $state('');
  let overwrite = $state(false);
  let saving = $state(false);
  let report = $state<TranslationImportReport | null>(null);

  /** Работы пишутся по-английски, поэтому английского перевода у них нет. */
  let workLangs = $derived(langs.filter((l) => l !== 'en'));
  let parsed = $derived(parseTranslations(text));
  let knownIds = $derived(new Set((batch?.rows ?? []).map((r) => r.id)));
  let unknown = $derived(parsed.items.filter((i) => !knownIds.has(i.id)).length);
  let works = $derived(new Set(parsed.items.map((i) => i.id)).size);
  let alts = $derived(parsed.items.reduce((n, i) => n + Object.keys(i.alts ?? {}).length, 0));

  function toggle(code: string) {
    langs = langs.includes(code) ? langs.filter((l) => l !== code) : [...langs, code];
    counts = {};
    batch = null;
  }

  function pickSection(s: TranslationSection) {
    if (s === section) return;
    section = s;
    batch = null;
    report = null;
    text = '';
  }

  async function loadCounts() {
    if (!langs.length) return;
    try {
      const [rest, w] = await Promise.all([
        api.getTranslationSections(langs),
        workLangs.length ? api.getMissingTranslations(workLangs, 1) : Promise.resolve(null),
      ]);
      const next: Partial<Record<TranslationSection, number>> = { works: w?.total ?? 0 };
      for (const c of rest) next[c.id as TranslationSection] = c.missing;
      counts = next;
    } catch {
      // Счётчики — подсказка; без них вкладка работает.
    }
  }

  async function fetchBatch(): Promise<Batch> {
    if (section === 'works') {
      const b = await api.getMissingTranslations(workLangs, limit);
      return {
        total: b.total,
        rows: b.items.map((i) => ({ id: i.id, name: i.name, need: i.need, photos: i.images.length })),
        copy: () => translationPrompt(b),
      };
    }
    const kind = section;
    const b = await api.getMissingSection(kind, langs, limit);
    return {
      total: b.total,
      rows: b.items.map((i) => ({ id: i.id, name: i.name, need: i.need, photos: 0 })),
      copy: () => sectionPrompt(kind, b),
    };
  }

  async function load() {
    if (loading || !langs.length) return;
    if (section === 'works' && !workLangs.length) return;
    loading = true;
    report = null;
    try {
      batch = await fetchBatch();
      counts = { ...counts, [section]: batch.total };
    } catch (e) {
      onMessage($t('adminTranslationsLoadError') + e, 'error');
    } finally {
      loading = false;
    }
  }

  async function copyTask() {
    if (!batch?.rows.length) return;
    try {
      await navigator.clipboard.writeText(batch.copy());
      copied = true;
      setTimeout(() => (copied = false), 2500);
    } catch (e) {
      onMessage($t('adminTranslationsCopyFailed') + e, 'error');
    }
  }

  async function save() {
    if (saving || !parsed.items.length) return;
    saving = true;
    try {
      report =
        section === 'works'
          ? await api.importTranslations(parsed.items, overwrite)
          : await api.importSection(section, parsed.items, overwrite);
      onMessage(
        $t('adminTranslationsReport')
          .replace('{saved}', String(report.saved))
          .replace('{skipped}', String(report.skipped))
          .replace('{alts}', String(report.alts)),
        report.errors.length ? 'error' : 'success',
      );
      if (!report.errors.length) text = '';
      // Что сохранилось, из выгрузки выпало: показываем следующую порцию.
      batch = await fetchBatch();
      counts = { ...counts, [section]: batch.total };
      loadCounts();
    } catch (e) {
      onMessage($t('adminTranslationsSaveError') + e, 'error');
    } finally {
      saving = false;
    }
  }

  onMount(loadCounts);
</script>

<div in:fade class="h-full overflow-auto p-6 sm:p-8 max-w-5xl mx-auto w-full">
  <h2 class="font-['Fraunces'] text-2xl text-[#34251c] mb-1">{$t('adminTabTranslations')}</h2>
  <p class="text-[12px] text-[#7c6554] mb-2 leading-snug max-w-prose">{$t('adminTranslationsIntro')}</p>
  <ol class="text-[12px] text-[#5f4636] mb-5 leading-snug list-decimal pl-5 space-y-0.5">
    <li>{$t('adminTranslationsStep1')}</li>
    <li>{$t('adminTranslationsStep2')}</li>
    <li>{$t('adminTranslationsStep3')}</li>
  </ol>

  <div class="flex flex-wrap gap-1.5 mb-3" role="tablist">
    {#each TRANSLATION_SECTIONS as s (s)}
      <button
        role="tab"
        aria-selected={section === s}
        onclick={() => pickSection(s)}
        class="flex items-center gap-1.5 px-3 py-1.5 text-[12px] border rounded-[3px] transition-colors {section === s
          ? 'bg-[#6f3b24] text-[#f8f1e7] border-[#6f3b24]'
          : 'border-[#d8c6b1] text-[#34251c] hover:bg-[#efe4d3]'}"
      >
        {$t(SECTION_LABEL[s])}
        {#if counts[s] !== undefined}
          <span
            class="text-[10px] px-1.5 rounded-full {counts[s]
              ? 'bg-[#f3dfb9] text-[#7a4f12]'
              : 'bg-[#dcebd6] text-[#2f6b3a]'}"
          >{counts[s] || '✓'}</span>
        {/if}
      </button>
    {/each}
  </div>
  <p class="text-[12px] text-[#7c6554] mb-4 leading-snug">{$t(SECTION_HINT[section])}</p>

  <div class="flex flex-wrap items-end gap-x-6 gap-y-3 mb-4">
    <fieldset>
      <legend class="text-[11px] uppercase tracking-[0.08em] text-[#5f4636] mb-1">{$t('adminTranslationsLangs')}</legend>
      <div class="flex flex-wrap gap-3">
        {#each SECTION_LANGS as l (l.code)}
          <label class="flex items-center gap-1.5 text-[12px] text-[#34251c] cursor-pointer">
            <input
              type="checkbox"
              checked={langs.includes(l.code)}
              disabled={section === 'works' && l.code === 'en'}
              onchange={() => toggle(l.code)}
            />
            {l.name}
          </label>
        {/each}
      </div>
    </fieldset>
    <label class="text-[12px] text-[#34251c]">
      <span class="block text-[11px] uppercase tracking-[0.08em] text-[#5f4636] mb-1">{$t('adminTranslationsBatch')}</span>
      <select
        bind:value={limit}
        class="bg-[#fdf9f2] border border-[#d8c6b1] rounded-[3px] px-2 py-1 text-[12px] focus:outline-none focus:border-[#c65f3c]"
      >
        {#each [5, 10, 20, 30, 50, 100] as n}
          <option value={n}>{n}</option>
        {/each}
      </select>
    </label>
    <button
      onclick={load}
      disabled={loading || !(section === 'works' ? workLangs.length : langs.length)}
      class="px-4 py-1.5 text-[12px] border border-[#6f3b24] bg-[#6f3b24] text-[#f8f1e7] rounded-[3px] hover:bg-[#5a2f1c] transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
    >{loading ? $t('adminTranslationsLoading') : $t('adminTranslationsLoad')}</button>
  </div>

  {#if batch}
    {#if !batch.rows.length}
      <p class="text-[13px] text-[#2f6b3a] py-6 text-center border border-dashed border-[#d8c6b1] rounded-[3px]">{$t('adminTranslationsNothing')}</p>
    {:else}
      <div class="flex items-center justify-between gap-3 flex-wrap mb-2">
        <p class="text-[12px] text-[#9a6b2f]">
          {$t('adminTranslationsTotal').replace('{n}', String(batch.total)).replace('{m}', String(batch.rows.length))}
        </p>
        <button
          onclick={copyTask}
          class="px-3 py-1.5 text-[12px] border border-[#6f3b24] text-[#6f3b24] rounded-[3px] hover:bg-[#6f3b24] hover:text-[#f8f1e7] transition-colors"
        >{copied ? $t('adminTranslationsCopied') : $t('adminTranslationsCopy')}</button>
      </div>
      <ul class="border border-[#d8c6b1] rounded-[3px] max-h-48 overflow-y-auto divide-y divide-[#e5d7c4] mb-6">
        {#each batch.rows as w (w.id)}
          <li class="flex items-center gap-2 px-2 py-1 text-[12px]">
            <span class="min-w-0 flex-1 truncate text-[#34251c]">{w.name}</span>
            <span class="shrink-0 text-[10px] uppercase text-[#7c6554]">{Object.keys(w.need).join(' · ')}</span>
            {#if w.photos}
              <span class="shrink-0 text-[10px] text-[#7c6554]" title={$t('adminTranslationsPhotos')}>🖼 {w.photos}</span>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}

  <label for="translations-text" class="block text-[11px] uppercase tracking-[0.08em] text-[#5f4636] mb-1">{$t('adminTranslationsField')}</label>
  <textarea
    id="translations-text"
    bind:value={text}
    rows="12"
    spellcheck="false"
    placeholder={$t('adminTranslationsPlaceholder')}
    class="w-full bg-[#fdf9f2] border border-[#d8c6b1] rounded-[3px] p-2 text-[12px] leading-relaxed text-[#34251c] focus:outline-none focus:border-[#c65f3c] font-mono"
  ></textarea>

  {#if text.trim()}
    {#if parsed.error}
      <p class="text-[11px] mt-1 text-[#a33]">⚠ {$t('adminTranslationsBadJson').replace('{e}', parsed.error)}</p>
    {/if}
    {#if parsed.items.length}
      <p class="text-[11px] mt-1 text-[#2f6b3a]">
        {$t('adminTranslationsParsed')
          .replace('{n}', String(parsed.items.length))
          .replace('{w}', String(works))
          .replace('{a}', String(alts))}
      </p>
    {/if}
    {#if unknown}
      <p class="text-[11px] text-[#9a6b2f]">⚠ {$t('adminTranslationsUnknown').replace('{n}', String(unknown))}</p>
    {/if}
  {/if}

  <div class="flex items-center gap-4 mt-3 flex-wrap">
    <button
      onclick={save}
      disabled={saving || !parsed.items.length}
      class="px-4 py-1.5 text-[12px] border border-[#6f3b24] bg-[#6f3b24] text-[#f8f1e7] rounded-[3px] hover:bg-[#5a2f1c] transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
    >{saving ? $t('adminTranslationsSaving') : $t('adminTranslationsSave')}</button>
    <label class="flex items-center gap-1.5 text-[12px] text-[#5f4636] cursor-pointer">
      <input type="checkbox" bind:checked={overwrite} />
      {$t('adminTranslationsOverwrite')}
    </label>
  </div>

  {#if report}
    <p class="text-[12px] mt-3 {report.errors.length ? 'text-[#9a6b2f]' : 'text-[#2f6b3a]'}">
      {$t('adminTranslationsReport')
        .replace('{saved}', String(report.saved))
        .replace('{skipped}', String(report.skipped))
        .replace('{alts}', String(report.alts))}
    </p>
    {#if report.errors.length}
      <p class="text-[11px] text-[#5f4636] mt-1">{$t('adminTranslationsErrors')}</p>
      <ul class="text-[11px] text-[#a33] list-disc pl-5 max-h-40 overflow-y-auto">
        {#each report.errors as e}
          <li>{e}</li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>
