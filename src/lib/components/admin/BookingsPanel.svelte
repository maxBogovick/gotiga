<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t, brandName } from '$lib/i18n';
  import type { BookingDto, FigurineListItem } from '$lib/types/api';

  let { onPendingCount = (_n: number) => {} } = $props();

  const PER_PAGE = 20;

  let items = $state<BookingDto[]>([]);
  let total = $state(0);
  let pendingCount = $state(0);
  let page = $state(1);
  let loading = $state(true);
  let error = $state('');
  let statusFilter = $state('');
  let figurineFilter = $state('');
  let figurines = $state<FigurineListItem[]>([]);
  let updatingId    = $state<string | null>(null);
  let notesMap      = $state<Record<string, string>>({});
  let curatorMap    = $state<Record<string, string>>({});
  let conflictErrors = $state<Record<string, string>>({});

  // ── Calendar view ──────────────────────────────────────────────
  let calMode = $state(false);
  let calAllBookings = $state<BookingDto[]>([]);
  let calLoading = $state(false);
  let calYear = $state(new Date().getFullYear());
  let calMonth = $state(new Date().getMonth()); // 0-indexed
  let calSelected = $state<string | null>(null); // 'YYYY-MM-DD'

  async function loadCalendar() {
    calLoading = true;
    try {
      const res = await api.listBookings({ page: 1, perPage: 500 });
      calAllBookings = res.items;
    } catch { /* ignore */ } finally {
      calLoading = false;
    }
  }

  async function toggleCalMode() {
    calMode = !calMode;
    if (calMode && calAllBookings.length === 0) await loadCalendar();
  }

  // Map: 'YYYY-MM-DD' → BookingDto[]
  let calByDay = $derived.by(() => {
    const map = new Map<string, BookingDto[]>();
    for (const b of calAllBookings) {
      if (b.status === 'cancelled' || b.status === 'rejected' || b.status === 'completed') continue;
      const start = new Date(b.startsAt + 'T00:00:00');
      const end   = new Date(b.endsAt   + 'T00:00:00');
      for (let d = new Date(start); d <= end; d.setDate(d.getDate() + 1)) {
        const key = d.toISOString().slice(0, 10);
        const arr = map.get(key) ?? [];
        arr.push(b);
        map.set(key, arr);
      }
    }
    return map;
  });

  let calDays = $derived.by(() => {
    const first = new Date(calYear, calMonth, 1);
    let startDow = first.getDay();
    startDow = startDow === 0 ? 6 : startDow - 1; // Mon=0 … Sun=6
    const daysInMonth = new Date(calYear, calMonth + 1, 0).getDate();
    const cells: Array<{ date: Date | null; key: string | null }> = [];
    for (let i = 0; i < startDow; i++) cells.push({ date: null, key: null });
    for (let d = 1; d <= daysInMonth; d++) {
      const date = new Date(calYear, calMonth, d);
      cells.push({ date, key: date.toISOString().slice(0, 10) });
    }
    while (cells.length % 7 !== 0) cells.push({ date: null, key: null });
    return cells;
  });

  function calPrev() {
    if (calMonth === 0) { calMonth = 11; calYear--; } else calMonth--;
    calSelected = null;
  }
  function calNext() {
    if (calMonth === 11) { calMonth = 0; calYear++; } else calMonth++;
    calSelected = null;
  }

  const MONTH_NAMES = ['Янв','Фев','Мар','Апр','Май','Июн','Июл','Авг','Сен','Окт','Ноя','Дек'];
  const DOW = ['Пн','Вт','Ср','Чт','Пт','Сб','Вс'];
  let calSelectedBookings = $derived(calSelected ? (calByDay.get(calSelected) ?? []) : []);
  const todayKey = new Date().toISOString().slice(0, 10);

  function isStale(b: BookingDto): boolean {
    return b.status === 'pending' && b.endsAt < todayKey;
  }

  let totalPages = $derived(Math.max(1, Math.ceil(total / PER_PAGE)));

  async function load(resetPage = false) {
    if (resetPage) page = 1;
    loading = true;
    error = '';
    try {
      const res = await api.listBookings({
        status: statusFilter || undefined,
        figurineId: figurineFilter || undefined,
        page,
        perPage: PER_PAGE,
      });
      items        = res.items;
      total        = res.total;
      pendingCount = res.pendingCount;
      onPendingCount(res.pendingCount);
    } catch {
      error = $t('adminBookingsLoadError');
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
      const curator = curatorMap[booking.id] ?? undefined;
      await api.updateBookingStatus(booking.id, status, notes, curator);
      await load();
    } catch (err) {
      const raw = err instanceof Error ? err.message : '';
      // Extract JSON error body from "API 409: {\"error\":\"...\"}"
      const match = raw.match(/API \d+: (.+)$/s);
      let msg = $t('adminBookingsUpdateError');
      if (match) {
        try { msg = JSON.parse(match[1]).error ?? msg; } catch { /* keep default */ }
      }
      conflictErrors = { ...conflictErrors, [booking.id]: msg };
    } finally {
      updatingId = null;
    }
  }

  onMount(async () => {
    figurines = await api.getAllFigurinesAdmin().catch(() => []);
    load();
  });

  let statusLabel = $derived<Record<string, string>>({
    pending: $t('adminBookingsPending'), confirmed: $t('adminBookingsConfirmed'), rejected: $t('adminBookingsRejected'), cancelled: $t('adminBookingsCancelled'), completed: $t('adminBookingsReturned'),
  });
  const statusColor: Record<string, string> = {
    pending:   'bg-amber-100 text-amber-800 border-amber-200',
    confirmed: 'bg-green-100 text-green-800 border-green-200',
    rejected:  'bg-red-100 text-red-800 border-red-200',
    cancelled: 'bg-gray-100 text-gray-600 border-gray-200',
    completed: 'bg-teal-50 text-teal-800 border-teal-200',
  };

  function makeMailtoLink(booking: BookingDto, type: 'confirm' | 'reject'): string {
    const subject = encodeURIComponent(
      type === 'confirm'
        ? `Your viewing request is confirmed: ${booking.figurineName}`
        : `Your viewing request: ${booking.figurineName}`
    );
    const period = `${formatDate(booking.startsAt)} — ${formatDate(booking.endsAt)}`;
    const body = encodeURIComponent(
      type === 'confirm'
        ? `Hello, ${booking.requesterName}!\n\nYour request to view the figure “${booking.figurineName}” (${period}) has been confirmed.${booking.adminNotes ? `\n\nNote: ${booking.adminNotes}` : ''}\n\nBest regards,\n${$brandName}`
        : `Hello, ${booking.requesterName}!\n\nUnfortunately, the request to view the figure “${booking.figurineName}” (${period}) cannot be accepted.${booking.adminNotes ? `\n\nReason: ${booking.adminNotes}` : ''}\n\nBest regards,\n${$brandName}`
    );
    return `mailto:${booking.requesterEmail}?subject=${subject}&body=${body}`;
  }

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
      {$t('adminTabBookings')}
      {#if pendingCount > 0}
        <span class="ml-2 inline-flex items-center justify-center w-5 h-5 rounded-full bg-amber-500 text-white text-[10px] font-bold">{pendingCount}</span>
      {/if}
    </h2>

    <!-- Figurine filter -->
    {#if figurines.length > 0}
      <select
        value={figurineFilter}
        onchange={(e) => { figurineFilter = (e.target as HTMLSelectElement).value; load(true); }}
        class="ml-auto text-[10px] border border-[#34251c]/20 text-[#5f4636] bg-[#fff9f0] px-2 py-1 focus:outline-none focus:border-[#34251c]/50 max-w-[180px] truncate"
      >
        <option value="">{$t('adminBookingsAllFigurines')}</option>
        {#each figurines as f}
          <option value={f.id}>{f.name}</option>
        {/each}
      </select>
    {/if}

    <div class="flex gap-1 flex-wrap">
      {#each [['', $t('adminBookingsAll')], ['pending', $t('adminBookingsPending')], ['confirmed', $t('adminBookingsConfirmed')], ['rejected', $t('adminBookingsRejected')], ['cancelled', $t('adminBookingsCancelled')], ['completed', $t('adminBookingsReturned')]] as [val, label]}
        <button
          onclick={() => { statusFilter = val; load(true); }}
          class="px-3 py-1 text-[10px] uppercase tracking-wide border transition-colors
            {statusFilter === val
              ? 'bg-[#34251c] text-[#fff9f0] border-[#34251c]'
              : 'border-[#34251c]/20 text-[#5f4636] hover:border-[#34251c]/50'}"
        >{label}</button>
      {/each}
    </div>

    <button onclick={() => load()} class="text-xs text-[#5f4636] hover:text-[#34251c] border border-[#34251c]/20 px-2 py-1 transition-colors" title={$t('adminRefresh')}>↺</button>

    <button
      onclick={toggleCalMode}
      class="ml-auto text-[10px] uppercase tracking-wide border px-3 py-1 transition-colors
        {calMode ? 'bg-[#34251c] text-[#fff9f0] border-[#34251c]' : 'border-[#34251c]/20 text-[#5f4636] hover:border-[#34251c]/50'}"
    >
      {calMode ? $t('adminBookingsList') : $t('adminBookingsCalendar')}
    </button>
  </div>

  <!-- Calendar view -->
  {#if calMode}
  <div class="flex-1 overflow-y-auto px-6 py-4 flex gap-6">
    <!-- Month grid -->
    <div class="flex-shrink-0" style="min-width:280px">
      <div class="flex items-center justify-between mb-3">
        <button onclick={calPrev} class="w-7 h-7 border border-[#34251c]/20 text-[#5f4636] hover:border-[#34251c]/50 text-xs transition-colors">←</button>
        <span class="text-sm font-['Fraunces'] text-[#34251c]">{MONTH_NAMES[calMonth]} {calYear}</span>
        <button onclick={calNext} class="w-7 h-7 border border-[#34251c]/20 text-[#5f4636] hover:border-[#34251c]/50 text-xs transition-colors">→</button>
      </div>

      {#if calLoading}
        <div class="text-center text-xs text-[#5f4636]/60 py-8">Loading…</div>
      {:else}
        <!-- Day-of-week header -->
        <div class="grid grid-cols-7 mb-1">
          {#each DOW as d}
            <div class="text-center text-[9px] uppercase tracking-wide text-[#5f4636]/50 py-1">{d}</div>
          {/each}
        </div>
        <!-- Days grid -->
        <div class="grid grid-cols-7 gap-0.5">
          {#each calDays as cell}
            {#if cell.key}
              {@const dayBookings = calByDay.get(cell.key) ?? []}
              {@const hasPending   = dayBookings.some(b => b.status === 'pending')}
              {@const hasConfirmed = dayBookings.some(b => b.status === 'confirmed')}
              {@const isToday = cell.key === todayKey}
              {@const isSelected = cell.key === calSelected}
              <button
                onclick={() => calSelected = calSelected === cell.key ? null : cell.key}
                class="relative flex flex-col items-center pt-1 pb-1.5 min-h-[36px] border transition-colors
                  {isSelected   ? 'bg-[#34251c] border-[#34251c]'      :
                   isToday      ? 'border-[#c65f3c]/50 bg-[#fff9f0]'   :
                   dayBookings.length > 0 ? 'border-[#34251c]/15 bg-white hover:border-[#34251c]/40 cursor-pointer' :
                                  'border-transparent hover:border-[#34251c]/10 cursor-default'}"
              >
                <span class="text-[11px] leading-none {isSelected ? 'text-[#f8f1e7]' : isToday ? 'text-[#c65f3c] font-semibold' : 'text-[#34251c]'}">
                  {cell.date!.getDate()}
                </span>
                {#if dayBookings.length > 0}
                  <div class="flex gap-0.5 mt-1">
                    {#if hasPending}
                      <span class="w-1.5 h-1.5 rounded-full bg-amber-500 {isSelected ? 'opacity-90' : ''}"></span>
                    {/if}
                    {#if hasConfirmed}
                      <span class="w-1.5 h-1.5 rounded-full bg-green-600 {isSelected ? 'opacity-90' : ''}"></span>
                    {/if}
                  </div>
                {/if}
              </button>
            {:else}
              <div class="min-h-[36px]"></div>
            {/if}
          {/each}
        </div>

        <!-- Legend -->
        <div class="flex gap-4 mt-3 text-[9px] text-[#5f4636]/60 uppercase tracking-wide">
          <span class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-amber-500 inline-block"></span>{$t('adminBookingsPending')}</span>
          <span class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-green-600 inline-block"></span>{$t('adminBookingsConfirmed')}</span>
        </div>
      {/if}
    </div>

    <!-- Selected day panel -->
    <div class="flex-1 min-w-0">
      {#if calSelected && calSelectedBookings.length > 0}
        <p class="text-[10px] uppercase tracking-wide text-[#5f4636]/60 mb-3">
          {new Date(calSelected + 'T00:00:00').toLocaleDateString('ru-RU', { day: 'numeric', month: 'long', year: 'numeric' })}
        </p>
        <div class="space-y-2">
          {#each calSelectedBookings as b (b.id)}
            <div class="border border-[#34251c]/10 bg-white p-3 {b.status === 'pending' ? 'border-l-4 border-l-amber-400' : ''}">
              <div class="flex items-start justify-between gap-2">
                <a href="/figurines/{b.figurineId}" target="_blank" rel="noopener"
                   class="font-['Fraunces'] text-sm text-[#34251c] hover:text-[#c65f3c] hover:underline transition-colors">
                  {b.figurineName} ↗
                </a>
                <span class="text-[9px] px-1.5 py-0.5 border rounded flex-shrink-0 {statusColor[b.status]}">{statusLabel[b.status]}</span>
              </div>
              <p class="text-xs text-[#5f4636] mt-0.5">{b.requesterName} · <a href="mailto:{b.requesterEmail}" class="text-[#c65f3c] hover:underline">{b.requesterEmail}</a>{#if b.requesterPhone} · {b.requesterPhone}{/if}</p>
              <p class="text-[10px] text-[#5f4636]/60 mt-0.5">{formatDate(b.startsAt)} — {formatDate(b.endsAt)}</p>
            </div>
          {/each}
        </div>
      {:else if calSelected}
        <p class="text-sm text-[#5f4636]/50 font-['Fraunces'] italic mt-6">{$t('adminBookingsEmpty')}</p>
      {:else}
        <p class="text-sm text-[#5f4636]/40 font-['Fraunces'] italic mt-6">{$t('adminBookingsSelectDay')}</p>
      {/if}
    </div>
  </div>

  <!-- List view -->
  {:else}
  <!-- Content -->
  <div class="flex-1 overflow-y-auto px-6 py-4">
    {#if loading}
      <div class="text-center text-[#5f4636] py-12 text-sm">{$t('adminLoading')}</div>
    {:else if error}
      <div class="text-center text-red-700 py-12 text-sm">{error}</div>
    {:else if items.length === 0}
      <div class="text-center text-[#5f4636]/60 py-12 font-['Fraunces'] text-lg">{$t('adminBookingsNoRequests')}</div>
    {:else}
      <div class="space-y-3">
        {#each items as booking (booking.id)}
          <div class="border border-[#34251c]/10 bg-white p-4 {booking.status === 'pending' ? (isStale(booking) ? 'border-l-4 border-l-orange-600 opacity-70' : 'border-l-4 border-l-amber-400') : ''}">
            <!-- Header -->
            <div class="flex items-start gap-3 mb-2">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 flex-wrap">
                  <a
  href="/figurines/{booking.figurineId}"
  target="_blank"
  rel="noopener noreferrer"
  onclick={(e) => e.stopPropagation()}
  class="font-['Fraunces'] text-[#34251c] font-semibold hover:text-[#c65f3c] hover:underline transition-colors"
>
  {booking.figurineName}
</a>
                </div>
                <div class="flex items-center gap-2 mt-1">
                  <svg width="11" height="11" viewBox="0 0 11 11" fill="none" stroke="#5f4636" stroke-width="1.3">
                    <rect x="0.5" y="1.5" width="10" height="9" rx="0.7"/>
                    <path d="M3 1.5V0.5M8 1.5V0.5M0.5 4.5h10"/>
                  </svg>
                  <span class="text-xs font-['Inter'] font-semibold text-[#34251c]">
                    {formatDate(booking.startsAt)} — {formatDate(booking.endsAt)}
                  </span>
                  {#if isStale(booking)}
                    <span class="text-[9px] px-1.5 py-0.5 bg-orange-100 text-orange-700 border border-orange-300 rounded font-['Inter'] uppercase tracking-wide">{$t('adminBookingsOverdue')}</span>
                  {/if}
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
              {#if booking.requesterPhone}
                · <span class="text-[#5f4636]">{booking.requesterPhone}</span>
              {/if}
            </div>

            <!-- Display type / venue / requirements -->
            {#if booking.displayType || booking.venue || booking.purpose}
              <div class="mt-2 flex flex-wrap gap-2 text-[10px] font-['Inter']">
                {#if booking.displayType}
                  <span class="px-1.5 py-0.5 bg-[#f8f1e7] border border-[#d8c6b1] text-[#5f4636]">
                    {booking.displayType === 'private' ? $t('adminShowingsPrivate') : booking.displayType === 'exhibition' ? $t('adminShowingsExhibition') : $t('adminBookingsPhotoVideo')}
                  </span>
                {/if}
                {#if booking.venue}
                  <span class="text-[#5f4636]/70">📍 {booking.venue}</span>
                {/if}
              </div>
            {/if}
            {#if booking.purpose}
              <p class="text-xs text-[#5f4636] italic border-l-2 border-[#d8c6b1] pl-2 mt-1.5">{booking.purpose}</p>
            {/if}

            <!-- Admin notes (rejection reason) + curator conditions inputs -->
            {#if booking.status === 'pending' || booking.status === 'confirmed'}
              <div class="mt-3 space-y-2">
                <input
                  type="text"
                  placeholder={$t('adminBookingsNotesPH')}
                  value={notesMap[booking.id] ?? booking.adminNotes ?? ''}
                  oninput={(e) => { notesMap[booking.id] = (e.target as HTMLInputElement).value; notesMap = {...notesMap}; }}
                  class="w-full border-b border-[#d8c6b1] bg-transparent text-xs py-1 text-[#34251c] font-['Inter'] focus:outline-none focus:border-[#c65f3c] placeholder-[#5f4636]/40"
                />
                <input
                  type="text"
                  placeholder={$t('adminBookingsCuratorPH')}
                  value={curatorMap[booking.id] ?? booking.curatorConditions ?? ''}
                  oninput={(e) => { curatorMap[booking.id] = (e.target as HTMLInputElement).value; curatorMap = {...curatorMap}; }}
                  class="w-full border-b border-[#d8c6b1] bg-transparent text-xs py-1 text-[#34251c] font-['Inter'] focus:outline-none focus:border-[#c65f3c] placeholder-[#5f4636]/40"
                />
              </div>
            {:else}
              {#if booking.adminNotes}
                <p class="text-xs text-[#5f4636]/70 font-['Inter'] mt-2 italic">{$t('adminBookingsNoteLabel')} {booking.adminNotes}</p>
              {/if}
              {#if booking.curatorConditions}
                <p class="text-xs text-[#34251c] font-['Inter'] mt-1.5 border-l-2 border-green-500/50 pl-2">{$t('adminBookingsCuratorLabel')} {booking.curatorConditions}</p>
              {/if}
            {/if}

            <!-- Conflict error -->
            {#if conflictErrors[booking.id]}
              <div class="mt-2 flex items-start gap-2 px-2 py-1.5 bg-red-50 border border-red-200 rounded text-xs text-red-800 font-['Inter']">
                <span class="flex-shrink-0 font-bold">⚠</span>
                <span>{conflictErrors[booking.id]}</span>
              </div>
            {/if}

            <!-- Actions -->
            <div class="flex gap-1 mt-3 pt-2 border-t border-[#34251c]/5 flex-wrap">
              {#if booking.status === 'pending'}
                <button
                  onclick={() => setStatus(booking, 'confirmed')}
                  disabled={updatingId === booking.id}
                  class="text-[10px] px-3 py-1 bg-green-700 text-white border border-green-700 hover:bg-green-800 transition-colors disabled:opacity-40 font-['Inter'] uppercase tracking-wide"
                >✓ {$t('adminBookingsConfirm')}</button>
                <button
                  onclick={() => setStatus(booking, 'rejected')}
                  disabled={updatingId === booking.id}
                  class="text-[10px] px-3 py-1 border border-red-300 text-red-700 hover:bg-red-50 transition-colors disabled:opacity-40 font-['Inter'] uppercase tracking-wide"
                >✕ {$t('adminBookingsReject')}</button>
              {:else if booking.status === 'confirmed'}
                <button
                  onclick={() => setStatus(booking, 'completed')}
                  disabled={updatingId === booking.id}
                  class="text-[10px] px-3 py-1 bg-teal-700 text-white border border-teal-700 hover:bg-teal-800 transition-colors disabled:opacity-40 font-['Inter'] uppercase tracking-wide"
                >↩ {$t('adminBookingsReturned')}</button>
                <button
                  onclick={() => setStatus(booking, 'cancelled')}
                  disabled={updatingId === booking.id}
                  class="text-[10px] px-3 py-1 border border-gray-300 text-gray-600 hover:bg-gray-50 transition-colors disabled:opacity-40 font-['Inter'] uppercase tracking-wide"
                >{$t('adminFormCancel')}</button>
              {/if}
              <div class="ml-auto flex gap-1">
                {#if booking.status === 'confirmed'}
                  <a
                    href={makeMailtoLink(booking, 'confirm')}
                    class="text-[10px] px-2 py-1 border border-green-600/40 text-green-700 hover:bg-green-50 transition-colors"
                    title="Email about confirmation"
                  >{$t('adminBookingsEmailConfirm')}</a>
                {:else if booking.status === 'rejected'}
                  <a
                    href={makeMailtoLink(booking, 'reject')}
                    class="text-[10px] px-2 py-1 border border-red-300/60 text-red-700 hover:bg-red-50 transition-colors"
                    title="Email about rejection"
                  >{$t('adminBookingsEmailReject')}</a>
                {:else}
                  <a
                    href="mailto:{booking.requesterEmail}?subject=Re: viewing request {booking.figurineName}"
                    class="text-[10px] px-2 py-1 border border-[#c65f3c]/30 text-[#c65f3c] hover:bg-[#c65f3c]/5 transition-colors"
                  >{$t('adminBookingsEmailLink')}</a>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
  {/if}

  <!-- Pagination (list mode only) -->
  {#if !calMode && (totalPages > 1 || total > 0)}
    <div class="flex items-center justify-between px-6 py-3 border-t border-[#34251c]/10 flex-shrink-0 bg-[#fff9f0]">
      <span class="text-[11px] text-[#5f4636]/70">
        {#if total > 0}{(page - 1) * PER_PAGE + 1}–{Math.min(page * PER_PAGE, total)} {$t('authOf')} {total}{/if}
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
