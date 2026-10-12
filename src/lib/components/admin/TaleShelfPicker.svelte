<script lang="ts">
  // Выбор рассказа выпадающим списком — так же, как выбирают наряд рамки и
  // работу в других формах стола. Прежде рассказы стояли колонкой в 260 px
  // слева и отнимали у бумаги пятую часть окна ради списка, который нужен в
  // момент выбора. Теперь это одна строка в шапке: закрытая, она называет
  // открытый рассказ, раскрытая — ищет, ведёт стрелками и переставляет порядок
  // на полке перетаскиванием.
  import { t } from '$lib/i18n';
  import type { Lang } from '$lib/i18n';
  import type { AdminTaleStat, GazetteLeaf, GazetteStatus } from '$lib/types/api';

  let {
    tales,
    stats,
    selectedId,
    draftTitle = '',
    lang,
    onopen,
    onmove,
    loading = false,
  }: {
    tales: GazetteLeaf[];
    stats: Record<string, AdminTaleStat>;
    selectedId: string | null;
    /** Заглавие несохранённой байки: у неё ещё нет строки в списке. */
    draftTitle?: string;
    lang: Lang;
    onopen: (tale: GazetteLeaf) => void;
    /** Индексы в полном списке: порядок полки один, а поиск лишь прячет строки. */
    onmove: (from: number, to: number) => void;
    loading?: boolean;
  } = $props();

  const TONE: Record<GazetteStatus, string> = {
    draft: '#b9a68f',
    scheduled: '#d99a1f',
    published: '#2f8f5b',
    archived: '#b0a08e',
  };

  let open = $state(false);
  let query = $state('');
  let active = $state(-1);
  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);
  let box = $state<HTMLElement | null>(null);
  let search = $state<HTMLInputElement | null>(null);

  function titleOf(tale: GazetteLeaf): string {
    return (lang === 'ru' ? tale.titleRu : tale.titleEn) || tale.titleEn || tale.titleRu || $t('adminTalesUntitled');
  }

  let rows = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const all = tales.map((tale, index) => ({ tale, index }));
    if (!q) return all;
    return all.filter(({ tale }) =>
      `${tale.titleEn} ${tale.titleRu} ${tale.figurineName ?? ''}`.toLowerCase().includes(q),
    );
  });
  let current = $derived(tales.find((tale) => tale.id === selectedId) ?? null);
  let canDrag = $derived(!query.trim());

  function unfold() {
    open = true;
    query = '';
    active = Math.max(0, tales.findIndex((tale) => tale.id === selectedId));
    queueMicrotask(() => {
      search?.focus();
      box?.querySelector(`[data-row="${active}"]`)?.scrollIntoView({ block: 'nearest' });
    });
  }

  function fold() {
    open = false;
    dragFrom = null;
    dragOver = null;
  }

  function pick(tale: GazetteLeaf) {
    fold();
    onopen(tale);
  }

  function keys(event: KeyboardEvent) {
    if (!open) {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        unfold();
      }
      return;
    }
    if (event.key === 'Escape') {
      event.stopPropagation();
      fold();
    } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const last = rows.length - 1;
      if (last < 0) return;
      active = event.key === 'ArrowDown' ? (active >= last ? 0 : active + 1) : active <= 0 ? last : active - 1;
      box?.querySelector(`[data-row="${active}"]`)?.scrollIntoView({ block: 'nearest' });
    } else if (event.key === 'Enter' && rows[active]) {
      event.preventDefault();
      pick(rows[active].tale);
    }
  }

  function drop(to: number) {
    const from = dragFrom;
    dragFrom = null;
    dragOver = null;
    if (from != null && from !== to) onmove(from, to);
  }

  function gap(tale: GazetteLeaf): Lang | null {
    const en = (tale.bodyEn ?? '').trim();
    const ru = (tale.bodyRu ?? '').trim();
    if (en && !ru) return 'ru';
    if (ru && !en) return 'en';
    return null;
  }
</script>

<div class="picker" onkeydown={keys} role="presentation">
  <button
    type="button"
    class="trigger"
    class:on={open}
    onclick={() => (open ? fold() : unfold())}
    aria-haspopup="listbox"
    aria-expanded={open}
    title={$t('adminTalesPick')}
  >
    {#if current}
      <span class="dot" style="background: {TONE[current.status]}"></span>
      <span class="name">{titleOf(current)}</span>
      {#if current.figurineName}<span class="sub">{current.figurineName}</span>{/if}
    {:else}
      <span class="dot dot--new"></span>
      <span class="name" class:ghost={!draftTitle}>{draftTitle || $t('adminTalesNew')}</span>
    {/if}
    <span class="caret" aria-hidden="true">{open ? '▴' : '▾'}</span>
  </button>

  {#if open}
    <div class="curtain" role="presentation" onclick={fold}></div>
    <div class="pop">
      <input
        bind:this={search}
        bind:value={query}
        oninput={() => (active = 0)}
        class="find"
        placeholder={$t('adminTalesSearch')}
        aria-label={$t('adminTalesSearch')}
      />
      <div class="meta">
        <span>{canDrag ? $t('adminTalesDragHint') : ''}</span>
        <span>{$t('adminTalesTotal').replace('{n}', String(tales.length))}</span>
      </div>
      <ul bind:this={box} class="list" role="listbox" aria-label={$t('adminTalesPick')}>
        {#if loading}
          <li class="empty">…</li>
        {:else if tales.length === 0}
          <li class="empty">{$t('adminTalesEmpty')}</li>
        {:else if rows.length === 0}
          <li class="empty">{$t('adminTalesNoMatches')}</li>
        {/if}
        {#each rows as { tale, index }, i (tale.id)}
          {@const waiting = stats[tale.id]?.sequelWishes ?? 0}
          {@const pending = stats[tale.id]?.pendingComments ?? 0}
          {@const hole = gap(tale)}
          {@const face = tale.imageUrls?.[0] ?? tale.imageUrl}
          <li
            class="row"
            class:drag-over={dragOver === index}
            class:dragging={dragFrom === index}
            draggable={canDrag}
            ondragstart={() => (dragFrom = index)}
            ondragover={(e) => { e.preventDefault(); dragOver = index; }}
            ondragleave={() => { if (dragOver === index) dragOver = null; }}
            ondrop={(e) => { e.preventDefault(); drop(index); }}
            ondragend={() => { dragFrom = null; dragOver = null; }}
          >
            <button
              type="button"
              role="option"
              data-row={i}
              aria-selected={tale.id === selectedId}
              class="item"
              class:cur={tale.id === selectedId}
              class:hot={i === active}
              onclick={() => pick(tale)}
              onpointerenter={() => (active = i)}
            >
              {#if canDrag}<span class="grip" aria-hidden="true">⋮⋮</span>{/if}
              {#if face}
                <img class="face" src={face} alt="" loading="lazy" />
              {:else}
                <span class="face face--none"></span>
              {/if}
              <span class="text">
                <span class="line">
                  <span class="dot" style="background: {TONE[tale.status]}"></span>
                  <span class="name">{titleOf(tale)}</span>
                </span>
                {#if tale.figurineName || hole}
                  <span class="line line--sub">
                    {#if tale.figurineName}<span class="sub">{tale.figurineName}</span>{/if}
                    {#if hole}<span class="gap">{$t('adminTalesLangGap').replace('{lang}', hole.toUpperCase())}</span>{/if}
                  </span>
                {/if}
              </span>
              <span class="tags">
                {#if pending > 0}<span class="tag tag--ember" title={$t('adminTalesPending')}>{pending}✍</span>{/if}
                {#if waiting > 0}<span class="tag" title={$t('adminTalesSequelWishes')}>+{waiting}</span>{/if}
                {#if tale.pinned}<span class="tag tag--ember" aria-label="pinned">◆</span>{/if}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  .picker {
    position: relative;
    flex: 0 1 380px;
    min-width: 220px;
  }

  .trigger {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border: 1px solid rgba(52, 37, 28, 0.22);
    border-radius: 0;
    background: rgba(255, 255, 255, 0.45);
    text-align: left;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .trigger:hover,
  .trigger.on {
    border-color: rgba(52, 37, 28, 0.5);
    background: rgba(255, 255, 255, 0.7);
  }

  .dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }
  .dot--new {
    background: none;
    border: 1px dashed #c65f3c;
  }

  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 400 15px/1.2 'Cormorant Garamond', Georgia, serif;
    color: #34251c;
  }
  .name.ghost { color: #8a6a55; font-style: italic; }

  .sub {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 400 11px/1.2 Inter, system-ui, sans-serif;
    color: #8a6a55;
  }
  .trigger .sub { flex: 0 1 auto; margin-left: auto; padding-left: 10px; }
  .caret { flex: none; margin-left: 4px; font-size: 10px; color: #8a6a55; }
  .trigger .sub + .caret { margin-left: 4px; }

  .curtain { position: fixed; inset: 0; z-index: 40; }

  .pop {
    position: absolute;
    z-index: 50;
    top: calc(100% + 4px);
    left: 0;
    width: 460px;
    max-width: calc(100vw - 32px);
    background: #f8f1e7;
    border: 1px solid rgba(52, 37, 28, 0.28);
    box-shadow: 0 14px 36px rgba(52, 37, 28, 0.24);
  }

  .find {
    display: block;
    width: 100%;
    height: 38px;
    padding: 0 12px;
    border: 0;
    border-bottom: 1px solid rgba(52, 37, 28, 0.12);
    border-radius: 0;
    background: transparent;
    font: 400 13px Inter, system-ui, sans-serif;
    box-shadow: none;
    outline: none;
  }
  .find:focus { box-shadow: none; background: transparent; }

  .meta {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 6px 12px;
    font: 500 9px/1.3 Inter, system-ui, sans-serif;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0 0 4px;
    max-height: min(62vh, 480px);
    overflow-y: auto;
  }

  .empty {
    padding: 14px 12px;
    font: italic 12px Inter, system-ui, sans-serif;
    color: #8a6a55;
  }

  .row { border-top: 1px solid rgba(52, 37, 28, 0.06); }
  .row.dragging { opacity: 0.4; }
  .row.drag-over { background: rgba(198, 95, 60, 0.12); }

  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 12px 7px 6px;
    border: 0;
    border-radius: 0;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .item.hot { background: rgba(52, 37, 28, 0.06); }
  .item.cur .name { color: #c65f3c; }

  .grip {
    flex: none;
    width: 12px;
    font-size: 11px;
    letter-spacing: -2px;
    color: #c9b8a1;
    cursor: grab;
  }

  .face {
    flex: none;
    width: 30px;
    height: 38px;
    object-fit: cover;
    border: 1px solid #d8c6b1;
    background: #1a120e;
  }
  .face--none { background: rgba(52, 37, 28, 0.05); }

  .text { flex: 1; min-width: 0; display: grid; gap: 2px; }
  .line { display: flex; align-items: center; gap: 7px; min-width: 0; }
  .line--sub { padding-left: 14px; gap: 10px; }
  .item .name { font-size: 16px; }

  .gap {
    flex: none;
    font: 500 9px/1.3 Inter, system-ui, sans-serif;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #c65f3c;
  }

  .tags { flex: none; display: flex; gap: 8px; }
  .tag { font: 500 11px/1 Inter, system-ui, sans-serif; color: #6f3b24; white-space: nowrap; }
  .tag--ember { color: #c65f3c; }
</style>
