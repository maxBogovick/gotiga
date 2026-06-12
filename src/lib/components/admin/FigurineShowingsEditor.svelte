<script lang="ts">
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { ShowingDto, SaveShowingRequest } from '$lib/types/api';

  let { figurineId }: { figurineId: string } = $props();

  let allShowings = $state<ShowingDto[]>([]);
  let loading = $state(false);
  let saveMsg = $state('');
  let editingId = $state<string | '__new__' | null>(null);

  const emptyForm = (): SaveShowingRequest => ({
    figurineId,
    title: '',
    showingType: 'exhibition',
    startsAt: '',
    endsAt: '',
    venue: null,
    notes: null,
  });

  let form = $state<SaveShowingRequest & { id?: string }>(emptyForm());

  let showings = $derived(allShowings.filter(s => s.figurineId === figurineId));

  async function load() {
    loading = true;
    try { allShowings = await api.listShowings(); } catch { /* ignore */ } finally { loading = false; }
  }

  function startAdd() {
    editingId = '__new__';
    form = emptyForm();
    saveMsg = '';
  }

  // <input type="date"> требует строго YYYY-MM-DD; отбрасываем возможную time-часть
  const toDateInput = (v: string) => (v || '').slice(0, 10);

  function startEdit(s: ShowingDto) {
    editingId = s.id;
    form = { id: s.id, figurineId: s.figurineId, title: s.title, showingType: s.showingType,
             startsAt: toDateInput(s.startsAt), endsAt: toDateInput(s.endsAt), venue: s.venue, notes: s.notes };
    saveMsg = '';
  }

  function cancel() { editingId = null; form = emptyForm(); saveMsg = ''; }

  // Есть ли в открытой форме хоть какие-то осмысленные данные
  let hasPendingData = $derived(
    editingId !== null && !!(form.title.trim() || form.startsAt || form.endsAt || form.venue?.trim())
  );

  async function save(): Promise<boolean> {
    saveMsg = '';
    if (!form.title.trim()) { saveMsg = $t('adminFigShowingsErrTitle'); return false; }
    if (!form.startsAt || !form.endsAt) { saveMsg = $t('adminFigShowingsErrDates'); return false; }
    // Даты в формате YYYY-MM-DD — лексикографическое сравнение совпадает с хронологией
    if (form.endsAt < form.startsAt) { saveMsg = $t('adminFigShowingsErrOrder'); return false; }
    try {
      const req: SaveShowingRequest = { id: form.id, figurineId, title: form.title.trim(),
        showingType: form.showingType, startsAt: form.startsAt, endsAt: form.endsAt,
        venue: form.venue?.trim() || null, notes: form.notes?.trim() || null };
      const saved = await api.saveShowing(req);
      await load();
      editingId = saved.id;
      form = { ...req, id: saved.id };
      return true;
    } catch {
      saveMsg = $t('adminMsgError');
      return false;
    }
  }

  /**
   * Вызывается родителем при сохранении главной формы фигурины.
   * Если в открытой инлайн-форме есть данные — коммитим показ своим эндпоинтом.
   * Возвращает false, если данные есть, но они невалидны (родитель прервёт сохранение).
   */
  export async function flush(): Promise<boolean> {
    if (!hasPendingData) { if (editingId !== null) cancel(); return true; }
    return save();
  }

  async function remove(s: ShowingDto) {
    if (!confirm($t('adminFigShowingsDeleteConfirm').replace('{title}', s.title))) return;
    try { await api.deleteShowing(s.id); await load(); if (editingId === s.id) cancel(); }
    catch { /* ignore */ }
  }

  function fmt(iso: string) {
    return new Date(iso + 'T00:00:00').toLocaleDateString('ru-RU', { day: '2-digit', month: 'short', year: 'numeric' });
  }

  // При смене выбранной фигурины компонент не ремаунтится (та же позиция в DOM),
  // поэтому сбрасываем состояние формы и перезагружаем данные вручную.
  $effect(() => {
    figurineId; // зависимость
    editingId = null;
    form = emptyForm();
    saveMsg = '';
    if (figurineId) load();
  });
</script>

<div class="border-t border-[#34251c]/10 pt-8 mb-8">
  <div class="flex justify-between items-center mb-4">
    <h3 class="text-xl font-gothic">{$t('adminFigShowingsHeading')}</h3>
    {#if editingId === null}
      <button
        type="button"
        onclick={startAdd}
        class="btn-gothic text-xs"
      >{$t('adminFigShowingsAdd')}</button>
    {/if}
  </div>

  {#if loading}
    <p class="text-xs text-[#5f4636]/60 py-2">{$t('adminFigShowingsLoading')}</p>
  {:else if showings.length === 0 && editingId === null}
    <p class="text-xs text-[#5f4636]/50 italic py-2">{$t('adminFigShowingsEmpty')}</p>
  {:else}
    <div class="space-y-1.5 mb-3">
      {#each showings as s}
        <div class="flex items-center gap-3 px-3 py-2 bg-[#f8f1e7] border border-[#34251c]/10
          {editingId === s.id ? 'border-[#c65f3c]/30' : ''}">
          <span class="text-[9px] uppercase tracking-wide font-bold font-['Inter'] px-1.5 py-0.5 rounded-sm flex-shrink-0
            {s.showingType === 'exhibition' ? 'bg-amber-100 text-amber-800' : 'bg-purple-100 text-purple-800'}">
            {s.showingType === 'exhibition' ? $t('adminFigShowingsTypeExhibition') : $t('adminFigShowingsTypePrivate')}
          </span>
          <span class="text-sm font-['Fraunces'] text-[#34251c] flex-1 truncate">{s.title}</span>
          <span class="text-[11px] text-[#5f4636]/60 font-['Inter'] flex-shrink-0">{fmt(s.startsAt)} — {fmt(s.endsAt)}</span>
          <button type="button" onclick={() => startEdit(s)}
            class="text-[10px] text-[#5f4636] hover:text-[#34251c] uppercase tracking-wide flex-shrink-0">{$t('adminFigShowingsEditRow')}</button>
          <button type="button" onclick={() => remove(s)}
            class="text-[10px] text-red-600 hover:text-red-800 flex-shrink-0">✕</button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Inline add/edit form. Показ коммитится при сохранении главной формы фигурины (flush). -->
  {#if editingId !== null}
    <form
      class="mt-3 p-4 border border-[#34251c]/15 bg-[#f8f1e7] space-y-3"
      onsubmit={(e) => e.preventDefault()}
    >
      <div class="flex items-center justify-between">
        <p class="text-[10px] uppercase tracking-wide text-[#5f4636] font-bold font-['Inter']">
          {editingId === '__new__' ? $t('adminFigShowingsNewTitle') : $t('adminFigShowingsEditTitle')}
        </p>
        <button type="button" onclick={cancel}
          class="text-[10px] text-[#5f4636]/70 hover:text-[#c65f3c] uppercase tracking-wide">✕ {$t('adminFigShowingsClose')}</button>
      </div>

      <!-- Type -->
      <div class="flex gap-4">
        {#each [['exhibition', $t('adminFigShowingsTypeExhibition')], ['private', $t('adminFigShowingsTypePrivate')]] as [val, lbl]}
          <label class="flex items-center gap-1.5 cursor-pointer">
            <input type="radio" name="fse-type" value={val} bind:group={form.showingType} class="accent-[#c65f3c]" />
            <span class="text-xs font-['Inter'] text-[#34251c]">{lbl}</span>
          </label>
        {/each}
      </div>

      <!-- Title -->
      <div>
        <span class="label block mb-1">{$t('adminFigShowingsTitleLabel')}</span>
        <input
          type="text"
          bind:value={form.title}
          required
          class="input-gothic"
          placeholder={$t('adminFigShowingsTitlePlaceholder')}
        />
      </div>

      <!-- Dates -->
      <div class="grid grid-cols-2 gap-3">
        <div>
          <span class="label block mb-1">{$t('adminFigShowingsFrom')}</span>
          <input type="date" bind:value={form.startsAt} required class="input-gothic" />
        </div>
        <div>
          <span class="label block mb-1">{$t('adminFigShowingsTo')}</span>
          <input type="date" bind:value={form.endsAt} min={form.startsAt} required class="input-gothic" />
        </div>
      </div>

      <!-- Venue -->
      <div>
        <span class="label block mb-1">{$t('adminFigShowingsVenue')}</span>
        <input type="text" bind:value={form.venue} class="input-gothic" placeholder={$t('adminFigShowingsVenuePlaceholder')} />
      </div>

      <!-- Hint + validation -->
      <div class="flex items-center gap-3 pt-1 min-h-[1.25rem]">
        {#if saveMsg}
          <span class="text-xs font-['Inter'] text-red-700">{saveMsg}</span>
        {:else}
          <span class="text-[11px] italic font-['Inter'] text-[#5f4636]/60">{$t('adminFigShowingsHint')}</span>
        {/if}
      </div>
    </form>
  {/if}
</div>
