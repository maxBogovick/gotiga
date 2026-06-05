<script lang="ts">
  import type { FigurineSchedule, ScheduleEntry } from '$lib/types/api';
  import { t, lang } from '$lib/i18n';

  let { schedule }: { schedule: FigurineSchedule } = $props();

  const MONTHS = 6;

  function startOfDay(d: Date): Date {
    const r = new Date(d); r.setHours(0,0,0,0); return r;
  }

  const today     = startOfDay(new Date());
  const rangeEnd  = (() => { const d = new Date(today); d.setMonth(d.getMonth() + MONTHS); return d; })();
  const totalMs   = rangeEnd.getTime() - today.getTime();

  function dateOf(ds: string): Date { return new Date(ds + 'T00:00:00'); }

  function clamp(d: Date): Date {
    if (d < today)    return today;
    if (d > rangeEnd) return rangeEnd;
    return d;
  }

  function leftPct(ds: string): number {
    return (clamp(dateOf(ds)).getTime() - today.getTime()) / totalMs * 100;
  }

  function widthPct(from: string, to: string): number {
    const cf = clamp(dateOf(from));
    const ct = clamp(dateOf(to));
    if (ct <= cf) return 0;
    return (ct.getTime() - cf.getTime()) / totalMs * 100;
  }

  // Month ticks — locale-aware
  let ticks = $derived(
    Array.from({ length: MONTHS + 1 }, (_, i) => {
      const d = new Date(today.getFullYear(), today.getMonth() + i, 1);
      if (d < today) return { label: $t('tlToday'), pct: 0 };
      if (d > rangeEnd) return null;
      return {
        label: i === 0
          ? $t('tlToday')
          : d.toLocaleDateString($lang, { month: 'short' }).replace('.', ''),
        pct: (d.getTime() - today.getTime()) / totalMs * 100,
      };
    }).filter(Boolean) as { label: string; pct: number }[]
  );

  // Only entries overlapping the visible range
  let entries = $derived(schedule.entries.filter(e =>
    dateOf(e.endsAt) >= today && dateOf(e.startsAt) <= rangeEnd && widthPct(e.startsAt, e.endsAt) > 0.2
  ));

  let hasEntries = $derived(entries.length > 0);

  function segColor(e: ScheduleEntry): string {
    if (e.entryType === 'showing')  return e.showingType === 'private' ? '#7c3aed' : '#b45309';
    if (e.entryType === 'booking')  return '#9e452d';
    return '#d97706'; // pending
  }

  function segLabel(e: ScheduleEntry): string {
    if (e.entryType === 'showing')
      return e.title ?? (e.showingType === 'private' ? $t('bookingShowingPrivate') : $t('bookingShowingExhibition'));
    if (e.entryType === 'booking') return $t('tlConfirmedBooking');
    return $t('tlPendingBooking');
  }

  function fmtRange(e: ScheduleEntry): string {
    const fmt = (ds: string) => new Date(ds + 'T00:00:00').toLocaleDateString($lang, {
      day: 'numeric', month: 'short'
    });
    return `${fmt(e.startsAt)} — ${fmt(e.endsAt)}`;
  }

  let hovered = $state<number | null>(null);
</script>

{#if hasEntries}
  <div class="timeline-wrap">
    <header class="tl-header">
      <span class="tl-title">{$t('tlSchedule')}</span>
      <div class="tl-rule"></div>
    </header>

    <!-- Track -->
    <div class="tl-track-area" role="img" aria-label="Расписание показов">
      <div class="tl-track">
        <!-- Today marker -->
        <div class="tl-today" title="Сегодня"></div>

        <!-- Segments -->
        {#each entries as e, i}
          {@const left  = leftPct(e.startsAt)}
          {@const width = widthPct(e.startsAt, e.endsAt)}
          {@const color = segColor(e)}
          {@const isPending = e.entryType === 'pending'}
          <div
            class="tl-seg {isPending ? 'tl-seg--pending' : ''}"
            style="left:{left}%; width:{width}%; background:{isPending ? 'transparent' : color}; border-color:{color};"
            role="button"
            tabindex="0"
            aria-label="{segLabel(e)}: {fmtRange(e)}"
            onmouseenter={() => hovered = i}
            onmouseleave={() => hovered = null}
            onfocus={() => hovered = i}
            onblur={() => hovered = null}
          >
            {#if width > 8}
              <span class="tl-seg-label">{segLabel(e)}</span>
            {/if}
          </div>

          <!-- Tooltip -->
          {#if hovered === i}
            <div class="tl-tooltip" style="left:{Math.min(left + width / 2, 88)}%">
              <p class="tl-tt-name">{segLabel(e)}</p>
              <p class="tl-tt-dates">{fmtRange(e)}</p>
            </div>
          {/if}
        {/each}
      </div>

      <!-- Month ticks -->
      <div class="tl-ticks">
        {#each ticks as tick}
          <span class="tl-tick" style="left:{tick.pct}%">{tick.label}</span>
        {/each}
      </div>
    </div>

    <!-- Legend -->
    <div class="tl-legend">
      {#if entries.some(e => e.entryType === 'showing' && e.showingType !== 'private')}
        <span class="leg"><span class="leg-dot" style="background:#b45309"></span>{$t('bookingShowingExhibition')}</span>
      {/if}
      {#if entries.some(e => e.entryType === 'showing' && e.showingType === 'private')}
        <span class="leg"><span class="leg-dot" style="background:#7c3aed"></span>{$t('bookingShowingPrivate')}</span>
      {/if}
      {#if entries.some(e => e.entryType === 'booking')}
        <span class="leg"><span class="leg-dot" style="background:#9e452d"></span>{$t('tlLegendReserved')}</span>
      {/if}
      {#if entries.some(e => e.entryType === 'pending')}
        <span class="leg"><span class="leg-dot leg-dot--pending" style="border-color:#d97706"></span>{$t('tlLegendPending')}</span>
      {/if}
    </div>
  </div>
{/if}

<style>
  .timeline-wrap {
    margin: 1.25rem 0;
    font-family: 'Inter', sans-serif;
  }

  /* Header — matches d-section-header style */
  .tl-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }
  .tl-title {
    font-size: 0.625rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: rgba(95,70,54,0.65);
    font-weight: 700;
    white-space: nowrap;
  }
  .tl-rule {
    flex: 1;
    height: 1px;
    background: rgba(52,37,28,0.1);
  }

  /* Track area */
  .tl-track-area { position: relative; padding-bottom: 1.25rem; }

  .tl-track {
    position: relative;
    height: 20px;
    background: rgba(52,37,28,0.06);
    border-radius: 2px;
    overflow: visible;
  }

  /* Today line */
  .tl-today {
    position: absolute;
    left: 0;
    top: -3px;
    bottom: -3px;
    width: 2px;
    background: rgba(52,37,28,0.3);
    border-radius: 1px;
    z-index: 2;
  }

  /* Segment */
  .tl-seg {
    position: absolute;
    top: 0; bottom: 0;
    border-radius: 2px;
    min-width: 4px;
    cursor: default;
    transition: filter 0.15s;
    display: flex;
    align-items: center;
    overflow: hidden;
    border: 1.5px solid transparent;
  }
  .tl-seg:hover { filter: brightness(1.12); }
  .tl-seg--pending {
    border-style: dashed !important;
    background: transparent !important;
  }
  .tl-seg-label {
    font-size: 0.55rem;
    color: #fff9f0;
    font-weight: 700;
    padding: 0 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
  }
  .tl-seg--pending .tl-seg-label { color: #92400e; }

  /* Tooltip */
  .tl-tooltip {
    position: absolute;
    top: -2.75rem;
    transform: translateX(-50%);
    background: #34251c;
    color: #fff9f0;
    border-radius: 3px;
    padding: 0.3rem 0.5rem;
    z-index: 10;
    pointer-events: none;
    white-space: nowrap;
    box-shadow: 0 2px 8px rgba(0,0,0,0.25);
  }
  .tl-tooltip::after {
    content: '';
    position: absolute;
    top: 100%; left: 50%;
    transform: translateX(-50%);
    border: 4px solid transparent;
    border-top-color: #34251c;
  }
  .tl-tt-name  { font-size: 0.65rem; font-weight: 700; margin: 0; color: #fff9f0; }
  .tl-tt-dates { font-size: 0.58rem; margin: 0.1rem 0 0; color: rgba(255,249,240,0.72); }

  /* Month ticks */
  .tl-ticks { position: relative; height: 1.25rem; margin-top: 0.25rem; }
  .tl-tick {
    position: absolute;
    transform: translateX(-50%);
    font-size: 0.55rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: rgba(95,70,54,0.5);
    white-space: nowrap;
    top: 0;
  }

  /* Legend */
  .tl-legend {
    display: flex;
    gap: 0.875rem;
    flex-wrap: wrap;
    margin-top: 0.25rem;
  }
  .leg {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.575rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: rgba(95,70,54,0.65);
  }
  .leg-dot {
    width: 10px; height: 10px;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .leg-dot--pending {
    background: transparent !important;
    border: 1.5px dashed;
  }
</style>
