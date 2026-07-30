<script lang="ts">
  /**
   * «Залы показа» — named, shared showing windows several works can point at.
   *
   * A room is hours (minutes from midnight, `until < from` wraps past midnight)
   * plus two optional narrowings: a weekday mask and a calendar date (an annual
   * MM-DD, or a one-off range). The preview clock evaluates every room as a guest
   * would see it at any chosen moment, without touching the system clock.
   *
   * The list itself is owned by the admin page — the figurine form needs the same
   * rooms for its per-work window selector.
   */
  import { api } from '$lib/api';
  import type { ShowingRoom } from '$lib/types/api';
  import { fade } from 'svelte/transition';
  import { t, lang } from '$lib/i18n';
  import { isShowingOpen, roomToWindow, minutesToClock, clockToMinutes } from '$lib/showing-window';

  let {
    rooms,
    onReload,
  }: {
    rooms: ShowingRoom[];
    onReload: () => Promise<void>;
  } = $props();

  // See a window "as a guest would" at any moment — nothing is saved.
  let previewAt = $state<Date>(new Date());

  function toLocalInput(d: Date): string {
    const p = (n: number) => String(n).padStart(2, '0');
    return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
  }

  let roomLocale = $derived($lang === 'ru' ? 'ru-RU' : 'en-US');
  // Mon..Sun short labels (2024-01-01 was a Monday).
  let weekdayLabels = $derived(
    Array.from({ length: 7 }, (_, i) =>
      new Intl.DateTimeFormat(roomLocale, { weekday: 'short' }).format(new Date(2024, 0, 1 + i))
    )
  );
  const dayBit = (mask: number | null | undefined, i: number) => (((mask ?? 0) >> i) & 1) === 1;
  function toggleDay(room: ShowingRoom, i: number) {
    const next = (room.openDaysMask ?? 0) ^ (1 << i);
    room.openDaysMask = next === 0 ? null : next; // no days set → every day
  }

  function dateMode(room: ShowingRoom): 'none' | 'annual' | 'range' {
    if (room.openMonthDay) return 'annual';
    if (room.openDateFrom || room.openDateUntil) return 'range';
    return 'none';
  }
  function setDateMode(room: ShowingRoom, mode: string) {
    room.openMonthDay = null;
    room.openDateFrom = null;
    room.openDateUntil = null;
    if (mode === 'annual') {
      const now = new Date();
      room.openMonthDay = `${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
    } else if (mode === 'range') {
      room.openDateFrom = new Date().toISOString().slice(0, 10);
    }
  }
  // Annual date <-> "MM-DD": a date input wants YYYY-MM-DD, so pad/strip the year.
  const annualToInput = (md: string | null | undefined) => (md ? `2000-${md}` : '');
  const inputToAnnual = (v: string) => (v ? v.slice(5) : null);

  function addRoom() {
    rooms.push({ id: crypto.randomUUID(), name: '', openFromMin: 23 * 60, openUntilMin: 4 * 60 });
  }

  async function saveRoom(room: ShowingRoom) {
    await api.saveShowingRoom(room);
    await onReload();
  }

  async function deleteRoom(id: string) {
    await api.deleteShowingRoom(id);
    await onReload();
  }
</script>

<div in:fade class="h-full overflow-auto p-6 sm:p-8 max-w-3xl mx-auto w-full">
  <h2 class="font-['Fraunces'] text-2xl text-[#34251c] mb-1">{$t('adminTabShowingRooms')}</h2>
  <p class="text-[12px] text-[#7c6554] mb-4 leading-snug max-w-prose">{$t('adminShowingRoomsIntro')}</p>

  <!-- Preview clock: evaluate every room "as a guest would" at this moment. -->
  <div class="flex flex-wrap items-end gap-3 mb-5 p-3 border border-[#34251c]/12 rounded-md bg-[#f3ead9]">
    <label class="block">
      <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminPreviewAt')}</span>
      <input type="datetime-local" value={toLocalInput(previewAt)} oninput={(e) => { if (e.currentTarget.value) previewAt = new Date(e.currentTarget.value); }} class="input-gothic" />
    </label>
    <button type="button" class="text-[11px] uppercase tracking-wide text-[#6f3b24] pb-2" onclick={() => previewAt = new Date()}>{$t('adminPreviewNow')}</button>
  </div>

  <div class="space-y-3">
    {#each rooms as room (room.id)}
      <div class="border border-[#34251c]/12 rounded-md p-3 bg-[#fff9f0] space-y-3">
        <div class="flex flex-wrap gap-3 items-end">
          <label class="block flex-1 min-w-[160px]">
            <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminShowingRoomName')}</span>
            <input bind:value={room.name} class="input-gothic" placeholder={$t('adminShowingRoomNamePlaceholder')} />
          </label>
          <label class="block w-28">
            <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFieldShowingFrom')}</span>
            <input type="time" value={minutesToClock(room.openFromMin)} oninput={(e) => room.openFromMin = clockToMinutes(e.currentTarget.value) ?? 0} class="input-gothic" />
          </label>
          <label class="block w-28">
            <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFieldShowingUntil')}</span>
            <input type="time" value={minutesToClock(room.openUntilMin)} oninput={(e) => room.openUntilMin = clockToMinutes(e.currentTarget.value) ?? 0} class="input-gothic" />
          </label>
          <div class="flex items-center gap-3 pb-2 ml-auto">
            <span class="text-[10px] uppercase tracking-wide px-2 py-1 rounded {isShowingOpen(roomToWindow(room), previewAt) ? 'bg-emerald-600/15 text-emerald-700' : 'bg-[#6f3b24]/12 text-[#6f3b24]'}">
              {isShowingOpen(roomToWindow(room), previewAt) ? $t('adminPreviewOpen') : $t('adminPreviewClosed')}
            </span>
            <button type="button" class="text-[11px] uppercase tracking-wide text-[#c65f3c]" onclick={() => saveRoom(room)}>{$t('adminSave')}</button>
            <button type="button" class="text-[11px] uppercase tracking-wide text-[#7c6554]" onclick={() => deleteRoom(room.id)}>{$t('adminDelete')}</button>
          </div>
        </div>

        <!-- Weekdays: empty = every day, pick e.g. Sat+Sun for weekends. -->
        <div>
          <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminShowingRoomDays')}</span>
          <div class="flex flex-wrap gap-1.5 mt-1">
            {#each weekdayLabels as label, i}
              <button
                type="button"
                class="px-2.5 py-1 rounded text-[11px] border transition-colors {dayBit(room.openDaysMask, i) ? 'bg-[#6f3b24] text-[#f8f1e7] border-[#6f3b24]' : 'border-[#34251c]/20 text-[#7c6554] hover:border-[#6f3b24]/40'}"
                onclick={() => toggleDay(room, i)}
              >{label}</button>
            {/each}
          </div>
        </div>

        <!-- Calendar date: none / annual (MM-DD) / one-off range. -->
        <div class="flex flex-wrap gap-3 items-end">
          <label class="block">
            <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminShowingRoomDate')}</span>
            <select value={dateMode(room)} onchange={(e) => setDateMode(room, e.currentTarget.value)} class="input-gothic">
              <option value="none">{$t('adminShowingDateNone')}</option>
              <option value="annual">{$t('adminShowingDateAnnual')}</option>
              <option value="range">{$t('adminShowingDateRange')}</option>
            </select>
          </label>
          {#if dateMode(room) === 'annual'}
            <label class="block">
              <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminShowingDateAnnual')}</span>
              <input type="date" value={annualToInput(room.openMonthDay)} oninput={(e) => room.openMonthDay = inputToAnnual(e.currentTarget.value)} class="input-gothic" />
            </label>
          {:else if dateMode(room) === 'range'}
            <label class="block">
              <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFieldShowingFrom')}</span>
              <input type="date" bind:value={room.openDateFrom} class="input-gothic" />
            </label>
            <label class="block">
              <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFieldShowingUntil')}</span>
              <input type="date" bind:value={room.openDateUntil} class="input-gothic" />
            </label>
          {/if}
        </div>
      </div>
    {/each}
    {#if rooms.length === 0}
      <p class="text-[12px] italic text-[#7c6554]">{$t('adminShowingRoomsEmpty')}</p>
    {/if}
  </div>

  <button type="button" class="mt-5 px-4 py-2 border border-[#6f3b24]/30 rounded-md text-[11px] uppercase tracking-wide text-[#6f3b24] hover:bg-[#6f3b24]/5" onclick={addRoom}>+ {$t('adminShowingRoomAdd')}</button>
  <p class="text-[10px] text-[#7c6554] mt-3 leading-snug">{$t('adminFieldShowingHint')}</p>
</div>

<style>
  /* Same field chrome as the rest of the admin — scoped styles don't cross
     component boundaries, so the primitives are restated here. */
  .input-gothic {
    width: 100%;
    background-color: #f8f1e7;
    border: 1px solid rgba(198, 95, 60, 0.2);
    padding: 0.65rem 0.75rem;
    font-size: 0.875rem;
    color: #34251c;
    outline: none;
    transition: border-color 0.2s;
    font-family: inherit;
  }

  .input-gothic:focus {
    border-color: rgba(198, 95, 60, 0.55);
  }
</style>
