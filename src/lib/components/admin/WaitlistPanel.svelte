<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { WaitlistEntryDto } from '$lib/types/api';

  let items      = $state<WaitlistEntryDto[]>([]);
  let loading    = $state(true);
  let error      = $state('');
  let figurineFilter = $state('');
  let removingId  = $state<string | null>(null);
  let notifyingId = $state<string | null>(null);  // figurine_id being notified
  let notifyResult = $state<Record<string, { notified: number; total: number } | null>>({});

  async function load() {
    loading = true; error = '';
    try {
      items = await api.adminListWaitlist(figurineFilter || undefined);
    } catch { error = 'Не удалось загрузить список'; }
    finally { loading = false; }
  }

  async function remove(id: string) {
    removingId = id;
    try { await api.adminRemoveFromWaitlist(id); items = items.filter(i => i.id !== id); }
    catch { /* ignore */ }
    finally { removingId = null; }
  }

  async function notifyAll(figurineId: string) {
    notifyingId = figurineId;
    try {
      const res = await api.adminNotifyWaitlist(figurineId);
      notifyResult = { ...notifyResult, [figurineId]: res };
      // Remove notified entries from local state
      items = items.filter(i => i.figurineId !== figurineId);
    } catch { /* ignore */ }
    finally { notifyingId = null; }
  }

  onMount(load);

  function formatTs(iso: string) {
    return new Date(iso).toLocaleString('ru-RU', { day: '2-digit', month: '2-digit', year: '2-digit', hour: '2-digit', minute: '2-digit' });
  }

  // Group by figurine for readability
  let grouped = $derived.by(() => {
    const map = new Map<string, { figurineName: string; figurineId: string; entries: WaitlistEntryDto[] }>();
    for (const e of items) {
      if (!map.has(e.figurineId)) map.set(e.figurineId, { figurineName: e.figurineName, figurineId: e.figurineId, entries: [] });
      map.get(e.figurineId)!.entries.push(e);
    }
    return [...map.values()];
  });
</script>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Toolbar -->
  <div class="flex items-center gap-3 px-6 py-3 border-b border-[#34251c]/10 flex-shrink-0 bg-[#fff9f0]">
    <h2 class="font-['Fraunces'] text-lg text-[#34251c]">
      Лист ожидания
      {#if items.length > 0}
        <span class="ml-2 inline-flex items-center justify-center w-5 h-5 rounded-full bg-[#6f3b24] text-white text-[10px] font-bold">{items.length}</span>
      {/if}
    </h2>
    <button onclick={load} class="ml-auto text-xs text-[#5f4636] hover:text-[#34251c] border border-[#34251c]/20 px-2 py-1 transition-colors" title="Обновить">↺</button>
  </div>

  <div class="flex-1 overflow-y-auto px-6 py-4">
    {#if loading}
      <div class="text-center text-[#5f4636] py-12 text-sm">Загрузка…</div>
    {:else if error}
      <div class="text-center text-red-700 py-12 text-sm">{error}</div>
    {:else if items.length === 0}
      <div class="text-center text-[#5f4636]/60 py-12 font-['Fraunces'] text-lg">Лист пуст</div>
    {:else}
      <div class="space-y-6">
        {#each grouped as group}
          {@const registered = group.entries.filter(e => e.userId).length}
          <div>
            <div class="flex items-center gap-2 mb-2">
              <a href="/figurines/{group.figurineId}" target="_blank" rel="noopener"
                class="font-['Fraunces'] text-base text-[#34251c] hover:text-[#c65f3c] hover:underline transition-colors">
                {group.figurineName} ↗
              </a>
              <span class="text-[10px] text-[#5f4636]/50 uppercase tracking-wide">{group.entries.length} чел.</span>
              {#if registered > 0}
                <span class="text-[10px] text-[#6a9e5a] uppercase tracking-wide">· {registered} с акк.</span>
              {/if}
              <button
                onclick={() => notifyAll(group.figurineId)}
                disabled={notifyingId === group.figurineId}
                class="ml-auto text-[10px] px-2 py-1 border border-[#6a9e5a]/50 text-[#6a9e5a] hover:bg-[#6a9e5a]/8 transition-colors disabled:opacity-40"
                title="Отправить сообщение всем зарегистрированным пользователям в листе ожидания"
              >
                {notifyingId === group.figurineId ? '…' : '✉ Уведомить всех'}
              </button>
              {#if notifyResult[group.figurineId]}
                <span class="text-[10px] text-[#6a9e5a]">
                  ✓ {notifyResult[group.figurineId]!.notified}/{notifyResult[group.figurineId]!.total}
                </span>
              {/if}
            </div>
            <div class="space-y-2">
              {#each group.entries as entry (entry.id)}
                <div class="border border-[#34251c]/10 bg-white p-3 flex items-start gap-3">
                  <span class="flex-shrink-0 w-7 h-7 rounded-full bg-[#f4ece0] border border-[#d8c6b1] flex items-center justify-center font-serif text-sm text-[#6f3b24]" title="Место в очереди">{entry.position}</span>
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-1.5">
                      <p class="text-sm font-medium text-[#34251c]">{entry.requesterName}</p>
                      {#if entry.userId}
                        <span class="text-[9px] bg-[#e8f4e8] text-[#2d6a3f] px-1.5 py-0.5 rounded" title="Зарегистрированный пользователь">акк</span>
                      {/if}
                    </div>
                    <p class="text-xs text-[#5f4636]">
                      <a href="mailto:{entry.requesterEmail}" class="text-[#c65f3c] hover:underline">{entry.requesterEmail}</a>
                      {#if entry.requesterPhone} · {entry.requesterPhone}{/if}
                    </p>
                    {#if entry.note}
                      <p class="text-xs text-[#5f4636]/70 italic mt-1 border-l-2 border-[#d8c6b1] pl-2">{entry.note}</p>
                    {/if}
                    <p class="text-[10px] text-[#5f4636]/40 mt-1">{formatTs(entry.createdAt)}</p>
                  </div>
                  <div class="flex gap-1 flex-shrink-0">
                    <button
                      onclick={() => remove(entry.id)}
                      disabled={removingId === entry.id}
                      class="text-[10px] px-2 py-1 border border-red-200 text-red-600 hover:bg-red-50 transition-colors disabled:opacity-40"
                    >✕</button>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
