<script lang="ts">
  // Одно поле листа: подпись, само поле, при нужде — пояснение под ним.
  //
  // Ради вида полей, а не ради подписи. Их на листе под сотню, и каждое несло
  // свою строчку утилит: где-то `py-1.5`, где-то `py-1`, где-то рамка чуть
  // темнее — разнобой, которого никто не выбирал, он просто накопился. Здесь
  // вид поля описан один раз и достаётся вложенному через `:global`, поэтому
  // ставится обычный `<input bind:value>` без единого класса.
  //
  // Перечислены типы поимённо: флажок и цвет — не строки, и общая рамка с
  // отступом превратила бы галочку в пустую коробку.
  import type { Snippet } from 'svelte';

  let {
    label,
    hint,
    wide = false,
    fault = false,
    faultNote,
    anchor,
    children,
  } = $props<{
    label?: string;
    /** Под полем, курсивом. Почему это число такое, а не что это за поле. */
    hint?: string;
    /** Занимает всю ширину ряда — длинному имени или приписке. */
    wide?: boolean;
    /** Сюда показывает отказ сервера. Поле само НИЧЕГО не решает: решает
     *  разбор годности, а поле только знает, что показали на него. */
    fault?: boolean;
    /** Что именно не так, в двух словах. Длинное слово стоит у кнопки. */
    faultNote?: string;
    /** Имя якоря — по нему отказ снизу приводит сюда и наводит курсор. */
    anchor?: string;
    children: Snippet;
  }>();
</script>

<label class="field" class:field--wide={wide} class:field--fault={fault} id={anchor}>
  {#if label}
    <span class="cap">{label}</span>
  {/if}
  {@render children()}
  {#if fault && faultNote}
    <span class="alarm">{faultNote}</span>
  {:else if hint}
    <span class="hint">{hint}</span>
  {/if}
</label>

<style>
  .field {
    display: block;
    min-width: 0;
  }
  .field--wide {
    grid-column: 1 / -1;
  }

  .cap {
    display: block;
    margin-bottom: 0.25rem;
    font-size: 9px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  /* Отказ виден и подписью, и рамкой поля: одна подпись цветом теряется у
     того, кто читает сразу всё, а одна рамка ничего не говорит о причине. */
  .field--fault .cap {
    color: #8f2f22;
  }
  .alarm {
    display: block;
    margin-top: 0.3rem;
    font-size: 10px;
    line-height: 1.45;
    color: #8f2f22;
  }

  .hint {
    display: block;
    margin-top: 0.3rem;
    font-size: 10px;
    line-height: 1.45;
    font-style: italic;
    color: #8a6a55;
  }

  .field :global(input[type='text']),
  .field :global(input[type='number']),
  .field :global(input[type='url']),
  .field :global(input:not([type])),
  .field :global(select),
  .field :global(textarea) {
    width: 100%;
    padding: 0.35rem 0.5rem;
    background: transparent;
    border: 1px solid rgba(52, 37, 28, 0.15);
    outline: none;
    font-size: 12px;
    color: #34251c;
    transition: border-color 120ms ease;
  }
  .field :global(input[type='number']) {
    font-variant-numeric: tabular-nums;
  }
  .field :global(textarea) {
    line-height: 1.5;
    resize: vertical;
  }
  .field--fault :global(input[type='text']),
  .field--fault :global(input[type='number']),
  .field--fault :global(input[type='url']),
  .field--fault :global(input:not([type])),
  .field--fault :global(select),
  .field--fault :global(textarea) {
    border-color: rgba(143, 47, 34, 0.6);
    background: rgba(143, 47, 34, 0.045);
  }

  .field :global(input:focus),
  .field :global(select:focus),
  .field :global(textarea:focus) {
    border-color: rgba(52, 37, 28, 0.38);
  }
  .field :global(input:disabled),
  .field :global(select:disabled),
  .field :global(textarea:disabled) {
    color: #8a6a55;
  }
</style>
