<script lang="ts">
  /**
   * Листок земли: что лежит на клетке и что это даёт тому, кто на ней стоит.
   *
   * Открывает его знак в углу клетки (`BattleScene`, `.ground-key`). Знак
   * говорит только род — запрет, опасность, подмога; число и границы правила
   * («издали», «в начале своего хода», «один раз за ход») в знак не помещаются,
   * а без них правило узнаётся, только заплатив за него ходом. Поэтому листок
   * называет землю, род и правило целыми фразами.
   *
   * Стоит поверх страницы (`portal`), а не в клетке: клетка — `<button>`, а
   * доска режет всё, что вышло за сукно, и листок у крайней колонки был бы
   * срезан. Место считается от знака и прижимается к окну.
   */
  import { onMount, tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import { portal } from '$lib/actions/portal';
  import { t } from '$lib/i18n';
  import {
    GROUND_DETAIL,
    GROUND_KIND,
    GROUND_KIND_KEY,
    GROUND_NAME,
    GROUND_SHORT,
  } from '$lib/battles';
  import BattleGroundMark from './BattleGroundMark.svelte';
  import BattleGroundSigil from './BattleGroundSigil.svelte';
  import type { BattleGround } from '$lib/types/api';

  let {
    ground,
    anchor,
    onclose,
  }: {
    ground: BattleGround;
    /** Прямоугольник знака в окне (`getBoundingClientRect`). */
    anchor: { left: number; top: number; width: number; height: number };
    onclose: () => void;
  } = $props();

  let el = $state<HTMLElement | null>(null);
  let left = $state(0);
  let top = $state(0);
  let placed = $state(false);

  const GAP = 8;
  const EDGE = 10;

  /** Под знаком, а если не помещается — над ним; по горизонтали — от знака,
   *  но не за край окна. Меряется после отрисовки: высота листка зависит от
   *  длины правила на языке читателя. */
  async function place() {
    await tick();
    if (!el) return;
    const box = el.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    let x = anchor.left + anchor.width / 2 - box.width / 2;
    x = Math.min(Math.max(EDGE, x), vw - box.width - EDGE);
    let y = anchor.top + anchor.height + GAP;
    if (y + box.height > vh - EDGE) y = anchor.top - GAP - box.height;
    y = Math.min(Math.max(EDGE, y), vh - box.height - EDGE);
    left = x;
    top = y;
    placed = true;
  }

  onMount(() => {
    place();
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      // Листок лежит поверх доски: Escape при нём открытом — про листок, а не
      // про выбранное под ним тело.
      e.stopImmediatePropagation();
      onclose();
    };
    const onDown = (e: PointerEvent) => {
      const target = e.target as HTMLElement | null;
      if (el?.contains(target)) return;
      // Знак сам решает, закрыть ли листок (повторное нажатие закрывает, а на
      // другой знак переезжает), — иначе закрытие здесь и открытие там
      // подрались бы за одно и то же нажатие.
      if (target?.closest?.('.ground-key')) return;
      onclose();
    };
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('pointerdown', onDown, true);
    window.addEventListener('resize', onclose);
    return () => {
      window.removeEventListener('keydown', onKey, true);
      window.removeEventListener('pointerdown', onDown, true);
      window.removeEventListener('resize', onclose);
    };
  });

  $effect(() => {
    void ground;
    void anchor;
    placed = false;
    place();
  });

  const kind = $derived(GROUND_KIND[ground]);
</script>

<div
  class="note note--{kind}"
  class:note--placed={placed}
  role="dialog"
  aria-label={$t(GROUND_NAME[ground])}
  bind:this={el}
  use:portal
  style="left:{left}px; top:{top}px"
  transition:fade={{ duration: 120 }}
>
  <div class="head">
    <span class="plate" aria-hidden="true"><BattleGroundMark {ground} /></span>
    <div class="title">
      <p class="kind"><BattleGroundSigil {ground} size="1em" /> {$t(GROUND_KIND_KEY[kind])}</p>
      <h3 class="name">{$t(GROUND_NAME[ground])}</h3>
      <p class="short">{$t(GROUND_SHORT[ground])}</p>
    </div>
    <button type="button" class="close" aria-label={$t('battleGroundClose')} onclick={onclose}>
      <svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true">
        <path d="M2 2 L10 10 M10 2 L2 10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
      </svg>
    </button>
  </div>
  <p class="detail">{$t(GROUND_DETAIL[ground])}</p>
</div>

<style>
  .note {
    position: fixed;
    z-index: 2000;
    box-sizing: border-box;
    width: min(20rem, calc(100vw - 20px));
    padding: 0.7rem 0.8rem 0.8rem;
    color: #34251c;
    background: #f8f1e7;
    border: 1px solid #d8c6b1;
    /* Двойная кромка и поворот на градус — оправа дома. */
    outline: 1px solid rgba(111, 59, 36, 0.35);
    outline-offset: -5px;
    border-radius: 3px;
    box-shadow: 0 10px 28px rgba(30, 18, 10, 0.38);
    font-family: Georgia, 'Fraunces', serif;
    transform: rotate(1deg);
    opacity: 0;
  }

  .note--placed {
    opacity: 1;
  }

  .head {
    display: flex;
    gap: 0.7rem;
    align-items: flex-start;
  }

  /* Рисунок земли в обрамлении: та же плитка и тот же ход, что на поле. */
  .plate {
    position: relative;
    flex: none;
    width: 3.6rem;
    aspect-ratio: 3 / 4;
    background: #2b211c;
    border: 1px solid #6f3b24;
    border-radius: 2px;
    overflow: hidden;
  }

  /* Знак рода стоит рядом с названием; второй, в углу плитки, был бы дублем. */
  .plate :global(.corner) {
    display: none;
  }

  .title {
    min-width: 0;
    flex: 1;
  }

  .kind {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    margin: 0;
    font-size: 0.64rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .note--hazard .kind {
    color: #9c3e26;
  }

  .name {
    margin: 0.1rem 0 0;
    font-size: 1.3rem;
    font-weight: 600;
    line-height: 1.1;
    text-transform: capitalize;
  }

  .short {
    margin: 0.2rem 0 0;
    font-size: 0.8rem;
    font-style: italic;
    color: #c65f3c;
  }

  .detail {
    margin: 0.65rem 0 0;
    padding-top: 0.6rem;
    border-top: 1px solid #d8c6b1;
    font-size: 0.86rem;
    line-height: 1.45;
  }

  .close {
    flex: none;
    display: grid;
    place-items: center;
    width: 1.5rem;
    height: 1.5rem;
    margin: -0.2rem -0.3rem 0 0;
    color: #6f3b24;
    background: transparent;
    border: 0;
    border-radius: 50%;
    cursor: pointer;
  }

  .close:hover,
  .close:focus-visible {
    background: rgba(111, 59, 36, 0.12);
  }
</style>
