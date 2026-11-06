<script lang="ts" module>
  export type MenuItem = {
    key: 'heal' | 'field' | 'unfield' | 'swap' | 'details';
    label: string;
    /** Вторая строка: что получится или почему нельзя. */
    note?: string;
    /** Цена в пыли: показывается значком справа. */
    price?: number;
    disabled?: boolean;
    onclick: () => void;
  };
  export type MenuAt = {
    left: number;
    top?: number;
    bottom?: number;
    /** Где у карты середина, считая от левого края меню: туда смотрит указатель. */
    arrow: number;
    /** Меню над картой (указатель снизу) или под ней (указатель сверху). */
    above: boolean;
  };
</script>

<script lang="ts">
  // Меню быстрых действий над картой отряда: то, что можно сделать с ней сейчас.
  // Лечение и замена — крупные кнопки, а если действие недоступно, строка под ним
  // говорит почему, а не прячет его. Указатель показывает, чьё это меню.
  import { t } from '$lib/i18n';

  let {
    title,
    art,
    health,
    items,
    at,
    onclose,
  }: {
    title: string;
    art?: string | null;
    /** Здоровье карты: полоса и число в заголовке. Нет — карта пала. */
    health?: { h: number; m: number } | null;
    items: MenuItem[];
    at: MenuAt;
    onclose: () => void;
  } = $props();

  const pct = $derived(health ? Math.max(0, Math.min(100, Math.round((health.h / Math.max(health.m, 1)) * 100))) : 0);

  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }
  function focusFirst(node: HTMLElement) {
    node.querySelector<HTMLButtonElement>('button.cm-item:not(:disabled)')?.focus();
  }
  function onpointerdown(e: PointerEvent) {
    if (!(e.target as Element | null)?.closest?.('.cm')) onclose();
  }
  const run = (item: MenuItem) => {
    if (item.disabled) return;
    onclose();
    item.onclick();
  };
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} {onpointerdown} onscroll={onclose} onresize={onclose} />

<div
  use:portal
  use:focusFirst
  class="cm"
  class:cm--above={at.above}
  role="menu"
  aria-label={title}
  style:left="{at.left}px"
  style:top={at.top !== undefined ? `${at.top}px` : undefined}
  style:bottom={at.bottom !== undefined ? `${at.bottom}px` : undefined}
  style:--arrow="{at.arrow}px"
>
  <span class="cm-pointer" aria-hidden="true"></span>

  <header class="cm-head">
    {#if art}<img class="cm-art" src={art} alt="" draggable="false" />{/if}
    <div class="cm-who">
      <b>{title}</b>
      {#if health}
        <span class="cm-hp">
          <span class="cm-bar" aria-hidden="true"><i style:width="{pct}%" class:low={pct <= 34} class:mid={pct > 34 && pct <= 66}></i></span>
          <span class="cm-num" class:low={pct <= 34} class:mid={pct > 34 && pct <= 66}>
            <svg viewBox="0 0 24 24" width="12" height="12" aria-hidden="true"><path d="M12 21s-7.5-4.6-9.6-9.2C.9 8.4 3 5 6.4 5c2 0 3.6 1.1 4.6 2.7h2C14 6.1 15.6 5 17.6 5 21 5 23.1 8.4 21.6 11.8 19.5 16.4 12 21 12 21z" fill="currentColor" /></svg>
            <b>{health.h}</b>/{health.m}
          </span>
        </span>
      {:else}
        <span class="cm-fallen">{$t('runFallen')}</span>
      {/if}
    </div>
    <button type="button" class="cm-x" aria-label={$t('runClose')} onclick={onclose}>×</button>
  </header>

  <ul>
    {#each items as item (item.key)}
      <li>
        <button
          type="button"
          role="menuitem"
          class="cm-item cm-item--{item.key}"
          disabled={item.disabled}
          onclick={() => run(item)}
        >
          <span class="cm-ico" aria-hidden="true">
            {#if item.key === 'heal'}
              <svg viewBox="0 0 24 24" width="22" height="22"><path d="M12 21s-7.5-4.6-9.6-9.2C.9 8.4 3 5 6.4 5c2 0 3.6 1.1 4.6 2.7h2C14 6.1 15.6 5 17.6 5 21 5 23.1 8.4 21.6 11.8 19.5 16.4 12 21 12 21z" fill="currentColor" /></svg>
            {:else if item.key === 'swap'}
              <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 8h14l-3-3M20 16H6l3 3" /></svg>
            {:else if item.key === 'unfield'}
              <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 6l12 12M18 6L6 18" /></svg>
            {:else if item.key === 'field'}
              <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="4" width="16" height="16" rx="1" /><path d="M9 12h6M12 9v6" /></svg>
            {:else}
              <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="8" /><path d="M12 11v5M12 8v.5" /></svg>
            {/if}
          </span>
          <span class="cm-text">
            <span class="cm-label">{item.label}</span>
            {#if item.note}<span class="cm-note">{item.note}</span>{/if}
          </span>
          {#if item.price != null && item.price > 0}
            <span class="cm-price" aria-hidden="true">
              <svg viewBox="0 0 24 24" width="14" height="14"><circle cx="12" cy="12" r="9" fill="currentColor" opacity="0.25" /><circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2" /><path d="M12 7l1.6 3.4 3.4.4-2.5 2.4.7 3.6-3.2-1.8-3.2 1.8.7-3.6L7 10.8l3.4-.4z" fill="currentColor" /></svg>
              {item.price}
            </span>
          {/if}
        </button>
      </li>
    {/each}
  </ul>
</div>

<style>
  .cm {
    --arrow: 50%;
    position: fixed;
    z-index: 75;
    box-sizing: border-box;
    width: min(18rem, calc(100vw - 16px));
    padding: 0.7rem 0.7rem 0.75rem;
    border: 1px solid #c9b49a;
    border-radius: 6px;
    background:
      radial-gradient(120% 80% at 0% 0%, rgba(255, 255, 255, 0.75), transparent 60%),
      linear-gradient(#fcf7ee, #f4e9d8);
    color: #34251c;
    box-shadow:
      inset 0 0 0 3px #fcf7ee,
      inset 0 0 0 4px #e1d0b8,
      0 18px 40px rgba(52, 37, 28, 0.45),
      0 2px 6px rgba(52, 37, 28, 0.25);
    font-family: Inter, system-ui, sans-serif;
    transform-origin: var(--arrow) 100%;
    animation: cm-in 0.16s cubic-bezier(0.2, 0.9, 0.3, 1.2);
  }
  .cm:not(.cm--above) {
    transform-origin: var(--arrow) 0%;
  }

  /* Указатель на карту, у которой открыто меню. */
  .cm-pointer {
    position: absolute;
    left: calc(var(--arrow) - 9px);
    width: 18px;
    height: 18px;
    border: 1px solid #c9b49a;
    background: #f4e9d8;
    transform: rotate(45deg);
  }
  .cm--above .cm-pointer {
    bottom: -10px;
    border-top: 0;
    border-left: 0;
  }
  .cm:not(.cm--above) .cm-pointer {
    top: -10px;
    border-right: 0;
    border-bottom: 0;
    background: #fcf7ee;
  }

  .cm-head {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.15rem 1.8rem 0.65rem 0.15rem;
    margin-bottom: 0.55rem;
    border-bottom: 1px solid #e1d0b8;
  }
  .cm-art {
    flex: none;
    width: 2.7rem;
    height: 3.6rem;
    object-fit: cover;
    border: 2px solid #b08a3a;
    border-radius: 3px;
    box-shadow: 0 2px 6px rgba(52, 37, 28, 0.35);
  }
  .cm-who {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    min-width: 0;
  }
  .cm-who > b {
    font-family: Fraunces, Georgia, serif;
    font-size: 1.12rem;
    line-height: 1.15;
  }
  .cm-hp {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .cm-bar {
    width: 4.2rem;
    height: 7px;
    overflow: hidden;
    border-radius: 999px;
    background: #e8dccb;
    box-shadow: inset 0 1px 2px rgba(52, 37, 28, 0.3);
  }
  .cm-bar i {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: linear-gradient(#7aa561, #4a6b3a);
  }
  .cm-bar i.mid {
    background: linear-gradient(#dbb85f, #b08a3a);
  }
  .cm-bar i.low {
    background: linear-gradient(#c75a49, #8f2f22);
  }
  .cm-num {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    color: #4a6b3a;
    font-size: 0.82rem;
  }
  .cm-num.mid {
    color: #8a6a22;
  }
  .cm-num.low {
    color: #8f2f22;
  }
  .cm-num b {
    font-family: Fraunces, Georgia, serif;
    font-size: 1.05em;
  }
  .cm-fallen {
    color: #8f2f22;
    font-style: italic;
    font-size: 0.85rem;
  }
  .cm-x {
    position: absolute;
    top: -0.15rem;
    right: -0.2rem;
    width: 1.9rem;
    height: 1.9rem;
    border: 0;
    border-radius: 50%;
    background: none;
    color: #6f3b24;
    font-size: 1.3rem;
    cursor: pointer;
  }
  .cm-x:hover {
    background: rgba(111, 59, 36, 0.1);
  }

  ul {
    display: grid;
    gap: 0.4rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .cm-item {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    width: 100%;
    min-height: 3rem;
    padding: 0.45rem 0.75rem;
    border: 1px solid #d8c6b1;
    border-radius: 5px;
    background: #fbf5ea;
    color: #34251c;
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: transform 120ms ease, box-shadow 120ms ease, background 120ms ease;
  }
  .cm-item:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 3px 8px rgba(52, 37, 28, 0.18);
  }
  .cm-item:active:not(:disabled) {
    transform: translateY(0);
  }
  .cm-item:focus-visible {
    outline: 2px solid #c65f3c;
    outline-offset: 2px;
  }
  .cm-item:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .cm-ico {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
    background: rgba(111, 59, 36, 0.1);
    color: #6f3b24;
  }
  .cm-text {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }
  .cm-label {
    font-weight: 600;
    font-size: 0.95rem;
  }
  .cm-note {
    font-size: 0.76rem;
    line-height: 1.25;
    color: #6f3b24;
  }

  /* Лечение — главная кнопка: зелёная, как полоса здоровья. */
  .cm-item--heal {
    min-height: 3.4rem;
    border-color: #2f4a23;
    background: linear-gradient(#78a55c, #46663a);
    color: #f7fbf2;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.35), 0 3px 10px rgba(74, 107, 58, 0.4);
  }
  .cm-item--heal:hover:not(:disabled) {
    background: linear-gradient(#82b066, #4b6e3e);
  }
  .cm-item--heal .cm-ico {
    background: rgba(255, 255, 255, 0.22);
    color: #fff;
  }
  .cm-item--heal .cm-note {
    color: #e6f1da;
  }
  .cm-item--heal .cm-label {
    font-size: 1.02rem;
  }

  /* Убрать с поля — красным: действие, которое снимает карту с выхода. */
  .cm-item--unfield {
    border-color: #c98b82;
    background: #fbefec;
    color: #8f2f22;
  }
  .cm-item--unfield .cm-ico {
    background: rgba(143, 47, 34, 0.12);
    color: #8f2f22;
  }
  .cm-item--field {
    border-color: #c65f3c;
  }
  .cm-item--field .cm-ico {
    background: rgba(198, 95, 60, 0.15);
    color: #c65f3c;
  }

  /* Замена — золотой рамкой, цена значком справа. */
  .cm-item--swap {
    border-color: #b08a3a;
    background: #fbf3df;
  }
  .cm-item--swap .cm-ico {
    background: rgba(176, 138, 58, 0.18);
    color: #8a6a22;
  }
  .cm-price {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 0.25rem;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    background: #b08a3a;
    color: #fff8e8;
    font-weight: 700;
    font-size: 0.85rem;
  }

  /* Подробности — тише остальных. */
  .cm-item--details {
    min-height: 2.5rem;
    border-color: transparent;
    background: transparent;
  }
  .cm-item--details:hover:not(:disabled) {
    background: rgba(111, 59, 36, 0.07);
    box-shadow: none;
  }
  .cm-item--details .cm-ico {
    width: 1.6rem;
    height: 1.6rem;
    background: none;
  }

  @keyframes cm-in {
    from {
      opacity: 0;
      transform: scale(0.92) translateY(6px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .cm {
      animation: none;
    }
    .cm-item {
      transition: none;
    }
  }
</style>
