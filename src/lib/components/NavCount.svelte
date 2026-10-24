<script lang="ts">
  // Числа у пункта меню: сколько на полке и, если есть, сколько нового с
  // прошлого визита. Печатает меню телефона и нижняя панель; шапка компьютера
  // ставит те же числа второй строкой под названием (свой вид, но те же
  // `navCounts.total` и `navCounts.fresh`).
  //
  // Пока число не пришло, не печатается ничего: пустое место лучше нуля.
  // «Новое» не печатается на самом разделе: человек уже там и видит всё.
  import { t } from '$lib/i18n';
  import { navCounts, type NavSection } from '$lib/stores/nav-counts.svelte';

  let {
    section,
    active = false,
    compact = false,
  }: {
    section: NavSection;
    /** Человек сейчас в этом разделе. */
    active?: boolean;
    /** Только «новое», без общего числа — для тесной нижней панели. */
    compact?: boolean;
  } = $props();

  let total = $derived(navCounts.total(section));
  let fresh = $derived(active ? 0 : navCounts.fresh(section));
  let freshText = $derived($t('navNewSince').replace('{n}', String(fresh)));
</script>

{#if !compact && total !== null && total > 0}
  <span class="nav-count">{total}</span>
{/if}
{#if fresh > 0}
  <span class="nav-new" title={freshText}>+{fresh}<span class="nav-count-sr"> — {freshText}</span></span>
{/if}

<style>
  .nav-count {
    margin-left: 6px;
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0;
    font-variant-numeric: tabular-nums;
    /* ink-tertiary на бумаге 6.4:1; над фотографией цвет берётся у пункта. */
    color: var(--color-ink-secondary);
    transform: translateY(-3px);
  }

  .nav-new {
    margin-left: 6px;
    padding: 3px 6px;
    border-radius: 9px;
    background: var(--color-ember-deep);
    color: #fbf6ec;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    transform: translateY(-3px);
  }

  .nav-count-sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  @media (max-width: 1024px) {
    .nav-new {
      margin-left: 8px;
    }
  }
</style>
