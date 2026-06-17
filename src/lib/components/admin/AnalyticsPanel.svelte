<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { FigurineListItem, Order, BookingDto } from '$lib/types/api';

  let loading = $state(true);
  let error   = $state('');

  // Raw data
  let figurines: FigurineListItem[] = $state([]);
  let bookings:  BookingDto[]       = $state([]);
  let orders:    Order[]            = $state([]);

  // ── Figurine stats ─────────────────────────────────────────────────────
  let statAvailable  = $derived(figurines.filter(f => f.status === 'available').length);
  let statReserved   = $derived(figurines.filter(f => f.status === 'reserved').length);
  let statSold       = $derived(figurines.filter(f => f.status === 'sold').length);
  let statWip        = $derived(figurines.filter(f => f.status === 'in_progress').length);
  let statTotal      = $derived(figurines.length);

  // ── Booking stats ──────────────────────────────────────────────────────
  let bookPending   = $derived(bookings.filter(b => b.status === 'pending').length);
  let bookConfirmed = $derived(bookings.filter(b => b.status === 'confirmed').length);
  let bookRejected  = $derived(bookings.filter(b => b.status === 'rejected').length);
  let bookCancelled = $derived(bookings.filter(b => b.status === 'cancelled').length);
  let bookTotal     = $derived(bookings.length);
  let bookConversion = $derived(
    bookTotal === 0 ? 0 : Math.round((bookConfirmed / bookTotal) * 100)
  );

  // ── Order stats ────────────────────────────────────────────────────────
  let ordRequests  = $derived(orders.filter(o => o.mode === 'request').length);
  let ordQuestions = $derived(orders.filter(o => o.mode === 'question').length);
  let ordNotify    = $derived(orders.filter(o => o.mode === 'notify').length);
  let ordTotal     = $derived(orders.length);

  // ── Top figurines by combined demand ──────────────────────────────────
  type FigStat = { id: string; name: string; bookings: number; orders: number; total: number };

  let topFigurines = $derived.by((): FigStat[] => {
    const map = new Map<string, FigStat>();
    for (const b of bookings) {
      const e = map.get(b.figurineId) ?? { id: b.figurineId, name: b.figurineName, bookings: 0, orders: 0, total: 0 };
      e.bookings++; e.total++;
      map.set(b.figurineId, e);
    }
    for (const o of orders) {
      const e = map.get(o.figurineId) ?? { id: o.figurineId, name: o.figurineName, bookings: 0, orders: 0, total: 0 };
      e.orders++; e.total++;
      map.set(o.figurineId, e);
    }
    return [...map.values()].sort((a, b) => b.total - a.total).slice(0, 8);
  });

  let maxDemand = $derived(topFigurines[0]?.total ?? 1);

  // ── Activity by week (last 8 weeks) ───────────────────────────────────
  type WeekBar = { label: string; bookings: number; orders: number };

  let weekBars = $derived.by((): WeekBar[] => {
    const now   = Date.now();
    const bars: WeekBar[] = Array.from({ length: 8 }, (_, i) => {
      const weekStart = now - (7 - i) * 7 * 86400_000;
      const weekEnd   = weekStart + 7 * 86400_000;
      const d = new Date(weekStart);
      const label = `${d.getDate()}.${String(d.getMonth() + 1).padStart(2, '0')}`;
      const bk = bookings.filter(b => { const t = new Date(b.createdAt).getTime(); return t >= weekStart && t < weekEnd; }).length;
      const or = orders.filter(o => { const t = new Date(o.createdAt).getTime(); return t >= weekStart && t < weekEnd; }).length;
      return { label, bookings: bk, orders: or };
    });
    return bars;
  });

  let maxWeekVal = $derived(Math.max(...weekBars.map(w => w.bookings + w.orders), 1));

  // ── Load ───────────────────────────────────────────────────────────────
  onMount(async () => {
    try {
      const [figs, bkPage, ordPage] = await Promise.all([
        api.getAllFigurinesAdmin(),
        api.listBookings({ perPage: 500 }),
        api.listOrders({ perPage: 500 }),
      ]);
      figurines = figs;
      bookings  = bkPage.items;
      orders    = ordPage.items;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });

  function pct(val: number, of: number) {
    return of === 0 ? 0 : Math.round((val / of) * 100);
  }
</script>

<div class="ap">
  {#if loading}
    <div class="ap-loading">Loading analytics…</div>
  {:else if error}
    <div class="ap-error">{error}</div>
  {:else}

    <!-- ── Overview cards ─────────────────────────────────────────────── -->
    <div class="ap-cards">
      <div class="ap-card">
        <p class="ap-card-value">{statTotal}</p>
        <p class="ap-card-label">Figures in archive</p>
        <div class="ap-mini-bar">
          <div class="ap-mini-seg ap-seg--available" style="width:{pct(statAvailable,statTotal)}%" title="Available: {statAvailable}"></div>
          <div class="ap-mini-seg ap-seg--reserved"  style="width:{pct(statReserved, statTotal)}%" title="Reserved: {statReserved}"></div>
          <div class="ap-mini-seg ap-seg--sold"      style="width:{pct(statSold,    statTotal)}%" title="Sold: {statSold}"></div>
          <div class="ap-mini-seg ap-seg--wip"       style="width:{pct(statWip,     statTotal)}%" title="In progress: {statWip}"></div>
        </div>
        <div class="ap-card-detail">
          <span class="dot dot--available"></span>{statAvailable} available ·
          <span class="dot dot--reserved"></span>{statReserved} booked ·
          <span class="dot dot--sold"></span>{statSold} sold ·
          <span class="dot dot--wip"></span>{statWip} in progress
        </div>
      </div>

      <div class="ap-card">
        <p class="ap-card-value">{bookTotal}</p>
        <p class="ap-card-label">Booking requests</p>
        <div class="ap-mini-bar">
          <div class="ap-mini-seg ap-seg--pending"   style="width:{pct(bookPending,   bookTotal)}%"></div>
          <div class="ap-mini-seg ap-seg--confirmed" style="width:{pct(bookConfirmed, bookTotal)}%"></div>
          <div class="ap-mini-seg ap-seg--rejected"  style="width:{pct(bookRejected,  bookTotal)}%"></div>
          <div class="ap-mini-seg ap-seg--cancelled" style="width:{pct(bookCancelled, bookTotal)}%"></div>
        </div>
        <div class="ap-card-detail">
          <span class="dot dot--pending"></span>{bookPending} pending ·
          <span class="dot dot--confirmed"></span>{bookConfirmed} confirmed ·
          <span class="dot dot--rejected"></span>{bookRejected + bookCancelled} rejected
        </div>
      </div>

      <div class="ap-card">
        <p class="ap-card-value">{ordTotal}</p>
        <p class="ap-card-label">Requests / inquiries</p>
        <div class="ap-mini-bar">
          <div class="ap-mini-seg ap-seg--request"  style="width:{pct(ordRequests,  ordTotal)}%"></div>
          <div class="ap-mini-seg ap-seg--question" style="width:{pct(ordQuestions, ordTotal)}%"></div>
          <div class="ap-mini-seg ap-seg--notify"   style="width:{pct(ordNotify,    ordTotal)}%"></div>
        </div>
        <div class="ap-card-detail">
          <span class="dot dot--request"></span>{ordRequests} requests ·
          <span class="dot dot--question"></span>{ordQuestions} questions ·
          <span class="dot dot--notify"></span>{ordNotify} subscriptions
        </div>
      </div>

      <div class="ap-card ap-card--highlight">
        <p class="ap-card-value ap-conversion">{bookConversion}%</p>
        <p class="ap-card-label">Booking conversion</p>
        <div class="ap-conversion-bar">
          <div class="ap-conversion-fill" style="width:{bookConversion}%"></div>
        </div>
        <p class="ap-card-detail">
          {bookConfirmed} of {bookTotal} requests confirmed
        </p>
      </div>
    </div>

    <!-- ── Weekly activity ────────────────────────────────────────────── -->
    <div class="ap-section">
      <h3 class="ap-section-title">Weekly activity (last 8 weeks)</h3>
      <div class="ap-weeks">
        {#each weekBars as w}
          <div class="ap-week-col">
            <div class="ap-week-bars">
              <div class="ap-week-bar ap-week-bar--orders"
                   style="height:{pct(w.orders, maxWeekVal)}%"
                   title="Requests: {w.orders}">
              </div>
              <div class="ap-week-bar ap-week-bar--bookings"
                   style="height:{pct(w.bookings, maxWeekVal)}%"
                   title="Bookings: {w.bookings}">
              </div>
            </div>
            <span class="ap-week-label">{w.label}</span>
          </div>
        {/each}
      </div>
      <div class="ap-legend">
        <span class="ap-legend-item"><span class="ap-legend-dot ap-legend-dot--bookings"></span>Bookings</span>
        <span class="ap-legend-item"><span class="ap-legend-dot ap-legend-dot--orders"></span>Requests</span>
      </div>
    </div>

    <!-- ── Top figurines ─────────────────────────────────────────────── -->
    {#if topFigurines.length > 0}
      <div class="ap-section">
        <h3 class="ap-section-title">Most requested figures</h3>
        <div class="ap-top-list">
          {#each topFigurines as f, i}
            <div class="ap-top-row">
              <span class="ap-top-rank">{i + 1}</span>
              <a href="/figurines/{f.id}" target="_blank" rel="noopener" class="ap-top-name">{f.name}</a>
              <div class="ap-top-bar-wrap">
                <div class="ap-top-bar" style="width:{pct(f.total, maxDemand)}%">
                  <div class="ap-top-bar-inner ap-top-bar--bookings" style="width:{pct(f.bookings, f.total)}%"></div>
                  <div class="ap-top-bar-inner ap-top-bar--orders"   style="width:{pct(f.orders,   f.total)}%"></div>
                </div>
              </div>
              <span class="ap-top-count">{f.total}</span>
              <span class="ap-top-detail">{f.bookings}b / {f.orders}r</span>
            </div>
          {/each}
        </div>
        <p class="ap-legend-small">b — bookings · r — requests</p>
      </div>
    {:else}
      <div class="ap-empty">No data yet — activity will appear as bookings and requests come in.</div>
    {/if}

  {/if}
</div>

<style>
  .ap {
    height: 100%;
    overflow-y: auto;
    padding: 1.5rem 2rem 3rem;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    color: #34251c;
  }

  .ap-loading, .ap-error, .ap-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 200px;
    font-size: 0.85rem;
    color: rgba(95,70,54,0.5);
    font-style: italic;
  }
  .ap-error { color: #a03020; }

  /* ── Cards ── */
  .ap-cards {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
    margin-bottom: 2rem;
  }
  @media (max-width: 1100px) { .ap-cards { grid-template-columns: repeat(2, 1fr); } }
  @media (max-width: 600px)  { .ap-cards { grid-template-columns: 1fr; } }

  .ap-card {
    background: #fff9f0;
    border: 1px solid rgba(52,37,28,0.1);
    padding: 1rem 1.1rem 0.9rem;
  }
  .ap-card--highlight { border-color: rgba(198,95,60,0.25); background: #fef5ee; }

  .ap-card-value {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 2rem;
    font-weight: 500;
    color: #34251c;
    margin: 0 0 0.15rem;
    line-height: 1;
  }
  .ap-conversion { color: #c65f3c; }

  .ap-card-label {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: rgba(95,70,54,0.55);
    margin: 0 0 0.6rem;
  }

  .ap-card-detail {
    font-size: 0.6rem;
    color: rgba(95,70,54,0.55);
    margin-top: 0.4rem;
    line-height: 1.6;
  }

  /* ── Mini bars inside cards ── */
  .ap-mini-bar {
    display: flex;
    height: 4px;
    border-radius: 2px;
    overflow: hidden;
    background: rgba(52,37,28,0.06);
  }
  .ap-mini-seg { height: 100%; min-width: 2px; transition: width 0.5s; }

  .ap-conversion-bar {
    height: 4px;
    border-radius: 2px;
    background: rgba(52,37,28,0.08);
    overflow: hidden;
    margin-bottom: 0.3rem;
  }
  .ap-conversion-fill {
    height: 100%;
    background: #c65f3c;
    border-radius: 2px;
    transition: width 0.6s ease-out;
  }

  /* Colors */
  .ap-seg--available { background: #5a7a4a; }
  .ap-seg--reserved  { background: #c65f3c; }
  .ap-seg--sold      { background: #6f3b24; }
  .ap-seg--wip       { background: #b0a090; }
  .ap-seg--pending   { background: #d4970a; }
  .ap-seg--confirmed { background: #5a7a4a; }
  .ap-seg--rejected  { background: #c0362c; }
  .ap-seg--cancelled { background: #b0a090; }
  .ap-seg--request   { background: #c65f3c; }
  .ap-seg--question  { background: #6f8ab0; }
  .ap-seg--notify    { background: #b0a090; }

  .dot {
    display: inline-block;
    width: 6px; height: 6px;
    border-radius: 50%;
    margin-right: 2px;
    vertical-align: middle;
  }
  .dot--available { background: #5a7a4a; }
  .dot--reserved  { background: #c65f3c; }
  .dot--sold      { background: #6f3b24; }
  .dot--wip       { background: #b0a090; }
  .dot--pending   { background: #d4970a; }
  .dot--confirmed { background: #5a7a4a; }
  .dot--rejected  { background: #c0362c; }
  .dot--request   { background: #c65f3c; }
  .dot--question  { background: #6f8ab0; }
  .dot--notify    { background: #b0a090; }

  /* ── Sections ── */
  .ap-section {
    margin-bottom: 2rem;
  }
  .ap-section-title {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 0.95rem;
    font-weight: 400;
    color: #34251c;
    margin: 0 0 1rem;
    padding-bottom: 0.4rem;
    border-bottom: 1px solid rgba(52,37,28,0.1);
  }

  /* ── Weekly chart ── */
  .ap-weeks {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    height: 80px;
    padding-bottom: 1.4rem;
    position: relative;
  }
  .ap-week-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    height: 100%;
    justify-content: flex-end;
    position: relative;
  }
  .ap-week-bars {
    width: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 1px;
    flex: 1;
  }
  .ap-week-bar {
    width: 45%;
    min-height: 2px;
    border-radius: 1px 1px 0 0;
    transition: height 0.5s ease-out;
  }
  .ap-week-bar--bookings { background: #c65f3c; }
  .ap-week-bar--orders   { background: rgba(111,139,176,0.7); }
  .ap-week-label {
    position: absolute;
    bottom: 0;
    font-size: 0.55rem;
    color: rgba(95,70,54,0.45);
    letter-spacing: 0.04em;
    white-space: nowrap;
  }

  .ap-legend {
    display: flex;
    gap: 1rem;
    margin-top: 0.4rem;
  }
  .ap-legend-item {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.62rem;
    color: rgba(95,70,54,0.6);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .ap-legend-dot {
    width: 8px; height: 8px;
    border-radius: 1px;
    flex-shrink: 0;
  }
  .ap-legend-dot--bookings { background: #c65f3c; }
  .ap-legend-dot--orders   { background: rgba(111,139,176,0.7); }

  /* ── Top figurines ── */
  .ap-top-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .ap-top-row {
    display: grid;
    grid-template-columns: 1.2rem 1fr 1fr auto auto;
    align-items: center;
    gap: 0.6rem;
    padding: 0.4rem 0.6rem;
    background: #fff9f0;
    border: 1px solid transparent;
    transition: border-color 0.2s;
  }
  .ap-top-row:hover { border-color: rgba(52,37,28,0.08); }

  .ap-top-rank {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 0.75rem;
    color: rgba(95,70,54,0.4);
    text-align: center;
  }
  .ap-top-name {
    font-size: 0.8rem;
    color: #34251c;
    text-decoration: none;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: color 0.2s;
  }
  .ap-top-name:hover { color: #c65f3c; }

  .ap-top-bar-wrap {
    background: rgba(52,37,28,0.06);
    height: 6px;
    border-radius: 3px;
    overflow: hidden;
  }
  .ap-top-bar {
    height: 100%;
    display: flex;
    border-radius: 3px;
    overflow: hidden;
    transition: width 0.5s ease-out;
  }
  .ap-top-bar-inner { height: 100%; }
  .ap-top-bar--bookings { background: #c65f3c; }
  .ap-top-bar--orders   { background: rgba(111,139,176,0.7); }

  .ap-top-count {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1rem;
    color: #34251c;
    min-width: 1.5rem;
    text-align: right;
  }
  .ap-top-detail {
    font-size: 0.6rem;
    color: rgba(95,70,54,0.45);
    min-width: 4rem;
    text-align: right;
    white-space: nowrap;
  }

  .ap-legend-small {
    font-size: 0.6rem;
    color: rgba(95,70,54,0.4);
    margin: 0.5rem 0 0;
    text-align: right;
  }
</style>
