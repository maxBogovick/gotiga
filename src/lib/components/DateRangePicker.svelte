<script lang="ts">
  import type { FigurineSchedule } from '$lib/types/api';
  import { t, lang } from '$lib/i18n';

  let {
    schedule = { entries: [] } as FigurineSchedule,
    startsAt = $bindable(''),
    endsAt   = $bindable(''),
    minDate  = new Date().toISOString().split('T')[0],
    onError  = (_msg: string) => {},
  }: {
    schedule?: FigurineSchedule;
    startsAt?: string;
    endsAt?: string;
    minDate?: string;
    onError?: (msg: string) => void;
  } = $props();

  // ── Locale-aware weekday/month names ─────────────────────────────────────
  // Mon=0 … Sun=6 — Jan 6 2025 is a Monday
  let weekdays = $derived(
    Array.from({ length: 7 }, (_, i) =>
      new Date(2025, 0, 6 + i).toLocaleDateString($lang, { weekday: 'short' })
        .replace('.', '').slice(0, 2).toUpperCase()
    )
  );

  function monthName(y: number, m: number): string {
    return new Date(y, m, 1).toLocaleDateString($lang, { month: 'long', year: 'numeric' });
  }

  // ── View state ────────────────────────────────────────────────────────────
  const now = new Date();
  let viewYear  = $state(now.getFullYear());
  let viewMonth = $state(now.getMonth());
  let hoverDate = $state('');

  // ── Helpers ───────────────────────────────────────────────────────────────
  function pad2(n: number) { return String(n).padStart(2, '0'); }
  function iso(y: number, m: number, d: number) { return `${y}-${pad2(m+1)}-${pad2(d)}`; }
  function weekdayOffset(y: number, m: number) { return (new Date(y, m, 1).getDay() + 6) % 7; }
  function daysIn(y: number, m: number) { return new Date(y, m+1, 0).getDate(); }

  function blockedBy(ds: string) {
    return schedule.entries.find(e =>
      (e.entryType === 'showing' || e.entryType === 'booking') &&
      ds >= e.startsAt && ds <= e.endsAt
    ) ?? null;
  }
  function pendingOn(ds: string) {
    return schedule.entries.find(e =>
      e.entryType === 'pending' && ds >= e.startsAt && ds <= e.endsAt
    ) ?? null;
  }
  function isPast(ds: string)     { return ds < minDate; }
  function isDisabled(ds: string) { return isPast(ds) || !!blockedBy(ds); }

  function rangeCrossesBlocked(from: string, to: string) {
    return schedule.entries.some(e =>
      (e.entryType === 'showing' || e.entryType === 'booking') &&
      from <= e.endsAt && to >= e.startsAt
    );
  }

  function effectiveEnd() { return (startsAt && !endsAt && hoverDate) ? hoverDate : endsAt; }
  function rangeEdges() {
    const s = startsAt, e = effectiveEnd();
    if (!s || !e) return { from: s, to: '' };
    return s <= e ? { from: s, to: e } : { from: e, to: s };
  }
  function isStart(ds: string) { const { from } = rangeEdges(); return !!from && ds === from; }
  function isEnd(ds: string)   { const { from, to } = rangeEdges(); return !!to && ds === to && from !== to; }
  function isInRange(ds: string) {
    const { from, to } = rangeEdges();
    return !!from && !!to && ds > from && ds < to;
  }
  function isSingle(ds: string) { const { from, to } = rangeEdges(); return ds === from && ds === to && !!from; }

  // ── Calendar grid ─────────────────────────────────────────────────────────
  let cells = $derived.by(() => {
    const result: (string | null)[] = [];
    const offset = weekdayOffset(viewYear, viewMonth);
    const total  = daysIn(viewYear, viewMonth);
    for (let i = 0; i < offset; i++) result.push(null);
    for (let d = 1; d <= total; d++) result.push(iso(viewYear, viewMonth, d));
    while (result.length % 7 !== 0) result.push(null);
    return result;
  });

  function prev() {
    if (viewMonth === 0) { viewYear--; viewMonth = 11; } else viewMonth--;
  }
  function next() {
    if (viewMonth === 11) { viewYear++; viewMonth = 0; } else viewMonth++;
  }

  function click(ds: string) {
    if (isDisabled(ds)) return;
    if (!startsAt || (startsAt && endsAt)) {
      startsAt = ds; endsAt = ''; onError(''); return;
    }
    const [from, to] = ds >= startsAt ? [startsAt, ds] : [ds, startsAt];
    if (rangeCrossesBlocked(from, to)) {
      const b = schedule.entries.find(e =>
        (e.entryType === 'showing' || e.entryType === 'booking') &&
        from <= e.endsAt && to >= e.startsAt
      );
      const base = b?.entryType === 'showing'
        ? `${$t('calConflictOverlapShowing')} «${b.title}»`
        : $t('calConflictOverlapBooking');
      onError(`${base} ${$t('calConflictSuffix')}`);
      startsAt = ds; endsAt = ''; return;
    }
    startsAt = from; endsAt = to; onError('');
  }

  function fmtResult(ds: string) {
    return new Date(ds + 'T00:00:00').toLocaleDateString($lang, {
      day: 'numeric', month: 'long', year: 'numeric'
    });
  }

  let hasBlocked = $derived(schedule.entries.some(e => e.entryType === 'showing' || e.entryType === 'booking'));
  let hasPending = $derived(schedule.entries.some(e => e.entryType === 'pending'));
</script>

<div class="cal">
  <!-- Header -->
  <div class="cal-head">
    <button type="button" onclick={prev} class="cal-nav" aria-label="Previous month">‹</button>
    <span class="cal-month">{monthName(viewYear, viewMonth)}</span>
    <button type="button" onclick={next} class="cal-nav" aria-label="Next month">›</button>
  </div>

  <!-- Grid -->
  <div class="cal-grid">
    {#each weekdays as wd}
      <div class="cal-wd">{wd}</div>
    {/each}

    {#each cells as ds}
      {#if ds === null}
        <div class="cal-empty"></div>
      {:else}
        {@const blocked = blockedBy(ds)}
        {@const pending = !blocked ? pendingOn(ds) : null}
        {@const past    = isPast(ds)}
        {@const start   = isStart(ds)}
        {@const end     = isEnd(ds)}
        {@const single  = isSingle(ds)}
        {@const range   = isInRange(ds)}
        <div class="cal-cell-wrap
          {range  ? 'range-bg' : ''}
          {start  ? 'range-bg range-bg-start' : ''}
          {end    ? 'range-bg range-bg-end'   : ''}
        ">
          <button
            type="button"
            class="cal-day
              {past    ? 'day-past'    : ''}
              {blocked ? 'day-blocked' : ''}
              {single  ? 'day-sel'     : ''}
              {start   ? 'day-sel'     : ''}
              {end     ? 'day-sel'     : ''}
              {range   ? 'day-range'   : ''}
              {pending && !start && !end && !single && !range ? 'day-pending' : ''}
              {!past && !blocked && !start && !end && !single && !range && !pending ? 'day-free' : ''}
            "
            title={blocked
              ? (blocked.entryType === 'showing'
                  ? `${$t('calBlockedShowing')} ${blocked.title}`
                  : $t('calTooltipBooked'))
              : pending ? $t('calTooltipPending') : ''}
            onclick={() => click(ds)}
            onmouseenter={() => { if (startsAt && !endsAt) hoverDate = ds; }}
            onmouseleave={() => { hoverDate = ''; }}
            disabled={isDisabled(ds)}
          >
            {parseInt(ds.slice(8))}
            {#if pending && !start && !end && !single}
              <span class="pending-dot" aria-hidden="true"></span>
            {/if}
          </button>
        </div>
      {/if}
    {/each}
  </div>

  <!-- Legend -->
  {#if hasBlocked || hasPending}
    <div class="cal-legend">
      {#if hasBlocked}
        <span class="leg"><span class="leg-dot leg-blocked"></span>{$t('calLegendBlocked')}</span>
      {/if}
      {#if hasPending}
        <span class="leg"><span class="leg-dot leg-pending"></span>{$t('calLegendPending')}</span>
      {/if}
      <span class="leg"><span class="leg-dot leg-sel"></span>{$t('calLegendSelected')}</span>
      <span class="leg"><span class="leg-dot leg-range"></span>{$t('calLegendRange')}</span>
    </div>
  {/if}

  <!-- Selected range display -->
  <div class="cal-result">
    {#if startsAt && endsAt}
      <span class="result-filled">{fmtResult(startsAt)} → {fmtResult(endsAt)}</span>
    {:else if startsAt}
      <span class="result-hint">{$t('calHintPickEnd')}</span>
    {:else}
      <span class="result-hint">{$t('calHintPickStart')}</span>
    {/if}
  </div>
</div>

<style>
  .cal { width: 100%; font-family: 'Inter', sans-serif; user-select: none; }

  .cal-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.5rem; }
  .cal-nav {
    width: 1.75rem; height: 1.75rem;
    display: flex; align-items: center; justify-content: center;
    border: 1px solid rgba(52,37,28,0.18); background: transparent;
    color: #5f4636; cursor: pointer; font-size: 1.1rem;
    transition: all 0.15s; border-radius: 2px;
  }
  .cal-nav:hover { border-color: rgba(52,37,28,0.4); color: #34251c; }
  .cal-month { font-family: 'Fraunces', serif; font-size: 0.875rem; color: #34251c; font-weight: 600; }

  .cal-grid { display: grid; grid-template-columns: repeat(7, 2rem); gap: 1px; justify-content: center; }
  .cal-wd {
    width: 2rem; height: 1.5rem; display: flex; align-items: center; justify-content: center;
    font-size: 0.575rem; text-transform: uppercase; letter-spacing: 0.06em;
    color: rgba(95,70,54,0.55); font-weight: 700;
  }
  .cal-empty { width: 2rem; height: 2rem; }

  .cal-cell-wrap {
    width: 2rem; height: 2rem; display: flex; align-items: center; justify-content: center; position: relative;
  }
  .range-bg::before { content: ''; position: absolute; inset: 2px 0; background: rgba(198,95,60,0.12); }
  .range-bg-start::before { border-radius: 50% 0 0 50%; left: 50%; }
  .range-bg-end::before   { border-radius: 0 50% 50% 0; right: 50%; }

  .cal-day {
    position: relative; z-index: 1;
    width: 1.875rem; height: 1.875rem;
    display: flex; align-items: center; justify-content: center;
    font-size: 0.72rem; border: none; background: transparent;
    cursor: pointer; border-radius: 50%;
    transition: background 0.1s, color 0.1s; flex-shrink: 0;
  }
  .day-free { color: #34251c; }
  .day-free:hover { background: rgba(198,95,60,0.15); }
  .day-past { color: rgba(95,70,54,0.25); cursor: default; font-size: 0.65rem; }
  .day-blocked { color: rgba(146,64,14,0.45); cursor: not-allowed; text-decoration: line-through; text-decoration-color: rgba(217,119,6,0.4); font-size: 0.65rem; }
  .day-sel { background: #6f3b24; color: #fff9f0; font-weight: 700; }
  .day-sel:hover { background: #9e452d; }
  .day-range { color: #34251c; }
  .day-pending { color: #92400e; position: relative; }
  .day-pending:hover { background: rgba(198,95,60,0.15); }

  .pending-dot { position: absolute; bottom: 2px; left: 50%; transform: translateX(-50%); width: 3px; height: 3px; border-radius: 50%; background: #d97706; }

  .cal-legend { display: flex; gap: 0.875rem; margin-top: 0.625rem; padding-top: 0.5rem; border-top: 1px solid rgba(52,37,28,0.07); justify-content: center; flex-wrap: wrap; }
  .leg { display: flex; align-items: center; gap: 0.3rem; font-size: 0.58rem; text-transform: uppercase; letter-spacing: 0.06em; color: #5f4636; }
  .leg-dot { width: 10px; height: 10px; border-radius: 50%; flex-shrink: 0; }
  .leg-blocked { background: rgba(217,119,6,0.15); border: 1.5px dashed rgba(217,119,6,0.5); border-radius: 2px; }
  .leg-sel     { background: #6f3b24; }
  .leg-range   { background: rgba(198,95,60,0.18); border-radius: 2px; }
  .leg-pending { background: transparent; border-bottom: 1.5px solid #d97706; border-radius: 0; position: relative; }
  .leg-pending::after { content: ''; position: absolute; bottom: -3px; left: 50%; transform: translateX(-50%); width: 3px; height: 3px; border-radius: 50%; background: #d97706; }

  .cal-result { margin-top: 0.5rem; min-height: 1.25rem; text-align: center; }
  .result-filled { font-size: 0.72rem; color: #34251c; font-weight: 600; font-family: 'Inter', sans-serif; }
  .result-hint   { font-size: 0.68rem; color: rgba(95,70,54,0.6); font-style: italic; }
</style>
