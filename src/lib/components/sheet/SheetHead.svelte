<script lang="ts">
  // Шапка листа: чьё это описание, и чем оно правится.
  //
  // Имя здесь не правится — оно правится на самой карте, как и всё, что на
  // карте напечатано. Шапка его ПОВТОРЯЕТ, потому что лист длинный: доехав до
  // умений, хранитель уже не видит ни карты, ни того, чью карту он правит.
  //
  // Линейка с ромбом отделяет шапку от панелей. Ромб — не украшение: без него
  // линейка неотличима от края первой панели и читается её верхней стороной.
  import type { Snippet } from 'svelte';

  let {
    title,
    lead,
    tag,
    rule = true,
    children,
  } = $props<{
    title: string;
    /** Прозой, под именем. Две строки — больше шапка не держит. */
    lead?: string;
    /** Одно слово справа от имени: состояние, чин, — то, что имя уточняет. */
    tag?: string;
    rule?: boolean;
    /** Управление, относящееся ко всему листу: язык правки, рубашка. */
    children?: Snippet;
  }>();
</script>

<header class="head">
  <div class="row">
    <div class="who">
      <h2 class="name">
        {title}
        {#if tag}<span class="tag">{tag}</span>{/if}
      </h2>
      {#if lead}
        <p class="lead">{lead}</p>
      {/if}
    </div>
    {#if children}
      <div class="tools">{@render children()}</div>
    {/if}
  </div>
  {#if rule}
    <div class="rule" aria-hidden="true"></div>
  {/if}
</header>

<style>
  .head {
    margin-bottom: 1.5rem;
  }

  .who {
    min-width: 0;
  }

  .row {
    display: flex;
    align-items: flex-start;
    gap: 1.5rem;
  }

  .name {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(1.5rem, 2.6vw, 2.1rem);
    line-height: 1.1;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .tag {
    margin-left: 0.6rem;
    font-family: Inter, system-ui, sans-serif;
    font-size: 9px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    vertical-align: middle;
    color: #8a6a55;
  }

  .lead {
    max-width: 62ch;
    margin-top: 0.35rem;
    font-size: 11px;
    line-height: 1.6;
    color: #8a6a55;
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-left: auto;
    flex-shrink: 0;
  }

  .rule {
    position: relative;
    height: 1px;
    margin-top: 0.9rem;
    background: rgba(52, 37, 28, 0.16);
  }
  .rule::after {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    width: 6px;
    height: 6px;
    background: #f8f1e7;
    border: 1px solid rgba(52, 37, 28, 0.3);
    transform: translate(-50%, -50%) rotate(45deg);
  }
</style>
