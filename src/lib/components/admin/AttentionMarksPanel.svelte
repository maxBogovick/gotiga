<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { AdminFigurineMarkStat, NoticedByGuestsSettings } from '$lib/types/api';

  // Ranking by "marks of attention" — the quiet wax-seal gesture visitors can leave
  // on a figurine's page. This is a private curation signal for the artisan only:
  // it is never surfaced on the public site (no star rating, no public leaderboard),
  // deliberately, to avoid the vanity-metric / negative-social-proof pattern that a
  // visible ranking of unique, often one-off (and often already sold) pieces would
  // create. It also intentionally includes sold/gone work so past resonance can
  // inform future curation and commission direction, not just what's for sale now.
  //
  // The "Home shelf" column below drives the public "Замечено гостями" / "Noticed
  // by guests" home-page section: pin figurines to force them onto that shelf (in
  // pin order), or exclude one to keep it out of the automatic fill forever. Both
  // are hybrid controls on top of the same private ranking — see project decision
  // that a hybrid (editorial + algorithmic) shelf beats either pure approach.

  let loading = $state(true);
  let error = $state('');
  let stats = $state<AdminFigurineMarkStat[]>([]);
  let search = $state('');
  let statusFilter = $state<'all' | 'available' | 'sold' | 'reserved' | 'in_progress'>('all');
  let expanded = $state(false);

  let settings = $state<NoticedByGuestsSettings>({ pinnedIds: [], excludedIds: [] });
  let savedSettingsJson = $state('');
  let settingsSaving = $state(false);
  let settingsError = $state('');
  let settingsDirty = $derived(JSON.stringify(settings) !== savedSettingsJson);

  let filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return stats.filter((s) => {
      if (statusFilter !== 'all' && s.status !== statusFilter) return false;
      if (q && !s.figurineName.toLowerCase().includes(q)) return false;
      return true;
    });
  });

  let totalMarks = $derived(stats.reduce((sum, s) => sum + s.markCount, 0));
  let totalLikes = $derived(stats.reduce((sum, s) => sum + (s.likeCount ?? 0), 0));
  let totalDesired = $derived(stats.reduce((sum, s) => sum + s.desiredCount, 0));
  let markedCount = $derived(stats.filter((s) => s.markCount > 0).length);
  let visibleRows = $derived(expanded ? filtered : filtered.slice(0, 8));

  let pinnedRows = $derived(
    settings.pinnedIds
      .map((id) => stats.find((s) => s.figurineId === id))
      .filter((s): s is AdminFigurineMarkStat => Boolean(s))
  );

  onMount(() => {
    void load();
  });

  async function load() {
    loading = true;
    error = '';
    try {
      const [markStats, shelfSettings] = await Promise.all([
        api.getFigurineMarkStats(),
        api.getNoticedByGuestsSettings(),
      ]);
      stats = markStats;
      settings = shelfSettings;
      savedSettingsJson = JSON.stringify(shelfSettings);
    } catch {
      error = 'Could not load marks.';
    } finally {
      loading = false;
    }
  }

  function isPinned(id: string) {
    return settings.pinnedIds.includes(id);
  }
  function isExcluded(id: string) {
    return settings.excludedIds.includes(id);
  }

  // A piece can't be pinned and excluded at once — pinning always wins over a
  // prior exclusion, and vice versa, since they're contradictory intents.
  // Always rebuild with the same key order (pinnedIds, excludedIds) so
  // `settingsDirty`'s JSON.stringify comparison isn't tripped up by key
  // ordering alone.
  function togglePin(id: string) {
    settings = isPinned(id)
      ? { pinnedIds: settings.pinnedIds.filter((x) => x !== id), excludedIds: settings.excludedIds }
      : { pinnedIds: [...settings.pinnedIds, id], excludedIds: settings.excludedIds.filter((x) => x !== id) };
  }

  function toggleExclude(id: string) {
    settings = isExcluded(id)
      ? { pinnedIds: settings.pinnedIds, excludedIds: settings.excludedIds.filter((x) => x !== id) }
      : { pinnedIds: settings.pinnedIds.filter((x) => x !== id), excludedIds: [...settings.excludedIds, id] };
  }

  async function saveShelf() {
    settingsSaving = true;
    settingsError = '';
    try {
      const saved = await api.saveNoticedByGuestsSettings(settings);
      settings = saved;
      savedSettingsJson = JSON.stringify(saved);
    } catch {
      settingsError = 'Could not save the home shelf.';
    } finally {
      settingsSaving = false;
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
      <span><strong>{totalLikes}</strong> likes (distinct visitors)</span>
      <span><strong>{markedCount}</strong> of {stats.length} pieces marked</span>
      <span title="Closest to commission intent"><strong>{totalDesired}</strong> ✒ want-one-like-this</span>
    </div>
  </header>

  {#if loading}
    <p class="state">Loading…</p>
  {:else if error}
    <p class="state state--error">{error}</p>
  {:else}
    <div class="shelf-bar">
      <div class="shelf-pins">
        <span class="shelf-pins-label">Home shelf order:</span>
        {#if pinnedRows.length === 0}
          <span class="muted-text">none pinned — auto-fills from ranking above</span>
        {:else}
          {#each pinnedRows as row (row.figurineId)}
            <span class="pin-chip">
              {row.figurineName}
              <button type="button" onclick={() => togglePin(row.figurineId)} aria-label={`Unpin ${row.figurineName}`}>×</button>
            </span>
          {/each}
        {/if}
      </div>
      <button type="button" class="shelf-save-btn" onclick={saveShelf} disabled={!settingsDirty || settingsSaving}>
        {settingsSaving ? 'Saving…' : settingsDirty ? 'Save shelf changes' : 'Saved'}
      </button>
    </div>
    {#if settingsError}<p class="state state--error">{settingsError}</p>{/if}

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
            <th class="num" title="Distinct visitors who liked">Likes</th>
            <th title="❧ touched · ✺ mesmerized · ✒ want one like this">Tones</th>
            <th class="num" title="touched×1 + mesmerized×2 + desired×3">Score</th>
            <th>Last mark</th>
            <th title="Controls the public 'Noticed by guests' home shelf">Home shelf</th>
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
              <td class="num">{row.likeCount ?? 0}</td>
              <td class="tones">
                <span title="Touched">❧{row.touchedCount}</span>
                <span title="Mesmerized">✺{row.mesmerizedCount}</span>
                <span class="tone-desired" title="Want one like this">✒{row.desiredCount}</span>
              </td>
              <td class="num">{row.weightedScore}</td>
              <td class="muted-text">{relativeTime(row.lastMarkedAt)}</td>
              <td class="shelf-actions">
                <button
                  type="button" class="shelf-btn" class:shelf-btn--on={isPinned(row.figurineId)}
                  onclick={() => togglePin(row.figurineId)}
                  title={isPinned(row.figurineId) ? 'Unpin from home shelf' : 'Pin to home shelf'}
                >📌</button>
                <button
                  type="button" class="shelf-btn shelf-btn--exclude" class:shelf-btn--on={isExcluded(row.figurineId)}
                  onclick={() => toggleExclude(row.figurineId)}
                  title={isExcluded(row.figurineId) ? 'Allow back into auto-fill' : 'Exclude from auto-fill forever'}
                >🚫</button>
              </td>
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
    font-family: 'Instrument Sans', ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
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

  .tones {
    display: flex;
    gap: 0.5rem;
    color: #6b7280;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .tone-desired {
    color: #92400e;
    font-weight: 600;
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

  .shelf-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
    margin-bottom: 0.6rem;
    padding: 0.5rem 0.6rem;
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 0.4rem;
  }

  .shelf-pins {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
    font-size: 0.76rem;
  }

  .shelf-pins-label {
    color: #6b7280;
    font-weight: 600;
    white-space: nowrap;
  }

  .pin-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.15rem 0.3rem 0.15rem 0.55rem;
    background: #fef3c7;
    color: #92400e;
    border-radius: 999px;
    font-weight: 600;
  }

  .pin-chip button {
    border: none;
    background: none;
    color: inherit;
    cursor: pointer;
    font-size: 0.85rem;
    line-height: 1;
    padding: 0.1rem 0.3rem;
  }

  .shelf-save-btn {
    border: 1px solid #d1d5db;
    background: #fff;
    color: #111827;
    border-radius: 0.4rem;
    padding: 0.32rem 0.7rem;
    font-size: 0.76rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .shelf-save-btn:disabled {
    color: #9ca3af;
    cursor: default;
  }

  .shelf-actions {
    display: flex;
    gap: 0.3rem;
    white-space: nowrap;
  }

  .shelf-btn {
    border: 1px solid #e5e7eb;
    background: #fff;
    border-radius: 0.3rem;
    padding: 0.15rem 0.4rem;
    font-size: 0.78rem;
    cursor: pointer;
    opacity: 0.55;
  }

  .shelf-btn:hover {
    opacity: 1;
  }

  .shelf-btn--on {
    opacity: 1;
    background: #fef3c7;
    border-color: #fde68a;
  }

  .shelf-btn--exclude.shelf-btn--on {
    background: #fee2e2;
    border-color: #fecaca;
  }
</style>
