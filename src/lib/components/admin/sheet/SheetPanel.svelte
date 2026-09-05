<script lang="ts">
  // Панель листа: рамка с угловыми скобками и подписью, сидящей НА линейке.
  //
  // Стол правки карты был колонкой в двадцать пять блоков подряд, разделённых
  // пунктиром: чтобы понять, где кончается тело и начинаются умения, надо было
  // прочесть обе подписи. Панель отвечает на этот вопрос до чтения — у названной
  // области есть край, и глаз находит её, не разбирая слов.
  //
  // Подпись стоит на верхней линейке, а не над ней: имя, оторванное от рамки,
  // читается как заголовок СЛЕДУЮЩЕГО куска и одинаково относится и к панели,
  // и к пустоте перед ней. Бумага под подписью берётся переменной, а не
  // прибита к пергаменту дома: панель ставят и на затенённую подложку, и
  // разрыв линейки тогда пришлось бы замазывать вторым цветом руками.
  import type { Snippet } from 'svelte';
  import { t } from '$lib/i18n';

  let {
    title,
    lead,
    note,
    corners = true,
    wide = false,
    fault = false,
    anchor,
    tone = 'plain',
    pad = 'normal',
    aside,
    children,
  } = $props<{
    /** Имя области. Без него панель — просто рамка, и это законно. */
    title?: string;
    /** Короткое пояснение в той же строке, через тире. Не переносится. */
    lead?: string;
    /** Прозой, внутри панели. Всё, что длиннее строки, — сюда. */
    note?: string;
    corners?: boolean;
    /** Во всю ширину листа. Панель — клетка сетки, а не строка колонки. */
    wide?: boolean;
    /** В этой области лежит то, на что показал отказ сервера. Панель красится
     *  целиком, чтобы её было видно с середины листа, не читая полей. */
    fault?: boolean;
    anchor?: string;
    tone?: 'plain' | 'sunk';
    pad?: 'normal' | 'tight' | 'none';
    /** Кнопки в правом верхнем углу — то, что делают С областью целиком. */
    aside?: Snippet;
    children: Snippet;
  }>();
</script>

<section
  class="panel"
  class:panel--sunk={tone === 'sunk'}
  class:panel--wide={wide}
  class:panel--fault={fault}
  class:has-title={!!title}
  id={anchor}
>
  {#if corners}
    <span class="brk brk--tl" aria-hidden="true"></span>
    <span class="brk brk--tr" aria-hidden="true"></span>
    <span class="brk brk--bl" aria-hidden="true"></span>
    <span class="brk brk--br" aria-hidden="true"></span>
  {/if}

  {#if title}
    <span class="legend">
      <span class="legend__name">{title}</span>
      {#if lead}<span class="legend__lead">— {lead}</span>{/if}
    </span>
  {/if}

  {#if aside}
    <span class="aside">{@render aside()}</span>
  {/if}

  <div class="body" class:body--tight={pad === 'tight'} class:body--bare={pad === 'none'}>
    {#if note}
      <details class="note-fold">
        <summary>{$t('adminBattlesHintOpen')}</summary>
        <p class="note">{note}</p>
      </details>
    {/if}
    {@render children()}
  </div>
</section>

<style>
  .panel {
    --paper: #f8f1e7;
    position: relative;
    border: 1px solid rgba(52, 37, 28, 0.16);
  }

  /* Клетка на всю строку. Живёт здесь, а не обёрткой снаружи: обёртка вокруг
     каждой панели — это ряд пустых `div`, единственное дело которых сказать,
     какой ширины панель, о чём панель и так знает. */
  .panel--wide {
    grid-column: 1 / -1;
  }

  /* Затенённая: тем, что стоит ВНУТРИ другой панели — иначе две рамки
     одного тона читаются как одна с лишней линией посередине. */
  .panel--sunk {
    --paper: #f3eadd;
    background: rgba(52, 37, 28, 0.028);
    border-color: rgba(52, 37, 28, 0.1);
  }

  /* Тревога. Красится край и скобки, а не подложка: залитая цветом панель
     перекрикивает поле, у которого отказ на самом деле и лежит. */
  .panel--fault {
    border-color: rgba(143, 47, 34, 0.5);
  }
  .panel--fault .brk {
    border-color: rgba(143, 47, 34, 0.6);
  }
  .panel--fault .legend__name {
    color: #8f2f22;
  }

  /* Скобки. Рисуются углом двух линеек, а не картинкой: на любом размере
     панели они одинаковы, и красятся теми же чернилами, что рамка. */
  .brk {
    position: absolute;
    width: 9px;
    height: 9px;
    border: 1px solid rgba(52, 37, 28, 0.32);
    pointer-events: none;
  }
  .brk--tl {
    top: -3px;
    left: -3px;
    border-right: 0;
    border-bottom: 0;
  }
  .brk--tr {
    top: -3px;
    right: -3px;
    border-left: 0;
    border-bottom: 0;
  }
  .brk--bl {
    bottom: -3px;
    left: -3px;
    border-right: 0;
    border-top: 0;
  }
  .brk--br {
    bottom: -3px;
    right: -3px;
    border-left: 0;
    border-top: 0;
  }

  .legend {
    position: absolute;
    top: 0;
    left: 0.9rem;
    max-width: calc(100% - 2.4rem);
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    padding: 0 0.5rem;
    transform: translateY(-50%);
    background: var(--paper);
    white-space: nowrap;
  }

  .legend__name {
    font-size: 10px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  /* Пояснение уступает имени и обрезается первым: имя области должно быть
     видно целиком на любой ширине колонки, пояснение — нет. */
  .legend__lead {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 10px;
    letter-spacing: 0.04em;
    color: #8a6a55;
  }

  .aside {
    position: absolute;
    top: 0;
    right: 0.9rem;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0 0.45rem;
    transform: translateY(-50%);
    background: var(--paper);
  }

  .body {
    padding: 0.9rem 0.85rem 0.85rem;
  }
  .has-title .body {
    padding-top: 1rem;
  }
  .body--tight {
    padding: 0.6rem 0.6rem 0.55rem;
  }
  .has-title .body--tight {
    padding-top: 0.8rem;
  }
  .body--bare {
    padding: 0;
  }

  .note-fold {
    margin-bottom: 0.7rem;
  }
  .note-fold summary {
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
    cursor: pointer;
  }
  .note {
    margin-top: 0.5rem;
    font-size: 11px;
    line-height: 1.5;
    font-style: italic;
    color: #8a6a55;
  }
</style>
