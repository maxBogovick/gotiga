<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { BookingDto } from '$lib/types/api';

  let { onPendingCount = (_n: number) => {} } = $props();

  const PER_PAGE = 20;

  let items = $state<BookingDto[]>([]);
  let total = $state(0);
  let pendingCount = $state(0);
  let page = $state(1);
  let loading = $state(true);
  let error = $state('');
  let statusFilter = $state('');
  let updatingId    = $state<string | null>(null);
  let notesMap      = $state<Record<string, string>>({});
  let conflictErrors = $state<Record<string, string>>({});

  let totalPages = $derived(Math.max(1, Math.ceil(total / PER_PAGE)));

  async function load(resetPage = false) {
    if (resetPage) page = 1;
    loading = true;
    error = '';
    try {
      const res = await api.listBookings({
        status: statusFilter || undefined,
        page,
        perPage: PER_PAGE,
      });
      items        = res.items;
      total        = res.total;
      pendingCount = res.pendingCount;
      onPendingCount(res.pendingCount);
    } catch {
      error = 'Не удалось загрузить брони';
    } finally {
      loading = false;
    }
  }

  async function goTo(p: number) { page = p; await load(); }

  async function setStatus(booking: BookingDto, status: string) {
    updatingId = booking.id;
    conflictErrors = { ...conflictErrors, [booking.id]: '' };
    try {
      const notes = notesMap[booking.id] ?? undefined;
      await api.updateBookingStatus(booking.id, status, notes);
      await load();
    } catch (err) {
      const raw = err instanceof Error ? err.message : '';
      // Extract JSON error body from "API 409: {\"error\":\"...\"}"
      const match = raw.match(/API \d+: (.+)$/s);
      let msg = 'Ошибка обновления статуса';
      if (match) {
        try { msg = JSON.parse(match[1]).error ?? msg; } catch { /* keep default */ }
      }
      conflictErrors = { ...conflictErrors, [booking.id]: msg };
    } finally {
      updatingId = null;
    }
  }

  onMount(() => load());

  const statusLabel: Record<string, string> = {
    pending: 'Новая', confirmed: 'Подтверждена', rejected: 'Отклонена', cancelled: 'Отменена',
  };
  const statusColor: Record<string, string> = {
    pending:   'bg-amber-100 text-amber-800 border-amber-200',
    confirmed: 'bg-green-100 text-green-800 border-green-200',
    rejected:  'bg-red-100 text-red-800 border-red-200',
    cancelled: 'bg-gray-100 text-gray-600 border-gray-200',
  };

  function formatDate(iso: string) {
    return new Date(iso + 'T00:00:00').toLocaleDateString('ru-RU', {
      day: '2-digit', month: 'short', year: 'numeric'
    });
  }
  function formatTs(iso: string) {
    return new Date(iso).toLocaleString('ru-RU', {
      day: '2-digit', month: '2-digit', year: '2-digit',
      hour: '2-digit', minute: '2-digit',
    });
  }

  function pageList(cur: number, max: number): (number | '…')[] {
    if (max <= 7) return Array.from({ length: max }, (_, i) => i + 1);
    const s = new Set([1, 2, cur - 1, cur, cur + 1, max - 1, max]);
    const sorted = [...s].filter(n => n >= 1 && n <= max).sort((a, b) => a - b);
    const result: (number | '…')[] = [];
    for (let i = 0; i < sorted.length; i++) {
      if (i > 0 && sorted[i] - sorted[i - 1] > 1) result.push('…');
      result.push(sorted[i]);
    }
    return result;
  }
</script>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Toolbar -->
  <div class="flex items-center gap-3 px-6 py-3 border-b border-[#34251c]/10 flex-shrink-0 bg-[#fff9f0]">
    <h2 class="font-['Fraunces'] text-lg text-[#34251c]">
      Брони
      {#if pendingCount > 0}
        <span class="ml-2 inline-flex items-center justify-center w-5 h-5 rounded-full bg-amber-500 text-white text-[10px] font-bold">{pendingCount}</span>
      {/if}
    </h2>

    <div class="flex gap-1 ml-auto flex-wrap">
      {#each [['', 'Все'], ['pending', 'Новые'], ['confirmed', 'Подтверждённые'], ['rejected', 'Отклонённые'], ['cancelled', 'Отменённые']] as [val, label]}
        <button
          onclick={() => { statusFilter = val; load(true); }}
          class="px-3 py-1 text-[10px] uppercase tracking-wide border transition-colors
            {statusFilter === val
              ? 'bg-[#34251c] text-[#fff9f0] border-[#34251c]'
              : 'border-[#34251c]/20 text-[#5f4636] hover:border-[#34251c]/50'}"
        >{label}</button>
      {/each}
    </div>

    <button onclick={() => load()} class="text-xs text-[#5f4636] hover:text-[#34251c] border border-[#34251c]/20 px-2 py-1 transition-colors" title="Обновить">↺</button>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-y-auto px-6 py-4">
    {#if loading}
      <div class="text-center text-[#5f4636] py-12 text-sm">Загрузка…</div>
    {:else if error}
      <div class="text-center text-red-700 py-12 text-sm">{error}</div>
    {:else if items.length === 0}
      <div class="text-center text-[#5f4636]/60 py-12 font-['Fraunces'] text-lg">Заявок нет</div>
    {:else}
      <div class="space-y-3">
        {#each items as booking (booking.id)}
          <div class="border border-[#34251c]/10 bg-white p-4 {booking.status === 'pending' ? 'border-l-4 border-l-amber-400' : ''}">
            <!-- Header -->
            <div class="flex items-start gap-3 mb-2">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 flex-wrap">
                  <a
                    href="/figurines/{booking.figurineId}"
                    target="_blank"
                    rel="noopener"
                    class="font-['Fraunces'] text-[#34251c] font-semibold hover:text-[#c65f3c] hover:underline transition-colors"
                  >{booking.figurineName} ↗</a>
                </div>
                <div class="flex items-center gap-2 mt-1">
                  <svg width="11" height="11" viewBox="0 0 11 11" fill="none" stroke="#5f4636" stroke-width="1.3">
                    <rect x="0.5" y="1.5" width="10" height="9" rx="0.7"/>
                    <path d="M3 1.5V0.5M8 1.5V0.5M0.5 4.5h10"/>
                  </svg>
                  <span class="text-xs font-['Inter'] font-semibold text-[#34251c]">
                    {formatDate(booking.startsAt)} — {formatDate(booking.endsAt)}
                  </span>
                </div>
                <div class="text-xs text-[#5f4636]/60 mt-0.5">{formatTs(booking.createdAt)}</div>
              </div>
              <div class="flex-shrink-0">
                <span class="text-[10px] px-2 py-0.5 border rounded {statusColor[booking.status]}">{statusLabel[booking.status]}</span>
              </div>
            </div>

            <!-- Contact -->
            <div class="text-sm text-[#34251c] mb-1">
              <span class="font-medium">{booking.requesterName}</span> ·
              <a href="mailto:{booking.requesterEmail}" class="text-[#c65f3c] hover:underline">{booking.requesterEmail}</a>
            </div>

            <!-- Purpose -->
            {#if booking.purpose}
              <p class="text-sm text-[#5f4636] italic border-l-2 border-[#d8c6b1] pl-2 mt-1.5">{booking.purpose}</p>
            {/if}

            <!-- Admin notes input -->
            {#if booking.status === 'pending'}
              <div class="mt-3">
                <input
                  type="text"
                  placeholder="Примечание (необязательно)…"
                  value={notesMap[booking.id] ?? ''}
                  oninput={(e) => { notesMap[booking.id] = (e.target as HTMLInputElement).value; notesMap = {...notesMap}; }}
                  class="w-full border-b border-[#d8c6b1] bg-transparent text-xs py-1 text-[#34251c] font-['Inter'] focus:outline-none focus:border-[#c65f3c] placeholder-[#5f4636]/40"
                />
              </div>
            {:else if booking.adminNotes}
              <p class="text-xs text-[#5f4636]/70 font-['Inter'] mt-2 italic">Примечание: {booking.adminNotes}</p>
            {/if}

            <!-- Conflict error -->
            {#if conflictErrors[booking.id]}
              <div class="mt-2 flex items-start gap-2 px-2 py-1.5 bg-red-50 border border-red-200 rounded text-xs text-red-800 font-['Inter']">
                <span class="flex-shrink-0 font-bold">⚠</span>
                <span>{conflictErrors[booking.id]}</span>
              </div>
            {/if}

            <!-- Actions -->
            <div class="flex gap-1 mt-3 pt-2 border-t border-[#34251c]/5">
              {#if booking.status === 'pending'}
                <button
                  onclick={() => setStatus(booking, 'confirmed')}
                  disabled={updatingId === booking.id}
                  class="text-[10px] px-3 py-1 bg-green-700 text-white border border-green-700 hover:bg-green-800 transition-colors disabled:opacity-40 font-['Inter'] uppercase tracking-wide"
                >✓ Подтвердить</button>
                <button
                  onclick={() => setStatus(booking, 'rejected')}
                  disabled={updatingId === booking.id}
                  class="text-[10px] px-3 py-1 border border-red-300 text-red-700 hover:bg-red-50 transition-colors disabled:opacity-40 font-['Inter'] uppercase tracking-wide"
                >✕ Отклонить</button>
              {:else if booking.status === 'confirmed'}
                <button
                  onclick={() => setStatus(booking, 'cancelled')}
                  disabled={updatingId === booking.id}
                  class="text-[10px] px-3 py-1 border border-gray-300 text-gray-600 hover:bg-gray-50 transition-colors disabled:opacity-40 font-['Inter'] uppercase tracking-wide"
                >Отменить</button>
              {/if}
              <a
                href="mailto:{booking.requesterEmail}?subject=Re: бронирование {booking.figurineName}"
                class="ml-auto text-[10px] px-2 py-1 border border-[#c65f3c]/30 text-[#c65f3c] hover:bg-[#c65f3c]/5 transition-colors"
              >✉ Написать</a>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Pagination -->
  {#if totalPages > 1 || total > 0}
    <div class="flex items-center justify-between px-6 py-3 border-t border-[#34251c]/10 flex-shrink-0 bg-[#fff9f0]">
      <span class="text-[11px] text-[#5f4636]/70">
        {#if total > 0}{(page - 1) * PER_PAGE + 1}–{Math.min(page * PER_PAGE, total)} из {total}{/if}
      </span>
      {#if totalPages > 1}
        <div class="flex items-center gap-1">
          <button onclick={() => goTo(page - 1)} disabled={page === 1 || loading}
            class="px-2 py-1 text-xs border border-[#34251c]/20 text-[#5f4636] hover:border-[#34251c]/50 disabled:opacity-30 disabled:cursor-default transition-colors">←</button>
          {#each pageList(page, totalPages) as p}
            {#if p === '…'}
              <span class="px-1 text-xs text-[#5f4636]/40">…</span>
            {:else}
              <button onclick={() => goTo(p as number)} disabled={loading}
                class="w-7 h-7 text-xs border transition-colors disabled:cursor-default
                  {page === p ? 'bg-[#34251c] text-[#fff9f0] border-[#34251c]' : 'border-[#34251c]/20 text-[#5f4636] hover:border-[#34251c]/50'}">{p}</button>
            {/if}
          {/each}
          <button onclick={() => goTo(page + 1)} disabled={page === totalPages || loading}
            class="px-2 py-1 text-xs border border-[#34251c]/20 text-[#5f4636] hover:border-[#34251c]/50 disabled:opacity-30 disabled:cursor-default transition-colors">→</button>
        </div>
      {/if}
    </div>
  {/if}
</div>
