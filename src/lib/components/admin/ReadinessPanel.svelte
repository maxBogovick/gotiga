<script lang="ts">
  /**
   * Вкладка «Что не доделано»: работы и вышедшие байки, у которых что-то
   * забыли. Ответ — тот же, что у метки в реестре и списка в форме работы
   * (`adminReadiness`); здесь он только разложен по разделам.
   */
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import type { WorkGaps } from '$lib/types/api';
  import {
    adminReadiness,
    workGapText,
    workGapHint,
    taleGapText,
    WORK_GAP_TAB,
  } from '$lib/stores/admin-readiness.svelte';

  let {
    onOpenWork,
    onOpenTale,
  }: {
    /** Открыть работу в реестре; `tab` — вкладка формы, где чинят первый пропуск. */
    onOpenWork: (id: string, tab: 'media' | 'object' | null) => void;
    onOpenTale: (id: string) => void;
  } = $props();

  let searchBusy = $state(false);
  let note = $state('');

  let report = $derived(adminReadiness.report);
  let shown = $derived(report?.works.filter((w) => !w.hidden) ?? []);
  let hidden = $derived(report?.works.filter((w) => w.hidden) ?? []);
  let searchStale = $derived(report?.works.some((w) => w.gaps.includes('search')) ?? false);
  let empty = $derived(!!report && report.works.length === 0 && report.tales.length === 0);

  /** Первая вкладка формы, на которой есть что чинить. */
  function firstTab(w: WorkGaps): 'media' | 'object' | null {
    for (const g of w.gaps) {
      const tab = WORK_GAP_TAB[g];
      if (tab) return tab;
    }
    return null;
  }

  async function searchNow() {
    searchBusy = true;
    note = '';
    try {
      const n = await adminReadiness.searchNow();
      note = $t('adminGapSearchDone').replace('{n}', String(n));
    } catch (e) {
      note = $t('adminGapSearchFailed') + e;
    } finally {
      searchBusy = false;
    }
  }

  onMount(() => {
    void adminReadiness.load();
  });
</script>

{#snippet workRow(w: WorkGaps)}
  <li class="flex items-start gap-3 py-2 border-b border-[#34251c]/10">
    <div class="min-w-0 flex-1">
      <div class="text-sm font-bold text-[#34251c] leading-snug">{w.name}</div>
      <div class="mt-1 flex flex-wrap gap-1.5">
        {#each w.gaps as gap (gap)}
          <span class="text-[11px] px-1.5 py-0.5 border border-amber-700/30 text-amber-900 bg-amber-50/60"
            title={workGapHint($t, gap)}>{workGapText($t, gap, w)}</span>
        {/each}
      </div>
    </div>
    <button onclick={() => onOpenWork(w.id, firstTab(w))} class="btn-gothic text-[10px] shrink-0">
      {$t('adminTodoOpen')}
    </button>
  </li>
{/snippet}

<div class="max-w-3xl mx-auto p-6 space-y-6 text-[#34251c]">
  <div class="flex items-start justify-between gap-4">
    <div>
      <h2 class="font-cinzel text-lg">{$t('adminTabTodo')}</h2>
      <p class="mt-1 text-xs text-[#5f4636] leading-relaxed">{$t('adminTodoIntro')}</p>
    </div>
    <button onclick={() => adminReadiness.load()} disabled={adminReadiness.loading}
      class="btn-gothic text-[10px] shrink-0 disabled:opacity-50">{$t('adminTodoReload')}</button>
  </div>

  {#if report && !report.searchChecked}
    <p class="text-[11px] text-[#5f4636]/80">{$t('adminTodoNoSearch')}</p>
  {/if}

  {#if searchStale}
    <div class="flex flex-wrap items-center gap-3 px-3 py-2 border border-amber-700/25 bg-amber-50/60 text-xs">
      <span class="flex-1 min-w-0">{$t('adminGapHint_search')}</span>
      <button onclick={searchNow} disabled={searchBusy}
        class="btn-gothic text-[10px] disabled:opacity-50 disabled:cursor-wait">{$t('adminGapSearchNow')}</button>
    </div>
  {/if}
  {#if note}<p class="text-xs text-[#5f4636]">{note}</p>{/if}

  {#if empty}
    <p class="py-10 text-center text-sm text-[#5f4636]">{$t('adminTodoAllDone')}</p>
  {/if}

  {#if shown.length}
    <section>
      <h3 class="label">{$t('adminTodoWorks')} · {shown.length}</h3>
      <ul>{#each shown as w (w.id)}{@render workRow(w)}{/each}</ul>
    </section>
  {/if}

  {#if report?.tales.length}
    <section>
      <h3 class="label">{$t('adminTodoTales')} · {report.tales.length}</h3>
      <ul>
        {#each report.tales as tale (tale.id)}
          <li class="flex items-start gap-3 py-2 border-b border-[#34251c]/10">
            <div class="min-w-0 flex-1">
              <div class="text-sm font-bold leading-snug">
                {tale.title}
                {#if tale.status === 'scheduled'}
                  <span class="ml-1 text-[10px] font-normal uppercase tracking-wide text-[#5f4636]">{$t('adminTodoScheduled')}</span>
                {/if}
              </div>
              <div class="mt-1 flex flex-wrap gap-1.5">
                {#each tale.gaps as gap (gap)}
                  <span class="text-[11px] px-1.5 py-0.5 border border-amber-700/30 text-amber-900 bg-amber-50/60">{taleGapText($t, gap)}</span>
                {/each}
              </div>
            </div>
            <button onclick={() => onOpenTale(tale.id)} class="btn-gothic text-[10px] shrink-0">{$t('adminTodoOpen')}</button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if hidden.length}
    <section class="opacity-75">
      <h3 class="label">{$t('adminTodoHiddenWorks')} · {hidden.length}</h3>
      <ul>{#each hidden as w (w.id)}{@render workRow(w)}{/each}</ul>
    </section>
  {/if}
</div>
