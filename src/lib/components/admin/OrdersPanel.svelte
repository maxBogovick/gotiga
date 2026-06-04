<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { Order } from '$lib/types/api';

  let orders = $state<Order[]>([]);
  let loading = $state(true);
  let error = $state('');
  let statusFilter = $state<'' | 'new' | 'seen' | 'replied'>('');
  let updatingId = $state<string | null>(null);

  async function load() {
    loading = true;
    error = '';
    try {
      orders = await api.listOrders(statusFilter || undefined);
    } catch {
      error = 'Не удалось загрузить заявки';
    } finally {
      loading = false;
    }
  }

  async function setStatus(order: Order, status: 'new' | 'seen' | 'replied') {
    updatingId = order.id;
    try {
      await api.updateOrderStatus(order.id, status);
      order.status = status;
      orders = [...orders];
    } catch {
      // ignore
    } finally {
      updatingId = null;
    }
  }

  onMount(load);

  const statusLabel: Record<string, string> = {
    new: 'Новая',
    seen: 'Просмотрена',
    replied: 'Отвечено',
  };

  const modeLabel: Record<string, string> = {
    request: 'Запрос',
    question: 'Вопрос',
    notify: 'Уведомить',
  };

  const statusColor: Record<string, string> = {
    new:     'bg-red-100 text-red-800 border-red-200',
    seen:    'bg-yellow-100 text-yellow-800 border-yellow-200',
    replied: 'bg-green-100 text-green-800 border-green-200',
  };

  const modeColor: Record<string, string> = {
    request:  'bg-[#c65f3c]/10 text-[#c65f3c]',
    question: 'bg-blue-50 text-blue-700',
    notify:   'bg-purple-50 text-purple-700',
  };

  function formatDate(iso: string) {
    return new Date(iso).toLocaleString('ru-RU', {
      day: '2-digit', month: '2-digit', year: '2-digit',
      hour: '2-digit', minute: '2-digit',
    });
  }

  let newCount = $derived(orders.filter(o => o.status === 'new').length);
</script>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Toolbar -->
  <div class="flex items-center gap-3 px-6 py-3 border-b border-[#34251c]/10 flex-shrink-0 bg-[#fff9f0]">
    <h2 class="font-['Fraunces'] text-lg text-[#34251c]">
      Заявки
      {#if newCount > 0}
        <span class="ml-2 inline-flex items-center justify-center w-5 h-5 rounded-full bg-red-500 text-white text-[10px] font-bold">{newCount}</span>
      {/if}
    </h2>

    <div class="flex gap-1 ml-auto">
      {#each [['', 'Все'], ['new', 'Новые'], ['seen', 'Просмотрены'], ['replied', 'Отвечено']] as [val, label]}
        <button
          onclick={() => { statusFilter = val as typeof statusFilter; load(); }}
          class="px-3 py-1 text-[10px] uppercase tracking-wide border transition-colors
            {statusFilter === val
              ? 'bg-[#34251c] text-[#fff9f0] border-[#34251c]'
              : 'border-[#34251c]/20 text-[#5f4636] hover:border-[#34251c]/50'}"
        >{label}</button>
      {/each}
    </div>

    <button
      onclick={load}
      class="text-xs text-[#5f4636] hover:text-[#34251c] border border-[#34251c]/20 px-2 py-1 transition-colors"
      title="Обновить"
    >↺</button>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-y-auto px-6 py-4">
    {#if loading}
      <div class="text-center text-[#5f4636] py-12 text-sm">Загрузка…</div>
    {:else if error}
      <div class="text-center text-red-700 py-12 text-sm">{error}</div>
    {:else if orders.length === 0}
      <div class="text-center text-[#5f4636]/60 py-12 text-sm font-['Fraunces'] text-lg">Заявок нет</div>
    {:else}
      <div class="space-y-3">
        {#each orders as order (order.id)}
          <div class="border border-[#34251c]/10 bg-white p-4 {order.status === 'new' ? 'border-l-4 border-l-red-400' : ''}">
            <!-- Header row -->
            <div class="flex items-start gap-3 mb-2">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 flex-wrap">
                  <a
                    href="/figurines/{order.figurineId}"
                    target="_blank"
                    rel="noopener"
                    class="font-['Fraunces'] text-[#34251c] font-semibold truncate hover:text-[#c65f3c] hover:underline transition-colors"
                    title="Открыть работу"
                  >{order.figurineName} ↗</a>
                  <span class="text-[10px] px-1.5 py-0.5 rounded {modeColor[order.mode]}">{modeLabel[order.mode]}</span>
                </div>
                <div class="text-xs text-[#5f4636]/60 mt-0.5">{formatDate(order.createdAt)}</div>
              </div>
              <!-- Status badge + changer -->
              <div class="flex items-center gap-1 flex-shrink-0">
                <span class="text-[10px] px-2 py-0.5 border rounded {statusColor[order.status]}">{statusLabel[order.status]}</span>
              </div>
            </div>

            <!-- Contact -->
            <div class="text-sm text-[#34251c] mb-1">
              {#if order.requesterName && order.requesterName !== '—'}
                <span class="font-medium">{order.requesterName}</span> ·
              {/if}
              <a href="mailto:{order.requesterEmail}" class="text-[#c65f3c] hover:underline">{order.requesterEmail}</a>
            </div>

            <!-- Message -->
            {#if order.message}
              <p class="text-sm text-[#5f4636] italic border-l-2 border-[#d8c6b1] pl-2 mt-2">{order.message}</p>
            {/if}

            <!-- Actions -->
            <div class="flex gap-1 mt-3 pt-2 border-t border-[#34251c]/5">
              {#each [['new', 'Новая'], ['seen', 'Просмотрена'], ['replied', 'Отвечено']] as [s, lbl]}
                <button
                  onclick={() => setStatus(order, s as 'new' | 'seen' | 'replied')}
                  disabled={order.status === s || updatingId === order.id}
                  class="text-[10px] px-2 py-1 border transition-colors disabled:opacity-40 disabled:cursor-default
                    {order.status === s
                      ? 'bg-[#34251c]/8 border-[#34251c]/20 text-[#34251c] font-semibold'
                      : 'border-[#34251c]/10 text-[#5f4636] hover:border-[#34251c]/30 hover:text-[#34251c]'}"
                >{lbl}</button>
              {/each}
              <a
                href="mailto:{order.requesterEmail}?subject=Re: {order.figurineName}"
                class="ml-auto text-[10px] px-2 py-1 border border-[#c65f3c]/30 text-[#c65f3c] hover:bg-[#c65f3c]/5 transition-colors"
              >✉ Ответить</a>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
