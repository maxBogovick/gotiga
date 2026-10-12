<script lang="ts">
  // Отчёт опыта «байка со сценой / без сцены» (`sceneAb.ts`).
  //
  // Главное здесь — не таблица, а строка вывода. Разница в долях двух групп
  // на малом числе читателей почти всегда случайна, и отчёт, который печатает
  // «со сценой +8 п.п.» при тридцати визитах, толкает на решение, которого
  // данные не дают. Поэтому вывод назван словами и говорит одно из трёх: мало
  // данных / различие не доказано / различие есть. «Не доказано» — не «нет
  // разницы»: это честнее и полезнее.
  import { api } from '$lib/api';
  import type { SceneExperimentArm } from '$lib/types/api';

  let { from, to }: { from: string; to: string } = $props();

  /** Меньше этого замеренных визитов на плечо вывод не делается. */
  const MIN_PER_ARM = 100;

  let arms = $state<SceneExperimentArm[]>([]);
  let rawFrom = $state('');
  let loading = $state(true);
  let error = $state('');

  $effect(() => {
    const f = from;
    const t = to;
    let current = true;
    loading = true;
    error = '';
    api
      .getSceneExperiment({ from: f, to: t })
      .then((r) => {
        if (!current) return;
        arms = r.arms;
        rawFrom = r.rawDataFrom;
      })
      .catch((e) => {
        if (current) error = String(e);
      })
      .finally(() => {
        if (current) loading = false;
      });
    return () => {
      current = false;
    };
  });

  interface TaleRow {
    path: string;
    on: SceneExperimentArm | null;
    off: SceneExperimentArm | null;
  }

  let tales = $derived.by<TaleRow[]>(() => {
    const byPath = new Map<string, TaleRow>();
    for (const a of arms) {
      const row = byPath.get(a.path) ?? { path: a.path, on: null, off: null };
      if (a.variant === 'scene:on') row.on = a;
      if (a.variant === 'scene:off') row.off = a;
      byPath.set(a.path, row);
    }
    return [...byPath.values()];
  });

  const pct = (n: number, d: number) => (d > 0 ? `${Math.round((n / d) * 100)}%` : '—');
  const secs = (ms: number | null) => (ms == null ? '—' : `${Math.round(ms / 1000)} s`);

  /**
   * Разность долей «ушёл глубже середины» и её 95% интервал. Приближение
   * нормальным законом годится при ста и более визитов на плечо — ровно
   * поэтому ниже этого порога вывод не печатается.
   */
  function verdict(row: TaleRow): { tone: 'wait' | 'none' | 'up' | 'down'; text: string } {
    const on = row.on;
    const off = row.off;
    const nOn = on?.engaged ?? 0;
    const nOff = off?.engaged ?? 0;
    if (!on || !off || nOn < MIN_PER_ARM || nOff < MIN_PER_ARM) {
      return {
        tone: 'wait',
        text: `Not enough data: ${nOn} and ${nOff} measured visits, at least ${MIN_PER_ARM} in each arm are needed before anything is said.`,
      };
    }
    const p1 = on.deep / nOn;
    const p0 = off.deep / nOff;
    const diff = p1 - p0;
    const se = Math.sqrt((p1 * (1 - p1)) / nOn + (p0 * (1 - p0)) / nOff);
    const lo = diff - 1.96 * se;
    const hi = diff + 1.96 * se;
    const pp = (x: number) => `${x >= 0 ? '+' : '−'}${Math.abs(x * 100).toFixed(1)} pp`;
    const span = `95% interval ${pp(lo)} … ${pp(hi)}`;
    if (lo > 0) return { tone: 'up', text: `With the scene, more readers go past the middle: ${pp(diff)} (${span}).` };
    if (hi < 0) return { tone: 'down', text: `With the scene, fewer readers go past the middle: ${pp(diff)} (${span}).` };
    return { tone: 'none', text: `No difference proven: ${pp(diff)} (${span}). Keep collecting, or accept that the scene does not change this.` };
  }
</script>

<section class="a-block scene-exp">
  <h3 class="block-label"><span>Tale scene: with / without</span></h3>
  {#if loading}
    <p class="muted">Loading…</p>
  {:else if error}
    <p class="muted">{error}</p>
  {:else if tales.length === 0}
    <p class="muted">No measured tale visits in this range yet. Half of the readers of a tale that has a scene see it, the other half do not; this fills in as they read.</p>
  {:else}
    <div class="table-scroll">
      <table class="exp-table">
        <thead>
          <tr>
            <th>Tale</th>
            <th>Arm</th>
            <th class="right" title="Page views that carried this arm.">Views</th>
            <th class="right" title="Visits that reported time and scroll on leaving. Every rate below is a share of these.">Measured</th>
            <th class="right" title="Share of measured visits that scrolled at least half of the page.">Past the middle</th>
            <th class="right" title="Share of measured visits that left within a few seconds.">Quick exit</th>
            <th class="right">Median time</th>
            <th class="right">Median scroll</th>
          </tr>
        </thead>
        <tbody>
          {#each tales as row}
            {#each [row.on, row.off] as a, i}
              {#if a}
                <tr class:first={i === 0 || !row.on}>
                  <td>{i === 0 || !row.on ? row.path : ''}</td>
                  <td>{a.variant === 'scene:on' ? 'with scene' : 'without'}</td>
                  <td class="right">{a.views}</td>
                  <td class="right">{a.engaged}</td>
                  <td class="right">{pct(a.deep, a.engaged)}</td>
                  <td class="right">{pct(a.quickExit, a.engaged)}</td>
                  <td class="right">{secs(a.medianDurationMs)}</td>
                  <td class="right">{a.medianScrollDepth == null ? '—' : `${Math.round(a.medianScrollDepth)}%`}</td>
                </tr>
              {/if}
            {/each}
            {@const v = verdict(row)}
            <tr class="verdict verdict--{v.tone}">
              <td colspan="8">{v.text}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    {#if rawFrom > from}
      <p class="section-note">Only goes back to {rawFrom} (raw event retention).</p>
    {/if}
    <p class="section-note">The split is by visitor and tale, so a reader who returns sees the same page. Previews with <code>?scene=on</code> / <code>?scene=off</code> in the address are not counted. "Past the middle" is about the whole page, comments included, not only the tale text.</p>
  {/if}
</section>

<style>
  .exp-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .exp-table th,
  .exp-table td {
    padding: 0.4rem 0.6rem;
    text-align: left;
    border-bottom: 1px solid var(--line, #d8c6b1);
  }

  .exp-table .right {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .exp-table tr.first td {
    border-top: 1px solid var(--line, #d8c6b1);
  }

  .verdict td {
    font-style: italic;
    padding-bottom: 0.9rem;
  }

  .verdict--up td { color: #2f6b3a; }
  .verdict--down td { color: #9c3a1e; }
  .verdict--wait td,
  .verdict--none td { color: var(--ink-secondary, #6b5648); }
</style>
