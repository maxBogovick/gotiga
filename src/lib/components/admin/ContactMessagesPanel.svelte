<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { ContactMessageDto } from '$lib/types/api';

  let items = $state<ContactMessageDto[]>([]);
  let loading = $state(true);
  let error = $state('');
  let busyId = $state<string | null>(null);

  async function load() {
    loading = true; error = '';
    try {
      items = await api.adminListContactMessages();
    } catch { error = $t('adminContactMessagesLoadError'); }
    finally { loading = false; }
  }

  async function markRead(id: string) {
    busyId = id;
    try {
      await api.adminMarkContactMessageRead(id);
      items = items.map(m => m.id === id ? { ...m, isRead: true } : m);
    } catch { /* ignore */ }
    finally { busyId = null; }
  }

  async function remove(id: string) {
    busyId = id;
    try { await api.adminRemoveContactMessage(id); items = items.filter(m => m.id !== id); }
    catch { /* ignore */ }
    finally { busyId = null; }
  }

  onMount(load);

  function formatTs(iso: string) {
    return new Date(iso).toLocaleString('ru-RU', { day: '2-digit', month: '2-digit', year: '2-digit', hour: '2-digit', minute: '2-digit' });
  }

  let unreadCount = $derived(items.filter(m => !m.isRead).length);
</script>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Toolbar -->
  <div class="flex items-center gap-3 px-6 py-3 border-b border-[#34251c]/10 flex-shrink-0 bg-[#fff9f0]">
    <h2 class="font-['Fraunces'] text-lg text-[#34251c]">
      {$t('adminContactMessagesHeading')}
      {#if unreadCount > 0}
        <span class="ml-2 inline-flex items-center justify-center min-w-5 h-5 px-1.5 rounded-full bg-[#c65f3c] text-white text-[10px] font-bold">{unreadCount}</span>
      {/if}
    </h2>
    <div class="ml-auto">
      <button onclick={load} class="text-xs text-[#5f4636] hover:text-[#34251c] border border-[#34251c]/20 px-2 py-1 transition-colors" title={$t('adminRefresh')}>↺</button>
    </div>
  </div>

  <div class="flex-1 overflow-y-auto px-6 py-4">
    {#if loading}
      <div class="text-center text-[#5f4636] py-12 text-sm">{$t('adminLoading')}</div>
    {:else if error}
      <div class="text-center text-red-700 py-12 text-sm">{error}</div>
    {:else if items.length === 0}
      <div class="text-center text-[#5f4636]/60 py-12 font-['Fraunces'] text-lg">{$t('adminContactMessagesEmpty')}</div>
    {:else}
      <p class="text-xs text-[#5f4636]/60 mb-3 italic">{$t('adminContactMessagesHint')}</p>
      <div class="space-y-2">
        {#each items as m (m.id)}
          <div class="border p-3 flex items-start gap-3 {m.isRead ? 'border-[#34251c]/10 bg-white' : 'border-[#c65f3c]/40 bg-[#fff6ee]'}">
            <div class="flex-1 min-w-0">
              <p class="text-sm font-medium text-[#34251c]">
                <a href="mailto:{m.email}" class="text-[#c65f3c] hover:underline">{m.email}</a>
                {#if !m.isRead}<span class="ml-2 text-[9px] uppercase tracking-wide text-[#c65f3c]">{$t('adminContactMessagesUnread')}</span>{/if}
              </p>
              <p class="text-sm text-[#34251c]/90 whitespace-pre-wrap mt-1">{m.message}</p>
              <p class="text-[10px] text-[#5f4636]/40 mt-1 uppercase tracking-wide">{m.lang} · {m.source} · {formatTs(m.createdAt)}</p>
            </div>
            <div class="flex flex-col gap-1.5 flex-shrink-0">
              {#if !m.isRead}
                <button
                  onclick={() => markRead(m.id)}
                  disabled={busyId === m.id}
                  class="text-[10px] px-2 py-1 border border-[#6f3b24]/30 text-[#6f3b24] hover:bg-[#6f3b24]/5 transition-colors disabled:opacity-40"
                >{$t('adminContactMessagesMarkRead')}</button>
              {/if}
              <button
                onclick={() => remove(m.id)}
                disabled={busyId === m.id}
                class="text-[10px] px-2 py-1 border border-red-200 text-red-600 hover:bg-red-50 transition-colors disabled:opacity-40"
              >✕</button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
