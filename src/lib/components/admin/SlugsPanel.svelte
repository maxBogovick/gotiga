<script lang="ts">
  /**
   * «Адреса работ» — the URL slug of every work, edited in one table.
   *
   * A row's draft lives in `drafts` until saved; an empty draft saved (or the ⟳
   * button) sends null, which tells the backend to rebuild the slug from the
   * name and marks it auto rather than manual.
   */
  import { api } from '$lib/api';
  import { figurineHref } from '$lib/figurineHref';
  import type { FigurineListItem } from '$lib/types/api';
  import { fade } from 'svelte/transition';
  import { t } from '$lib/i18n';

  let {
    figurines,
    onReload,
    onMessage,
  }: {
    figurines: FigurineListItem[];
    /** Re-fetch the registry after a backfill rewrote stored slugs. */
    onReload: () => Promise<void>;
    onMessage: (text: string, type?: string) => void;
  } = $props();

  let drafts = $state<Record<string, string>>({});
  let savingId = $state<string | null>(null);
  let backfilling = $state(false);

  let missingCount = $derived(figurines.filter((f) => !f.slug).length);

  // Current draft for a row: an explicit edit if present, else the stored slug.
  function draft(fig: FigurineListItem): string {
    return drafts[fig.id] ?? fig.slug ?? '';
  }
  function changed(fig: FigurineListItem): boolean {
    return draft(fig).trim() !== (fig.slug ?? '');
  }

  async function runBackfill() {
    if (backfilling) return;
    backfilling = true;
    try {
      const res = await api.backfillSlugs();
      await onReload();
      drafts = {}; // stored values changed — drop stale drafts
      onMessage($t('adminSlugBackfillDone').replace('{n}', String(res.affected)), 'success');
    } catch (e) {
      onMessage($t('adminSlugError') + e, 'error');
    } finally {
      backfilling = false;
    }
  }

  // Persist one row. `regenerate` sends null so the backend rebuilds from the name.
  async function save(fig: FigurineListItem, regenerate = false) {
    if (savingId) return;
    savingId = fig.id;
    try {
      const override = regenerate ? null : (draft(fig).trim() || null);
      const stored = await api.setFigurineSlug(fig.id, override);
      const row = figurines.find((f) => f.id === fig.id);
      if (row) {
        row.slug = stored;
        // A blank/regenerate override is auto; an explicit value is manual.
        row.slugManual = override !== null;
      }
      delete drafts[fig.id];
      onMessage($t('adminSlugSaved').replace('{slug}', stored), 'success');
    } catch (e) {
      onMessage($t('adminSlugError') + e, 'error');
    } finally {
      savingId = null;
    }
  }
</script>

<div in:fade class="h-full overflow-auto p-6 sm:p-8 max-w-3xl mx-auto w-full">
  <h2 class="font-['Fraunces'] text-2xl text-[#34251c] mb-1">{$t('adminTabSlugs')}</h2>
  <p class="text-[12px] text-[#7c6554] mb-4 leading-snug max-w-prose">{$t('adminSlugsIntro')}</p>

  <div class="flex items-center gap-3 mb-5 flex-wrap">
    <button
      onclick={runBackfill}
      disabled={backfilling || missingCount === 0}
      class="px-3 py-1.5 text-[12px] border border-[#6f3b24] text-[#6f3b24] rounded-[3px] hover:bg-[#6f3b24] hover:text-[#f8f1e7] transition-colors disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-[#6f3b24]"
    >
      {backfilling ? $t('adminSlugBackfillBusy') : $t('adminSlugBackfillBtn')}
    </button>
    <span class="text-[11px] text-[#7c6554]">
      {missingCount === 0
        ? $t('adminSlugAllSet')
        : $t('adminSlugMissingCount').replace('{n}', String(missingCount))}
    </span>
  </div>

  <div class="overflow-x-auto border border-[#d8c6b1] rounded-[3px]">
    <table class="w-full text-[12px] border-collapse">
      <thead>
        <tr class="bg-[#efe4d3] text-left text-[#5f4636] uppercase tracking-[0.08em] text-[9px]">
          <th class="px-3 py-2 font-medium">{$t('adminSlugColName')}</th>
          <th class="px-3 py-2 font-medium">{$t('adminSlugColSlug')}</th>
          <th class="px-3 py-2 font-medium w-px whitespace-nowrap">{$t('adminSlugColType')}</th>
          <th class="px-3 py-2 font-medium w-px whitespace-nowrap">{$t('adminSlugColActions')}</th>
        </tr>
      </thead>
      <tbody>
        {#each figurines as fig (fig.id)}
          <tr class="border-t border-[#e5d7c4] align-middle">
            <td class="px-3 py-2 text-[#34251c]">
              <a href={figurineHref(fig)} target="_blank" rel="noopener" class="hover:text-[#c65f3c] hover:underline">{fig.name}</a>
            </td>
            <td class="px-3 py-2">
              <div class="flex items-center gap-1.5">
                <span class="text-[#9a8571] select-none">/figurines/</span>
                <input
                  value={draft(fig)}
                  oninput={(e) => drafts[fig.id] = e.currentTarget.value}
                  placeholder={fig.slug ?? $t('adminSlugMissingPlaceholder')}
                  class="flex-1 min-w-[8rem] bg-[#fdf9f2] border border-[#d8c6b1] rounded-[3px] px-2 py-1 font-mono text-[11px] text-[#34251c] focus:outline-none focus:border-[#c65f3c]"
                />
              </div>
            </td>
            <td class="px-3 py-2 whitespace-nowrap">
              {#if !fig.slug}
                <span class="inline-block px-1.5 py-0.5 text-[9px] uppercase tracking-[0.08em] rounded-[3px] border border-[#d8c6b1] text-[#9a8571]">{$t('adminSlugBadgeMissing')}</span>
              {:else if fig.slugManual}
                <span class="inline-block px-1.5 py-0.5 text-[9px] uppercase tracking-[0.08em] rounded-[3px] border border-[#c65f3c]/50 text-[#c65f3c] bg-[#c65f3c]/8">{$t('adminSlugBadgeManual')}</span>
              {:else}
                <span class="inline-block px-1.5 py-0.5 text-[9px] uppercase tracking-[0.08em] rounded-[3px] border border-[#6f3b24]/25 text-[#7c6554]">{$t('adminSlugBadgeAuto')}</span>
              {/if}
            </td>
            <td class="px-3 py-2 whitespace-nowrap text-right">
              <button
                onclick={() => save(fig)}
                disabled={savingId !== null || !changed(fig)}
                class="px-2 py-1 text-[11px] border border-[#6f3b24] text-[#6f3b24] rounded-[3px] hover:bg-[#6f3b24] hover:text-[#f8f1e7] transition-colors disabled:opacity-30 disabled:hover:bg-transparent disabled:hover:text-[#6f3b24]"
              >{$t('adminSlugSaveBtn')}</button>
              <button
                onclick={() => save(fig, true)}
                disabled={savingId !== null}
                title={$t('adminSlugRegenTitle')}
                class="ml-1 px-2 py-1 text-[11px] border border-[#d8c6b1] text-[#7c6554] rounded-[3px] hover:border-[#6f3b24] hover:text-[#6f3b24] transition-colors disabled:opacity-30"
              >⟳</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
