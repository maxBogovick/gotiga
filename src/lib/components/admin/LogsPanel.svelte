<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api, authenticatedApiUrl, currentAuthHeaders } from '$lib/api';
  import type { AdminLogEntry, AdminLogsQuery, AdminLogsSortBy, AdminLogsSortDir } from '$lib/types/api';

  const LEVELS = ['', 'ERROR', 'WARN', 'INFO', 'DEBUG', 'TRACE'];
  const METHODS = ['', 'GET', 'POST', 'PUT', 'PATCH', 'DELETE'];
  const LIMIT = 250;
  const MAX_ROWS = 1200;

  let items = $state<AdminLogEntry[]>([]);
  let loading = $state(false);
  let live = $state(true);
  let error = $state('');
  let droppedTotal = $state(0);
  let nextOffset = $state<number | null>(null);
  let sortBy = $state<AdminLogsSortBy>('time');
  let sortDir = $state<AdminLogsSortDir>('desc');
  let selected = $state<AdminLogEntry | null>(null);
  let controller: AbortController | null = null;

  let filters = $state({
    q: '',
    level: '',
    method: '',
    route: '',
    target: '',
    requestId: '',
    statusClass: '',
    status: '',
    minLatencyMs: '',
    maxLatencyMs: '',
    from: '',
    to: '',
  });

  const COLUMNS: { key: AdminLogsSortBy; label: string }[] = [
    { key: 'time', label: 'Time' },
    { key: 'level', label: 'Level' },
    { key: 'request', label: 'Request' },
    { key: 'route', label: 'Route' },
    { key: 'status', label: 'Status' },
    { key: 'latency', label: 'Latency' },
    { key: 'message', label: 'Message' },
  ];

  function query(offset?: number | null): AdminLogsQuery {
    const from = toIsoOrUndefined(filters.from);
    const to = toIsoOrUndefined(filters.to);
    const status = numberFilter(filters.status);
    const statusClass = numberFilter(filters.statusClass);
    const minLatencyMs = numberFilter(filters.minLatencyMs);
    const maxLatencyMs = numberFilter(filters.maxLatencyMs);
    return {
      offset: offset ?? undefined,
      sortBy,
      sortDir,
      q: filters.q.trim() || undefined,
      level: filters.level || undefined,
      method: filters.method || undefined,
      route: filters.route.trim() || undefined,
      target: filters.target.trim() || undefined,
      requestId: filters.requestId.trim() || undefined,
      statusClass,
      status,
      minLatencyMs,
      maxLatencyMs,
      from,
      to,
      limit: LIMIT,
    };
  }

  async function load(reset = true) {
    loading = true;
    error = '';
    try {
      const page = await api.adminListLogs(query(reset ? null : nextOffset));
      droppedTotal = page.droppedTotal;
      nextOffset = page.nextOffset;
      // Offset pagination over a live-growing table can re-return rows already on
      // screen (the live stream prepends newer logs, shifting the offset window).
      // Duplicate ids here would produce duplicate keys in the keyed {#each},
      // which crashes Svelte's reconciler with "Invalid array length".
      items = dedupeById(reset ? page.items : [...items, ...page.items]).slice(0, MAX_ROWS);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load logs';
    } finally {
      loading = false;
    }
  }

  function applyPreset(kind: 'errors' | 'warn' | 'slow' | '5xx' | '15m' | 'clear') {
    if (kind === 'clear') {
      filters = { q: '', level: '', method: '', route: '', target: '', requestId: '', statusClass: '', status: '', minLatencyMs: '', maxLatencyMs: '', from: '', to: '' };
    } else if (kind === 'errors') {
      filters.level = 'ERROR';
    } else if (kind === 'warn') {
      filters.level = 'WARN';
    } else if (kind === 'slow') {
      filters.minLatencyMs = '500';
    } else if (kind === '5xx') {
      filters.statusClass = '500';
    } else if (kind === '15m') {
      const from = new Date(Date.now() - 15 * 60 * 1000);
      filters.from = toLocalInput(from);
    }
    load(true);
  }

  function toLocalInput(date: Date) {
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
  }

  function toIsoOrUndefined(value: string) {
    if (!value) return undefined;
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? undefined : date.toISOString();
  }

  function numberFilter(value: string) {
    const trimmed = value.trim();
    if (!trimmed) return undefined;
    const number = Number(trimmed);
    return Number.isFinite(number) ? number : undefined;
  }

  function prepend(item: AdminLogEntry) {
    if (!matchesActiveFilters(item)) return;
    items = sortRows(dedupeById([item, ...items])).slice(0, MAX_ROWS);
  }

  function dedupeById(rows: AdminLogEntry[]): AdminLogEntry[] {
    const seen = new Set<number>();
    const out: AdminLogEntry[] = [];
    for (const row of rows) {
      if (seen.has(row.id)) continue;
      seen.add(row.id);
      out.push(row);
    }
    return out;
  }

  function changeSort(column: AdminLogsSortBy) {
    if (sortBy === column) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    } else {
      sortBy = column;
      sortDir = column === 'time' ? 'desc' : 'asc';
    }
    load(true);
  }

  function sortValue(item: AdminLogEntry, column: AdminLogsSortBy): string | number | null {
    if (column === 'time') return Date.parse(item.ts) || 0;
    if (column === 'level') return item.level;
    if (column === 'request') return item.requestId;
    if (column === 'route') return item.route;
    if (column === 'status') return item.status;
    if (column === 'latency') return item.latencyMs;
    return item.message;
  }

  function sortRows(rows: AdminLogEntry[]) {
    const direction = sortDir === 'asc' ? 1 : -1;
    return [...rows].sort((a, b) => {
      const av = sortValue(a, sortBy);
      const bv = sortValue(b, sortBy);
      if (av == null && bv == null) return (a.id - b.id) * direction;
      if (av == null) return 1;
      if (bv == null) return -1;
      if (typeof av === 'number' && typeof bv === 'number') return (av - bv || a.id - b.id) * direction;
      return (String(av).localeCompare(String(bv)) || a.id - b.id) * direction;
    });
  }

  function sortLabel(column: AdminLogsSortBy) {
    if (sortBy !== column) return '';
    return sortDir === 'asc' ? ' ascending' : ' descending';
  }

  function matchesActiveFilters(item: AdminLogEntry) {
    if (filters.level && item.level !== filters.level) return false;
    if (filters.method && item.method !== filters.method) return false;
    if (filters.route.trim() && item.route !== filters.route.trim()) return false;
    if (filters.target.trim() && item.target !== filters.target.trim()) return false;
    if (filters.requestId.trim() && item.requestId !== filters.requestId.trim()) return false;
    const status = numberFilter(filters.status);
    const statusClass = numberFilter(filters.statusClass);
    const minLatencyMs = numberFilter(filters.minLatencyMs);
    const maxLatencyMs = numberFilter(filters.maxLatencyMs);
    if (status != null && item.status !== status) return false;
    if (statusClass != null && Math.floor((item.status ?? 0) / 100) * 100 !== statusClass) return false;
    if (minLatencyMs != null && (item.latencyMs ?? -1) < minLatencyMs) return false;
    if (maxLatencyMs != null && (item.latencyMs ?? Number.MAX_SAFE_INTEGER) > maxLatencyMs) return false;
    const q = filters.q.trim().toLowerCase();
    if (q && !`${item.message} ${item.target} ${item.route ?? ''} ${JSON.stringify(item.fields)}`.toLowerCase().includes(q)) return false;
    return true;
  }

  async function startStream() {
    controller?.abort();
    if (!live) return;
    const activeController = new AbortController();
    controller = activeController;

    while (live && controller === activeController) {
      try {
        const res = await fetch(authenticatedApiUrl('/admin/logs/stream'), {
          headers: currentAuthHeaders(),
          signal: activeController.signal,
        });
        if (!res.ok || !res.body) throw new Error(`Live stream failed: ${res.status}`);
        error = '';
        const reader = res.body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';
        while (live && controller === activeController) {
          const { done, value } = await reader.read();
          if (done) break;
          buffer += decoder.decode(value, { stream: true });
          const chunks = buffer.split('\n\n');
          buffer = chunks.pop() ?? '';
          for (const chunk of chunks) {
            const item = parseSseLog(chunk);
            if (item) prepend(item);
          }
        }
      } catch (e) {
        if (activeController.signal.aborted) break;
        error = e instanceof Error ? e.message : 'Live stream stopped';
      }
      if (live && controller === activeController) await delay(1500, activeController.signal);
    }
  }

  function parseSseLog(chunk: string) {
    const data = chunk
      .split('\n')
      .filter((line) => line.startsWith('data:'))
      .map((line) => line.slice(line.startsWith('data: ') ? 6 : 5))
      .join('\n');
    if (!data) return null;
    try {
      return JSON.parse(data) as AdminLogEntry;
    } catch {
      return null;
    }
  }

  function delay(ms: number, signal: AbortSignal) {
    return new Promise<void>((resolve) => {
      const id = window.setTimeout(resolve, ms);
      signal.addEventListener('abort', () => {
        window.clearTimeout(id);
        resolve();
      }, { once: true });
    });
  }

  function toggleLive() {
    live = !live;
    if (live) startStream();
    else controller?.abort();
  }

  function copy(value: string | null | undefined) {
    if (value) navigator.clipboard?.writeText(value);
  }

  function exportJsonl() {
    const blob = new Blob(items.map((item) => JSON.stringify(item) + '\n'), { type: 'application/x-ndjson' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `gotiga-logs-${new Date().toISOString()}.jsonl`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function levelClass(level: string) {
    return level === 'ERROR' ? 'level-error' : level === 'WARN' ? 'level-warn' : level === 'INFO' ? 'level-info' : 'level-debug';
  }

  onMount(async () => {
    await load(true);
    startStream();
  });

  onDestroy(() => controller?.abort());
</script>

<section class="logs-panel">
  <header class="logs-header">
    <div>
      <h2>Server Logs</h2>
      <p>{items.length} loaded · {droppedTotal} dropped</p>
    </div>
    <div class="header-actions">
      <button class:active={live} onclick={toggleLive}>{live ? 'Live' : 'Paused'}</button>
      <button onclick={() => load(true)} disabled={loading}>{loading ? 'Loading' : 'Refresh'}</button>
      <button onclick={exportJsonl}>Export JSONL</button>
    </div>
  </header>

  <div class="presets">
    <button onclick={() => applyPreset('errors')}>Errors</button>
    <button onclick={() => applyPreset('warn')}>Warnings</button>
    <button onclick={() => applyPreset('slow')}>Slow &gt; 500ms</button>
    <button onclick={() => applyPreset('5xx')}>5xx</button>
    <button onclick={() => applyPreset('15m')}>Last 15m</button>
    <button onclick={() => applyPreset('clear')}>Clear</button>
  </div>

  <form class="filters" onsubmit={(e) => { e.preventDefault(); load(true); }}>
    <input bind:value={filters.q} placeholder="Text search" />
    <select bind:value={filters.level}>{#each LEVELS as level}<option value={level}>{level || 'Level'}</option>{/each}</select>
    <select bind:value={filters.method}>{#each METHODS as method}<option value={method}>{method || 'Method'}</option>{/each}</select>
    <select bind:value={filters.statusClass}>
      <option value="">Status class</option>
      <option value="200">2xx</option>
      <option value="300">3xx</option>
      <option value="400">4xx</option>
      <option value="500">5xx</option>
    </select>
    <input bind:value={filters.status} inputmode="numeric" placeholder="Status" />
    <input bind:value={filters.minLatencyMs} inputmode="numeric" placeholder="Min ms" />
    <input bind:value={filters.maxLatencyMs} inputmode="numeric" placeholder="Max ms" />
    <input bind:value={filters.route} placeholder="Route" />
    <input bind:value={filters.target} placeholder="Target" />
    <input bind:value={filters.requestId} placeholder="Request ID" />
    <input bind:value={filters.from} type="datetime-local" />
    <input bind:value={filters.to} type="datetime-local" />
    <button type="submit">Apply</button>
  </form>

  {#if error}<p class="error">{error}</p>{/if}

  <div class="logs-table">
    <div class="row row-head">
      {#each COLUMNS as column}
        <button
          type="button"
          class:sorted={sortBy === column.key}
          aria-label={`Sort by ${column.label}${sortLabel(column.key)}`}
          title={`Sort by ${column.label}${sortLabel(column.key)}`}
          onclick={() => changeSort(column.key)}
        >
          <span>{column.label}</span>
          {#if sortBy === column.key}<span aria-hidden="true">{sortDir === 'asc' ? '↑' : '↓'}</span>{/if}
        </button>
      {/each}
    </div>
    {#each items as item (item.id)}
      <button class="row" onclick={() => selected = item}>
        <span>{new Date(item.ts).toLocaleTimeString()}</span>
        <span class={levelClass(item.level)}>{item.level}</span>
        <span class="mono" title={item.requestId ?? ''}>{item.requestId ? item.requestId.slice(0, 10) : '—'}</span>
        <span class="mono" title={item.route ?? item.target}>{item.route ?? item.target}</span>
        <span>{item.status ?? '—'}</span>
        <span>{item.latencyMs != null ? `${item.latencyMs}ms` : '—'}</span>
        <span class="message">{item.message || JSON.stringify(item.fields)}</span>
      </button>
    {/each}
  </div>

  <footer class="logs-footer">
    <button onclick={() => load(false)} disabled={!nextOffset || loading}>Load older</button>
  </footer>

  {#if selected}
    <aside class="drawer">
      <div class="drawer-head">
        <div>
          <strong>{selected.level}</strong>
          <span>{new Date(selected.ts).toLocaleString()}</span>
        </div>
        <button onclick={() => selected = null}>Close</button>
      </div>
      <dl>
        <dt>Request ID</dt><dd><button onclick={() => copy(selected?.requestId)}>{selected.requestId ?? '—'}</button></dd>
        <dt>Route</dt><dd>{selected.route ?? '—'}</dd>
        <dt>Target</dt><dd>{selected.target}</dd>
        <dt>Status</dt><dd>{selected.status ?? '—'}</dd>
        <dt>Latency</dt><dd>{selected.latencyMs != null ? `${selected.latencyMs}ms` : '—'}</dd>
      </dl>
      <pre>{JSON.stringify(selected, null, 2)}</pre>
    </aside>
  {/if}
</section>

<style>
  .logs-panel { height: 100%; display: flex; flex-direction: column; background: #f7f8fa; color: #16202a; }
  .logs-header { display: flex; justify-content: space-between; gap: 1rem; padding: 1rem; border-bottom: 1px solid #d8dee7; }
  h2 { font-size: 1.15rem; font-weight: 700; margin: 0; }
  p { margin: .25rem 0 0; font-size: .75rem; color: #637083; }
  button, input, select { border: 1px solid #cfd7e3; background: #ffffff; color: #16202a; font-size: .75rem; min-height: 2rem; padding: .35rem .55rem; }
  button { cursor: pointer; text-transform: uppercase; letter-spacing: .04em; }
  button:disabled { opacity: .45; cursor: default; }
  .header-actions, .presets { display: flex; gap: .5rem; flex-wrap: wrap; align-items: center; }
  .header-actions .active { background: #e7f7ee; border-color: #7bc596; }
  .presets { padding: .75rem 1rem 0; }
  .filters { display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: .5rem; padding: .75rem 1rem; border-bottom: 1px solid #d8dee7; }
  .filters input, .filters select { min-width: 0; }
  .error { margin: 0 1rem .75rem; color: #b42318; }
  .logs-table { flex: 1; overflow: auto; font-family: Inter, system-ui, sans-serif; }
  .row { width: 100%; display: grid; grid-template-columns: 90px 70px 96px minmax(160px, 1fr) 70px 80px minmax(220px, 1.5fr); gap: .55rem; align-items: center; border: 0; border-bottom: 1px solid #e3e8ef; text-align: left; background: transparent; min-height: 2.15rem; }
  .row:hover { background: #edf4ff; }
  .row > span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row-head { position: sticky; top: 0; z-index: 2; background: #e8edf4; font-size: .68rem; font-weight: 700; text-transform: uppercase; color: #4f5d6f; }
  .row-head button { min-height: 2.15rem; display: inline-flex; align-items: center; gap: .25rem; justify-content: flex-start; border: 0; background: transparent; color: inherit; padding: 0; font: inherit; text-align: left; }
  .row-head button:hover, .row-head button.sorted { color: #111827; }
  .mono { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
  .level-error { color: #b42318; font-weight: 800; }
  .level-warn { color: #a15c08; font-weight: 800; }
  .level-info { color: #256353; font-weight: 700; }
  .level-debug { color: #526173; }
  .message { color: #283444; }
  .logs-footer { padding: .75rem 1rem; border-top: 1px solid #d8dee7; }
  .drawer { position: fixed; top: 0; right: 0; width: min(620px, 94vw); height: 100vh; z-index: 60; background: #ffffff; border-left: 1px solid #cfd7e3; box-shadow: -20px 0 50px rgba(15,23,42,.18); padding: 1rem; overflow: auto; }
  .drawer-head { display: flex; justify-content: space-between; align-items: flex-start; gap: 1rem; border-bottom: 1px solid #d8dee7; padding-bottom: .75rem; }
  .drawer-head span { display: block; font-size: .75rem; color: #637083; margin-top: .25rem; }
  dl { display: grid; grid-template-columns: 110px minmax(0, 1fr); gap: .45rem .75rem; font-size: .8rem; }
  dt { color: #637083; text-transform: uppercase; font-size: .65rem; }
  dd { margin: 0; min-width: 0; overflow-wrap: anywhere; }
  pre { white-space: pre-wrap; overflow-wrap: anywhere; background: #111827; color: #f8fafc; padding: .9rem; font-size: .72rem; }
  @media (max-width: 1100px) {
    .filters { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .row { grid-template-columns: 80px 62px 1fr 64px; }
    .row span:nth-child(3), .row span:nth-child(4), .row span:nth-child(6), .row-head button:nth-child(3), .row-head button:nth-child(4), .row-head button:nth-child(6) { display: none; }
  }
</style>
