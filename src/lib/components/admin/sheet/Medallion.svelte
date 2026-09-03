<script lang="ts">
  // Медальон: один выбор из короткого перечня, показанный лицом.
  //
  // Заменяет собой выпадающий список там, и только там, где перечень КОРОТОК и
  // ПОСТОЯНЕН. Выпадающий список честен, пока вариантов много и они меняются;
  // когда их четыре и они не менялись ни разу, он прячет весь выбор под одно
  // слово и заставляет открыть себя, чтобы вспомнить, из чего вообще выбирают.
  // Медальоны показывают все варианты сразу и стоят ровно столько места,
  // сколько занимают, — поэтому двадцать два глагола умений остались списком.
  //
  // Круг тёмный у выбранного и светлый у прочих: разница по СВЕТЛОТЕ видна и
  // боковым зрением, и на выцветшем экране, а разница по цвету каймы — нет.
  import type { Snippet } from 'svelte';
  import BattleIcon from '../BattleIcon.svelte';

  let {
    icon,
    caption,
    note,
    selected = false,
    disabled = false,
    size = 44,
    title,
    onclick,
    children,
  } = $props<{
    /** Имя глифа из `BattleIcon`. Не нужно, если рисунок передан вручную. */
    icon?: string;
    caption?: string;
    /** Вторая строка под подписью — чем этот выбор отличается от соседнего. */
    note?: string;
    selected?: boolean;
    disabled?: boolean;
    size?: number;
    title?: string;
    /** Без него медальон — не кнопка, а просто изображение выбора. */
    onclick?: () => void;
    children?: Snippet;
  }>();
</script>

<svelte:element
  this={onclick ? 'button' : 'div'}
  type={onclick ? 'button' : undefined}
  role={onclick ? 'button' : undefined}
  class="med"
  class:med--on={selected}
  class:med--off={!selected}
  class:med--dead={disabled}
  disabled={onclick ? disabled : undefined}
  aria-pressed={onclick ? selected : undefined}
  {title}
  onclick={disabled ? undefined : onclick}
>
  <span class="disc" style="--d:{size}px">
    {#if children}
      {@render children()}
    {:else if icon}
      <BattleIcon name={icon} size={Math.round(size * 0.42)} weight={1.05} />
    {/if}
  </span>
  {#if caption}
    <span class="cap">{caption}</span>
  {/if}
  {#if note}
    <span class="note">{note}</span>
  {/if}
</svelte:element>

<style>
  .med {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.3rem;
    max-width: 7.5rem;
    padding: 0;
    background: none;
    border: 0;
    text-align: center;
    color: inherit;
  }
  .med:not(:disabled) {
    cursor: pointer;
  }

  .disc {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--d);
    height: var(--d);
    border-radius: 50%;
    transition:
      background 140ms ease,
      box-shadow 140ms ease,
      color 140ms ease;
  }

  /* Двойная кайма — тень в три слоя, а не две вложенные коробки: обод должен
     лежать СНАРУЖИ круга, иначе он съедает место у рисунка внутри. */
  .med--off .disc {
    background: #efe4d4;
    color: #8a6a55;
    box-shadow:
      0 0 0 1px rgba(52, 37, 28, 0.18),
      0 0 0 3px #f8f1e7,
      0 0 0 4px rgba(52, 37, 28, 0.1);
  }
  .med--on .disc {
    background: #34251c;
    color: #f8f1e7;
    box-shadow:
      0 0 0 1px rgba(52, 37, 28, 0.5),
      0 0 0 3px #f8f1e7,
      0 0 0 4px #c65f3c;
  }
  .med:not(:disabled):hover .disc {
    box-shadow:
      0 0 0 1px rgba(52, 37, 28, 0.4),
      0 0 0 3px #f8f1e7,
      0 0 0 4px rgba(111, 59, 36, 0.55);
  }
  .med:focus-visible {
    outline: none;
  }
  .med:focus-visible .disc {
    box-shadow:
      0 0 0 1px rgba(52, 37, 28, 0.4),
      0 0 0 3px #f8f1e7,
      0 0 0 4px #c65f3c;
  }

  .cap {
    font-size: 9px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    line-height: 1.35;
    color: #5f4636;
  }
  .med--on .cap {
    color: #34251c;
  }
  .note {
    font-size: 10px;
    line-height: 1.3;
    color: #8a6a55;
  }

  /* Недоступный не прячется: карта его вид хранит и ждёт движка, и форма
     обязана показывать, что такой выбор есть, — но не обещать, что он играет. */
  .med--dead {
    opacity: 0.45;
  }
</style>
