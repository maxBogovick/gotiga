<script lang="ts">
  import type { FigurineSchedule, ScheduleEntry } from '$lib/types/api';
  import { t, lang } from '$lib/i18n';

  let { schedule }: { schedule: FigurineSchedule } = $props();

  const MONTHS = 6;
  const COLLAPSED_MONTHS = 3;
  let showFullCalendar = $state(false);

  // ── Date helpers ──────────────────────────────────────────────
  function startOfDay(d: Date): Date {
    const r = new Date(d);
    r.setHours(0, 0, 0, 0);
    return r;
  }

  function dateOf(ds: string): Date {
    return new Date(ds + 'T00:00:00');
  }

  const today = startOfDay(new Date());

  // Build 6 calendar months starting from today's month
  interface CalMonth {
    year: number;
    month: number; // 0-based
    label: string;
    days: CalDay[];
  }

  interface CalDay {
    date: Date;
    dayNum: number;
    entries: ScheduleEntry[];
    isToday: boolean;
    isEmpty: boolean; // padding cell
  }

  // Map date string → entries that cover it
  function buildDayMap(): Map<string, ScheduleEntry[]> {
    const map = new Map<string, ScheduleEntry[]>();
    for (const e of schedule.entries) {
      const start = dateOf(e.startsAt);
      const end   = dateOf(e.endsAt);
      const cur   = new Date(start);
      while (cur <= end) {
        const key = cur.toISOString().slice(0, 10);
        if (!map.has(key)) map.set(key, []);
        map.get(key)!.push(e);
        cur.setDate(cur.getDate() + 1);
      }
    }
    return map;
  }

  function buildCalendar(): CalMonth[] {
    const dayMap = buildDayMap();
    const months: CalMonth[] = [];

    for (let mi = 0; mi < MONTHS; mi++) {
      const year  = today.getFullYear();
      const month = today.getMonth() + mi;
      const first = new Date(year, month, 1);
      const last  = new Date(year, month + 1, 0);
      const label = first.toLocaleDateString($lang, { month: 'short' }).replace('.', '');

      // Monday-first weekday offset (0=Mon … 6=Sun)
      const startDow = (first.getDay() + 6) % 7;

      const days: CalDay[] = [];

      // Leading empty cells
      for (let p = 0; p < startDow; p++) {
        days.push({ date: new Date(0), dayNum: 0, entries: [], isToday: false, isEmpty: true });
      }

      for (let d = 1; d <= last.getDate(); d++) {
        const date    = new Date(year, month, d);
        const key     = date.toISOString().slice(0, 10);
        const entries = dayMap.get(key) ?? [];
        const isToday = date.getTime() === today.getTime();
        days.push({ date, dayNum: d, entries, isToday, isEmpty: false });
      }

      months.push({ year: first.getFullYear(), month: first.getMonth(), label, days });
    }

    return months;
  }

  // ── Color logic ───────────────────────────────────────────────
  function cellColor(day: CalDay): string {
    if (day.isEmpty) return 'transparent';
    if (day.entries.length === 0) return '';

    // Priority: booking > showing private > showing exhibition > pending
    const e = day.entries[0];
    if (e.entryType === 'booking')  return 'booking';
    if (e.entryType === 'showing' && e.showingType === 'private') return 'private';
    if (e.entryType === 'showing')  return 'exhibition';
    return 'pending';
  }

  // ── Tooltip ───────────────────────────────────────────────────
  interface TooltipEntry {
    name: string;
    dates: string;
  }

  interface TooltipState {
    visible: boolean;
    /** left offset relative to .cal-wrap in px */
    x: number;
    /** top offset relative to .cal-wrap in px */
    y: number;
    entries: TooltipEntry[];
  }

  let tooltip = $state<TooltipState>({ visible: false, x: 0, y: 0, entries: [] });
  let wrapEl = $state<HTMLElement | null>(null);

  function segLabel(e: ScheduleEntry): string {
    if (e.entryType === 'showing')
      return e.title ?? (e.showingType === 'private' ? $t('bookingShowingPrivate') : $t('bookingShowingExhibition'));
    if (e.entryType === 'booking') return $t('tlLegendReserved');
    return $t('tlPendingBooking');
  }

  function fmtRange(e: ScheduleEntry): string {
    const fmt = (ds: string) =>
      new Date(ds + 'T00:00:00').toLocaleDateString($lang, { day: 'numeric', month: 'short' });
    return `${fmt(e.startsAt)} — ${fmt(e.endsAt)}`;
  }

  function showTooltip(event: MouseEvent | FocusEvent, day: CalDay) {
    if (day.isEmpty || day.entries.length === 0) return;

    const cell = event.currentTarget as HTMLElement;
    const cellRect = cell.getBoundingClientRect();
    const wrapRect = wrapEl!.getBoundingClientRect();

    // Position tooltip centred above the cell
    const rawX = cellRect.left - wrapRect.left + cellRect.width / 2;
    const rawY = cellRect.top  - wrapRect.top;

    tooltip = {
      visible: true,
      x: rawX,
      y: rawY,
      entries: day.entries.map(e => ({ name: segLabel(e), dates: fmtRange(e) })),
    };
  }

  function hideTooltip() {
    tooltip = { ...tooltip, visible: false };
  }

  // ── Legend entries (only types present in schedule) ──────────
  const LEGEND = [
    { key: 'exhibition', label: () => $t('tlLegendOnView') },
    { key: 'private',    label: () => $t('bookingShowingPrivate') },
    { key: 'booking',    label: () => $t('tlLegendReserved') },
    { key: 'pending',    label: () => $t('tlLegendPending') },
  ] as const;

  function hasType(key: string): boolean {
    return schedule.entries.some(e => {
      if (key === 'exhibition') return e.entryType === 'showing' && e.showingType !== 'private';
      if (key === 'private')    return e.entryType === 'showing' && e.showingType === 'private';
      return e.entryType === key;
    });
  }

  let calendar = $derived(buildCalendar());
  let visibleCalendar = $derived(showFullCalendar ? calendar : calendar.slice(0, COLLAPSED_MONTHS));
  let canExpandCalendar = $derived(calendar.length > COLLAPSED_MONTHS);
  let hasEntries = $derived(schedule.entries.length > 0);

  // Weekday header labels (Mon–Sun)
  const DOW_KEYS = ['tlMon','tlTue','tlWed','tlThu','tlFri','tlSat','tlSun'] as const;
</script>

{#if hasEntries}
  <div class="cal-wrap" bind:this={wrapEl}>
    <!-- Legend -->
    <div class="cal-legend" aria-label={$t('tlSchedule')}>
      {#each LEGEND as leg}
        {#if hasType(leg.key)}
          <span class="leg-item">
            <span class="leg-swatch leg-{leg.key}" aria-hidden="true"></span>
            {leg.label()}
          </span>
        {/if}
      {/each}
    </div>

    <div class="cal-grid">
      {#each visibleCalendar as month}
        <div class="month-block">
          <p class="month-name">{month.label}</p>

          <!-- Weekday headers -->
          <div class="days-grid">
            {#each DOW_KEYS as k}
              <span class="dow">{$t(k)}</span>
            {/each}

            {#each month.days as day}
              {#if day.isEmpty}
                <span class="day-cell day-empty" aria-hidden="true"></span>
              {:else}
                {@const color = cellColor(day)}
                <button
                  class="day-cell day-{color || 'free'} {day.isToday ? 'day-today' : ''}"
                  aria-label="{day.dayNum} {month.label}{day.entries.length ? ': ' + day.entries.map(segLabel).join(', ') : ''}"
                  onmouseenter={(e) => showTooltip(e, day)}
                  onmouseleave={hideTooltip}
                  onfocus={(e) => showTooltip(e, day)}
                  onblur={hideTooltip}
                >
                  {#if day.entries.length > 1}
                    <span class="multi-dot" aria-hidden="true"></span>
                  {/if}
                </button>
              {/if}
            {/each}
          </div>
        </div>
      {/each}
    </div>

    {#if canExpandCalendar}
      <button type="button" class="cal-toggle" onclick={() => (showFullCalendar = !showFullCalendar)}>
        {showFullCalendar ? $t('tlShowLess') : $t('tlShowMore')}
      </button>
    {/if}

    <!-- Tooltip -->
    {#if tooltip.visible}
      <div
        class="cal-tooltip"
        role="tooltip"
        style="left:{tooltip.x}px; top:{tooltip.y}px;"
      >
        {#each tooltip.entries as entry, i}
          {#if i > 0}<hr class="tt-sep" aria-hidden="true">{/if}
          <p class="tt-name">{entry.name}</p>
          <p class="tt-dates">{entry.dates}</p>
        {/each}
      </div>
    {/if}

  </div>
{/if}

<style>
  /* ── Wrapper ───────────────────────────────────────────────── */
  .cal-wrap {
    position: relative;
    margin: 0.75rem 0 0;
    padding: 1rem;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 8%, transparent);
    border-radius: 8px;
    background: color-mix(in srgb, var(--color-canvas-raised, #fff9f0) 54%, transparent);
    font-family: 'Inter', system-ui, sans-serif;
  }

  /* ── Month grid ────────────────────────────────────────────── */
  .cal-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1.25rem 1rem;
  }

  @media (max-width: 600px) {
    .cal-grid {
      grid-template-columns: 1fr;
      gap: 1rem;
    }
  }

  /* ── Single month ──────────────────────────────────────────── */
  .month-block {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .month-name {
    font-size: 0.6rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: rgba(95, 70, 54, 0.55);
    margin: 0 0 2px;
    text-align: center;
  }

  /* ── 7-column day grid ─────────────────────────────────────── */
  .days-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
  }

  /* Weekday headers */
  .dow {
    font-size: 0.5rem;
    text-align: center;
    color: rgba(95, 70, 54, 0.35);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding-bottom: 2px;
  }

  /* ── Day cells ─────────────────────────────────────────────── */
  .day-cell {
    aspect-ratio: 1;
    width: 100%;
    border-radius: 4px;
    border: none;
    cursor: default;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.1s, filter 0.1s;
    padding: 0;
  }

  .day-empty {
    background: transparent;
    pointer-events: none;
  }

  .day-free {
    background: rgba(52, 37, 28, 0.045);
    cursor: default;
  }

  /* Entry type colors */
  .day-exhibition {
    background: #b45309;
    cursor: pointer;
  }
  .day-private {
    background: #7c3aed;
    cursor: pointer;
  }
  .day-booking {
    background: #9e452d;
    cursor: pointer;
  }
  .day-pending {
    border: 1px solid rgba(217, 119, 6, 0.45);
    background: rgba(217, 119, 6, 0.045);
    cursor: pointer;
  }

  /* Today ring */
  .day-today {
    box-shadow: 0 0 0 1.5px rgba(52, 37, 28, 0.45);
    z-index: 1;
  }

  /* Hover / focus */
  button.day-cell:hover,
  button.day-cell:focus-visible {
    transform: scale(1.25);
    filter: brightness(1.15);
    z-index: 2;
    outline: none;
  }

  /* Multi-entry indicator dot */
  .multi-dot {
    position: absolute;
    bottom: 2px;
    right: 2px;
    width: 3px;
    height: 3px;
    background: rgba(255, 249, 240, 0.86);
    border-radius: 50%;
  }

  /* ── Tooltip ───────────────────────────────────────────────── */
  .cal-tooltip {
    position: absolute;
    /* centred horizontally above the hovered cell, lifted 8px above it */
    transform: translate(-50%, calc(-100% - 10px));
    background: #34251c;
    color: #fff9f0;
    border-radius: 4px;
    padding: 0.4rem 0.6rem;
    white-space: nowrap;
    pointer-events: none;
    z-index: 30;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.28);
    display: flex;
    flex-direction: column;
    gap: 0;
    min-width: 120px;
  }
  .cal-tooltip::after {
    content: '';
    position: absolute;
    top: 100%;
    left: 50%;
    transform: translateX(-50%);
    border: 5px solid transparent;
    border-top-color: #34251c;
  }
  .tt-name {
    font-size: 0.63rem;
    font-weight: 700;
    margin: 0;
    color: #fff9f0;
    letter-spacing: 0.01em;
  }
  .tt-dates {
    font-size: 0.575rem;
    margin: 0.1rem 0 0;
    color: rgba(255, 249, 240, 0.65);
    letter-spacing: 0.01em;
  }
  .tt-sep {
    border: none;
    border-top: 1px solid rgba(255, 249, 240, 0.15);
    margin: 0.35rem 0;
  }

  /* ── Legend ────────────────────────────────────────────────── */
  .cal-legend {
    display: flex;
    gap: 0.75rem 1rem;
    flex-wrap: wrap;
    margin: 0 0 1rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid rgba(52, 37, 28, 0.08);
  }
  .leg-item {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.575rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: rgba(95, 70, 54, 0.65);
  }
  .leg-swatch {
    width: 10px;
    height: 10px;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .leg-exhibition { background: #b45309; }
  .leg-private    { background: #7c3aed; }
  .leg-booking    { background: #9e452d; }
  .leg-pending    { border: 1px solid rgba(217, 119, 6, 0.45); background: rgba(217, 119, 6, 0.045); }

  .cal-toggle {
    display: flex;
    width: fit-content;
    margin: 1rem auto 0;
    padding: 0;
    border: 0;
    color: rgba(95, 70, 54, 0.7);
    background: transparent;
    cursor: pointer;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.6rem;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    text-decoration: underline;
    text-underline-offset: 0.22rem;
  }

  .cal-toggle:hover {
    color: #34251c;
  }

  .cal-toggle:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--color-ember, #b45309) 55%, transparent);
    outline-offset: 3px;
  }
</style>
