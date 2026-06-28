<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type {
    AdminFigurineAnalyticsDetail,
    AdminFigurineAnalyticsListPage,
    AnalyticsSignal,
  } from '$lib/types/api';

  type Period = '7d' | '30d' | '90d';
  type PerformanceFilter = 'all' | 'active' | 'with_submissions' | 'no_submissions' | 'high_cta';
  type SortKey =
    | 'name'
    | 'status'
    | 'views'
    | 'uniqueVisitors'
    | 'engagedViews'
    | 'ctaClicks'
    | 'submissions'
    | 'conversionRate';

  const columns: { key?: SortKey; label: string; align?: 'left' | 'right' }[] = [
    { key: 'name', label: 'Card', align: 'left' },
    { key: 'status', label: 'Status', align: 'left' },
    { label: 'Source', align: 'left' },
    { label: 'Country', align: 'left' },
    { label: 'Device', align: 'left' },
    { label: 'Browser', align: 'left' },
    { key: 'views', label: 'Views' },
    { key: 'uniqueVisitors', label: 'Unique' },
    { key: 'engagedViews', label: 'Engaged' },
    { key: 'ctaClicks', label: 'CTA' },
    { key: 'submissions', label: 'Submissions' },
    { key: 'conversionRate', label: 'Conv.' },
  ];

  let loading = $state(true);
  let detailLoading = $state(false);
  let error = $state('');
  let period = $state<Period>('30d');
  let sort = $state<SortKey>('views');
  let dir = $state<'asc' | 'desc'>('desc');
  let search = $state('');
  let statusFilter = $state('all');
  let signalFilter = $state('all');
  let sourceFilter = $state('all');
  let countryFilter = $state('all');
  let deviceFilter = $state('all');
  let browserFilter = $state('all');
  let performanceFilter = $state<PerformanceFilter>('all');
  let minViews = $state('');
  let selectedId = $state<string | null>(null);
  let page = $state<AdminFigurineAnalyticsListPage | null>(null);
  let detail = $state<AdminFigurineAnalyticsDetail | null>(null);

  let range = $derived.by(() => {
    const days = period === '7d' ? 7 : period === '90d' ? 90 : 30;
    const to = new Date();
    const from = new Date();
    from.setDate(to.getDate() - days + 1);
    return { from: isoDate(from), to: isoDate(to), days };
  });

  let filteredItems = $derived.by(() => {
    const items = page?.items ?? [];
    const q = search.trim().toLowerCase();
    const min = Number.parseInt(minViews, 10);
    return items.filter((item) => {
      const source = item.topSource ?? 'unknown';
      const country = item.topCountry ?? 'unknown';
      const device = item.topDevice ?? 'unknown';
      const browser = item.topBrowser ?? 'unknown';
      if (q && ![
        item.name,
        item.status,
        signalLabel(item.signal),
        source,
        country,
        device,
        browser,
      ].some((value) => value.toLowerCase().includes(q))) return false;
      if (statusFilter !== 'all' && item.status !== statusFilter) return false;
      if (signalFilter !== 'all' && item.signal !== signalFilter) return false;
      if (sourceFilter !== 'all' && source !== sourceFilter) return false;
      if (countryFilter !== 'all' && country !== countryFilter) return false;
      if (deviceFilter !== 'all' && device !== deviceFilter) return false;
      if (browserFilter !== 'all' && browser !== browserFilter) return false;
      if (Number.isFinite(min) && item.views < min) return false;
      if (performanceFilter === 'active' && item.views === 0) return false;
      if (performanceFilter === 'with_submissions' && item.submissions === 0) return false;
      if (performanceFilter === 'no_submissions' && item.views > 0 && item.submissions > 0) return false;
      if (performanceFilter === 'high_cta' && rate(item.ctaClicks, item.engagedViews) < 25) return false;
      return true;
    });
  });

  let selectedItem = $derived(
    filteredItems.find((item) => item.figurineId === selectedId) ?? null
  );

  let activeItems = $derived((page?.items ?? []).filter((item) => item.views > 0));
  let priorityItems = $derived.by(() =>
    [...(page?.items ?? [])]
      .filter((item) => item.signal === 'attention_no_submissions' || item.signal === 'high_conversion' || item.signal === 'low_visibility')
      .sort((a, b) => signalWeight(b.signal) - signalWeight(a.signal) || b.views - a.views)
      .slice(0, 5)
  );
  let topAttention = $derived([...(page?.items ?? [])].sort((a, b) => b.engagedViews - a.engagedViews)[0] ?? null);
  let topConverter = $derived([...(page?.items ?? [])].sort((a, b) => b.conversionRate - a.conversionRate || b.submissions - a.submissions)[0] ?? null);
  let noSubmissionCount = $derived((page?.items ?? []).filter((item) => item.views >= 10 && item.submissions === 0).length);

  let maxDaily = $derived(Math.max(...(detail?.daily ?? []).map((d) => d.views), 1));
  let maxSource = $derived(Math.max(...(detail?.sources ?? []).map((s) => s.views), 1));
  let selectedEngagementRate = $derived(rate(detail?.summary.engagedViews ?? 0, detail?.summary.views ?? 0));
  let selectedClickRate = $derived(rate(detail?.summary.ctaClicks ?? 0, detail?.summary.engagedViews ?? 0));
  let selectedConversionRate = $derived(detail?.summary.conversionRate ?? 0);
  let sourceOptions = $derived(optionValues(page?.items.map((item) => item.topSource) ?? []));
  let countryOptions = $derived(optionValues(page?.items.map((item) => item.topCountry) ?? []));
  let deviceOptions = $derived(optionValues(page?.items.map((item) => item.topDevice) ?? []));
  let browserOptions = $derived(optionValues(page?.items.map((item) => item.topBrowser) ?? []));
  let hasFilters = $derived(
    search.trim() !== '' ||
    statusFilter !== 'all' ||
    signalFilter !== 'all' ||
    sourceFilter !== 'all' ||
    countryFilter !== 'all' ||
    deviceFilter !== 'all' ||
    browserFilter !== 'all' ||
    performanceFilter !== 'all' ||
    minViews.trim() !== ''
  );

  onMount(() => {
    void loadList();
  });

  $effect(() => {
    if (!selectedId) return;
    void loadDetail(selectedId);
  });

  async function loadList() {
    loading = true;
    error = '';
    try {
      page = await api.listFigurineAnalytics({ ...range, sort, dir });
      if (selectedId && !page.items.some((item) => item.figurineId === selectedId)) {
        selectedId = null;
        detail = null;
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
      detail = await api.getFigurineAnalytics(id, range);
    } catch (e) {
      error = String(e);
    } finally {
      detailLoading = false;
    }
  }

  function isoDate(date: Date): string {
    return date.toISOString().slice(0, 10);
  }

  function setPeriod(next: Period) {
    period = next;
    void loadList();
  }

  function setSort(key: SortKey) {
    if (sort === key) {
      dir = dir === 'asc' ? 'desc' : 'asc';
    } else {
      sort = key;
      dir = key === 'name' || key === 'status' ? 'asc' : 'desc';
    }
    void loadList();
  }

  function selectFigurine(id: string) {
    selectedId = id;
  }

  function clearFilters() {
    search = '';
    statusFilter = 'all';
    signalFilter = 'all';
    sourceFilter = 'all';
    countryFilter = 'all';
    deviceFilter = 'all';
    browserFilter = 'all';
    performanceFilter = 'all';
    minViews = '';
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
      case 'low_visibility': return 'muted';
      case 'growing_interest': return 'info';
      case 'low_data': return 'quiet';
      default: return 'neutral';
    }
  }

  function signalWeight(signal: AnalyticsSignal): number {
    switch (signal) {
      case 'attention_no_submissions': return 5;
      case 'high_conversion': return 4;
      case 'growing_interest': return 3;
      case 'low_visibility': return 2;
      case 'low_data': return 1;
      default: return 0;
    }
  }

  function rate(value: number, total: number): number {
    return total <= 0 ? 0 : Math.round((value / total) * 10000) / 100;
  }

  function pct(value: number, max: number): number {
    return max <= 0 ? 0 : Math.max(2, Math.round((value / max) * 100));
  }

  function fmt(value: number): string {
    return new Intl.NumberFormat().format(value);
  }

  function optionValues(values: Array<string | null | undefined>): string[] {
    return [...new Set(values.map((v) => v || 'unknown'))].sort((a, b) => a.localeCompare(b));
  }

  function breakdownMax(items: { views: number }[] | undefined): number {
    return Math.max(...(items ?? []).map((item) => item.views), 1);
  }

  function shortText(value: string, max = 72): string {
    return value.length <= max ? value : `${value.slice(0, max - 1)}...`;
  }

  function sourceLabel(value: string): string {
    return value.replace(/_/g, ' ');
  }
</script>

<div class="dashboard">
  <header class="dash-header">
    <div class="title-block">
      <p class="eyebrow">Card analytics</p>
      <h2>Figurine performance</h2>
      <p class="header-note">{range.from} to {range.to}</p>
    </div>

    <div class="toolbar" aria-label="Dashboard controls">
      <div class="period-tabs" aria-label="Analytics period">
        {#each ['7d', '30d', '90d'] as p}
          <button
            type="button"
            class:active={period === p}
            onclick={() => setPeriod(p as Period)}
          >{p}</button>
        {/each}
      </div>
      <label class="search-box">
        <span>Search</span>
        <input
          placeholder="Name, status, signal"
          bind:value={search}
          aria-label="Find figurine analytics"
        />
      </label>
      <button type="button" class="refresh-btn" onclick={() => loadList()} disabled={loading}>
        Refresh
      </button>
    </div>
  </header>

  {#if loading}
    <div class="state">Loading analytics...</div>
  {:else if error}
    <div class="state state--error">{error}</div>
  {:else if page}
    <section class="kpi-grid" aria-label="Analytics summary">
      <div class="kpi kpi--primary">
        <span>Total views</span>
        <strong>{fmt(page.summary.views)}</strong>
        <small>{fmt(activeItems.length)} active of {fmt(page.total)} cards</small>
      </div>
      <div class="kpi">
        <span>Unique visitors</span>
        <strong>{fmt(page.summary.uniqueVisitors)}</strong>
        <small>Privacy-preserving daily estimate</small>
      </div>
      <div class="kpi">
        <span>Engagement rate</span>
        <strong>{rate(page.summary.engagedViews, page.summary.views).toFixed(2)}%</strong>
        <small>{fmt(page.summary.engagedViews)} engaged views</small>
      </div>
      <div class="kpi">
        <span>CTA rate</span>
        <strong>{rate(page.summary.ctaClicks, page.summary.engagedViews).toFixed(2)}%</strong>
        <small>{fmt(page.summary.ctaClicks)} clicks after attention</small>
      </div>
      <div class="kpi">
        <span>Submissions</span>
        <strong>{fmt(page.summary.submissions)}</strong>
        <small>{page.summary.conversionRate.toFixed(2)}% from engaged views</small>
      </div>
    </section>

    <section class="insight-band" aria-label="Priority insights">
      <article class="insight insight--warn">
        <span class="insight-label">Needs attention</span>
        <strong>{fmt(noSubmissionCount)}</strong>
        <p>Cards have enough views but no submissions.</p>
      </article>
      <article class="insight">
        <span class="insight-label">Most attention</span>
        <strong title={topAttention?.name}>{topAttention ? shortText(topAttention.name, 78) : 'No data'}</strong>
        <p>{topAttention ? `${fmt(topAttention.engagedViews)} engaged views` : 'Engagement will appear after visits.'}</p>
      </article>
      <article class="insight insight--good">
        <span class="insight-label">Best converter</span>
        <strong title={topConverter?.name}>{topConverter ? shortText(topConverter.name, 78) : 'No data'}</strong>
        <p>{topConverter ? `${topConverter.conversionRate.toFixed(2)}% conversion` : 'Conversion needs more data.'}</p>
      </article>
    </section>

    <section class="filters-panel" aria-label="Analytics filters">
      <label>
        <span>Status</span>
        <select bind:value={statusFilter}>
          <option value="all">All statuses</option>
          <option value="available">Available</option>
          <option value="reserved">Reserved</option>
          <option value="sold">Sold</option>
          <option value="in_progress">In progress</option>
        </select>
      </label>
      <label>
        <span>Signal</span>
        <select bind:value={signalFilter}>
          <option value="all">All signals</option>
          <option value="attention_no_submissions">Attention</option>
          <option value="high_conversion">High conversion</option>
          <option value="growing_interest">Growing interest</option>
          <option value="low_visibility">Low visibility</option>
          <option value="low_data">Low data</option>
          <option value="normal">Normal</option>
        </select>
      </label>
      <label>
        <span>Source</span>
        <select bind:value={sourceFilter}>
          <option value="all">All sources</option>
          {#each sourceOptions as value}<option value={value}>{sourceLabel(value)}</option>{/each}
        </select>
      </label>
      <label>
        <span>Country</span>
        <select bind:value={countryFilter}>
          <option value="all">All countries</option>
          {#each countryOptions as value}<option value={value}>{value.toUpperCase()}</option>{/each}
        </select>
      </label>
      <label>
        <span>Device</span>
        <select bind:value={deviceFilter}>
          <option value="all">All devices</option>
          {#each deviceOptions as value}<option value={value}>{sourceLabel(value)}</option>{/each}
        </select>
      </label>
      <label>
        <span>Browser</span>
        <select bind:value={browserFilter}>
          <option value="all">All browsers</option>
          {#each browserOptions as value}<option value={value}>{sourceLabel(value)}</option>{/each}
        </select>
      </label>
      <label>
        <span>Performance</span>
        <select bind:value={performanceFilter}>
          <option value="all">All cards</option>
          <option value="active">Has views</option>
          <option value="with_submissions">Has submissions</option>
          <option value="no_submissions">Views, no submissions</option>
          <option value="high_cta">CTA rate >= 25%</option>
        </select>
      </label>
      <label>
        <span>Min views</span>
        <input type="number" min="0" step="1" bind:value={minViews} placeholder="0" />
      </label>
      <button type="button" class="clear-btn" onclick={clearFilters} disabled={!hasFilters}>
        Clear filters
      </button>
    </section>

    <div class="workspace">
      <section class="table-panel table-panel--primary" aria-label="Figurine comparison">
        <div class="panel-head">
          <h4>Card comparison</h4>
          <span>{fmt(filteredItems.length)} rows</span>
        </div>
        <div class="table-scroll">
          <table>
            <thead>
              <tr>
                {#each columns as column}
                  <th class:right={column.align !== 'left'}>
                    {#if column.key}
                      {@const key = column.key}
                      <button type="button" onclick={() => setSort(key)}>
                        {column.label}
                        {#if sort === column.key}<span>{dir === 'asc' ? 'Asc' : 'Desc'}</span>{/if}
                      </button>
                    {:else}
                      <span class="static-head">{column.label}</span>
                    {/if}
                  </th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each filteredItems as item}
                <tr class:selected={selectedId === item.figurineId} onclick={() => selectFigurine(item.figurineId)}>
                  <td class="name-cell">
                    {#if item.faceUrl}<img src={item.faceUrl} alt="" />{/if}
                    <div>
                      <a href="/figurines/{item.figurineId}" target="_blank" rel="noopener" title={item.name} onclick={(event) => event.stopPropagation()}>{item.name}</a>
                      <span class="badge badge--{signalTone(item.signal)}">{signalLabel(item.signal)}</span>
                    </div>
                  </td>
                  <td>{item.status.replace('_', ' ')}</td>
                  <td>{sourceLabel(item.topSource ?? 'unknown')}</td>
                  <td>{(item.topCountry ?? 'unknown').toUpperCase()}</td>
                  <td>{sourceLabel(item.topDevice ?? 'unknown')}</td>
                  <td>{sourceLabel(item.topBrowser ?? 'unknown')}</td>
                  <td>{fmt(item.views)}</td>
                  <td>{fmt(item.uniqueVisitors)}</td>
                  <td>{fmt(item.engagedViews)}</td>
                  <td>{fmt(item.ctaClicks)}</td>
                  <td>{fmt(item.submissions)}</td>
                  <td>{item.conversionRate.toFixed(2)}%</td>
                </tr>
              {:else}
                <tr>
                  <td colspan="12" class="empty-row">No cards match this filter.</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>

      <aside class="priority-panel" aria-label="Action queue">
        <div class="panel-head">
          <h4>Action queue</h4>
          <span>Top signals</span>
        </div>
        <div class="priority-list">
          {#each priorityItems as item}
            <button type="button" class:selected={selectedId === item.figurineId} onclick={() => selectFigurine(item.figurineId)}>
              <span class="badge badge--{signalTone(item.signal)}">{signalLabel(item.signal)}</span>
              <strong title={item.name}>{shortText(item.name, 62)}</strong>
              <small>{fmt(item.views)} views · {fmt(item.submissions)} submissions</small>
            </button>
          {:else}
            <p class="muted">Signals will appear after more visits.</p>
          {/each}
        </div>

        <div class="rail-divider"></div>

        <section class="rail-detail" aria-label="Selected figurine drilldown">
          <div class="panel-head">
            <h4>Row detail</h4>
            <span>{selectedId ? 'Selected' : 'Click a row'}</span>
          </div>
          {#if !selectedId}
            <p class="muted">No row selected.</p>
          {:else if detailLoading}
            <div class="state state--compact">Loading selected row...</div>
          {:else if detail && selectedItem}
            <div class="rail-card">
              {#if selectedItem.faceUrl}
                <img src={selectedItem.faceUrl} alt="" />
              {/if}
              <div>
                <span class="badge badge--{signalTone(detail.signal)}">{signalLabel(detail.signal)}</span>
                <strong title={detail.figurine.name}>{shortText(detail.figurine.name, 68)}</strong>
                <a href="/figurines/{detail.figurine.id}" target="_blank" rel="noopener">Open public card</a>
              </div>
            </div>

            <div class="rail-metrics">
              <div><span>Views</span><strong>{fmt(detail.summary.views)}</strong></div>
              <div><span>Unique</span><strong>{fmt(detail.summary.uniqueVisitors)}</strong></div>
              <div><span>Engaged</span><strong>{selectedEngagementRate.toFixed(2)}%</strong></div>
              <div><span>CTA</span><strong>{selectedClickRate.toFixed(2)}%</strong></div>
              <div><span>Conv.</span><strong>{selectedConversionRate.toFixed(2)}%</strong></div>
            </div>

            <div class="mini-section">
              <div class="panel-head">
                <h4>Funnel</h4>
                <span>Selected row</span>
              </div>
              <div class="funnel">
                <div><span>Views</span><strong>{fmt(detail.funnel.views)}</strong><i style="width:100%"></i></div>
                <div><span>Engaged</span><strong>{fmt(detail.funnel.engagedViews)}</strong><i style="width:{pct(detail.funnel.engagedViews, detail.funnel.views)}%"></i></div>
                <div><span>CTA clicks</span><strong>{fmt(detail.funnel.ctaClicks)}</strong><i style="width:{pct(detail.funnel.ctaClicks, detail.funnel.views)}%"></i></div>
                <div><span>Submissions</span><strong>{fmt(detail.funnel.submissions)}</strong><i style="width:{pct(detail.funnel.submissions, detail.funnel.views)}%"></i></div>
              </div>
            </div>

            <div class="mini-section">
              <div class="panel-head">
                <h4>Sources</h4>
                <span>{fmt(detail.sources.length)}</span>
              </div>
              <div class="source-list source-list--compact">
                {#each detail.sources as source}
                  <div class="source-row source-row--compact">
                    <span>{sourceLabel(source.source)}</span>
                    <div><i style="width:{pct(source.views, maxSource)}%"></i></div>
                    <strong>{fmt(source.views)}</strong>
                  </div>
                {:else}
                  <p class="muted">No source data yet.</p>
                {/each}
              </div>
            </div>

            <div class="mini-section">
              <div class="panel-head">
                <h4>Geo</h4>
                <span>Country</span>
              </div>
              <div class="source-list source-list--compact">
                {#each detail.countries as point}
                  <div class="source-row source-row--compact">
                    <span>{point.key.toUpperCase()}</span>
                    <div><i style="width:{pct(point.views, breakdownMax(detail.countries))}%"></i></div>
                    <strong>{fmt(point.views)}</strong>
                  </div>
                {:else}
                  <p class="muted">No geo data yet.</p>
                {/each}
              </div>
            </div>

            <div class="split-mini">
              <div class="mini-section">
                <div class="panel-head">
                  <h4>Devices</h4>
                  <span>{fmt(detail.devices.length)}</span>
                </div>
                <div class="compact-list">
                  {#each detail.devices as point}
                    <span>{sourceLabel(point.key)} <strong>{fmt(point.views)}</strong></span>
                  {:else}
                    <p class="muted">No device data.</p>
                  {/each}
                </div>
              </div>
              <div class="mini-section">
                <div class="panel-head">
                  <h4>Browsers</h4>
                  <span>{fmt(detail.browsers.length)}</span>
                </div>
                <div class="compact-list">
                  {#each detail.browsers as point}
                    <span>{sourceLabel(point.key)} <strong>{fmt(point.views)}</strong></span>
                  {:else}
                    <p class="muted">No browser data.</p>
                  {/each}
                </div>
              </div>
            </div>

            <div class="mini-section">
              <div class="panel-head">
                <h4>Referrers / UTM</h4>
                <span>Top</span>
              </div>
              <div class="compact-list">
                {#each detail.referrers as point}
                  <span>{shortText(point.key, 26)} <strong>{fmt(point.views)}</strong></span>
                {/each}
                {#each detail.utmSources as point}
                  <span>utm:{shortText(point.key, 22)} <strong>{fmt(point.views)}</strong></span>
                {/each}
                {#if detail.referrers.length === 0 && detail.utmSources.length === 0}
                  <p class="muted">No referrer data.</p>
                {/if}
              </div>
            </div>

            <div class="mini-section">
              <div class="panel-head">
                <h4>Visitor/IP cohorts</h4>
                <span>Anonymized</span>
              </div>
              <div class="compact-list">
                {#each detail.visitorCohorts as point}
                  <span>#{point.key} <strong>{fmt(point.views)}</strong></span>
                {:else}
                  <p class="muted">No visitor cohort data.</p>
                {/each}
              </div>
            </div>
          {/if}
        </section>
      </aside>
    </div>

    {#if selectedId}
      <section class="focus-panel detail-panel" aria-label="Selected figurine drilldown">
        {#if detailLoading}
          <div class="state state--inline">Loading selected row...</div>
        {:else if detail && selectedItem}
          <div class="analysis-grid analysis-grid--full">
            <section class="panel" aria-label="Trend">
              <div class="panel-head">
                <h4>Daily trend</h4>
                <span>{range.days} days</span>
              </div>
              <div class="trend">
                {#each detail.daily as point}
                  <div class="bar-col" title="{point.day}: {point.views} views, {point.submissions} submissions">
                    <div class="bar-stack">
                      <i class="bar bar--views" style="height:{pct(point.views, maxDaily)}%"></i>
                      <i class="bar bar--subs" style="height:{pct(point.submissions, maxDaily)}%"></i>
                    </div>
                    <span>{point.day.slice(5)}</span>
                  </div>
                {:else}
                  <div class="empty-plot">No daily data yet.</div>
                {/each}
              </div>
              <div class="legend">
                <span><i class="legend-dot legend-dot--views"></i>Views</span>
                <span><i class="legend-dot legend-dot--subs"></i>Submissions</span>
              </div>
            </section>

          </div>
        {/if}
      </section>
    {/if}
  {/if}
</div>

<style>
  .dashboard {
    height: 100%;
    overflow: auto;
    padding: 1rem;
    color: #111827;
    background: #f3f4f6;
    font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-size: 13px;
  }

  .dash-header,
  .toolbar,
  .kpi-grid,
  .insight-band,
  .filters-panel,
  .workspace,
  .analysis-grid {
    display: grid;
    gap: 0.85rem;
  }

  .dash-header {
    grid-template-columns: minmax(16rem, 0.8fr) minmax(34rem, 1.2fr);
    align-items: start;
    margin-bottom: 0.75rem;
  }

  .title-block {
    min-width: 0;
  }

  .eyebrow,
  .header-note,
  .insight-label,
  .panel-head span,
  .kpi span,
  .funnel span {
    color: #6b7280;
    font-size: 0.72rem;
    letter-spacing: 0;
  }

  .eyebrow {
    margin: 0 0 0.15rem;
    text-transform: uppercase;
    font-weight: 760;
  }

  h2,
  h4,
  p {
    margin: 0;
  }

  h2 {
    font-size: 1.2rem;
    line-height: 1.15;
    font-weight: 760;
    letter-spacing: 0;
  }

  h4 {
    font-size: 0.78rem;
    font-weight: 760;
  }

  .toolbar {
    grid-template-columns: auto minmax(14rem, 22rem) auto;
    align-items: end;
    justify-content: end;
    gap: 0.5rem;
    min-width: 0;
  }

  .period-tabs {
    display: inline-flex;
    height: 2rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    background: #fff;
    overflow: hidden;
  }

  .period-tabs button,
  .refresh-btn,
  th button,
  .priority-list button {
    border: 0;
    background: transparent;
    color: inherit;
    cursor: pointer;
    font: inherit;
  }

  .period-tabs button {
    width: 2.75rem;
    height: 100%;
    font-size: 0.76rem;
  }

  .period-tabs button.active {
    background: #111827;
    color: #fff;
  }

  .search-box {
    display: grid;
    gap: 0.2rem;
  }

  .search-box {
    min-width: 0;
  }

  .search-box span {
    color: #6b7280;
    font-size: 0.68rem;
  }

  .search-box input,
  .filters-panel select,
  .filters-panel input,
  .refresh-btn {
    height: 2.25rem;
    min-width: 0;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    background: #fff;
    color: #111827;
  }

  .search-box input {
    width: 100%;
    padding: 0 0.65rem;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .refresh-btn {
    padding: 0 0.75rem;
    font-size: 0.76rem;
    font-weight: 650;
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .kpi-grid {
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 0.55rem;
    margin-bottom: 0.55rem;
  }

  .kpi,
  .insight,
  .focus-panel,
  .priority-panel,
  .panel,
  .table-panel,
  .state {
    background: #fff;
    border: 1px solid #d9dee5;
    border-radius: 8px;
    box-shadow: 0 1px 2px rgba(15, 23, 42, 0.04);
  }

  .kpi {
    min-width: 0;
    padding: 0.72rem 0.78rem;
    border-top: 2px solid #d9dee2;
  }

  .kpi--primary {
    border-top-color: #2563eb;
  }

  .kpi strong {
    display: block;
    margin: 0.18rem 0;
    font-size: 1.42rem;
    line-height: 1;
    font-weight: 780;
    font-variant-numeric: tabular-nums;
  }

  .kpi small,
  .priority-list small {
    color: #6b7280;
    font-size: 0.68rem;
  }

  .insight-band {
    grid-template-columns: 1fr 1fr 1fr;
    gap: 0.55rem;
    margin-bottom: 0.55rem;
  }

  .insight {
    min-width: 0;
    padding: 0.75rem 0.82rem;
    border-left: 3px solid #94a3b8;
  }

  .insight--warn {
    border-left-color: #c58b2a;
  }

  .insight--good {
    border-left-color: #35835a;
  }

  .insight strong {
    display: -webkit-box;
    min-height: 1.25rem;
    margin: 0.16rem 0;
    overflow: hidden;
    color: #111827;
    font-size: 0.92rem;
    line-height: 1.35;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
  }

  .filters-panel {
    grid-template-columns: repeat(8, minmax(0, 1fr)) auto;
    align-items: end;
    gap: 0.5rem;
    margin-bottom: 0.55rem;
    padding: 0.7rem;
    background: #fff;
    border: 1px solid #d9dee5;
    border-radius: 8px;
    box-shadow: 0 1px 2px rgba(15, 23, 42, 0.04);
  }

  .filters-panel label {
    display: grid;
    min-width: 0;
    gap: 0.2rem;
  }

  .filters-panel label span {
    color: #6b7280;
    font-size: 0.66rem;
  }

  .filters-panel select,
  .filters-panel input {
    width: 100%;
    padding: 0 0.55rem;
  }

  .clear-btn {
    height: 2.25rem;
    padding: 0 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    background: #fff;
    color: #111827;
    cursor: pointer;
    font: inherit;
    font-size: 0.76rem;
    font-weight: 650;
    white-space: nowrap;
  }

  .clear-btn:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .workspace {
    grid-template-columns: minmax(0, 1fr) 22rem;
    align-items: start;
    gap: 0.55rem;
    margin-bottom: 0.55rem;
  }

  .focus-panel,
  .priority-panel,
  .panel,
  .table-panel {
    padding: 0.85rem;
  }

  .name-cell a {
    color: #2563eb;
    text-decoration: none;
  }

  .name-cell a:hover {
    text-decoration: underline;
  }

  .analysis-grid {
    grid-template-columns: minmax(0, 1.35fr) minmax(16rem, 0.85fr);
    gap: 0.55rem;
    margin-bottom: 0.55rem;
  }

  .panel-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.55rem;
  }

  .trend {
    height: 10rem;
    display: flex;
    align-items: end;
    gap: 0.28rem;
    border-bottom: 1px solid #d1d5db;
    overflow: hidden;
  }

  .bar-col {
    flex: 1;
    min-width: 0.45rem;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: end;
    align-items: center;
    gap: 0.25rem;
  }

  .bar-stack {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: end;
  }

  .bar {
    display: block;
    width: 100%;
    min-height: 2px;
  }

  .bar--views {
    background: #2f6f8f;
  }

  .bar--subs {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 40%;
    background: #35835a;
  }

  .bar-col span {
    font-size: 0.58rem;
    color: #6b7280;
    writing-mode: vertical-rl;
  }

  .legend {
    display: flex;
    gap: 0.85rem;
    margin-top: 0.6rem;
    color: #6b7280;
    font-size: 0.68rem;
  }

  .legend-dot {
    display: inline-block;
    width: 0.55rem;
    height: 0.55rem;
    margin-right: 0.3rem;
  }

  .legend-dot--views {
    background: #2f6f8f;
  }

  .legend-dot--subs {
    background: #35835a;
  }

  .funnel {
    display: grid;
    gap: 0.55rem;
  }

  .funnel div {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.25rem 0.75rem;
    align-items: center;
  }

  .funnel i {
    grid-column: 1 / -1;
    display: block;
    height: 0.48rem;
    border-radius: 999px;
    background: #2563eb;
  }

  .source-list {
    display: grid;
    gap: 0.55rem;
  }

  .source-list--compact {
    gap: 0.38rem;
  }

  .source-row {
    display: grid;
    grid-template-columns: 7rem minmax(0, 1fr) 3.5rem 4.5rem;
    gap: 0.65rem;
    align-items: center;
    font-size: 0.78rem;
  }

  .source-row div {
    height: 0.5rem;
    border-radius: 999px;
    background: #e5e7eb;
    overflow: hidden;
  }

  .source-row i {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: #059669;
  }

  .source-row--compact {
    grid-template-columns: 5.5rem minmax(0, 1fr) 2.4rem;
    gap: 0.45rem;
    font-size: 0.7rem;
  }

  .split-mini {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.65rem;
  }

  .compact-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .compact-list span {
    max-width: 100%;
    padding: 0.22rem 0.42rem;
    border: 1px solid #e5e7eb;
    border-radius: 999px;
    background: #f8fafc;
    color: #4b5563;
    font-size: 0.66rem;
  }

  .compact-list strong {
    color: #111827;
    font-variant-numeric: tabular-nums;
  }

  .priority-list {
    display: grid;
    gap: 0.55rem;
  }

  .priority-list button {
    display: grid;
    gap: 0.18rem;
    width: 100%;
    padding: 0.65rem;
    text-align: left;
    border: 1px solid #e5e7eb;
    border-radius: 7px;
    background: #fff;
  }

  .priority-list button:hover,
  .priority-list button.selected {
    border-color: #2563eb;
    background: #eff6ff;
  }

  .priority-list strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rail-divider {
    height: 1px;
    margin: 0.85rem -0.85rem;
    background: #e5e7eb;
  }

  .rail-detail {
    display: grid;
    gap: 0.65rem;
  }

  .rail-card {
    display: grid;
    grid-template-columns: 3rem minmax(0, 1fr);
    gap: 0.65rem;
    align-items: start;
  }

  .rail-card img {
    width: 3rem;
    height: 3rem;
    border-radius: 7px;
    object-fit: cover;
    background: #e5e7eb;
  }

  .rail-card div {
    display: grid;
    min-width: 0;
    gap: 0.2rem;
  }

  .rail-card strong {
    overflow: hidden;
    color: #111827;
    font-size: 0.84rem;
    line-height: 1.3;
    text-overflow: ellipsis;
  }

  .rail-card a {
    color: #2563eb;
    font-size: 0.72rem;
    text-decoration: none;
  }

  .rail-card a:hover {
    text-decoration: underline;
  }

  .rail-metrics {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.45rem;
  }

  .rail-metrics div {
    padding: 0.5rem;
    border: 1px solid #e5e7eb;
    border-radius: 7px;
    background: #f8fafc;
  }

  .rail-metrics span {
    display: block;
    color: #6b7280;
    font-size: 0.64rem;
  }

  .rail-metrics strong {
    display: block;
    margin-top: 0.1rem;
    font-size: 0.88rem;
    font-variant-numeric: tabular-nums;
  }

  .mini-section {
    display: grid;
    gap: 0.45rem;
  }

  .badge {
    display: inline-flex;
    width: fit-content;
    max-width: 100%;
    padding: 0.12rem 0.38rem;
    border: 1px solid #d1d5db;
    border-radius: 999px;
    background: #f9fafb;
    color: #4b5563;
    font-size: 0.62rem;
    font-weight: 720;
    line-height: 1.25;
    text-transform: uppercase;
  }

  .badge--good {
    background: #e8f5ee;
    border-color: #bfdfce;
    color: #256344;
  }

  .badge--warn {
    background: #fff4d8;
    border-color: #ead39d;
    color: #765414;
  }

  .badge--muted,
  .badge--quiet {
    background: #f0f2f3;
    color: #5d6670;
  }

  .badge--info {
    background: #e6f2fb;
    border-color: #bdd8ea;
    color: #285f85;
  }

  .table-panel {
    padding-bottom: 0;
  }

  .table-scroll {
    max-height: clamp(22rem, 44vh, 36rem);
    overflow: auto;
    margin: 0 -0.85rem;
  }

  table {
    width: 100%;
    min-width: 58rem;
    border-collapse: collapse;
    font-size: 0.76rem;
  }

  th,
  td {
    border-top: 1px solid #e5e7eb;
    padding: 0.5rem 0.65rem;
    text-align: left;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  th {
    position: sticky;
    top: 0;
    z-index: 1;
    background: #fff;
  }

  th.right,
  td:not(:first-child):not(:nth-child(2)) {
    text-align: right;
  }

  th button {
    display: inline-flex;
    gap: 0.35rem;
    align-items: center;
    color: #6b7280;
    font-size: 0.66rem;
    font-weight: 720;
    text-transform: uppercase;
  }

  .static-head {
    color: #6b7280;
    font-size: 0.66rem;
    font-weight: 720;
    text-transform: uppercase;
  }

  th.right button {
    justify-content: flex-end;
    width: 100%;
  }

  th button span {
    color: #2563eb;
    font-size: 0.58rem;
  }

  tbody tr {
    cursor: pointer;
  }

  tbody tr:hover,
  tbody tr.selected {
    background: #eff6ff;
  }

  .name-cell {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    min-width: 20rem;
    max-width: 26rem;
  }

  .name-cell img {
    width: 2rem;
    height: 2rem;
    border-radius: 6px;
    object-fit: cover;
    background: #e5e7eb;
  }

  .name-cell div {
    display: grid;
    gap: 0.18rem;
    min-width: 0;
  }

  .name-cell a {
    display: block;
    max-width: 20rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .state {
    min-height: 16rem;
    display: grid;
    place-items: center;
    color: #6b7280;
  }

  .state--inline {
    min-height: 10rem;
  }

  .state--compact {
    min-height: 5rem;
  }

  .state--error {
    color: #9d2c1e;
  }

  .empty-row,
  .empty-plot,
  .muted {
    color: #6b7280;
    font-size: 0.76rem;
  }

  .empty-plot {
    width: 100%;
    align-self: center;
    text-align: center;
  }

  @media (max-width: 1420px) {
    .dash-header {
      grid-template-columns: 1fr;
    }

    .toolbar {
      justify-content: start;
      grid-template-columns: auto minmax(14rem, 1fr) auto;
    }

    .filters-panel {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }

    .clear-btn {
      width: fit-content;
    }
  }

  @media (max-width: 1180px) {
    .workspace,
    .analysis-grid {
      grid-template-columns: 1fr;
    }

    .kpi-grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .filters-panel {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .priority-panel {
      order: -1;
    }

    .table-scroll {
      max-height: 32rem;
    }
  }

  @media (max-width: 760px) {
    .dashboard {
      padding: 0.75rem;
    }

    .toolbar,
    .insight-band {
      grid-template-columns: 1fr;
    }

    .kpi-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .search-box {
      min-width: 0;
      max-width: none;
    }

    .source-row {
      grid-template-columns: 6rem minmax(0, 1fr) 3rem;
    }

    .split-mini {
      grid-template-columns: 1fr;
    }
  }
</style>
