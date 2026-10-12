<script lang="ts">
  /**
   * Where the home page puts this work, computed with the page's own rules
   * (`homePlacement`) from the registry list plus what the form currently holds —
   * so it answers "will it show up, and where?" before saving.
   *
   * `compact` is the chip for the form's top bar; the full form adds the reason
   * and a "put it first" shortcut next to the sort-order field.
   */
  import { t } from '$lib/i18n';
  import { GALLERY_LIMIT, homePlacement } from '$lib/home-hero';
  import type { Figurine, FigurineListItem } from '$lib/types/api';

  let {
    figurine,
    figurines,
    compact = false,
  }: { figurine: Figurine; figurines: FigurineListItem[]; compact?: boolean } = $props();

  let placement = $derived(
    homePlacement(figurines, {
      id: figurine.id,
      name: figurine.name,
      status: figurine.status,
      sortOrder: figurine.sortOrder,
      isVisible: figurine.isVisible,
      firstLookUntil: figurine.firstLookUntil,
      createdAt: figurines.find((f) => f.id === figurine.id)?.createdAt ?? null,
    })
  );

  let text = $derived.by(() => {
    const p = placement;
    const n = String(p.position ?? '');
    const total = String(p.total);
    switch (p.state) {
      case 'shown':
        return $t('adminPlaceShown').replace('{n}', n).replace('{total}', total);
      case 'beyond':
        return $t('adminPlaceBeyond').replace('{n}', n).replace('{total}', total).replace('{limit}', String(GALLERY_LIMIT));
      case 'hidden':
        return $t('adminPlaceHidden');
      case 'inProgress':
        return $t('adminPlaceWip');
      case 'firstLook':
        return $t('adminPlaceFirstLook').replace('{date}', p.until ? new Date(p.until).toLocaleString() : '');
    }
  });

  let tone = $derived(
    placement.state === 'shown'
      ? 'border-emerald-800/30 bg-emerald-50/60 text-emerald-900'
      : placement.state === 'beyond'
        ? 'border-amber-700/40 bg-amber-50 text-amber-900'
        : 'border-[#34251c]/20 bg-[#34251c]/5 text-[#5f4636]'
  );

  /** Everything the home lists except this work — what "first" has to beat. */
  function putFirst() {
    const others = figurines.filter((f) => f.id !== figurine.id);
    const min = others.length ? Math.min(...others.map((f) => f.sortOrder ?? 0)) : 1;
    figurine.sortOrder = min - 1;
  }
</script>

{#if compact}
  <span class="shrink-0 text-[10px] tracking-wide px-2 py-1 border {tone}" title={text}>
    {#if placement.position}
      {$t('adminPlaceChip').replace('{n}', String(placement.position))}
    {:else}
      {$t('adminPlaceChipNone')}
    {/if}
  </span>
{:else}
  <div class="border px-3 py-2 text-xs leading-relaxed {tone}">
    <div>{text}</div>
    <div class="mt-1 flex items-center gap-3 text-[10px] opacity-80">
      <span>{$t('adminPlaceHint')}</span>
      {#if placement.position && placement.position > 1}
        <button type="button" onclick={putFirst}
          class="uppercase tracking-wide underline underline-offset-2 hover:opacity-100">{$t('adminPlaceFirst')}</button>
      {/if}
    </div>
  </div>
{/if}
