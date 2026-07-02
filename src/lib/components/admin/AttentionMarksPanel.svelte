<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { AdminFigurineMarkStat } from '$lib/types/api';

  // Ranking by "marks of attention" — the quiet wax-seal gesture visitors can leave
  // on a figurine's page. This is a private curation signal for the artisan only:
  // it is never surfaced on the public site (no star rating, no public leaderboard),
  // deliberately, to avoid the vanity-metric / negative-social-proof pattern that a
  // visible ranking of unique, often one-off (and often already sold) pieces would
  // create. It also intentionally includes sold/gone work so past resonance can
  // inform future curation and commission direction, not just what's for sale now.

  let loading = $state(true);
  let error = $state('');
  let stats = $state<AdminFigurineMarkStat[]>([]);
  let search = $state('');
  let statusFilter = $state<'all' | 'available' | 'sold' | 'reserved' | 'in_progress'>('all');
  let expanded = $state(false);

  let filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return stats.filter((s) => {
      if (statusFilter !== 'all' && s.status !== statusFilter) return false;
      if (q && !s.figurineName.toLowerCase().includes(q)) return false;
      return true;
    });
  });

  let totalMarks = $derived(stats.reduce((sum, s) => sum + s.markCount, 0));
  let markedCount = $derived(stats.filter((s) => s.markCount > 0).length);
  let visibleRows = $derived(expanded ? filtered : filtered.slice(0, 8));

  onMount(() => {
    void load();
  });

  async function load() {
    loading = true;
    error = '';
    try {
      stats = await api.getFigurineMarkStats();
    } catch {
      error = 'Could not load marks.';
    } finally {
      loading = false;
    }
  }

  function relativeTime(iso: string | null): string {
    if (!iso) return '—';
    const days = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
    if (days <= 0) return 'today';
    if (days === 1) return 'yesterday';
    if (days < 30) return `${days}d ago`;
    if (days < 365) return `${Math.floor(days / 30)}mo ago`;
    return `${Math.floor(days / 365)}y ago`;
  }
</script>

<section class="marks-panel">
  <header class="marks-head">
    <div>
      <p class="eyebrow">Private — never shown publicly</p>
      <h3>Marks of attention</h3>
    </div>
    <div class="marks-summary">
      <span><strong>{totalMarks}</strong> marks total</span>
      <span><strong>{markedCount}</strong> of {stats.length} pieces marked</span>
    </div>
  </header>

  {#if loading}
    <p class="state">Loading…</p>
  {:else if error}
    <p class="state state--error">{error}</p>
  {:else}
    <div class="marks-toolbar">
      <input
        class="marks-search"
        placeholder="Search figurine…"
        bind:value={search}
        aria-label="Search figurines by name"
      />
      <select bind:value={statusFilter} aria-label="Filter by status">
        <option value="all">All statuses</option>
        <option value="available">Available</option>
        <option value="reserved">Reserved</option>
        <option value="sold">Sold</option>
        <option value="in_progress">In progress</option>
      </select>
    </div>

    {#if filtered.length === 0}
      <p class="state">No matches.</p>
    {:else}
      <table class="marks-table">
        <thead>
          <tr>
            <th>#</th>
            <th>Figurine</th>
            <th>Status</th>
            <th class="num">Marks</th>
            <th>Last mark</th>
          </tr>
        </thead>
        <tbody>
          {#each visibleRows as row, i (row.figurineId)}
            <tr class:muted={row.markCount === 0} class:hidden-row={!row.isVisible}>
              <td class="rank">{i + 1}</td>
              <td class="name">
                {row.figurineName}
                {#if !row.isVisible}<span class="hidden-badge">hidden</span>{/if}
              </td>
              <td><span class="status-badge status-{row.status}">{row.status.replace('_', ' ')}</span></td>
              <td class="num">{row.markCount}</td>
              <td class="muted-text">{relativeTime(row.lastMarkedAt)}</td>
            </tr>
          {/each}
        </tbody>
      </table>

      {#if filtered.length > 8}
        <button type="button" class="expand-btn" onclick={() => (expanded = !expanded)}>
          {expanded ? 'Show fewer' : `Show all ${filtered.length}`}
        </button>
      {/if}
    {/if}
  {/if}
</section>

<style>
  .marks-panel {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 0.6rem;
    padding: 0.9rem 1rem;
    margin-bottom: 0.85rem;
    color: #111827;
    font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-size: 13px;
  }

  .marks-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.7rem;
    flex-wrap: wrap;
  }

  .eyebrow {
    color: #9ca3af;
    font-size: 0.68rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    margin: 0 0 0.15rem;
  }

  h3 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 700;
  }

  .marks-summary {
    display: flex;
    gap: 1rem;
    font-size: 0.78rem;
    color: #6b7280;
  }

  .marks-summary strong {
    color: #111827;
  }

  .state {
    color: #6b7280;
    padding: 0.4rem 0;
  }

  .state--error {
    color: #b91c1c;
  }

  .marks-toolbar {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.6rem;
  }

  .marks-search,
  .marks-toolbar select {
    border: 1px solid #d1d5db;
    border-radius: 0.4rem;
    padding: 0.32rem 0.55rem;
    font-size: 0.78rem;
    color: #111827;
    background: #fff;
  }

  .marks-search {
    flex: 1;
    min-width: 10rem;
  }

  .marks-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.78rem;
  }

  .marks-table th {
    text-align: left;
    color: #6b7280;
    font-weight: 600;
    font-size: 0.68rem;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    padding: 0.35rem 0.5rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .marks-table td {
    padding: 0.4rem 0.5rem;
    border-bottom: 1px solid #f3f4f6;
  }

  .marks-table .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .rank {
    color: #9ca3af;
    width: 1.5rem;
  }

  .name {
    font-weight: 600;
  }

  .muted td {
    color: #9ca3af;
  }

  .muted td.name {
    font-weight: 500;
  }

  .muted-text {
    color: #9ca3af;
  }

  .hidden-row {
    opacity: 0.7;
  }

  .hidden-badge {
    margin-left: 0.4rem;
    font-size: 0.62rem;
    text-transform: uppercase;
    color: #9ca3af;
    border: 1px solid #e5e7eb;
    border-radius: 0.25rem;
    padding: 0.05rem 0.3rem;
  }

  .status-badge {
    display: inline-block;
    padding: 0.1rem 0.45rem;
    border-radius: 0.3rem;
    font-size: 0.68rem;
    text-transform: capitalize;
  }

  .status-available { background: #dcfce7; color: #166534; }
  .status-sold { background: #f3f4f6; color: #6b7280; }
  .status-reserved { background: #fef3c7; color: #92400e; }
  .status-in_progress { background: #dbeafe; color: #1e40af; }

  .expand-btn {
    margin-top: 0.5rem;
    background: none;
    border: none;
    color: #2563eb;
    font-size: 0.76rem;
    font-weight: 600;
    cursor: pointer;
    padding: 0.2rem 0;
  }

  .expand-btn:hover {
    text-decoration: underline;
  }
</style>
