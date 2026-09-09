<script lang="ts">
  // Одно число тела на плашке.
  //
  // Правится на месте, а не в поле рядом: плашка показывает то же самое, что
  // напечатано на карте, и второе поле для того же числа — это второе место,
  // где оно живёт. Поле здесь без рамки: рамка внутри тёмной плашки нарисовала
  // бы девять маленьких коробок и вернула бы форму, от которой уходили.
  //
  // Но «без рамки» стоило ровно того, чего и должно было: девять чисел стали
  // похожи на надпись, и что их можно набрать, было видно только тому, кто
  // попробовал. Поэтому под числом пунктир — тот же, которым в доме подчёркнуто
  // всё, на что нажимают, — а на наведении ячейка светлеет. Пунктир стоит
  // ВСЕГДА, а не по наведению: подсказка, которую надо сперва найти мышью, не
  // подсказка.
  //
  // И подпись словом. Значка мало: сердце угадывают все, а «оберег» от «брони»
  // по двум щитам не отличит никто, включая того, кто их рисовал.
  import BattleIcon from '$lib/components/BattleIcon.svelte';

  let {
    icon,
    label,
    value = $bindable(0),
    min = 0,
    max = 999,
    step = 1,
    readonly = false,
    tone = 'plain',
    fault = false,
    anchor,
  } = $props<{
    icon: string;
    label: string;
    value?: number;
    min?: number;
    max?: number;
    step?: number;
    readonly?: boolean;
    /** `quiet` — число, которое карта носит, но правила пока не читают. */
    tone?: 'plain' | 'quiet';
    /** Сюда показал отказ разбора годности. */
    fault?: boolean;
    anchor?: string;
  }>();

  /** Целое число набирают заново чаще, чем подкручивают на единицу. */
  function selectOnFocus(e: FocusEvent & { currentTarget: HTMLInputElement }) {
    e.currentTarget.select();
  }

  /** Наведённое `type="number"` крутит своё значение обычным колесом прокрутки
   *  в Chrome и Firefox. Плашка стоит посреди длинного листа, и прокрутка мимо
   *  неё молча меняла бы здоровье: снимаем наводку и отдаём колесо странице. */
  function blurOnWheel(e: WheelEvent & { currentTarget: HTMLInputElement }) {
    e.currentTarget.blur();
  }
</script>

<label
  class="cell"
  class:cell--quiet={tone === 'quiet'}
  class:cell--fixed={readonly}
  class:cell--fault={fault}
  id={anchor}
>
  <span class="top">
    <span class="glyph"><BattleIcon name={icon} size={12} weight={1.2} /></span>
    <span class="cap">{label}</span>
  </span>
  {#if readonly}
    <span class="val val--fixed">{value}</span>
  {:else}
    <input
      class="val"
      type="number"
      {min}
      {max}
      {step}
      bind:value
      onfocus={selectOnFocus}
      onwheel={blurOnWheel}
    />
  {/if}
</label>

<style>
  .cell {
    /* Растёт сама: `--min` приходит с плашки наследованием, а не пропсом —
       ширина ячейки это свойство ряда, в котором она стоит, и знать его
       должна плашка, а не каждая из девяти. */
    flex: 1 1 var(--min, 5.2rem);
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.5rem 0.55rem 0.45rem;
    background: #34251c;
    cursor: text;
    transition: background 120ms ease;
  }
  .cell:hover,
  .cell:focus-within {
    background: #3d2c21;
  }
  .cell--fixed {
    cursor: default;
  }

  /* Одна ячейка из девяти. Обводка внутрь, а не рамка: рамка снаружи сдвинула
     бы соседей на пиксель, и вся плашка поехала бы от одного отказа. */
  .cell--fault {
    background: #4a201a;
    box-shadow: inset 0 0 0 1px #8f2f22;
  }
  .cell--fault:hover,
  .cell--fault:focus-within {
    background: #55251d;
  }

  .top {
    display: flex;
    align-items: flex-start;
    gap: 0.3rem;
    min-width: 0;
  }

  .glyph {
    display: flex;
    flex-shrink: 0;
    color: #d8c6b1;
  }
  /* Подпись переносится, а не обрезается троеточием. Обрезанная подпись — это
     ровно та беда, от которой подпись и заводили: «звеньев / клет…» не лучше
     значка без слова. Переносится она только там, где не влезла: у коротких
     слов тела строка остаётся одна, а сетка всё равно выравнивает ячейки по
     самой высокой. */
  .cap {
    font-size: 8px;
    letter-spacing: 0.11em;
    text-transform: uppercase;
    line-height: 1.2;
    overflow-wrap: break-word;
    color: rgba(216, 198, 177, 0.72);
  }

  .val {
    width: 100%;
    padding: 0 0 0.1rem;
    background: none;
    border: 0;
    border-bottom: 1px dashed rgba(248, 241, 231, 0.3);
    outline: none;
    font-size: 15px;
    line-height: 1.1;
    font-variant-numeric: tabular-nums;
    color: #f8f1e7;
    text-align: left;
  }
  .val:focus {
    border-bottom: 1px solid #c65f3c;
  }
  .val--fixed {
    border-bottom-color: transparent;
  }

  /* Мана печатается на лице карты и не играет. Приглушена, а не убрана:
     число, которое ни на что не влияет, признаётся в этом там, где его
     набирают. */
  .cell--quiet .glyph,
  .cell--quiet .cap {
    color: rgba(216, 198, 177, 0.45);
  }
  .cell--quiet .val {
    color: rgba(248, 241, 231, 0.62);
  }

  /* Прибавка браузера. Стрелки внутри тёмной ячейки — три пикселя серого,
     которые нельзя ни попасть мышью, ни прочесть. */
  .val::-webkit-outer-spin-button,
  .val::-webkit-inner-spin-button {
    appearance: none;
    margin: 0;
  }
  .val[type='number'] {
    -moz-appearance: textfield;
    appearance: textfield;
  }
</style>
