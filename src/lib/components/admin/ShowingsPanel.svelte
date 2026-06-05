<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { ShowingDto, SaveShowingRequest, FigurineListItem } from '$lib/types/api';

  let showings = $state<ShowingDto[]>([]);
  let figurines = $state<FigurineListItem[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let error = $state('');
  let saveMsg = $state('');
  let selectedId = $state<string | null>(null);

  const empty: SaveShowingRequest = {
    figurineId: '',
    title: '',
    showingType: 'exhibition',
    startsAt: '',
    endsAt: '',
    venue: null,
    notes: null,
  };

  let form = $state<SaveShowingRequest & { id?: string }>({ ...empty });

  let selected = $derived(showings.find(s => s.id === selectedId) ?? null);

  async function load() {
    loading = true;
    error = '';
    try {
      [showings, figurines] = await Promise.all([
        api.listShowings(),
        api.getAllFigurinesAdmin(),
      ]);
    } catch {
      error = 'Не удалось загрузить данные';
    } finally {
      loading = false;
    }
  }

  function selectShowing(showing: ShowingDto) {
    selectedId = showing.id;
    form = {
      id: showing.id,
      figurineId: showing.figurineId,
      title: showing.title,
      showingType: showing.showingType,
      startsAt: showing.startsAt,
      endsAt: showing.endsAt,
      venue: showing.venue,
      notes: showing.notes,
    };
    saveMsg = '';
  }

  function createNew() {
    selectedId = '__new__';
    form = { ...empty };
    saveMsg = '';
  }

  async function save() {
    if (!form.figurineId || !form.title || !form.startsAt || !form.endsAt) return;
    saving = true;
    saveMsg = '';
    try {
      const req: SaveShowingRequest = {
        id: form.id,
        figurineId: form.figurineId,
        title: form.title,
        showingType: form.showingType,
        startsAt: form.startsAt,
        endsAt: form.endsAt,
        venue: form.venue || null,
        notes: form.notes || null,
      };
      const saved = await api.saveShowing(req);
      saveMsg = 'Сохранено ✓';
      await load();
      selectedId = saved.id;
      form = { ...req, id: saved.id };
    } catch {
      saveMsg = 'Ошибка сохранения';
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!selected || !confirm('Удалить этот показ?')) return;
    try {
      await api.deleteShowing(selected.id);
      selectedId = null;
      form = { ...empty };
      await load();
    } catch {
      error = 'Ошибка удаления';
    }
  }

  function figurineName(id: string) {
    return figurines.find(f => f.id === id)?.name ?? id;
  }

  function formatDate(iso: string) {
    return new Date(iso + 'T00:00:00').toLocaleDateString('ru-RU', {
      day: '2-digit', month: 'short', year: 'numeric'
    });
  }

  onMount(() => load());
</script>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Toolbar -->
  <div class="flex items-center gap-3 px-6 py-3 border-b border-[#34251c]/10 flex-shrink-0 bg-[#fff9f0]">
    <h2 class="font-['Fraunces'] text-lg text-[#34251c]">Расписание показов</h2>
    <button
      onclick={createNew}
      class="ml-auto px-3 py-1.5 text-[11px] uppercase tracking-wide border border-[#34251c]/30 text-[#34251c] hover:bg-[#34251c]/5 transition-colors"
    >✚ Новый показ</button>
    <button onclick={() => load()} class="text-xs text-[#5f4636] hover:text-[#34251c] border border-[#34251c]/20 px-2 py-1 transition-colors" title="Обновить">↺</button>
  </div>

  {#if error}
    <div class="px-6 py-2 text-sm text-red-700">{error}</div>
  {/if}

  <div class="flex-1 overflow-hidden flex">
    <!-- List -->
    <div class="w-72 flex-shrink-0 border-r border-[#34251c]/10 overflow-y-auto bg-white/40">
      {#if loading}
        <div class="p-6 text-center text-sm text-[#5f4636]">Загрузка…</div>
      {:else if showings.length === 0}
        <div class="p-6 text-center text-[#5f4636]/60 font-['Fraunces'] text-base">Показов нет</div>
      {:else}
        {#each showings as s}
          <button
            onclick={() => selectShowing(s)}
            class="w-full text-left px-4 py-3 border-b border-[#34251c]/5 transition-colors hover:bg-[#fff9f0]
              {selectedId === s.id ? 'bg-[#fff9f0] border-l-2 border-l-[#c65f3c]' : ''}"
          >
            <div class="flex items-center gap-2 mb-0.5">
              <span class="text-[9px] uppercase tracking-wide font-bold font-['Inter'] px-1.5 py-0.5 rounded-sm
                {s.showingType === 'exhibition' ? 'bg-amber-100 text-amber-800' : 'bg-purple-100 text-purple-800'}">
                {s.showingType === 'exhibition' ? 'Выставка' : 'Частный'}
              </span>
            </div>
            <div class="text-sm font-['Fraunces'] text-[#34251c] truncate">{s.title}</div>
            <div class="text-[11px] text-[#5f4636]/70 font-['Inter'] mt-0.5">{figurineName(s.figurineId)}</div>
            <div class="text-[11px] text-[#5f4636]/60 font-['Inter']">{formatDate(s.startsAt)} — {formatDate(s.endsAt)}</div>
          </button>
        {/each}
      {/if}
    </div>

    <!-- Editor -->
    <div class="flex-1 overflow-y-auto px-6 py-5">
      {#if selectedId === null}
        <div class="flex items-center justify-center h-full text-[#5f4636]/50 font-['Fraunces'] text-base">
          Выберите показ или создайте новый
        </div>
      {:else}
        <form class="space-y-5 max-w-lg" onsubmit={(e) => { e.preventDefault(); save(); }}>

          <!-- Figurine -->
          <div>
            <label class="block text-xs font-['Inter'] font-bold tracking-[0.06em] text-[#5f4636] uppercase mb-1.5">Фигурка *</label>
            <select
              bind:value={form.figurineId}
              required
              class="w-full border border-[#d8c6b1] bg-[#fff9f0] text-sm text-[#34251c] px-3 py-2 focus:outline-none focus:border-[#c65f3c]/50"
            >
              <option value="">— Выберите фигурку —</option>
              {#each figurines as f}
                <option value={f.id}>{f.name}</option>
              {/each}
            </select>
          </div>

          <!-- Type -->
          <div>
            <label class="block text-xs font-['Inter'] font-bold tracking-[0.06em] text-[#5f4636] uppercase mb-1.5">Тип *</label>
            <div class="flex gap-3">
              {#each [['exhibition', 'Выставка'], ['private', 'Частный показ']] as [val, lbl]}
                <label class="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="showing-type"
                    value={val}
                    bind:group={form.showingType}
                    class="accent-[#c65f3c]"
                  />
                  <span class="text-sm font-['Inter'] text-[#34251c]">{lbl}</span>
                </label>
              {/each}
            </div>
          </div>

          <!-- Title -->
          <div>
            <label for="showing-title" class="block text-xs font-['Inter'] font-bold tracking-[0.06em] text-[#5f4636] uppercase mb-1.5">Название *</label>
            <input
              id="showing-title"
              type="text"
              bind:value={form.title}
              required
              placeholder="Например: Осенний вернисаж 2024"
              class="w-full border-b border-[#d8c6b1] bg-transparent py-1.5 text-sm text-[#34251c] font-['Fraunces'] focus:outline-none focus:border-[#c65f3c] transition-colors"
            />
          </div>

          <!-- Dates -->
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label for="showing-starts" class="block text-xs font-['Inter'] font-bold tracking-[0.06em] text-[#5f4636] uppercase mb-1.5">С *</label>
              <input
                id="showing-starts"
                type="date"
                bind:value={form.startsAt}
                required
                class="w-full border-b border-[#d8c6b1] bg-transparent py-1.5 text-sm text-[#34251c] font-['Inter'] focus:outline-none focus:border-[#c65f3c] transition-colors"
              />
            </div>
            <div>
              <label for="showing-ends" class="block text-xs font-['Inter'] font-bold tracking-[0.06em] text-[#5f4636] uppercase mb-1.5">По *</label>
              <input
                id="showing-ends"
                type="date"
                bind:value={form.endsAt}
                min={form.startsAt}
                required
                class="w-full border-b border-[#d8c6b1] bg-transparent py-1.5 text-sm text-[#34251c] font-['Inter'] focus:outline-none focus:border-[#c65f3c] transition-colors"
              />
            </div>
          </div>

          <!-- Venue -->
          <div>
            <label for="showing-venue" class="block text-xs font-['Inter'] font-bold tracking-[0.06em] text-[#5f4636] uppercase mb-1.5">Место проведения</label>
            <input
              id="showing-venue"
              type="text"
              bind:value={form.venue}
              placeholder="Галерея, адрес…"
              class="w-full border-b border-[#d8c6b1] bg-transparent py-1.5 text-sm text-[#34251c] font-['Inter'] focus:outline-none focus:border-[#c65f3c] transition-colors"
            />
          </div>

          <!-- Notes -->
          <div>
            <label for="showing-notes" class="block text-xs font-['Inter'] font-bold tracking-[0.06em] text-[#5f4636] uppercase mb-1.5">Примечания</label>
            <textarea
              id="showing-notes"
              bind:value={form.notes}
              rows="2"
              placeholder="Дополнительная информация…"
              class="w-full border border-[#d8c6b1] bg-[#f8f1e7] px-3 py-2 text-sm text-[#34251c] font-['Inter'] focus:outline-none focus:border-[#c65f3c]/50 resize-none"
            ></textarea>
          </div>

          <!-- Actions -->
          <div class="flex items-center gap-3 pt-2">
            <button
              type="submit"
              disabled={saving}
              class="px-6 py-2 bg-[#34251c] text-[#fff9f0] text-xs font-['Inter'] uppercase tracking-wide hover:bg-[#6f3b24] transition-colors disabled:opacity-50"
            >{saving ? 'Сохранение…' : 'Сохранить'}</button>

            {#if selected}
              <button
                type="button"
                onclick={remove}
                class="px-4 py-2 border border-red-300 text-red-700 text-xs font-['Inter'] uppercase tracking-wide hover:bg-red-50 transition-colors"
              >Удалить</button>
            {/if}

            {#if saveMsg}
              <span class="text-xs font-['Inter'] {saveMsg.includes('Ошибка') ? 'text-red-700' : 'text-green-700'}">{saveMsg}</span>
            {/if}
          </div>

        </form>
      {/if}
    </div>
  </div>
</div>
