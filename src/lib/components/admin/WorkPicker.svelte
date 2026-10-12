<script lang="ts">
  // Выбор работы для рассказа выпадающим списком с поиском. Прежде это было
  // поле, под которым список раскрывался вверх и перекрывал бумагу, а выбранная
  // работа превращалась в блок из трёх ссылок и сетки фотографий: выбрать
  // другую можно было, только сперва нажав «Отвязать».
  import { t } from '$lib/i18n';
  import type { FigurineListItem } from '$lib/types/api';

  let {
    figurines,
    value = $bindable(''),
  }: {
    figurines: FigurineListItem[];
    /** id выбранной работы; пустая строка — работа не связана. */
    value?: string;
  } = $props();

  let open = $state(false);
  let query = $state('');
  let active = $state(0);
  let box = $state<HTMLElement | null>(null);
  let search = $state<HTMLInputElement | null>(null);

  let chosen = $derived(figurines.find((f) => f.id === value) ?? null);
  let rows = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return q ? figurines.filter((f) => f.name.toLowerCase().includes(q)) : figurines;
  });

  function unfold() {
    open = true;
    query = '';
    active = Math.max(0, figurines.findIndex((f) => f.id === value));
    queueMicrotask(() => {
      search?.focus();
      box?.querySelector(`[data-row="${active}"]`)?.scrollIntoView({ block: 'nearest' });
    });
  }

  function fold() {
    open = false;
  }

  function pick(id: string) {
    value = id;
    fold();
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
      pick(rows[active].id);
    }
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
  >
    {#if chosen}
      {#if chosen.faceImageUrl}
        <img class="face" src={chosen.faceImageUrl} alt="" />
      {:else}
        <span class="face face--none"></span>
      {/if}
      <span class="name">{chosen.name}</span>
    {:else}
      <span class="face face--none face--dashed"></span>
      <span class="name ghost">{$t('adminTalesWorkNone')}</span>
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
        placeholder={$t('adminTalesWorkSearch')}
        aria-label={$t('adminTalesWorkSearch')}
      />
      <ul bind:this={box} class="list" role="listbox" aria-label={$t('adminTalesWork')}>
        {#if !query.trim()}
          <li>
            <button type="button" class="item item--none" class:cur={!value} onclick={() => pick('')}>
              <span class="face face--none face--dashed"></span>
              <span class="name ghost">{$t('adminTalesWorkNone')}</span>
            </button>
          </li>
        {/if}
        {#if rows.length === 0}
          <li class="empty">{$t('adminTalesNoMatches')}</li>
        {/if}
        {#each rows as fig, i (fig.id)}
          <li>
            <button
              type="button"
              role="option"
              data-row={i}
              aria-selected={fig.id === value}
              class="item"
              class:cur={fig.id === value}
              class:hot={i === active}
              onclick={() => pick(fig.id)}
              onpointerenter={() => (active = i)}
            >
              {#if fig.faceImageUrl}
                <img class="face" src={fig.faceImageUrl} alt="" loading="lazy" />
              {:else}
                <span class="face face--none"></span>
              {/if}
              <span class="name">{fig.name}</span>
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  .picker { position: relative; }

  .trigger {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 6px 10px 6px 6px;
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

  .face {
    flex: none;
    width: 28px;
    height: 36px;
    object-fit: cover;
    border: 1px solid #d8c6b1;
    background: #1a120e;
  }
  .face--none { background: rgba(52, 37, 28, 0.05); }
  .face--dashed { border-style: dashed; border-color: rgba(52, 37, 28, 0.3); }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 400 16px/1.2 'Cormorant Garamond', Georgia, serif;
    color: #34251c;
  }
  .name.ghost { font: italic 12px Inter, system-ui, sans-serif; color: #8a6a55; }
  .caret { flex: none; font-size: 10px; color: #8a6a55; }

  .curtain { position: fixed; inset: 0; z-index: 40; }

  .pop {
    position: absolute;
    z-index: 50;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    min-width: 260px;
    background: #f8f1e7;
    border: 1px solid rgba(52, 37, 28, 0.28);
    box-shadow: 0 14px 36px rgba(52, 37, 28, 0.24);
  }

  .find {
    display: block;
    width: 100%;
    height: 36px;
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

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: min(54vh, 360px);
    overflow-y: auto;
  }

  .empty { padding: 12px; font: italic 12px Inter, system-ui, sans-serif; color: #8a6a55; }

  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 5px 10px 5px 6px;
    border: 0;
    border-top: 1px solid rgba(52, 37, 28, 0.06);
    border-radius: 0;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .item.hot { background: rgba(52, 37, 28, 0.06); }
  .item.cur .name { color: #c65f3c; }
  .item--none { border-top: 0; }
</style>
