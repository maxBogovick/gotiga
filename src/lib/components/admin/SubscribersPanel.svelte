<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { SubscriberDto } from '$lib/types/api';

  let items = $state<SubscriberDto[]>([]);
  let loading = $state(true);
  let error = $state('');
  let removingId = $state<string | null>(null);

  async function load() {
    loading = true; error = '';
    try {
      items = await api.adminListSubscribers();
    } catch { error = $t('adminSubscribersLoadError'); }
    finally { loading = false; }
  }

  async function remove(id: string) {
    removingId = id;
    try { await api.adminRemoveSubscriber(id); items = items.filter(i => i.id !== id); }
    catch { /* ignore */ }
    finally { removingId = null; }
  }

  function csvCell(v: string): string {
    // RFC-4180 quoting: wrap in quotes and double any inner quote.
    return `"${v.replace(/"/g, '""')}"`;
  }

  function exportCsv() {
    const header = ['email', 'name', 'source', 'lang', 'created_at'];
    const rows = items.map(s =>
      [s.email, s.name ?? '', s.source, s.lang, s.createdAt].map(csvCell).join(',')
    );
    // BOM so Excel reads UTF-8 names correctly.
    const csv = '﻿' + [header.join(','), ...rows].join('\r\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `gotiga-subscribers-${new Date().toISOString().slice(0, 10)}.csv`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  }

  onMount(load);

  function formatTs(iso: string) {
    return new Date(iso).toLocaleString('ru-RU', { day: '2-digit', month: '2-digit', year: '2-digit', hour: '2-digit', minute: '2-digit' });
  }
</script>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Toolbar -->
  <div class="flex items-center gap-3 px-6 py-3 border-b border-[#34251c]/10 flex-shrink-0 bg-[#fff9f0]">
    <h2 class="font-['Fraunces'] text-lg text-[#34251c]">
      {$t('adminSubscribersHeading')}
      {#if items.length > 0}
        <span class="ml-2 inline-flex items-center justify-center min-w-5 h-5 px-1.5 rounded-full bg-[#6f3b24] text-white text-[10px] font-bold">{items.length}</span>
      {/if}
    </h2>
    <div class="ml-auto flex items-center gap-2">
      <button
        onclick={exportCsv}
        disabled={items.length === 0}
        class="text-xs text-[#6f3b24] hover:text-[#34251c] border border-[#6f3b24]/30 hover:border-[#6f3b24]/60 px-3 py-1 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
      >↧ {$t('adminSubscribersExport')}</button>
      <button onclick={load} class="text-xs text-[#5f4636] hover:text-[#34251c] border border-[#34251c]/20 px-2 py-1 transition-colors" title={$t('adminRefresh')}>↺</button>
    </div>
  </div>

  <div class="flex-1 overflow-y-auto px-6 py-4">
    {#if loading}
      <div class="text-center text-[#5f4636] py-12 text-sm">{$t('adminLoading')}</div>
    {:else if error}
      <div class="text-center text-red-700 py-12 text-sm">{error}</div>
    {:else if items.length === 0}
      <div class="text-center text-[#5f4636]/60 py-12 font-['Fraunces'] text-lg">{$t('adminSubscribersEmpty')}</div>
    {:else}
      <p class="text-xs text-[#5f4636]/60 mb-3 italic">{$t('adminSubscribersHint')}</p>
      <div class="space-y-2">
        {#each items as s (s.id)}
          <div class="border border-[#34251c]/10 bg-white p-3 flex items-start gap-3">
            <div class="flex-1 min-w-0">
              <p class="text-sm font-medium text-[#34251c]">
                <a href="mailto:{s.email}" class="text-[#c65f3c] hover:underline">{s.email}</a>
              </p>
              <p class="text-xs text-[#5f4636]">
                {#if s.name}{s.name} · {/if}<span class="uppercase tracking-wide text-[10px] text-[#5f4636]/50">{s.lang} · {s.source}</span>
              </p>
              <p class="text-[10px] text-[#5f4636]/40 mt-1">{formatTs(s.createdAt)}</p>
            </div>
            <button
              onclick={() => remove(s.id)}
              disabled={removingId === s.id}
              class="text-[10px] px-2 py-1 border border-red-200 text-red-600 hover:bg-red-50 transition-colors disabled:opacity-40 flex-shrink-0"
            >✕</button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
