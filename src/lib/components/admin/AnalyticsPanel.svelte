<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import iso from 'iso-3166-1';
  import WorldMap from '$lib/components/admin/WorldMap.svelte';
  import type {
    AdminAnalyticsOverview,
    AdminFigurineAnalyticsDetail,
    AdminFigurineAnalyticsListPage,
    AdminFigurineAnalyticsListItem,
    AnalyticsSignal,
    AnalyticsDailyPoint,
    CommissionFunnel,
    AnalyticsAnnotation,
    LifeOfHouseTrend,
  } from '$lib/types/api';

  // ── Date range — everything below is computed in UTC on purpose. Mixing
  // local-Date arithmetic (getDate/setDate) with .toISOString() output is what
  // let the old picker's "today" drift a day near local midnight; the server
  // buckets every event by UTC calendar day, so the contract here is: the
  // picker's "today", "this month" etc. are also UTC days, not local ones.
  type Preset = 'today' | '7d' | '30d' | '90d' | 'this_month' | 'last_month' | 'custom';

  function todayUtc(): string {
    return new Date().toISOString().slice(0, 10);
  }
  function shiftUtc(dateStr: string, days: number): string {
    const d = new Date(`${dateStr}T00:00:00Z`);
    d.setUTCDate(d.getUTCDate() + days);
    return d.toISOString().slice(0, 10);
  }
  function ymd(y: number, m: number, day: number): string {
    return new Date(Date.UTC(y, m, day)).toISOString().slice(0, 10);
  }
  function utcYM(dateStr: string): { y: number; m: number } {
    const d = new Date(`${dateStr}T00:00:00Z`);
    return { y: d.getUTCFullYear(), m: d.getUTCMonth() };
  }
  function presetRange(preset: Preset, customFrom: string, customTo: string): { from: string; to: string } {
    const to = todayUtc();
    switch (preset) {
      case 'today': return { from: to, to };
      case '7d': return { from: shiftUtc(to, -6), to };
      case '90d': return { from: shiftUtc(to, -89), to };
      case 'this_month': {
        const { y, m } = utcYM(to);
        return { from: ymd(y, m, 1), to };
      }
      case 'last_month': {
        const { y, m } = utcYM(to);
        const py = m === 0 ? y - 1 : y;
        const pm = m === 0 ? 11 : m - 1;
        return { from: ymd(py, pm, 1), to: ymd(y, m, 0) };
      }
      case 'custom':
        return { from: customFrom || shiftUtc(to, -29), to: customTo || to };
      case '30d':
      default:
        return { from: shiftUtc(to, -29), to };
    }
  }

  type SortKey = 'name' | 'status' | 'views' | 'uniqueVisitors' | 'engagedViews' | 'submissions' | 'conversionRate';
  type PerformanceFilter = 'all' | 'has_views' | 'has_submissions';
  type Tab = 'pulse' | 'traffic' | 'works' | 'community';

  const CTA_LABELS: Record<string, string> = {
    request: 'Order request',
    reserve: 'Reserve intent',
    booking: 'Loan booking',
    waitlist: 'Waitlist',
    commission: 'Create similar',
  };

  const TAB_LABELS: Record<Tab, string> = {
    pulse: 'Pulse',
    traffic: 'Traffic',
    works: 'Works',
    community: 'Community',
  };

  // ── Persisted UI state — date range, active tab, and every Works filter
  // survive a reload; none of the fetched DATA is cached, only these
  // choices, so the panel reopens where you left it instead of resetting.
  const STORAGE_KEY = 'gotiga_admin_analytics_state';
  type PersistedState = {
    preset: Preset;
    customFrom: string;
    customTo: string;
    compareEnabled: boolean;
    activeTab: Tab;
    search: string;
    statusFilter: string;
    signalFilter: string;
    performanceFilter: PerformanceFilter;
    countryFilter: string;
    sourceFilter: string;
    deviceFilter: string;
    seriesFilter: string;
    growingFilter: boolean;
    sort: SortKey;
    dir: 'asc' | 'desc';
  };
  function loadPersisted(): Partial<PersistedState> {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      return raw ? JSON.parse(raw) : {};
    } catch {
      return {};
    }
  }
  const persisted = loadPersisted();

  let loading = $state(true);
  let error = $state('');
  let preset = $state<Preset>(persisted.preset ?? '30d');
  let customFrom = $state(persisted.customFrom ?? '');
  let customTo = $state(persisted.customTo ?? '');
  let compareEnabled = $state(persisted.compareEnabled ?? true);
  let activeTab = $state<Tab>(persisted.activeTab ?? 'pulse');
  let overview = $state<AdminAnalyticsOverview | null>(null);
  let page = $state<AdminFigurineAnalyticsListPage | null>(null);
  let commissionFunnel = $state<CommissionFunnel | null>(null);
  let annotations = $state<AnalyticsAnnotation[]>([]);
  let lifeOfHouse = $state<LifeOfHouseTrend | null>(null);
  let newAnnotationDay = $state('');
  let newAnnotationLabel = $state('');
  let annotationSaving = $state(false);
  let annotationError = $state('');
  let backfillRunning = $state(false);
  let backfillMessage = $state('');

  let sort = $state<SortKey>(persisted.sort ?? 'views');
  let dir = $state<'asc' | 'desc'>(persisted.dir ?? 'desc');
  let search = $state(persisted.search ?? '');
  let statusFilter = $state(persisted.statusFilter ?? 'all');
  let signalFilter = $state(persisted.signalFilter ?? 'all');
  let performanceFilter = $state<PerformanceFilter>(persisted.performanceFilter ?? 'all');
  /** ISO alpha-2, or 'all'. Shared by the geography map/list and the Works
   * table's country filter — clicking a country in one place drives both. */
  let countryFilter = $state(persisted.countryFilter ?? 'all');
  let sourceFilter = $state(persisted.sourceFilter ?? 'all');
  let deviceFilter = $state(persisted.deviceFilter ?? 'all');
  let seriesFilter = $state(persisted.seriesFilter ?? 'all');
  /** Independent of `signalFilter` on purpose — see `signalCounts` below for
   * why "growing" needs its own axis instead of living inside the signal enum. */
  let growingFilter = $state(persisted.growingFilter ?? false);

  let selectedId = $state<string | null>(null);
  let detail = $state<AdminFigurineAnalyticsDetail | null>(null);
  let detailLoading = $state(false);
  let drilldownRowEl = $state<HTMLTableRowElement | null>(null);

  let range = $derived(presetRange(preset, customFrom, customTo));

  // Bring the just-opened drilldown into view instead of leaving the admin to
  // hunt for it — it renders inline right under the clicked row, but that row
  // can still be scrolled out of the table's own viewport.
  $effect(() => {
    if (detail && !detailLoading && drilldownRowEl) {
      drilldownRowEl.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }
  });

  // Persist every UI choice above whenever any of them changes — reading
  // each one here (via the object literal) is what makes the effect re-run
  // on any of their changes, not just the first.
  $effect(() => {
    const state: PersistedState = {
      preset, customFrom, customTo, compareEnabled, activeTab,
      search, statusFilter, signalFilter, performanceFilter,
      countryFilter, sourceFilter, deviceFilter, seriesFilter, growingFilter,
      sort, dir,
    };
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    } catch {
      // Private browsing / storage full — losing persistence isn't fatal.
    }
  });

  // ── Data loading ────────────────────────────────────────────────────────
  onMount(() => {
    void loadAll();
  });

  async function loadAll() {
    loading = true;
    error = '';
    try {
      const [ov, lp, cf, ann, loh] = await Promise.all([
        api.getAnalyticsOverview({ from: range.from, to: range.to }),
        api.listFigurineAnalytics({ from: range.from, to: range.to }),
        api.getCommissionFunnel({ from: range.from, to: range.to }),
        api.listAnalyticsAnnotations({ from: range.from, to: range.to }),
        api.getLifeOfHouseTrend({ from: range.from, to: range.to }),
      ]);
      overview = ov;
      page = lp;
      commissionFunnel = cf;
      annotations = ann;
      lifeOfHouse = loh;
      if (selectedId && !lp.items.some((i) => i.figurineId === selectedId)) {
        selectedId = null;
        detail = null;
      } else if (selectedId) {
        void loadDetail(selectedId);
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function loadDetail(id: string) {
    detailLoading = true;
    try {
      detail = await api.getFigurineAnalytics(id, { from: range.from, to: range.to });
    } catch {
      detail = null;
    } finally {
      detailLoading = false;
    }
  }

  function setPreset(p: Preset) {
    preset = p;
    void loadAll();
  }

  function applyCustomRange() {
    if (!customFrom || !customTo || customFrom > customTo) return;
    preset = 'custom';
    void loadAll();
  }

  /** One-off repair action: re-runs the daily aggregation over the full
   * analytics history (idempotent — safe to click more than once). Needed
   * after a fix to the aggregation query itself, since the automatic
   * hot-window refresh only ever recomputes yesterday+today. */
  async function runBackfill() {
    if (!confirm('Re-aggregate the full analytics history now? This is safe to re-run, but touches every day on record and may take a few seconds.')) return;
    backfillRunning = true;
    backfillMessage = '';
    try {
      const result = await api.backfillAnalytics();
      backfillMessage = `Backfilled ${result.from} – ${result.to}`;
      await loadAll();
    } catch (e) {
      backfillMessage = `Backfill failed: ${e}`;
    } finally {
      backfillRunning = false;
    }
  }

  async function addAnnotation() {
    if (!newAnnotationDay || !newAnnotationLabel.trim()) return;
    annotationSaving = true;
    annotationError = '';
    try {
      const created = await api.createAnalyticsAnnotation({ day: newAnnotationDay, label: newAnnotationLabel.trim() });
      annotations = [...annotations, created].sort((a, b) => a.day.localeCompare(b.day));
      newAnnotationDay = '';
      newAnnotationLabel = '';
    } catch (e) {
      annotationError = String(e);
    } finally {
      annotationSaving = false;
    }
  }

  async function removeAnnotation(id: string) {
    const prev = annotations;
    annotations = annotations.filter((a) => a.id !== id);
    try {
      await api.deleteAnalyticsAnnotation(id);
    } catch {
      annotations = prev;
    }
  }

  function csvCell(v: string): string {
    return `"${v.replace(/"/g, '""')}"`;
  }

  function exportWorksCsv() {
    const header = ['name', 'status', 'series', 'views', 'uniqueVisitors', 'engagedViews', 'submissions', 'conversionRate', 'signal', 'topCountry', 'topSource', 'topDevice'];
    const rows = sortedItems.map((item) =>
      [
        item.name,
        item.status,
        item.series ?? '',
        String(item.views),
        String(item.uniqueVisitors),
        String(item.engagedViews),
        String(item.submissions),
        item.conversionRate.toFixed(2),
        signalLabel(item.signal),
        item.topCountry ? countryName(item.topCountry) : '',
        item.topSource ? sourceLabel(item.topSource) : '',
        item.topDevice ? sourceLabel(item.topDevice) : '',
      ].map(csvCell).join(',')
    );
    // BOM so Excel reads UTF-8 (Cyrillic names, etc.) correctly.
    const csv = '﻿' + [header.join(','), ...rows].join('\r\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `gotiga-analytics-works-${range.from}_${range.to}.csv`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  }

  function exportGeoCsv() {
    if (!overview) return;
    const header = ['country', 'code', 'views', 'uniqueVisitors', 'sharePct'];
    const total = overview.geo.reduce((sum, g) => sum + g.views, 0);
    const rows = overview.geo.map((g) =>
      [
        countryName(g.key),
        g.key.toUpperCase(),
        String(g.views),
        String(g.uniqueVisitors),
        rate(g.views, total).toFixed(2),
      ].map(csvCell).join(',')
    );
    const csv = '﻿' + [header.join(','), ...rows].join('\r\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `gotiga-analytics-geography-${range.from}_${range.to}.csv`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  }

  function selectFigurine(id: string) {
    selectedId = selectedId === id ? null : id;
    if (selectedId) void loadDetail(selectedId);
  }

  function clearAllFilters() {
    search = '';
    statusFilter = 'all';
    signalFilter = 'all';
    performanceFilter = 'all';
    countryFilter = 'all';
    sourceFilter = 'all';
    deviceFilter = 'all';
    seriesFilter = 'all';
    growingFilter = false;
  }

  let activeFilterCount = $derived(
    (search.trim() ? 1 : 0) +
    (statusFilter !== 'all' ? 1 : 0) +
    (signalFilter !== 'all' ? 1 : 0) +
    (performanceFilter !== 'all' ? 1 : 0) +
    (countryFilter !== 'all' ? 1 : 0) +
    (sourceFilter !== 'all' ? 1 : 0) +
    (deviceFilter !== 'all' ? 1 : 0) +
    (seriesFilter !== 'all' ? 1 : 0) +
    (growingFilter ? 1 : 0)
  );

  function setSort(key: SortKey) {
    if (sort === key) {
      dir = dir === 'asc' ? 'desc' : 'asc';
    } else {
      sort = key;
      dir = key === 'name' || key === 'status' ? 'asc' : 'desc';
    }
  }

  // ── Derived, client-side only — no request on sort/filter change ────────
  let filteredItems = $derived.by(() => {
    const items = page?.items ?? [];
    const q = search.trim().toLowerCase();
    return items.filter((item) => {
      if (q && !`${item.name} ${item.status} ${signalLabel(item.signal)}`.toLowerCase().includes(q)) return false;
      if (statusFilter !== 'all' && item.status !== statusFilter) return false;
      if (signalFilter !== 'all' && item.signal !== signalFilter) return false;
      if (performanceFilter === 'has_views' && item.views === 0) return false;
      if (performanceFilter === 'has_submissions' && item.submissions === 0) return false;
      if (growingFilter && !item.isGrowing) return false;
      // Existence filters, like the ones above — "had at least one view
      // from this country/source/device", not a metric recompute. Views,
      // engaged %, submissions etc. stay whole-figurine totals; scoping
      // those per country would need engaged/submissions attribution the
      // site doesn't collect (mirrors the "Submissions aren't yet
      // attributed per channel" caveat on the Sources block above).
      if (countryFilter !== 'all' && !item.countries.includes(countryFilter)) return false;
      if (sourceFilter !== 'all' && item.topSource !== sourceFilter) return false;
      if (deviceFilter !== 'all' && item.topDevice !== deviceFilter) return false;
      if (seriesFilter !== 'all' && item.series !== seriesFilter) return false;
      return true;
    });
  });

  function optionValues(pick: (item: AdminFigurineAnalyticsListItem) => string | null | undefined): string[] {
    const set = new Set<string>();
    for (const item of page?.items ?? []) {
      const v = pick(item);
      if (v) set.add(v);
    }
    return [...set].sort();
  }

  let countryOptions = $derived.by(() => {
    const set = new Set<string>();
    for (const item of page?.items ?? []) for (const c of item.countries) set.add(c);
    return [...set].sort((a, b) => countryName(a).localeCompare(countryName(b)));
  });
  let sourceOptions = $derived(optionValues((i) => i.topSource));
  let deviceOptions = $derived(optionValues((i) => i.topDevice));
  let seriesOptions = $derived(optionValues((i) => i.series));

  function countryName(code: string): string {
    if (!code || code === 'unknown') return 'Unknown';
    return iso.whereAlpha2(code)?.country ?? code.toUpperCase();
  }

  let sortedItems = $derived.by(() => {
    const items = [...filteredItems];
    const mul = dir === 'asc' ? 1 : -1;
    items.sort((a, b) => {
      const av = a[sort as keyof AdminFigurineAnalyticsListItem];
      const bv = b[sort as keyof AdminFigurineAnalyticsListItem];
      if (typeof av === 'string' && typeof bv === 'string') return av.localeCompare(bv) * mul;
      return (((av as number) ?? 0) - ((bv as number) ?? 0)) * mul;
    });
    return items;
  });

  let selectedItem = $derived(sortedItems.find((i) => i.figurineId === selectedId) ?? null);

  /** Works viewed from the currently-selected country, independent of every
   * other Works-tab filter (status/signal/source/…) — the Geography tab
   * shouldn't silently inherit a filter left set on the Works tab. */
  let topWorksForSelectedCountry = $derived.by(() => {
    if (countryFilter === 'all' || !page) return [];
    return [...page.items]
      .filter((i) => i.countries.includes(countryFilter))
      .sort((a, b) => b.views - a.views)
      .slice(0, 6);
  });

  /** Site-wide signal counts — the Pulse tab's digest and the Works tab's
   * badge both read from here. `growing` counts every work with positive
   * week-over-week growth, not just the ones whose *priority-picked* signal
   * happens to be "Growing interest" (see `isGrowing` on the API type for
   * why that distinction matters — signal is a single value, so a work that
   * is both growing and, say, attention-worthy would otherwise never be
   * counted as growing here). */
  let signalCounts = $derived.by(() => {
    const items = page?.items ?? [];
    let attention = 0, growing = 0, highConversion = 0;
    for (const i of items) {
      if (i.signal === 'attention_no_submissions') attention++;
      if (i.isGrowing) growing++;
      if (i.signal === 'high_conversion') highConversion++;
    }
    return { attention, growing, highConversion };
  });

  function jumpToSignal(kind: 'attention' | 'growing' | 'high_conversion') {
    activeTab = 'works';
    if (kind === 'growing') {
      growingFilter = true;
      signalFilter = 'all';
    } else {
      signalFilter = kind === 'attention' ? 'attention_no_submissions' : 'high_conversion';
      growingFilter = false;
    }
  }

  let overviewChart = $derived.by(() => bucketDaily(overview?.daily ?? [], range.from, range.to));
  let detailChart = $derived.by(() => bucketDaily(detail?.daily ?? [], range.from, range.to));

  let viewsDelta = $derived(overview ? delta(overview.summary.views, overview.previousSummary.views) : null);
  let uniquesDelta = $derived(overview ? delta(overview.summary.uniqueVisitors, overview.previousSummary.uniqueVisitors) : null);
  let submissionsDelta = $derived(overview ? delta(overview.summary.submissions, overview.previousSummary.submissions) : null);
  let conversionLabel = $derived(overview ? rateLabel(overview.summary.submissions, overview.summary.engagedViews) : null);

  // ── Formatting helpers ───────────────────────────────────────────────────
  const nf = new Intl.NumberFormat('en-US');
  function fmt(n: number): string { return nf.format(n); }

  function rate(value: number, total: number): number {
    return total <= 0 ? 0 : Math.round((value / total) * 10000) / 100;
  }

  /** Rates on a denominator below this are volatile enough to mislead —
   * caption them as low-data rather than print a bare confident percentage. */
  const MIN_RATE_SAMPLE = 10;
  function rateLabel(value: number, total: number): { text: string; lowData: boolean } {
    if (total <= 0) return { text: '—', lowData: false };
    const pct = rate(value, total);
    return { text: `${pct.toFixed(1)}%`, lowData: total < MIN_RATE_SAMPLE };
  }

  /** Delta vs the previous period. `previous === 0` has no baseline to compare
   * against — that's "new", not +Infinity% or NaN. */
  function delta(current: number, previous: number): { text: string; tone: 'up' | 'down' | 'flat' | 'new'; lowData: boolean } {
    if (previous === 0) {
      return current > 0
        ? { text: 'new', tone: 'new', lowData: false }
        : { text: '—', tone: 'flat', lowData: false };
    }
    const pct = ((current - previous) / previous) * 100;
    const rounded = Math.round(pct);
    const text = `${rounded > 0 ? '+' : ''}${rounded}%`;
    const tone = rounded > 0 ? 'up' : rounded < 0 ? 'down' : 'flat';
    return { text, tone, lowData: previous < MIN_RATE_SAMPLE };
  }

  function pct(value: number, max: number): number {
    return max <= 0 ? 0 : Math.max(2, Math.round((value / max) * 100));
  }

  function msLabel(ms: number | null): string {
    if (ms === null) return 'No data';
    const s = Math.round(ms / 1000);
    if (s < 60) return `${s}s`;
    return `${Math.floor(s / 60)}m ${s % 60}s`;
  }

  function signalLabel(signal: AnalyticsSignal): string {
    switch (signal) {
      case 'high_conversion': return 'High conversion';
      case 'attention_no_submissions': return 'Attention, no submissions';
      case 'low_visibility': return 'Low visibility';
      case 'growing_interest': return 'Growing interest';
      case 'low_data': return 'Low data';
      default: return 'Normal';
    }
  }

  function signalTone(signal: AnalyticsSignal): string {
    switch (signal) {
      case 'high_conversion': return 'good';
      case 'attention_no_submissions': return 'warn';
      case 'growing_interest': return 'info';
      case 'low_visibility': return 'faded';
      case 'low_data': return 'quiet';
      default: return 'normal';
    }
  }

  /** One-line explanation of *why* a row's signal fired, with its actual
   * numbers — the badge label alone ("Attention, no submissions") names the
   * state but not the threshold that triggered it. */
  function signalExplain(item: AdminFigurineAnalyticsListItem): string {
    switch (item.signal) {
      case 'low_data':
        return `Fewer than 10 views (${item.views}) in range — too little data to read.`;
      case 'high_conversion':
        return `${item.conversionRate.toFixed(1)}% conversion (≥12%) on ${item.submissions} submission${item.submissions === 1 ? '' : 's'} (≥2).`;
      case 'attention_no_submissions':
        return `${item.engagedViews} engaged view${item.engagedViews === 1 ? '' : 's'} / ${item.ctaClicks} CTA click${item.ctaClicks === 1 ? '' : 's'}, 0 submissions.`;
      case 'growing_interest':
        return 'Views this week are up at least 30% on the week before.';
      case 'low_visibility':
        return `Only ${item.views} views in range (<25).`;
      default:
        return `${item.views} views, ${item.submissions} submissions — nothing notable either way.`;
    }
  }

  function sourceLabel(value: string): string {
    return value.replace(/_/g, ' ');
  }

  function shortText(value: string, max = 60): string {
    return value.length <= max ? value : `${value.slice(0, max - 1)}…`;
  }

  /** Group daily points into ISO-week-ish (UTC, 7-day) buckets once the range
   * gets long enough that one bar per day stops being readable. Days with no
   * aggregate row (zero activity — the backend omits all-zero rows) are
   * zero-filled first so the x-axis stays continuous; otherwise a run of quiet
   * days would silently vanish and make unrelated bars look chronologically
   * adjacent. */
  type ChartBar = { label: string; views: number; submissions: number; days: string[] };
  function bucketDaily(points: AnalyticsDailyPoint[], from: string, to: string): ChartBar[] {
    const byDay = new Map(points.map((p) => [p.day, p]));
    const totalDays = Math.round((new Date(`${to}T00:00:00Z`).getTime() - new Date(`${from}T00:00:00Z`).getTime()) / 86_400_000) + 1;
    const filled = Array.from({ length: totalDays }, (_, i) => {
      const day = shiftUtc(from, i);
      const p = byDay.get(day);
      return { day, views: p?.views ?? 0, submissions: p?.submissions ?? 0 };
    });
    if (totalDays <= 45) {
      return filled.map((p) => ({ label: p.day.slice(5), views: p.views, submissions: p.submissions, days: [p.day] }));
    }
    const buckets = new Map<number, ChartBar>();
    filled.forEach((p, dayIndex) => {
      const bucketIndex = Math.floor(dayIndex / 7);
      const existing = buckets.get(bucketIndex);
      if (existing) {
        existing.views += p.views;
        existing.submissions += p.submissions;
        existing.days.push(p.day);
      } else {
        buckets.set(bucketIndex, { label: p.day.slice(5), views: p.views, submissions: p.submissions, days: [p.day] });
      }
    });
    return [...buckets.entries()].sort((a, b) => a[0] - b[0]).map(([, v]) => v);
  }

  function annotationsForDays(days: string[]): AnalyticsAnnotation[] {
    return annotations.filter((a) => days.includes(a.day));
  }

  /** Last-14-days-ending-at-`to`, zero-filled — the same shape as the Works
   * table's row sparklines, reused here for consistency. */
  function spark14(daily: { day: string; value: number }[], to: string): number[] {
    const byDay = new Map(daily.map((d) => [d.day, d.value]));
    return Array.from({ length: 14 }, (_, i) => byDay.get(shiftUtc(to, i - 13)) ?? 0);
  }
</script>

<div class="dashboard">
  <header class="dash-header">
    <div class="title-block">
      <p class="eyebrow">The house, measured</p>
      <h2>Analytics</h2>
    </div>

    <div class="toolbar">
      <div class="period-tabs">
        {#each ([['today','Today'],['7d','7d'],['30d','30d'],['90d','90d'],['this_month','This month'],['last_month','Last month']] as [Preset, string][]) as [p, label]}
          <button type="button" class:active={preset === p} onclick={() => setPreset(p)}>{label}</button>
        {/each}
      </div>
      <div class="custom-range">
        <input type="date" bind:value={customFrom} aria-label="Custom range start" />
        <span>–</span>
        <input type="date" bind:value={customTo} aria-label="Custom range end" />
        <button type="button" onclick={applyCustomRange}>Apply</button>
      </div>
      <label class="compare-toggle">
        <input type="checkbox" bind:checked={compareEnabled} />
        Compare to previous period
      </label>
      <button type="button" class="backfill-btn" onclick={runBackfill} disabled={backfillRunning} title="Re-run daily aggregation over the full history — use after a fix to the aggregation logic itself.">
        {backfillRunning ? 'Backfilling…' : '↺ Backfill history'}
      </button>
    </div>
  </header>
  {#if backfillMessage}<p class="backfill-message">{backfillMessage}</p>{/if}

  {#if loading && !overview}
    <div class="state">Loading analytics…</div>
  {:else if error && !overview}
    <div class="state state--error">{error}</div>
  {:else if overview && page}
    <nav class="tab-bar">
      {#each (['pulse', 'traffic', 'works', 'community'] as Tab[]) as t}
        <button type="button" class:active={activeTab === t} onclick={() => { activeTab = t; }}>
          {TAB_LABELS[t]}
          {#if t === 'works' && signalCounts.attention > 0}<span class="tab-badge">{signalCounts.attention}</span>{/if}
        </button>
      {/each}
    </nav>

    <p class="range-caption">
      {range.from} – {range.to}
      {#if compareEnabled}<span class="muted"> · vs {overview.previousFrom} – {overview.previousTo}</span>{/if}
      {#if loading}<span class="reload-note">· Updating…</span>{/if}
      {#if error}<span class="reload-note reload-note--error">· {error}</span>{/if}
    </p>

    {#if activeTab === 'pulse'}
    <div class="signal-digest">
      {#if signalCounts.attention > 0}
        <button type="button" class="digest-chip digest-chip--warn" onclick={() => jumpToSignal('attention')}>⚠ {signalCounts.attention} need{signalCounts.attention === 1 ? 's' : ''} attention</button>
      {/if}
      {#if signalCounts.growing > 0}
        <button type="button" class="digest-chip digest-chip--info" onclick={() => jumpToSignal('growing')}>📈 {signalCounts.growing} growing</button>
      {/if}
      {#if signalCounts.highConversion > 0}
        <button type="button" class="digest-chip digest-chip--good" onclick={() => jumpToSignal('high_conversion')}>✓ {signalCounts.highConversion} converting well</button>
      {/if}
      {#if signalCounts.attention === 0 && signalCounts.growing === 0 && signalCounts.highConversion === 0}
        <span class="muted">Nothing urgent right now — every work reads normal or low-data.</span>
      {/if}
    </div>

    <details class="glossary">
      <summary>What do these terms mean?</summary>
      <dl>
        <dt>Views / Daily uniques</dt><dd>A figurine page load; uniques are distinct visitors per day (privacy-preserving daily hash — the same person on two different days counts twice, by design, since visits aren't linked across days).</dd>
        <dt>Engaged %</dt><dd>Share of views with meaningful time on the page or scroll depth.</dd>
        <dt>Conversion</dt><dd>Submissions ÷ engaged views.</dd>
        <dt>Signal</dt><dd>An automatic read of a work's numbers — High conversion (≥12% conversion, ≥2 submissions), Attention (engaged but 0 submissions), Growing interest (+30% week over week), Low visibility (&lt;25 views), Low data (&lt;10 views).</dd>
        <dt>Funnel steps</dt><dd>Visited → Viewed works → Opened /commission → Started the form → Submitted. Only "Submitted" is exact; the rest are client-side events that undercount (missed by Do-Not-Track, bots, or a direct link to the form).</dd>
      </dl>
    </details>

    <!-- ── OVERVIEW ─────────────────────────────────────────────────── -->
    <section class="a-block">
      <h3 class="block-label"><span>Overview</span></h3>
      <div class="overview-grid">
        <div class="trend-card">
          <div class="trend">
            {#each overviewChart as bar}
              {@const maxViews = Math.max(...overviewChart.map((b) => b.views), 1)}
              {@const marks = annotationsForDays(bar.days)}
              <div class="bar-col" title="{bar.label}: {fmt(bar.views)} views, {fmt(bar.submissions)} submissions{marks.length ? ' — ' + marks.map((m) => m.label).join('; ') : ''}">
                <div class="bar-stack">
                  <i class="bar bar--views" style="height:{pct(bar.views, maxViews)}%"></i>
                  <i class="bar bar--subs" style="height:{pct(bar.submissions, maxViews)}%"></i>
                  {#if marks.length}<i class="annotation-mark"></i>{/if}
                </div>
                <span>{bar.label}</span>
              </div>
            {:else}
              <div class="empty-plot">No visits yet in this range.</div>
            {/each}
          </div>
          <div class="legend">
            <span><i class="legend-dot legend-dot--views"></i>Views</span>
            <span><i class="legend-dot legend-dot--subs"></i>Submissions</span>
            <span><i class="legend-dot legend-dot--mark"></i>Annotation</span>
          </div>

          <div class="annotations-panel">
            <div class="annotation-add">
              <input type="date" bind:value={newAnnotationDay} aria-label="Annotation date" />
              <input type="text" placeholder="e.g. Posted to Instagram" bind:value={newAnnotationLabel} maxlength="200" aria-label="Annotation label" />
              <button type="button" onclick={addAnnotation} disabled={annotationSaving || !newAnnotationDay || !newAnnotationLabel.trim()}>Add</button>
            </div>
            {#if annotationError}<p class="state state--error state--compact">{annotationError}</p>{/if}
            {#if annotations.length > 0}
              <ul class="annotation-list">
                {#each annotations as a}
                  <li><span class="muted">{a.day}</span> {a.label} <button type="button" class="annotation-remove" onclick={() => removeAnnotation(a.id)} aria-label="Remove annotation">×</button></li>
                {/each}
              </ul>
            {/if}
          </div>
        </div>

        <div class="kpi-grid">
          <div class="kpi">
            <span>Visits</span>
            <strong>{fmt(overview.summary.views)}</strong>
            {#if compareEnabled && viewsDelta}<small class="delta delta--{viewsDelta.tone}" class:low-data={viewsDelta.lowData}>{viewsDelta.text}{#if viewsDelta.lowData} · low data{/if}</small>{/if}
          </div>
          <div class="kpi">
            <span>Daily uniques</span>
            <strong>{fmt(overview.summary.uniqueVisitors)}</strong>
            {#if compareEnabled && uniquesDelta}<small class="delta delta--{uniquesDelta.tone}" class:low-data={uniquesDelta.lowData}>{uniquesDelta.text}{#if uniquesDelta.lowData} · low data{/if}</small>{/if}
            <small class="hint">Counted per day — the same visitor across two days counts twice, by design (privacy-preserving daily hash, no cross-day tracking).</small>
          </div>
          <div class="kpi">
            <span>Submissions</span>
            <strong>{fmt(overview.summary.submissions)}</strong>
            {#if compareEnabled && submissionsDelta}<small class="delta delta--{submissionsDelta.tone}" class:low-data={submissionsDelta.lowData}>{submissionsDelta.text}{#if submissionsDelta.lowData} · low data{/if}</small>{/if}
          </div>
          <div class="kpi">
            <span>Conversion</span>
            <strong>{conversionLabel?.text}</strong>
            <small class="hint">{fmt(overview.summary.submissions)} of {fmt(overview.summary.engagedViews)} engaged views{#if conversionLabel?.lowData} · low data{/if}</small>
          </div>
        </div>
      </div>
    </section>

    <!-- ── COMMISSION FUNNEL ────────────────────────────────────────── -->
    <section class="a-block">
      <h3 class="block-label"><span>Commission funnel</span></h3>
      {#if commissionFunnel}
        {@const steps = [
          { label: 'Visited', value: commissionFunnel.visited },
          { label: 'Viewed works', value: commissionFunnel.viewedWorks },
          { label: 'Opened /commission', value: commissionFunnel.openedCommissionPage },
          { label: 'Started the form', value: commissionFunnel.startedForm },
          { label: 'Submitted', value: commissionFunnel.submitted },
        ]}
        {@const maxVal = Math.max(commissionFunnel.visited, 1)}
        <div class="commission-funnel">
          {#each steps as step, i}
            {@const prevValue = i > 0 ? steps[i - 1].value : step.value}
            <div class="commission-step">
              <span class="step-label">{step.label}</span>
              <div class="step-bar"><i style="width:{pct(step.value, maxVal)}%"></i></div>
              <span class="step-value">{fmt(step.value)}</span>
              {#if i > 0}
                {@const stepRate = rateLabel(step.value, prevValue)}
                <span class="step-rate" class:low-data={stepRate.lowData}>{stepRate.text}</span>
              {:else}
                <span class="step-rate"></span>
              {/if}
            </div>
          {/each}
        </div>
        {#if commissionFunnel.rawDataFrom > commissionFunnel.from}
          <p class="section-note">Visited/viewed/opened/started counts only go back to {commissionFunnel.rawDataFrom} (raw event retention); "Submitted" covers the full selected range.</p>
        {/if}
        <p class="section-note">Every step but "Submitted" is a distinct-visitor count from client-side events (missed by DNT, bots, or a direct link to the form) — "Submitted" is exact, from the commissions table.</p>
      {:else}
        <p class="muted">Loading…</p>
      {/if}
    </section>
    {/if}

    {#if activeTab === 'traffic'}
    <details class="glossary">
      <summary>What do these terms mean?</summary>
      <dl>
        <dt>Source</dt><dd>Search, social, newsletter — an external channel a visit is attributed to. Internal = browsing from one page on this site to another. Referral = linked in from another site. Direct = typed URL or no referrer at all.</dd>
        <dt>Country</dt><dd>Resolved offline from the visitor's IP address via GeoIP — the IP itself is never stored, only the resolved country.</dd>
        <dt>"—" / unknown</dt><dd>The visit couldn't be geolocated — no GeoIP database configured on the server, or an unresolvable/private IP.</dd>
      </dl>
    </details>

    <!-- ── SOURCES ──────────────────────────────────────────────────── -->
    <section class="a-block">
      <h3 class="block-label"><span>Sources</span></h3>
      {#if overview.sources.length === 0}
        <p class="muted">No source data yet.</p>
      {:else}
        {@const maxViews = Math.max(...overview.sources.map((s) => s.views), 1)}
        {@const totalViews = overview.sources.reduce((sum, s) => sum + s.views, 0)}
        <div class="source-table">
          {#each overview.sources as s}
            <div class="source-row">
              <span class="source-name">{sourceLabel(s.source)}</span>
              <div class="source-bar"><i style="width:{pct(s.views, maxViews)}%"></i></div>
              <span class="source-views">{fmt(s.views)}</span>
              <span class="source-share muted">{rate(s.views, totalViews).toFixed(1)}%</span>
            </div>
          {/each}
        </div>
        <p class="section-note">Submissions aren't yet attributed per channel — that needs last-touch tracking on the order/booking/waitlist/commission forms themselves, not just the figurine page view.</p>
      {/if}
    </section>

    <!-- ── GEOGRAPHY ────────────────────────────────────────────────── -->
    <section class="a-block">
      <h3 class="block-label"><span>Geography</span></h3>
      {#if overview.geo.length === 0}
        <p class="muted">No geography data yet.</p>
      {:else}
        {@const known = overview.geo.filter((g) => g.key !== 'unknown')}
        {@const totalGeoViews = overview.geo.reduce((sum, g) => sum + g.views, 0)}
        {@const maxGeoViews = Math.max(...known.map((g) => g.views), 1)}
        {@const unknownGeoViews = overview.geo.find((g) => g.key === 'unknown')?.views ?? 0}
        <div class="geo-toolbar">
          <button type="button" class="csv-btn" onclick={exportGeoCsv}>↧ Export CSV</button>
        </div>
        <div class="geo-grid">
          <div class="geo-map-card">
            <WorldMap
              data={overview.geo}
              selected={countryFilter === 'all' ? null : countryFilter}
              onSelect={(code) => { countryFilter = code ?? 'all'; }}
            />
          </div>
          <div class="geo-list-card">
            {#if known.length === 0}
              <p class="muted">No visits with a resolved country yet{#if unknownGeoViews > 0} — {fmt(unknownGeoViews)} visit{unknownGeoViews === 1 ? '' : 's'} couldn't be geolocated{/if}. This needs a GeoIP database configured on the server (<code>GEOIP_DB_PATH</code>) — without one, every visit resolves to "unknown".</p>
            {:else}
              <div class="country-table">
                {#each known.slice(0, 12) as g}
                  {@const code = g.key.toUpperCase()}
                  <button
                    type="button"
                    class="country-row"
                    class:active={countryFilter === code}
                    onclick={() => { countryFilter = countryFilter === code ? 'all' : code; }}
                  >
                    <span class="country-name">{countryName(g.key)}</span>
                    <div class="country-bar"><i style="width:{pct(g.views, maxGeoViews)}%"></i></div>
                    <span class="country-views">{fmt(g.views)}</span>
                    <span class="country-share muted">{rate(g.views, totalGeoViews).toFixed(1)}%</span>
                  </button>
                {/each}
              </div>
            {/if}
            {#if countryFilter !== 'all'}
              <div class="geo-drill">
                <p class="drill-label">
                  Top works viewed from {countryName(countryFilter)}
                  <button type="button" class="clear-country" onclick={() => { countryFilter = 'all'; }}>Clear ×</button>
                </p>
                {#if topWorksForSelectedCountry.length === 0}
                  <p class="muted">No works recorded a view from here yet.</p>
                {:else}
                  <ul class="drill-list">
                    {#each topWorksForSelectedCountry as item}
                      <li><span class="drill-name">{shortText(item.name, 34)}</span><span class="drill-views">{fmt(item.views)}</span></li>
                    {/each}
                  </ul>
                {/if}
              </div>
            {/if}
          </div>
        </div>
        <p class="section-note">Country is resolved offline from the visitor's IP (GeoIP), which is never stored — only the resolved country persists. Clicking a country also filters the Works tab to works viewed from there.</p>
      {/if}
    </section>
    {/if}

    {#if activeTab === 'works'}
    <details class="glossary">
      <summary>What do these terms mean?</summary>
      <dl>
        <dt>Country / Source / Device filters</dt><dd>"Had at least one matching view" — an existence filter, not a metric recompute. Views, Engaged %, Submissions and Conv. shown stay totals across all countries/sources/devices even when one of these is active.</dd>
        <dt>Trend (14d)</dt><dd>Always the last 14 days ending today, regardless of the selected date range above — a fixed, comparable shape for every row.</dd>
        <dt>Low data</dt><dd>Italicized values are based on fewer than 10 samples and can swing sharply from a single visit.</dd>
      </dl>
    </details>

    <!-- ── WORKS ────────────────────────────────────────────────────── -->
    <section class="a-block">
      <h3 class="block-label"><span>Works</span></h3>

      <div class="filters-head">
        <span class="filters-count">{activeFilterCount > 0 ? `${activeFilterCount} filter${activeFilterCount === 1 ? '' : 's'} active` : 'No filters active'}</span>
        {#if activeFilterCount > 0}<button type="button" class="clear-filters-btn" onclick={clearAllFilters}>Clear all</button>{/if}
      </div>

      <div class="works-filters">
        <input class="search-input" placeholder="Search works…" bind:value={search} aria-label="Search works" />
        <select bind:value={statusFilter} aria-label="Filter by status">
          <option value="all">All statuses</option>
          <option value="available">Available</option>
          <option value="reserved">Reserved</option>
          <option value="sold">Sold</option>
          <option value="in_progress">In progress</option>
        </select>
        <select bind:value={signalFilter} aria-label="Filter by signal">
          <option value="all">All signals</option>
          <option value="attention_no_submissions">Attention, no submissions</option>
          <option value="high_conversion">High conversion</option>
          <option value="growing_interest">Growing interest</option>
          <option value="low_visibility">Low visibility</option>
          <option value="low_data">Low data</option>
          <option value="normal">Normal</option>
        </select>
        <button type="button" class="toggle-chip" class:active={growingFilter} onclick={() => { growingFilter = !growingFilter; }} title="Positive week-over-week growth, regardless of which signal badge won">📈 Growing only</button>
        <select bind:value={performanceFilter} aria-label="Filter by activity">
          <option value="all">All works</option>
          <option value="has_views">Has views</option>
          <option value="has_submissions">Has submissions</option>
        </select>
        <select bind:value={countryFilter} aria-label="Filter by country" title="Had at least one view from this country in range">
          <option value="all">All countries</option>
          {#each countryOptions as c}
            <option value={c}>{countryName(c)}</option>
          {/each}
        </select>
        <select bind:value={sourceFilter} aria-label="Filter by top source">
          <option value="all">All sources</option>
          {#each sourceOptions as s}
            <option value={s}>{sourceLabel(s)}</option>
          {/each}
        </select>
        <select bind:value={deviceFilter} aria-label="Filter by top device">
          <option value="all">All devices</option>
          {#each deviceOptions as d}
            <option value={d}>{sourceLabel(d)}</option>
          {/each}
        </select>
        {#if seriesOptions.length > 0}
          <select bind:value={seriesFilter} aria-label="Filter by series">
            <option value="all">All series</option>
            {#each seriesOptions as s}
              <option value={s}>{s}</option>
            {/each}
          </select>
        {/if}
        <button type="button" class="csv-btn" onclick={exportWorksCsv} disabled={sortedItems.length === 0}>↧ Export CSV</button>
      </div>

      {#if countryFilter !== 'all' || sourceFilter !== 'all' || deviceFilter !== 'all'}
        <p class="filter-banner">Narrowed to works with at least one matching view — Views / Engaged % / Submissions / Conv. shown remain totals across all countries, sources and devices, not scoped to this filter.</p>
      {/if}

      <div class="table-scroll">
        <table>
          <thead>
            <tr>
              <th><button type="button" onclick={() => setSort('name')}>Work {#if sort === 'name'}<span>{dir === 'asc' ? '▲' : '▼'}</span>{/if}</button></th>
              <th><button type="button" onclick={() => setSort('status')}>Status {#if sort === 'status'}<span>{dir === 'asc' ? '▲' : '▼'}</span>{/if}</button></th>
              <th class="right"><button type="button" onclick={() => setSort('views')}>Views {#if sort === 'views'}<span>{dir === 'asc' ? '▲' : '▼'}</span>{/if}</button></th>
              <th class="right" title="Daily uniques — the same visitor across two days counts twice"><button type="button" onclick={() => setSort('uniqueVisitors')}>Uniques {#if sort === 'uniqueVisitors'}<span>{dir === 'asc' ? '▲' : '▼'}</span>{/if}</button></th>
              <th class="right"><button type="button" onclick={() => setSort('engagedViews')}>Engaged % {#if sort === 'engagedViews'}<span>{dir === 'asc' ? '▲' : '▼'}</span>{/if}</button></th>
              <th class="right"><button type="button" onclick={() => setSort('submissions')}>Submissions {#if sort === 'submissions'}<span>{dir === 'asc' ? '▲' : '▼'}</span>{/if}</button></th>
              <th class="right"><button type="button" onclick={() => setSort('conversionRate')}>Conv. {#if sort === 'conversionRate'}<span>{dir === 'asc' ? '▲' : '▼'}</span>{/if}</button></th>
              <th title="Most common country among this work's views">Country</th>
              <th title="Daily views, last 14 days — a fixed window, independent of the date range above">Trend (14d)</th>
              <th>Signal</th>
            </tr>
          </thead>
          <tbody>
            {#each sortedItems as item (item.figurineId)}
              {@const eng = rateLabel(item.engagedViews, item.views)}
              {@const maxSpark = Math.max(...item.sparkline, 1)}
              {@const isSelected = selectedId === item.figurineId}
              <tr class:selected={isSelected} onclick={() => selectFigurine(item.figurineId)}>
                <td class="name-cell">
                  {#if item.faceUrl}<img src={item.faceUrl} alt="" />{/if}
                  <a href="/figurines/{item.figurineId}" target="_blank" rel="noopener" title={item.name} onclick={(e) => e.stopPropagation()}>{shortText(item.name, 42)}</a>
                </td>
                <td>{item.status.replace('_', ' ')}</td>
                <td class="right">{fmt(item.views)}</td>
                <td class="right">{fmt(item.uniqueVisitors)}</td>
                <td class="right" class:low-data={eng.lowData}>{eng.text}</td>
                <td class="right">{fmt(item.submissions)}</td>
                <td class="right">{item.conversionRate.toFixed(1)}%</td>
                <td class="country-cell">
                  {#if !item.topCountry || item.topCountry === 'unknown'}
                    <span class="muted" title="No visit could be geolocated for this work in range">—</span>
                  {:else}
                    <span class:low-data={item.views < MIN_RATE_SAMPLE} title="{countryName(item.topCountry)}{item.views < MIN_RATE_SAMPLE ? ' · low data' : ''}">{item.topCountry.toUpperCase()}</span>
                  {/if}
                </td>
                <td class="spark-cell">
                  <div class="spark">
                    {#each item.sparkline as v}
                      <i style="height:{pct(v, maxSpark)}%"></i>
                    {/each}
                  </div>
                </td>
                <td>
                  <span class="badge badge--{signalTone(item.signal)}" title={signalExplain(item)}>{signalLabel(item.signal)}</span>
                  {#if item.isGrowing && item.signal !== 'growing_interest'}
                    <span class="growing-mark" title="Also growing week over week (+30% or more)">📈</span>
                  {/if}
                </td>
              </tr>
              {#if isSelected}
                <tr class="drilldown-row" bind:this={drilldownRowEl}>
                  <td colspan="10">
                    {#if detailLoading}
                      <div class="state state--compact">Loading…</div>
                    {:else if detail && selectedItem}
                      <div class="drilldown">
                        <div class="drilldown-head">
                          <strong>{detail.figurine.name}</strong>
                          <span class="badge badge--{signalTone(detail.signal)}">{signalLabel(detail.signal)}</span>
                          <button type="button" class="close-btn" onclick={(e) => { e.stopPropagation(); selectFigurine(selectedId!); }}>Close ×</button>
                        </div>

                        <div class="drilldown-grid">
                          <div class="panel">
                            <h4>Daily trend</h4>
                            <div class="trend trend--small">
                              {#each detailChart as bar}
                                {@const maxViews = Math.max(...detailChart.map((b) => b.views), 1)}
                                <div class="bar-col" title="{bar.label}: {fmt(bar.views)} views">
                                  <div class="bar-stack">
                                    <i class="bar bar--views" style="height:{pct(bar.views, maxViews)}%"></i>
                                  </div>
                                  <span>{bar.label}</span>
                                </div>
                              {:else}
                                <div class="empty-plot">No daily data.</div>
                              {/each}
                            </div>
                          </div>

                          <div class="panel">
                            <h4>Starts → submitted</h4>
                            <div class="funnel-table">
                              {#each detail.ctaFunnel.filter((s) => s.starts > 0 || s.submitted > 0) as step}
                                {@const conv = rateLabel(step.submitted, step.starts)}
                                <div class="funnel-row">
                                  <span>{CTA_LABELS[step.ctaType] ?? step.ctaType}</span>
                                  <span class="muted">{fmt(step.starts)} started</span>
                                  <span>{fmt(step.submitted)} submitted</span>
                                  <span class:low-data={conv.lowData}>{conv.text}</span>
                                </div>
                              {:else}
                                <p class="muted">No CTA activity yet.</p>
                              {/each}
                            </div>
                            <p class="section-note">Starts are client-side clicks (missed by Do-Not-Track, bots, or direct links to a form); submitted is exact — conversion can read over 100%.</p>
                          </div>

                          <div class="panel">
                            <h4>Engagement</h4>
                            <div class="stat-row"><span>Median time on card</span><strong>{msLabel(detail.medianDurationMs)}</strong></div>
                            <div class="stat-row"><span>Median scroll depth</span><strong>{detail.medianScrollDepth === null ? 'No data' : `${Math.round(detail.medianScrollDepth)}%`}</strong></div>
                            {#if detail.rawDataFrom > range.from}
                              <p class="section-note">Medians, sources, device and browser below only go back to {detail.rawDataFrom} (raw event retention) — the country breakdown is a permanent rollup, so it covers the full selected range.</p>
                            {/if}
                          </div>

                          <div class="panel">
                            <h4>Sources</h4>
                            <div class="compact-list">
                              {#each detail.sources as s}
                                <span>{sourceLabel(s.source)} <strong>{fmt(s.views)}</strong></span>
                              {:else}
                                <p class="muted">No source data.</p>
                              {/each}
                            </div>
                          </div>

                          <div class="panel">
                            <h4>Geo &amp; device</h4>
                            <div class="compact-list">
                              {#each detail.countries as c}
                                <span title={countryName(c.key)}>{c.key === 'unknown' ? '—' : c.key.toUpperCase()} <strong>{fmt(c.views)}</strong></span>
                              {/each}
                              {#each detail.devices as d}
                                <span>{sourceLabel(d.key)} <strong>{fmt(d.views)}</strong></span>
                              {/each}
                            </div>
                          </div>

                          <div class="panel">
                            <h4>Language &amp; entry block</h4>
                            <div class="compact-list">
                              {#each detail.languages as l}
                                <span>{l.key.toUpperCase()} <strong>{fmt(l.views)}</strong></span>
                              {/each}
                              {#each detail.internalSources as s}
                                <span>{sourceLabel(s.key)} <strong>{fmt(s.views)}</strong></span>
                              {:else}
                                <p class="muted">No internal referral data.</p>
                              {/each}
                            </div>
                          </div>
                        </div>
                      </div>
                    {/if}
                  </td>
                </tr>
              {/if}
            {:else}
              <tr><td colspan="10" class="empty-row">No works match this filter.</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
    {/if}

    {#if activeTab === 'community'}
    <details class="glossary">
      <summary>What do these terms mean?</summary>
      <dl>
        <dt>Marks of attention</dt><dd>Visitors tapping "noticed" / favorite on a work.</dd>
        <dt>Book of the House</dt><dd>Newsletter signups from the home page.</dd>
        <dt>Comments</dt><dd>Comments posted across all works.</dd>
      </dl>
    </details>

    <!-- ── LIFE OF THE HOUSE ────────────────────────────────────────── -->
    <section class="a-block">
      <h3 class="block-label"><span>Life of the house</span></h3>
      {#if lifeOfHouse}
        {@const marksSpark = spark14(lifeOfHouse.daily.map((d) => ({ day: d.day, value: d.marks })), range.to)}
        {@const subsSpark = spark14(lifeOfHouse.daily.map((d) => ({ day: d.day, value: d.subscribers })), range.to)}
        {@const commentsSpark = spark14(lifeOfHouse.daily.map((d) => ({ day: d.day, value: d.comments })), range.to)}
        {@const marksDelta = delta(lifeOfHouse.marksTotal, lifeOfHouse.previousMarksTotal)}
        {@const subsDelta = delta(lifeOfHouse.subscribersTotal, lifeOfHouse.previousSubscribersTotal)}
        {@const commentsDelta = delta(lifeOfHouse.commentsTotal, lifeOfHouse.previousCommentsTotal)}
        <div class="life-grid">
          {#each [
            { label: 'Marks of attention', total: lifeOfHouse.marksTotal, d: marksDelta, spark: marksSpark },
            { label: 'Book of the House signups', total: lifeOfHouse.subscribersTotal, d: subsDelta, spark: subsSpark },
            { label: 'Comments', total: lifeOfHouse.commentsTotal, d: commentsDelta, spark: commentsSpark },
          ] as row}
            {@const maxSpark = Math.max(...row.spark, 1)}
            <div class="life-row">
              <div class="life-label">
                <span>{row.label}</span>
                <strong>{fmt(row.total)}</strong>
                {#if compareEnabled}<small class="delta delta--{row.d.tone}" class:low-data={row.d.lowData}>{row.d.text}</small>{/if}
              </div>
              <div class="spark life-spark">
                {#each row.spark as v}
                  <i style="height:{pct(v, maxSpark)}%"></i>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <p class="muted">Loading…</p>
      {/if}
    </section>
    {/if}
  {/if}
</div>

<style>
  .dashboard {
    height: 100%;
    overflow: auto;
    padding: 1.25rem 1.5rem 2.5rem;
    color: #34251c;
    background: #f8f1e7;
    font-family: 'Inter', system-ui, -apple-system, sans-serif;
    font-size: 13px;
  }

  h2, h3, h4, p { margin: 0; }

  h2 {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.3rem;
    font-weight: 600;
  }

  .eyebrow {
    margin: 0 0 0.15rem;
    color: #6f3b24;
    font-size: 0.68rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .dash-header {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 0.85rem;
    margin-bottom: 0.5rem;
    padding-bottom: 0.85rem;
    border-bottom: 1px solid #d8c6b1;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.65rem;
  }

  .period-tabs {
    display: inline-flex;
    border: 1px solid #d8c6b1;
    background: #fff;
  }

  .period-tabs button {
    padding: 0.35rem 0.6rem;
    border: 0;
    border-right: 1px solid #d8c6b1;
    background: transparent;
    color: #6f3b24;
    font-size: 0.72rem;
    cursor: pointer;
  }

  .period-tabs button:last-child { border-right: 0; }

  .period-tabs button.active {
    background: #6f3b24;
    color: #f8f1e7;
  }

  .custom-range {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .custom-range input,
  .search-input,
  .works-filters select {
    border: 1px solid #d8c6b1;
    background: #fff;
    color: #34251c;
    font: inherit;
    padding: 0.3rem 0.45rem;
  }

  .custom-range button {
    border: 1px solid #6f3b24;
    background: #fff;
    color: #6f3b24;
    padding: 0.3rem 0.6rem;
    font-size: 0.72rem;
    cursor: pointer;
  }

  .custom-range button:hover { background: #6f3b24; color: #f8f1e7; }

  .compare-toggle {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.72rem;
    color: #6f3b24;
  }

  .backfill-btn {
    border: 1px dashed #b0a08e;
    background: #fff;
    color: #8a6f5c;
    padding: 0.3rem 0.6rem;
    font-size: 0.7rem;
    cursor: pointer;
    white-space: nowrap;
  }

  .backfill-btn:hover:not(:disabled) { border-color: #6f3b24; color: #6f3b24; }
  .backfill-btn:disabled { opacity: 0.6; cursor: default; }

  .backfill-message {
    margin: -0.4rem 0 0.85rem;
    font-size: 0.72rem;
    color: #6f3b24;
    font-style: italic;
  }

  .tab-bar {
    display: flex;
    gap: 0.25rem;
    margin: 0.9rem 0 0;
    border-bottom: 1px solid #d8c6b1;
  }

  .tab-bar button {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.5rem 0.9rem;
    margin-bottom: -1px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: #8a6f5c;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.86rem;
    cursor: pointer;
  }

  .tab-bar button:hover { color: #6f3b24; }

  .tab-bar button.active {
    color: #6f3b24;
    font-weight: 600;
    border-bottom-color: #c65f3c;
  }

  .tab-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.1rem;
    height: 1.1rem;
    padding: 0 0.3rem;
    border-radius: 999px;
    background: #a3402b;
    color: #fff;
    font-size: 0.62rem;
    font-weight: 700;
  }

  .signal-digest {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    margin: 0.9rem 0 1.1rem;
  }

  .digest-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    border: 1px solid #d8c6b1;
    background: #fff;
    color: #6f3b24;
    padding: 0.4rem 0.7rem;
    font-size: 0.76rem;
    cursor: pointer;
  }

  .digest-chip:hover { background: #fbf3e7; }
  .digest-chip--warn { border-color: #a3402b; color: #a3402b; }
  .digest-chip--info { border-color: #c65f3c; color: #c65f3c; }
  .digest-chip--good { border-color: #3c6e3f; color: #3c6e3f; }

  .glossary {
    margin-bottom: 1.1rem;
    border: 1px solid #d8c6b1;
    background: #fff;
    padding: 0.55rem 0.85rem;
  }

  .glossary summary {
    cursor: pointer;
    color: #6f3b24;
    font-size: 0.76rem;
    font-weight: 600;
  }

  .glossary dl {
    margin: 0.6rem 0 0;
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 0.3rem 0.9rem;
    font-size: 0.72rem;
  }

  .glossary dt { color: #6f3b24; font-weight: 600; white-space: nowrap; }
  .glossary dd { margin: 0; color: #5f4636; }

  .range-caption {
    margin-bottom: 0.85rem;
    font-size: 0.74rem;
    color: #6f3b24;
  }

  .reload-note {
    margin-left: 0.4rem;
    color: #8a6f5c;
    font-style: italic;
  }

  .reload-note--error { color: #a3402b; }

  .muted { color: #8a6f5c; }

  .a-block {
    margin-bottom: 1.4rem;
  }

  .block-label {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.7rem;
    color: #6f3b24;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.78rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .block-label::after {
    content: '';
    flex: 1;
    height: 1px;
    background: #d8c6b1;
  }

  .overview-grid {
    display: grid;
    grid-template-columns: minmax(0, 1.5fr) minmax(16rem, 1fr);
    gap: 0.9rem;
    align-items: stretch;
  }

  .trend-card,
  .panel,
  .table-scroll,
  .drilldown {
    background: #fff;
    border: 1px solid #d8c6b1;
    padding: 0.85rem;
  }

  .trend {
    height: 9rem;
    display: flex;
    align-items: end;
    gap: 0.25rem;
    border-bottom: 1px solid #d8c6b1;
  }

  .trend--small { height: 7rem; }

  .bar-col {
    flex: 1;
    min-width: 0.4rem;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: end;
    align-items: center;
    gap: 0.2rem;
  }

  .bar-stack {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: end;
  }

  .bar { display: block; width: 100%; min-height: 2px; }
  .bar--views { background: #6f3b24; }
  .bar--subs { position: absolute; right: 0; bottom: 0; width: 40%; background: #c65f3c; }

  .annotation-mark {
    position: absolute;
    left: 50%;
    top: -0.5rem;
    width: 0.4rem;
    height: 0.4rem;
    transform: translateX(-50%) rotate(45deg);
    background: #b08820;
  }

  .bar-col span {
    font-size: 0.56rem;
    color: #8a6f5c;
    writing-mode: vertical-rl;
  }

  .legend {
    display: flex;
    gap: 0.85rem;
    margin-top: 0.55rem;
    color: #8a6f5c;
    font-size: 0.66rem;
  }

  .legend-dot {
    display: inline-block;
    width: 0.5rem;
    height: 0.5rem;
    margin-right: 0.28rem;
  }

  .legend-dot--views { background: #6f3b24; }
  .legend-dot--subs { background: #c65f3c; }
  .legend-dot--mark { background: #b08820; transform: rotate(45deg); }

  .annotations-panel {
    margin-top: 0.7rem;
    padding-top: 0.6rem;
    border-top: 1px solid #ece1d1;
  }

  .annotation-add {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  .annotation-add input {
    border: 1px solid #d8c6b1;
    background: #fff;
    color: #34251c;
    font: inherit;
    font-size: 0.72rem;
    padding: 0.25rem 0.4rem;
  }

  .annotation-add input[type="text"] { flex: 1; min-width: 10rem; }

  .annotation-add button {
    border: 1px solid #6f3b24;
    background: #fff;
    color: #6f3b24;
    padding: 0.25rem 0.6rem;
    font-size: 0.72rem;
    cursor: pointer;
  }

  .annotation-add button:disabled { opacity: 0.4; cursor: default; }

  .annotation-list {
    list-style: none;
    margin: 0.5rem 0 0;
    padding: 0;
    display: grid;
    gap: 0.3rem;
    font-size: 0.72rem;
  }

  .annotation-list li { display: flex; align-items: center; gap: 0.4rem; }

  .annotation-remove {
    margin-left: auto;
    border: 0;
    background: none;
    color: #a3402b;
    cursor: pointer;
    font-size: 0.85rem;
    line-height: 1;
  }

  .kpi-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.6rem;
  }

  .kpi {
    padding: 0.7rem 0.75rem;
    background: #fff;
    border: 1px solid #d8c6b1;
    border-top: 2px solid #6f3b24;
  }

  .kpi span { display: block; color: #8a6f5c; font-size: 0.68rem; }

  .kpi strong {
    display: block;
    margin: 0.2rem 0;
    font-size: 1.3rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  .kpi small { display: block; font-size: 0.66rem; }
  .kpi .hint { color: #8a6f5c; margin-top: 0.15rem; }

  .delta { font-weight: 700; }
  .delta--up { color: #3c6e3f; }
  .delta--down { color: #a3402b; }
  .delta--flat { color: #8a6f5c; }
  .delta--new { color: #6f3b24; }
  .low-data { font-weight: 400; font-style: italic; color: #8a6f5c; }

  .source-table {
    display: grid;
    gap: 0.4rem;
    background: #fff;
    border: 1px solid #d8c6b1;
    padding: 0.85rem;
  }

  .source-row {
    display: grid;
    grid-template-columns: 8rem minmax(0, 1fr) 3.5rem 3.5rem;
    gap: 0.6rem;
    align-items: center;
    font-size: 0.78rem;
  }

  .source-name { text-transform: capitalize; }

  .source-bar {
    height: 0.5rem;
    background: #f0e6d8;
  }

  .source-bar i { display: block; height: 100%; background: #c65f3c; }

  .source-views { text-align: right; font-variant-numeric: tabular-nums; }
  .source-share { text-align: right; font-variant-numeric: tabular-nums; }

  .geo-grid {
    display: grid;
    grid-template-columns: minmax(0, 1.6fr) minmax(16rem, 1fr);
    gap: 0.9rem;
    align-items: start;
  }

  .geo-map-card,
  .geo-list-card {
    background: #fff;
    border: 1px solid #d8c6b1;
    padding: 0.85rem;
  }

  .country-table {
    display: grid;
    gap: 0.15rem;
  }

  .country-row {
    display: grid;
    grid-template-columns: 7rem minmax(0, 1fr) 3rem 3rem;
    gap: 0.6rem;
    align-items: center;
    width: 100%;
    border: 0;
    background: transparent;
    padding: 0.22rem 0.15rem;
    font: inherit;
    font-size: 0.76rem;
    color: #34251c;
    text-align: left;
    cursor: pointer;
  }

  .country-row:hover { background: #fbf3e7; }

  .country-row.active {
    background: #f5e5cc;
    box-shadow: inset 3px 0 0 #c65f3c;
  }

  .country-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .country-bar { height: 0.5rem; background: #f0e6d8; }
  .country-bar i { display: block; height: 100%; background: #c65f3c; }
  .country-views { text-align: right; font-variant-numeric: tabular-nums; }
  .country-share { text-align: right; font-variant-numeric: tabular-nums; }

  .geo-drill {
    margin-top: 0.7rem;
    padding-top: 0.6rem;
    border-top: 1px solid #ece1d1;
  }

  .drill-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.4rem;
    color: #6f3b24;
    font-size: 0.72rem;
  }

  .clear-country {
    margin-left: auto;
    border: 0;
    background: none;
    color: #a3402b;
    font-size: 0.68rem;
    cursor: pointer;
  }

  .drill-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.25rem;
    font-size: 0.74rem;
  }

  .drill-list li {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .drill-views { font-variant-numeric: tabular-nums; color: #8a6f5c; }

  .country-cell {
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.02em;
  }

  .section-note {
    margin-top: 0.5rem;
    color: #8a6f5c;
    font-size: 0.7rem;
    font-style: italic;
  }

  .commission-funnel {
    display: grid;
    gap: 0.45rem;
    background: #fff;
    border: 1px solid #d8c6b1;
    padding: 0.85rem;
  }

  .commission-step {
    display: grid;
    grid-template-columns: 9rem minmax(0, 1fr) 4rem 4.5rem;
    gap: 0.65rem;
    align-items: center;
    font-size: 0.78rem;
  }

  .step-bar {
    height: 0.55rem;
    background: #f0e6d8;
  }

  .step-bar i { display: block; height: 100%; background: #6f3b24; }

  .step-value {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .step-rate {
    text-align: right;
    color: #8a6f5c;
    font-variant-numeric: tabular-nums;
  }

  .filters-head {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.4rem;
    font-size: 0.72rem;
    color: #8a6f5c;
  }

  .clear-filters-btn {
    border: 0;
    background: none;
    color: #a3402b;
    font-size: 0.72rem;
    cursor: pointer;
  }

  .filter-banner {
    margin: 0 0 0.65rem;
    padding: 0.45rem 0.6rem;
    background: #fbf3e7;
    border-left: 2px solid #c65f3c;
    color: #6f3b24;
    font-size: 0.72rem;
    font-style: italic;
  }

  .toggle-chip {
    border: 1px solid #d8c6b1;
    background: #fff;
    color: #6f3b24;
    padding: 0.3rem 0.55rem;
    font: inherit;
    font-size: 0.72rem;
    cursor: pointer;
  }

  .toggle-chip.active {
    background: #6f3b24;
    color: #f8f1e7;
    border-color: #6f3b24;
  }

  .growing-mark {
    margin-left: 0.3rem;
    font-size: 0.7rem;
    cursor: help;
  }

  .geo-toolbar {
    display: flex;
    justify-content: flex-end;
    margin-bottom: 0.5rem;
  }

  .works-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.65rem;
  }

  .search-input { min-width: 14rem; }

  .csv-btn {
    margin-left: auto;
    border: 1px solid #6f3b24;
    background: #fff;
    color: #6f3b24;
    padding: 0.3rem 0.6rem;
    font-size: 0.72rem;
    cursor: pointer;
  }

  .csv-btn:disabled { opacity: 0.4; cursor: default; }

  .table-scroll {
    max-height: min(70vh, 42rem);
    overflow-x: auto;
    overflow-y: auto;
    padding: 0;
  }

  table {
    width: 100%;
    min-width: 52rem;
    border-collapse: collapse;
    font-size: 0.76rem;
  }

  th, td {
    padding: 0.5rem 0.6rem;
    border-top: 1px solid #ece1d1;
    text-align: left;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  th {
    position: sticky;
    top: 0;
    background: #fff;
    border-bottom: 1px solid #d8c6b1;
  }

  th button {
    border: 0;
    background: transparent;
    color: #6f3b24;
    font: inherit;
    font-size: 0.66rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    cursor: pointer;
  }

  th.right button { margin-left: auto; display: block; }
  td.right, th.right { text-align: right; }

  tbody tr { cursor: pointer; }
  tbody tr:hover { background: #fbf3e7; }

  tbody tr.selected {
    background: #f5e5cc;
    box-shadow: inset 3px 0 0 #c65f3c;
  }

  .drilldown-row {
    cursor: default;
  }

  .drilldown-row:hover { background: transparent; }

  .drilldown-row td {
    padding: 0;
    white-space: normal;
    cursor: default;
  }

  .name-cell {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    max-width: 16rem;
  }

  .name-cell img {
    width: 1.8rem;
    height: 1.8rem;
    object-fit: cover;
    background: #ece1d1;
  }

  .name-cell a {
    color: #34251c;
    text-decoration: none;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .name-cell a:hover { text-decoration: underline; color: #c65f3c; }

  .spark-cell { width: 5rem; }

  .spark {
    display: flex;
    align-items: end;
    gap: 1px;
    height: 1.6rem;
    width: 4.5rem;
  }

  .spark i { flex: 1; display: block; background: #d8c6b1; min-height: 1px; }

  .life-grid {
    display: grid;
    gap: 0.5rem;
    background: #fff;
    border: 1px solid #d8c6b1;
    padding: 0.85rem;
  }

  .life-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 8rem;
    align-items: center;
    gap: 1rem;
  }

  .life-label {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    font-size: 0.8rem;
  }

  .life-label span { color: #6f3b24; }
  .life-label strong { font-variant-numeric: tabular-nums; }

  .life-spark {
    height: 2rem;
    width: 8rem;
  }

  .badge {
    display: inline-block;
    padding: 0.1rem 0.4rem;
    border: 1px solid #d8c6b1;
    color: #6f3b24;
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .badge--good { border-color: #3c6e3f; color: #3c6e3f; }
  .badge--warn { border-color: #a3402b; color: #a3402b; }
  .badge--info { border-color: #c65f3c; color: #c65f3c; }
  .badge--faded { border-color: #b0a08e; color: #a08c72; }
  .badge--quiet { border-style: dashed; border-color: #d8c6b1; color: #8a6f5c; }
  .badge--normal { border-color: #d8c6b1; color: #6f3b24; }

  .empty-row, .empty-plot { color: #8a6f5c; font-size: 0.76rem; }
  .empty-plot { width: 100%; align-self: center; text-align: center; }

  .drilldown-head {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.75rem;
  }

  .drilldown-head strong {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.95rem;
  }

  .close-btn {
    margin-left: auto;
    border: 1px solid #d8c6b1;
    background: #fff;
    color: #6f3b24;
    padding: 0.25rem 0.55rem;
    font-size: 0.7rem;
    cursor: pointer;
  }

  .drilldown-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.65rem;
  }

  .panel h4 {
    margin-bottom: 0.5rem;
    color: #6f3b24;
    font-size: 0.7rem;
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .funnel-table { display: grid; gap: 0.4rem; }

  .funnel-row {
    display: grid;
    grid-template-columns: 1fr auto auto auto;
    gap: 0.5rem;
    font-size: 0.74rem;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    padding: 0.3rem 0;
    border-top: 1px solid #ece1d1;
  }

  .stat-row:first-of-type { border-top: 0; }

  .compact-list { display: flex; flex-wrap: wrap; gap: 0.35rem; }

  .compact-list span {
    padding: 0.18rem 0.4rem;
    border: 1px solid #ece1d1;
    color: #6f3b24;
    font-size: 0.66rem;
  }

  .compact-list strong { color: #34251c; }

  .state {
    min-height: 12rem;
    display: grid;
    place-items: center;
    color: #8a6f5c;
  }

  .state--compact { min-height: 4rem; }
  .state--error { color: #a3402b; }

  @media (max-width: 1100px) {
    .overview-grid { grid-template-columns: 1fr; }
    .kpi-grid { grid-template-columns: 1fr 1fr; }
    .drilldown-grid { grid-template-columns: 1fr 1fr; }
    .geo-grid { grid-template-columns: 1fr; }
  }

  @media (max-width: 700px) {
    .kpi-grid { grid-template-columns: 1fr; }
    .drilldown-grid { grid-template-columns: 1fr; }
  }
</style>
