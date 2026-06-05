<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { ShowingDto, SaveShowingRequest } from '$lib/types/api';

  let { figurineId }: { figurineId: string } = $props();

  let allShowings = $state<ShowingDto[]>([]);
  let loading = $state(false);
  let saving = $state(false);
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

  function startEdit(s: ShowingDto) {
    editingId = s.id;
    form = { id: s.id, figurineId: s.figurineId, title: s.title, showingType: s.showingType,
             startsAt: s.startsAt, endsAt: s.endsAt, venue: s.venue, notes: s.notes };
    saveMsg = '';
  }

  function cancel() { editingId = null; form = emptyForm(); saveMsg = ''; }

  async function save() {
    if (!form.title || !form.startsAt || !form.endsAt) return;
    saving = true;
    saveMsg = '';
    try {
      const req: SaveShowingRequest = { id: form.id, figurineId, title: form.title,
        showingType: form.showingType, startsAt: form.startsAt, endsAt: form.endsAt,
        venue: form.venue || null, notes: form.notes || null };
      const saved = await api.saveShowing(req);
      saveMsg = 'Сохранено ✓';
      await load();
      editingId = saved.id;
      form = { ...req, id: saved.id };
    } catch {
      saveMsg = 'Ошибка сохранения';
    } finally {
      saving = false;
    }
  }

  async function remove(s: ShowingDto) {
    if (!confirm(`Удалить показ «${s.title}»?`)) return;
    try { await api.deleteShowing(s.id); await load(); if (editingId === s.id) cancel(); }
    catch { /* ignore */ }
  }

  function fmt(iso: string) {
    return new Date(iso + 'T00:00:00').toLocaleDateString('ru-RU', { day: '2-digit', month: 'short', year: 'numeric' });
  }

  onMount(() => { if (figurineId) load(); });
</script>

<div class="border-t border-[#34251c]/10 pt-8 mb-8">
  <div class="flex justify-between items-center mb-4">
    <h3 class="text-xl font-gothic">Показы</h3>
    {#if editingId === null}
      <button
        type="button"
        onclick={startAdd}
        class="btn-gothic text-xs"
      >✚ Добавить показ</button>
    {/if}
  </div>

  {#if loading}
    <p class="text-xs text-[#5f4636]/60 py-2">Загрузка…</p>
  {:else if showings.length === 0 && editingId === null}
    <p class="text-xs text-[#5f4636]/50 italic py-2">Показов нет</p>
  {:else}
    <div class="space-y-1.5 mb-3">
      {#each showings as s}
        <div class="flex items-center gap-3 px-3 py-2 bg-[#f8f1e7] border border-[#34251c]/10
          {editingId === s.id ? 'border-[#c65f3c]/30' : ''}">
          <span class="text-[9px] uppercase tracking-wide font-bold font-['Inter'] px-1.5 py-0.5 rounded-sm flex-shrink-0
            {s.showingType === 'exhibition' ? 'bg-amber-100 text-amber-800' : 'bg-purple-100 text-purple-800'}">
            {s.showingType === 'exhibition' ? 'Выставка' : 'Частный'}
          </span>
          <span class="text-sm font-['Fraunces'] text-[#34251c] flex-1 truncate">{s.title}</span>
          <span class="text-[11px] text-[#5f4636]/60 font-['Inter'] flex-shrink-0">{fmt(s.startsAt)} — {fmt(s.endsAt)}</span>
          <button type="button" onclick={() => startEdit(s)}
            class="text-[10px] text-[#5f4636] hover:text-[#34251c] uppercase tracking-wide flex-shrink-0">Ред.</button>
          <button type="button" onclick={() => remove(s)}
            class="text-[10px] text-red-600 hover:text-red-800 flex-shrink-0">✕</button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Inline add/edit form -->
  {#if editingId !== null}
    <form
      class="mt-3 p-4 border border-[#34251c]/15 bg-[#f8f1e7] space-y-3"
      onsubmit={(e) => { e.preventDefault(); save(); }}
    >
      <p class="text-[10px] uppercase tracking-wide text-[#5f4636] font-bold font-['Inter']">
        {editingId === '__new__' ? 'Новый показ' : 'Редактировать показ'}
      </p>

      <!-- Type -->
      <div class="flex gap-4">
        {#each [['exhibition', 'Выставка'], ['private', 'Частный']] as [val, lbl]}
          <label class="flex items-center gap-1.5 cursor-pointer">
            <input type="radio" name="fse-type" value={val} bind:group={form.showingType} class="accent-[#c65f3c]" />
            <span class="text-xs font-['Inter'] text-[#34251c]">{lbl}</span>
          </label>
        {/each}
      </div>

      <!-- Title -->
      <div>
        <span class="label block mb-1">Название *</span>
        <input
          type="text"
          bind:value={form.title}
          required
          class="input-gothic"
          placeholder="Название выставки или показа"
        />
      </div>

      <!-- Dates -->
      <div class="grid grid-cols-2 gap-3">
        <div>
          <span class="label block mb-1">С *</span>
          <input type="date" bind:value={form.startsAt} required class="input-gothic" />
        </div>
        <div>
          <span class="label block mb-1">По *</span>
          <input type="date" bind:value={form.endsAt} min={form.startsAt} required class="input-gothic" />
        </div>
      </div>

      <!-- Venue -->
      <div>
        <span class="label block mb-1">Место</span>
        <input type="text" bind:value={form.venue} class="input-gothic" placeholder="Галерея, адрес…" />
      </div>

      <!-- Actions -->
      <div class="flex items-center gap-3 pt-1">
        <button type="submit" disabled={saving}
          class="btn-gothic text-xs min-w-[100px]">{saving ? 'Сохранение…' : 'Сохранить'}</button>
        <button type="button" onclick={cancel}
          class="btn-gothic text-xs opacity-60">Отмена</button>
        {#if saveMsg}
          <span class="text-xs font-['Inter'] {saveMsg.includes('Ошибка') ? 'text-red-700' : 'text-green-700'}">{saveMsg}</span>
        {/if}
      </div>
    </form>
  {/if}
</div>
